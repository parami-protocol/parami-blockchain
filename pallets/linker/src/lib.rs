#![cfg_attr(not(feature = "std"), no_std)]

pub use btc::hashing;
pub use ocw::images;
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod btc;
mod did;
mod impls;
mod ocw;
mod types;
mod witness;

use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    ensure,
};
use frame_system::offchain::CreateSignedTransaction;
use scale_info::TypeInfo;
use sp_runtime::traits::{MaybeSerializeDeserialize, Member};
use sp_std::prelude::*;

macro_rules! is_task {
    ($profile:expr, $prefix:expr) => {
        $profile.starts_with($prefix) && $profile.last() != Some(&b'/')
    };
}

pub(crate) use is_task;

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
                    tracing::error!("An error occurred in OCW: {:?}", e);
                }
            }
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        Deadline,
        Exists,
        HttpFetchingError,
        InvalidAddress,
        InvalidSignature,
        TaskNotExists,
        UnexpectedAddress,
        UnsupportedSite,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Link a sociality account to a DID
        ///
        /// Link will become pending, and will be checked with the offchain worker or a validator
        ///
        /// # Arguments
        ///
        /// * `site` - Account type
        /// * `profile` - Profile URL
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
                Telegram if is_task!(profile, b"https://t.me/") => {}
                Twitter if is_task!(profile, b"https://twitter.com/") => {}
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

        /// Link a cryptographic account to a DID
        ///
        ///
        /// # Arguments
        ///
        /// * `crypto` - Account type
        /// * `address` - Account address
        ///   * When dealing with BTC, DOT, SOL, TRX, the address should in the format of base58
        ///   * When dealing with ETH, the address should in the format of binary or hex
        /// * `signature` - Account signature
        ///   * When dealing with DOT, SOL, the signature should have a prefix of `0x00`
        #[pallet::weight(1_000_000_000)]
        pub fn link_crypto(
            origin: OriginFor<T>,
            crypto: types::AccountType,
            address: Vec<u8>,
            signature: types::Signature,
        ) -> DispatchResult {
            let (did, _) = T::CallOrigin::ensure_origin(origin)?;

            ensure!(
                !<LinksOf<T>>::contains_key(&did, crypto),
                Error::<T>::Exists
            );

            ensure!(address.len() >= 2, Error::<T>::InvalidAddress);

            let bytes = Self::generate_message(&did);

            let recovered = Self::recover_address(crypto, address.clone(), signature, bytes)?;

            ensure!(recovered == address, Error::<T>::UnexpectedAddress);

            <LinksOf<T>>::insert(&did, crypto, address.clone());

            Self::deposit_event(Event::<T>::AccountLinked(did, crypto, address));

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
