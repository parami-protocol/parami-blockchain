#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    pallet_prelude::*,
    traits::{Currency, EnsureOrigin, ExistenceRequirement::KeepAlive},
    transactional,
};
use frame_system::pallet_prelude::*;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
    traits::{Saturating, StaticLookup},
    RuntimeDebug,
};
use sp_std::vec::Vec;

mod mock;
mod tests;

pub use module::*;

#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum Erc20Event<Balance, AccountId> {
    Transfer {
        /// Value
        #[codec(compact)]
        value: Balance,
        // From
        from: Vec<u8>,
    },
    Withdraw {
        /// Value
        #[codec(compact)]
        value: Balance,
        // Who
        who: Vec<u8>,
        // True for success.
        status: bool,
    },
    Redeem {
        /// Value
        #[codec(compact)]
        value: Balance,
        // from
        from: Vec<u8>,
        // to
        to: AccountId,
    },

    Despoit {
        #[codec(compact)]
        value: Balance,
        to: Vec<u8>,
        from: AccountId,
    },
}

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
#[frame_support::pallet]
pub mod module {
    use super::*;
    use frame_support::traits::WithdrawReasons;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency trait.
        type Currency: Currency<Self::AccountId>;

        type ConfigOrigin: EnsureOrigin<Self::Origin>;
    }

    #[pallet::error]
    pub enum Error<T> {
        /// bridge admin not set
        BridgeAdminNotSet,
        /// no permission
        NoPermission,

        InsufficientBalance,

        TransforFail,
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(crate) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Received Erc20 Transfer event \[tx_hash\]
        Transfer(Vec<u8>),
        /// Received Erc20 Withdraw event \[tx_hash, status\]
        Withdraw(Vec<u8>, bool),
        /// Received Erc20 Redeem event \[tx_hash\]
        Redeem(Vec<u8>),
        /// Despoit ad3 assets
        Desposit(Vec<u8>, T::AccountId, BalanceOf<T>),
    }

    #[pallet::pallet]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
        fn on_runtime_upgrade() -> Weight {
            0
        }

        fn integrity_test() {}
    }
    //
    // #[pallet::genesis_config]
    // pub struct GenesisConfig<T: Config> {
    //     _phantom: PhantomData<T>,
    // }

    // #[cfg(feature = "std")]
    // impl<T: Config> Default for GenesisConfig<T> {
    //     fn default() -> Self {
    //         Self {
    //             _phantom: Default::default(),
    //         }
    //     }
    // }
    //
    // #[pallet::genesis_build]
    // impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
    //     fn build(&self) {}
    // }

    /// The privileged account.
    #[pallet::storage]
    #[pallet::getter(fn bridge_admin)]
    pub(super) type BridgeAdmin<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

    /// Erc20 transfer transactions.
    ///
    /// `tx_hash` map to `Erc20Transfer`
    #[pallet::storage]
    #[pallet::getter(fn erc20_txs)]
    pub type Erc20Txs<T: Config> =
        StorageMap<_, Identity, Vec<u8>, Erc20Event<BalanceOf<T>, T::AccountId>>;

    /// Erc20 balances in parami
    ///
    /// `eth_addr` map to value.
    #[pallet::storage]
    #[pallet::getter(fn erc20_balances)]
    pub type Erc20Balances<T: Config> =
        StorageMap<_, Blake2_128Concat, Vec<u8>, BalanceOf<T>, ValueQuery>;

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Set the privileged account
        #[pallet::weight((100_000, DispatchClass::Operational))]
        #[transactional]
        pub fn set_bridge_admin(
            origin: OriginFor<T>,
            admin: <T::Lookup as StaticLookup>::Source,
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            let admin = T::Lookup::lookup(admin)?;
            BridgeAdmin::<T>::put(admin);
            Ok((None, Pays::No).into())
        }

        /// Received a `Redeem` event from ethereum erc20 contract.
        ///
        /// - `tx_hash`: The transaction hash of this erc20 event in ethereum.
        /// - `from_eth_addr`:  redeem `value` by ethereum account `from_eth_addr`.
        /// - `to`:  the beneficiary account in Parami.
        /// - `value`:  value to be redeem
        #[pallet::weight((100_000, DispatchClass::Operational, Pays::Yes))]
        #[transactional]
        pub fn redeem(
            origin: OriginFor<T>,
            tx_hash: Vec<u8>,
            from_eth_addr: Vec<u8>,
            to: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(
                who == Self::bridge_admin().ok_or(Error::<T>::BridgeAdminNotSet)?,
                Error::<T>::NoPermission
            );
            let to = T::Lookup::lookup(to)?;
            Erc20Txs::<T>::mutate_exists(tx_hash.clone(), |maybe_tx| {
                if maybe_tx.is_none() {
                    let _ = T::Currency::deposit_creating(&to, value);
                    *maybe_tx = Some(Erc20Event::Redeem {
                        value,
                        from: from_eth_addr,
                        to,
                    });
                    Self::deposit_event(Event::Redeem(tx_hash));
                }
            });
            Ok((None, Pays::No).into())
        }

        #[pallet::weight((100_000, DispatchClass::Operational, Pays::No))]
        #[transactional]
        pub fn desposit(
            origin: OriginFor<T>,
            to_eth_addr: Vec<u8>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(
                T::Currency::free_balance(&who) > value,
                Error::<T>::InsufficientBalance
            );
            let _ =
                T::Currency::withdraw(&who, value, WithdrawReasons::TRANSACTION_PAYMENT, KeepAlive);
            Self::deposit_event(Event::Desposit(to_eth_addr, who, value));

            Ok((None, Pays::No).into())
        }

        /// Received a `Transfer` event from ethereum erc20 contract.
        ///
        /// - `tx_hash`: The transaction hash of this erc20 event in ethereum.
        /// - `value`: Amount transferred.
        /// - `eth_addr`: `value` was transferred from `eth_addr`.
        #[pallet::weight((100_000, DispatchClass::Operational, Pays::Yes))]
        #[transactional]
        pub fn transfer(
            origin: OriginFor<T>,
            tx_hash: Vec<u8>,
            eth_addr: Vec<u8>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(
                who == Self::bridge_admin().ok_or(Error::<T>::BridgeAdminNotSet)?,
                Error::<T>::NoPermission
            );
            Erc20Txs::<T>::mutate_exists(tx_hash.clone(), |maybe_tx| {
                if maybe_tx.is_none() {
                    Erc20Balances::<T>::mutate(&eth_addr, |balance| {
                        *balance = balance.saturating_add(value);
                    });
                    *maybe_tx = Some(Erc20Event::Transfer {
                        value,
                        from: eth_addr,
                    });
                    Self::deposit_event(Event::Transfer(tx_hash));
                }
            });
            Ok((None, Pays::No).into())
        }

        /// Received a `Withdraw` event from ethereum erc20 contract.
        ///
        /// - `tx_hash`: The transaction hash of this erc20 event in ethereum.
        /// - `from_eth_addr`:  withdraw `value` by ethereum account `from_eth_addr`.
        /// - `to`:  the beneficiary account in Parami.
        /// - `value`:  value to be withdraw
        #[pallet::weight((100_000, DispatchClass::Operational, Pays::Yes))]
        #[transactional]
        pub fn withdraw(
            origin: OriginFor<T>,
            tx_hash: Vec<u8>,
            from_eth_addr: Vec<u8>,
            to: <T::Lookup as StaticLookup>::Source,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(
                who == Self::bridge_admin().ok_or(Error::<T>::BridgeAdminNotSet)?,
                Error::<T>::NoPermission
            );
            let to = T::Lookup::lookup(to)?;
            Erc20Txs::<T>::mutate_exists(tx_hash.clone(), |maybe_tx| {
                if maybe_tx.is_none() {
                    Erc20Balances::<T>::mutate(from_eth_addr.clone(), |balance| {
                        if *balance >= value {
                            *balance -= value;
                            let _ = T::Currency::deposit_creating(&to, value);
                            *maybe_tx = Some(Erc20Event::Withdraw {
                                value,
                                who: from_eth_addr,
                                status: true,
                            });
                            Self::deposit_event(Event::Withdraw(tx_hash, true));
                        } else {
                            *maybe_tx = Some(Erc20Event::Withdraw {
                                value,
                                who: from_eth_addr,
                                status: false,
                            });
                            Self::deposit_event(Event::Withdraw(tx_hash, false));
                        }
                    });
                }
            });
            Ok((None, Pays::No).into())
        }
    }
}
