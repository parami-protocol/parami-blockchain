#![cfg_attr(not(feature = "std"), no_std)]

pub use ocw::images;
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod did;

mod ocw;

mod types;

use codec::Encode;
use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    ensure,
};
use frame_system::offchain::CreateSignedTransaction;
use scale_info::TypeInfo;
use sp_core::crypto::KeyTypeId;
use sp_runtime::traits::{MaybeSerializeDeserialize, Member};
use sp_std::prelude::*;

const OFFCHAIN_KEY_TYPE: KeyTypeId = KeyTypeId(*b"lnk!");

macro_rules! is_stask {
    ($profile:expr, $prefix:expr) => {
        $profile.starts_with($prefix) && $profile.last() != Some(&b'/')
    };
}

pub(crate) use is_stask;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The DID type
        type DecentralizedId: Parameter
            + Member
            + MaybeSerializeDeserialize
            + Ord
            + Default
            + Copy
            + sp_std::hash::Hash
            + AsRef<[u8]>
            + AsMut<[u8]>
            + MaxEncodedLen
            + TypeInfo;

        /// Lifetime of a pending account
        #[pallet::constant]
        type PendingLifetime: Get<Self::BlockNumber>;

        /// Unsigned Call Priority
        #[pallet::constant]
        type UnsignedPriority: Get<TransactionPriority>;

        /// The origin which may do calls
        type CallOrigin: EnsureOrigin<
            Self::Origin,
            Success = (Self::DecentralizedId, Self::AccountId),
        >;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Accounts pending to be checked with the offchain worker
    #[pallet::storage]
    #[pallet::getter(fn pendings_of)]
    pub(super) type PendingOf<T: Config> = StorageDoubleMap<
        _,
        Identity,
        T::DecentralizedId,
        Twox64Concat,
        types::AccountType,
        types::Pending<T::BlockNumber>,
    >;

    /// Linked accounts of a DID
    #[pallet::storage]
    #[pallet::getter(fn links_of)]
    pub(super) type LinksOf<T: Config> = StorageDoubleMap<
        _,
        Identity,
        T::DecentralizedId,
        Twox64Concat,
        types::AccountType,
        Vec<u8>,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Account linked \[did, type, account\]
        AccountLinked(T::DecentralizedId, types::AccountType, Vec<u8>),
        /// Pending link failed \[did, type, account\]
        ValidationFailed(T::DecentralizedId, types::AccountType, Vec<u8>),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn offchain_worker(block_number: T::BlockNumber) {
            match Self::ocw_begin_block(block_number) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("An error occurred in OCW: {:?}", e);
                }
            }
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        Deadline,
        Exists,
        HttpFetchingError,
        InvalidETHAddress,
        InvalidSignature,
        TaskNotExists,
        UnexpectedAddress,
        UnsupportedSite,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(1_000_000_000)]
        pub fn link_sociality(
            origin: OriginFor<T>,
            site: types::AccountType,
            profile: Vec<u8>,
        ) -> DispatchResult {
            use sp_runtime::traits::Saturating;
            use types::AccountType::*;

            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            ensure!(!<LinksOf<T>>::contains_key(&did, &site), Error::<T>::Exists);
            ensure!(
                !<PendingOf<T>>::contains_key(&did, &site),
                Error::<T>::Exists
            );

            match site {
                Telegram if is_stask!(profile, b"https://t.me/") => {}
                _ => Err(Error::<T>::UnsupportedSite)?,
            };

            let created = <frame_system::Pallet<T>>::block_number();
            let lifetime = T::PendingLifetime::get();
            let deadline = created.saturating_add(lifetime);

            <PendingOf<T>>::insert(
                &did,
                site,
                types::Pending {
                    profile,
                    deadline,
                    created,
                },
            );

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn link_eth(
            origin: OriginFor<T>,
            address: Vec<u8>,
            signature: types::Signature,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            ensure!(
                !<LinksOf<T>>::contains_key(&did, types::AccountType::Ethereum),
                Error::<T>::Exists
            );

            ensure!(address.len() >= 2, Error::<T>::InvalidETHAddress);

            let mut bytes = Self::generate_message(&did);

            let mut length = Self::usize_to_u8_array(bytes.len())?;
            let mut data = b"\x19Ethereum Signed Message:\n".encode();
            data.append(&mut length);
            data.append(&mut bytes);
            let hash = sp_io::hashing::keccak_256(&data);

            let pubkey = sp_io::crypto::secp256k1_ecdsa_recover(&signature, &hash)
                .map_err(|_| Error::<T>::InvalidSignature)?;
            let pk = sp_io::hashing::keccak_256(&pubkey);

            ensure!(&pk[12..32] == &address, Error::<T>::UnexpectedAddress);

            <LinksOf<T>>::insert(&did, types::AccountType::Ethereum, address.clone());

            Self::deposit_event(Event::<T>::AccountLinked(
                did,
                types::AccountType::Bitcoin,
                address,
            ));

            Ok(())
        }

        #[pallet::weight(10000)]
        pub fn submit_link_unsigned(
            origin: OriginFor<T>,
            did: T::DecentralizedId,
            site: types::AccountType,
            profile: Vec<u8>,
            ok: bool,
        ) -> DispatchResultWithPostInfo {
            let _ = ensure_none(origin)?;

            ensure!(!<LinksOf<T>>::contains_key(&did, &site), Error::<T>::Exists);

            let task = <PendingOf<T>>::get(&did, &site).ok_or(Error::<T>::TaskNotExists)?;

            if ok {
                ensure!(task.profile == profile, Error::<T>::UnexpectedAddress);

                <LinksOf<T>>::insert(&did, &site, task.profile.clone());

                Self::deposit_event(Event::<T>::AccountLinked(did, site.clone(), task.profile));
            } else {
                Self::deposit_event(Event::<T>::ValidationFailed(
                    did,
                    site.clone(),
                    task.profile,
                ));
            }

            <PendingOf<T>>::remove(&did, &site);

            Ok(().into())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T>
    where
        T: Config,
    {
        pub links: Vec<(T::DecentralizedId, types::AccountType, Vec<u8>)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                links: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            for (did, typ, dat) in &self.links {
                <LinksOf<T>>::insert(did, typ, dat);
            }
        }
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            let valid_tx = |provide| {
                ValidTransaction::with_tag_prefix("linker")
                    .priority(T::UnsignedPriority::get())
                    .and_provides([&provide])
                    .longevity(3)
                    .propagate(true)
                    .build()
            };

            match call {
                Call::submit_link_unsigned { .. } => valid_tx(b"submit_link_unsigned".to_vec()),
                _ => InvalidTransaction::Call.into(),
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    fn usize_to_u8_array(length: usize) -> Result<Vec<u8>, &'static str> {
        if length >= 100 {
            Err("Unexpected message length!")?;
        }

        let digits = b"0123456789".encode();
        let tens = length / 10;
        let ones = length % 10;

        let mut vec_res: Vec<u8> = Vec::new();
        if tens != 0 {
            vec_res.push(digits[tens]);
        }
        vec_res.push(digits[ones]);
        Ok(vec_res)
    }

    pub fn generate_message(did: &T::DecentralizedId) -> Vec<u8> {
        use base58::ToBase58;

        let mut bytes = b"Link: ".to_vec();

        let did = did.as_ref();
        let did = did.to_base58();
        let mut did = did.as_bytes().to_vec();

        let mut prefix = b"did:ad3:".to_vec();

        bytes.append(&mut prefix);
        bytes.append(&mut did);
        bytes
    }
}

pub mod crypto {
    use crate::OFFCHAIN_KEY_TYPE;
    use sp_core::sr25519::{Public as Sr25519Public, Signature as Sr25519Signature};
    use sp_runtime::{
        app_crypto::{app_crypto, sr25519},
        traits::Verify,
        MultiSignature, MultiSigner,
    };

    app_crypto!(sr25519, OFFCHAIN_KEY_TYPE);

    pub struct LinkerAuthId;

    impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for LinkerAuthId {
        type RuntimeAppPublic = Public;
        type GenericSignature = Sr25519Signature;
        type GenericPublic = Sr25519Public;
    }

    impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
        for LinkerAuthId
    {
        type RuntimeAppPublic = Public;
        type GenericSignature = Sr25519Signature;
        type GenericPublic = Sr25519Public;
    }
}
