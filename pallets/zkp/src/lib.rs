#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub use ocw::zkp;

#[rustfmt::skip]
pub mod weights;

// #[cfg(test)]
// mod mock;

//#[cfg(test)]
//mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod ocw;
mod types;
mod impls;

use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    traits::{Currency},
    PalletId,
};
use frame_system::offchain::CreateSignedTransaction;
use parami_did::{EnsureDid};
use sp_runtime::traits::Hash;
use sp_std::prelude::*;

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <CurrencyOf<T> as Currency<AccountOf<T>>>::Balance;
type CurrencyOf<T> = <T as parami_did::Config>::Currency;
type DidOf<T> = <T as parami_did::Config>::DecentralizedId;
type HashOf<T> = <<T as frame_system::Config>::Hashing as Hash>::Output;
type NegativeImbOf<T> = <CurrencyOf<T> as Currency<AccountOf<T>>>::NegativeImbalance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config:
    frame_system::Config
    + parami_did::Config //
    + CreateSignedTransaction<Call<Self>> {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The pallet id, used for deriving "pot" accounts to receive donation
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Unsigned Call Priority
        #[pallet::constant]
        type UnsignedPriority: Get<TransactionPriority>;

        /// Lifetime of a pending account
        #[pallet::constant]
        type PendingLifetime: Get<Self::BlockNumber>;

        /// The origin which may forcibly trust or block a registrar
        type ForceOrigin: EnsureOrigin<Self::Origin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    /// Accounts pending to be checked with the offchain worker
    #[pallet::storage]
    #[pallet::getter(fn pendings_of)]
    pub(super) type PendingOf<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        Vec<u8>,
        types::Pending<T::BlockNumber, DidOf<T>>,
    >;

    #[pallet::storage]
    #[pallet::getter(fn verified)]
    pub(super) type Verified<T: Config> = StorageDoubleMap<
        _,
        Identity,
        DidOf<T>,
        Blake2_256,
        Vec<u8>,
        types::Proof<T::BlockNumber>, //result
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn vetoed)]
    pub(super) type Vetoed<T: Config> = StorageMap<
        _,
        Identity,
        DidOf<T>,
        types::Proof<T::BlockNumber>, //result
        ValueQuery,
    >;

    /// Encrypt key of the did
    #[pallet::storage]
    #[pallet::getter(fn ek_of)]
    pub(super) type EkOf<T: Config> = StorageMap<
        _,
        Identity,
        DidOf<T>,
        Vec<u8>,//key
    >;

    /// DID of a registrar
    #[pallet::storage]
    #[pallet::getter(fn registrar)]
    pub(super) type Registrar<T: Config> = StorageMap<_, Identity, DidOf<T>, bool>;

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Verify proof Ok \[did, verify_by\]
        VerifyOk(DidOf<T>, DidOf<T>),
        /// Verify proof Failed \[did, verify_by\]
        VerifyFailed(DidOf<T>, DidOf<T>),
    }

    #[pallet::error]
    pub enum Error<T> {
        Blocked,
        Deadline,
        ExistentialDeposit,
        Exists,
        IpfsError,
        HttpFetchingError,
        InvalidJson,
        NoEk,
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

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Verify a proof from a DID
        ///
        /// Verify will be inserted into the pending list and will be verified by the off-chain worker
        ///
        /// # Arguments
        ///
        /// * `ipfs` - proof file from IPFS
        #[pallet::weight(0)]
        pub fn verify_it(
            origin: OriginFor<T>,
            ipfs: Vec<u8>,
        ) -> DispatchResult {
            let (did, _) = EnsureDid::<T>::ensure_origin(origin)?;
            // let res = ocw::zkp::verify(ek, challenge, encrypted_pairs, proof, range, cipher_x);
            // if res {
            //     Self::deposit_event(Event::<T>::VerifyOk);
            // } else {
            //     Self::deposit_event(Event::<T>::VerifyFailed);
            // }
            Self::insert_pending(did, ipfs);
            Ok(())
        }

        #[pallet::weight(0)]
        pub fn setEk(
            origin: OriginFor<T>,
            ek: Vec<u8>,
        ) -> DispatchResult {
            let (did, _) = EnsureDid::<T>::ensure_origin(origin)?;
            // let res = ocw::zkp::verify(ek, challenge, encrypted_pairs, proof, range, cipher_x);
            // if res {
            //     Self::deposit_event(Event::<T>::VerifyOk);
            // } else {
            //     Self::deposit_event(Event::<T>::VerifyFailed);
            // }
            EkOf::<T>::insert(did, ek);
            Ok(())
        }

        // chain internal use only
        #[pallet::weight(1000)]
        pub fn submit_verify(
            _origin: OriginFor<T>,
            did: DidOf<T>,
            ipfs: Vec<u8>,
            range: Vec<u8>,
            result: bool,
        ) -> DispatchResultWithPostInfo {
            // let registrar = if let Err(_) = ensure_none(origin.clone()) {
            //     let (registrar, _) = EnsureDid::<T>::ensure_origin(origin)?;
            //     ensure!(
            //         <Registrar<T>>::get(&registrar) == Some(true),
            //         Error::<T>::Blocked
            //     );
            //     registrar
            // } else {
            //     DidOf::<T>::default()
            // };

            if result {
                // Self::insert_verified(did, ipfs, registrar)?;
                Self::insert_verified(did, ipfs, range, DidOf::<T>::default())?;
            } else {
                // Self::veto_pending(did, ipfs, registrar)?;
                Self::veto_pending(did, ipfs, range, DidOf::<T>::default())?;
            }

            Ok(().into())
        }
    }

    // #[pallet::genesis_config]
    // pub struct GenesisConfig<T: Config> {
    //     pub eks: Vec<(DidOf<T>, Vec<u8>)>,
    //     pub verified: Vec<(DidOf<T>,Vec<u8)>,
    // }
    //
    // #[cfg(feature = "std")]
    // impl<T: Config> Default for GenesisConfig<T> {
    //     fn default() -> Self {
    //         Self {
    //             eks: Default::default(),
    //             verified: Default::default(),
    //         }
    //     }
    // }
    // #[pallet::genesis_build]
    // impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
    //     fn build(&self) {
    //         for (did, typ, dat) in &self.links {
    //             <LinksOf<T>>::insert(did, typ, dat);
    //             <Linked<T>>::insert(typ, dat, true);
    //         }
    //
    //         for registrar in &self.registrars {
    //             <Registrar<T>>::insert(registrar, true);
    //         }
    //     }
    // }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            let valid_tx = |provide| {
                ValidTransaction::with_tag_prefix("zkp")
                    .priority(T::UnsignedPriority::get())
                    .and_provides([&provide])
                    .longevity(3)
                    .propagate(true)
                    .build()
            };

            match call {
                Call::submit_verify { .. } => valid_tx(b"submit_verify".to_vec()),
                _ => InvalidTransaction::Call.into(),
            }
        }
    }
}
