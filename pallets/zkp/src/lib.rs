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

type DidOf<T> = <T as parami_did::Config>::DecentralizedId;

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

    /// pending tasks key: ipfs_url
    #[pallet::storage]
    #[pallet::getter(fn pendings)]
    pub(super) type PendingTasks<T: Config> = StorageMap<
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
        #[pallet::weight(<T as Config>::WeightInfo::submit_proof(ipfs.len() as u32))]
        pub fn submit_proof(
            origin: OriginFor<T>,
            ipfs: Vec<u8>,
        ) -> DispatchResult {
            let (did, _) = EnsureDid::<T>::ensure_origin(origin)?;
            Self::insert_pending(did, ipfs);
            Ok(())
        }

        //todo: need refactor for production
        #[pallet::weight(0)]
        pub fn set_ek(
            origin: OriginFor<T>,
            ek: Vec<u8>,
        ) -> DispatchResult {
            let (did, _) = EnsureDid::<T>::ensure_origin(origin)?;
            EkOf::<T>::insert(did, ek);
            Ok(())
        }

        // chain internal use only
        // todo: we need add some authentication for this call
        #[pallet::weight(1000)]
        pub fn submit_verify(
            origin: OriginFor<T>,
            did: DidOf<T>,
            ipfs: Vec<u8>,
            range: Vec<u8>,
            result: bool,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin);
            if result {
                Self::insert_verified(did, ipfs, range, DidOf::<T>::default())?;
            } else {
                Self::veto_pending(did, ipfs, range, DidOf::<T>::default())?;
            }

            Ok(().into())
        }
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match source {
                TransactionSource::Local | TransactionSource::InBlock => { /* allowed */ }
                _ => return InvalidTransaction::Call.into(),
            };

            let valid_tx = |provide| {
                ValidTransaction::with_tag_prefix("zkp")
                    .priority(T::UnsignedPriority::get())
                    .and_provides([&provide])
                    .longevity(3)
                    .propagate(false)
                    .build()
            };

            match call {
                Call::submit_verify { .. } => valid_tx(b"submit_verify".to_vec()),
                _ => InvalidTransaction::Call.into(),
            }
        }
    }
}
