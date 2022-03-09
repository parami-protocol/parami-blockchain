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

use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    ensure,
    traits::{Currency, NamedReservableCurrency, OnUnbalanced},
    PalletId,
};
use frame_system::offchain::CreateSignedTransaction;
use parami_did::{EnsureDid, Pallet as Did};
use parami_traits::Tags;
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
    pub trait Config: frame_system::Config {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The pallet id, used for deriving "pot" accounts to receive donation
        // #[pallet::constant]
        // type PalletId: Get<PalletId>;

        /// Unsigned Call Priority
        #[pallet::constant]
        type UnsignedPriority: Get<TransactionPriority>;

        /// The origin which may forcibly trust or block a registrar
        type ForceOrigin: EnsureOrigin<Self::Origin>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Account linked \[did, type, account, by\]
        VerifyOk,
        /// Account unlinked \[did, type, by\]
        VerifyFailed,
    }

    // #[pallet::hooks]
    // impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
    //     fn offchain_worker(block_number: T::BlockNumber) {
    //         match Self::ocw_begin_block(block_number) {
    //             Ok(_) => {}
    //             Err(e) => {
    //                 tracing::error!("An error occurred in OCW: {:?}", e);
    //             }
    //         }
    //     }
    // }

    #[pallet::error]
    pub enum Error<T> {
        WrongDataBits
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Link a sociality account to a DID
        ///
        /// Link will become pending, and will be checked with the offchain worker or a registrar
        ///
        /// # Arguments
        ///
        /// * `site` - Account type
        /// * `profile` - Profile URL
        #[pallet::weight(1000)]
        pub fn verifyIt(
            origin: OriginFor<T>,
            ek: Vec<u8>, challenge: Vec<u8>, encrypted_pairs: Vec<u8>, proof: Vec<u8>, range: Vec<u8>, cipher_x: Vec<u8>,
        ) -> DispatchResult {
            let res = ocw::zkp::verify(ek, challenge, encrypted_pairs, proof, range, cipher_x);
            if res {
                Self::deposit_event(Event::<T>::VerifyOk);
            } else {
                Self::deposit_event(Event::<T>::VerifyFailed);
            }
            //Self::insert_pending(did, site, profile)
            Ok(())
        }
    }
}
