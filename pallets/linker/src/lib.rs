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
    traits::{Currency, NamedReservableCurrency, OnUnbalanced},
    PalletId,
};
use frame_system::offchain::CreateSignedTransaction;
use parami_did::{EnsureDid, Pallet as Did};
use parami_traits::Tags;
use sp_runtime::traits::Hash;
use sp_std::prelude::*;

macro_rules! is_task {
    ($profile:expr, $prefix:expr) => {
        $profile.starts_with($prefix) && $profile.last() != Some(&b'/')
    };
}

pub(crate) use is_task;

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
        + CreateSignedTransaction<Call<Self>>
    {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Minimum deposit to become a registrar
        #[pallet::constant]
        type MinimumDeposit: Get<BalanceOf<Self>>;

        /// The pallet id, used for deriving "pot" accounts to receive donation
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Lifetime of a pending account
        #[pallet::constant]
        type PendingLifetime: Get<Self::BlockNumber>;

        /// Handler for the unbalanced reduction when slashing an registrar
        type Slash: OnUnbalanced<NegativeImbOf<Self>>;

        /// The means of storing the tags and personas of a DID.
        type Tags: Tags<DecentralizedId = DidOf<Self>, Hash = HashOf<Self>>;

        /// Unsigned Call Priority
        #[pallet::constant]
        type UnsignedPriority: Get<TransactionPriority>;

        /// The origin which may forcibly trust or block a registrar
        type ForceOrigin: EnsureOrigin<Self::Origin>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Linked accounts of a DID
    #[pallet::storage]
    #[pallet::getter(fn links_of)]
    pub(super) type LinksOf<T: Config> = StorageDoubleMap<
        _,
        Identity,
        DidOf<T>,
        Twox64Concat,
        types::AccountType,
        Vec<u8>, //
    >;

    /// Accounts pending to be checked with the offchain worker
    #[pallet::storage]
    #[pallet::getter(fn pendings_of)]
    pub(super) type PendingOf<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        types::AccountType,
        Identity,
        DidOf<T>,
        types::Pending<T::BlockNumber>,
    >;

    /// DID of a registrar
    #[pallet::storage]
    #[pallet::getter(fn registrar)]
    pub(super) type Registrar<T: Config> = StorageMap<_, Identity, DidOf<T>, bool>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Account linked \[did, type, account\]
        AccountLinked(DidOf<T>, types::AccountType, Vec<u8>),
        /// Registrar was blocked \[id\]
        Blocked(DidOf<T>),
        /// Registrar deposited \[id, value\]
        Deposited(DidOf<T>, BalanceOf<T>),
        /// Registrar was trusted \[id\]
        Trusted(DidOf<T>),
        /// Pending link failed \[did, type, account\]
        ValidationFailed(DidOf<T>, types::AccountType, Vec<u8>),
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
        Blocked,
        Deadline,
        ExistentialDeposit,
        Exists,
        HttpFetchingError,
        InvalidAddress,
        InvalidSignature,
        NotExists,
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

            let (did, _) = EnsureDid::<T>::ensure_origin(origin)?;

            ensure!(!<LinksOf<T>>::contains_key(&did, &site), Error::<T>::Exists);
            ensure!(
                !<PendingOf<T>>::contains_key(&site, &did),
                Error::<T>::Exists
            );

            match site {
                Discord if is_task!(profile, b"https://discordapp.com/users/") => {}
                Facebook if is_task!(profile, b"https://www.facebook.com/") => {}
                Github if is_task!(profile, b"https://github.com/") => {}
                HackerNews if is_task!(profile, b"https://news.ycombinator.com/user?id=") => {}
                Mastodon => {}
                Reddit if is_task!(profile, b"https://www.reddit.com/user/") => {}
                Telegram if is_task!(profile, b"https://t.me/") => {}
                Twitter if is_task!(profile, b"https://twitter.com/") => {}
                _ => Err(Error::<T>::UnsupportedSite)?,
            };

            let created = <frame_system::Pallet<T>>::block_number();
            let lifetime = T::PendingLifetime::get();
            let deadline = created.saturating_add(lifetime);

            <PendingOf<T>>::insert(
                &site,
                &did,
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
            let (did, _) = EnsureDid::<T>::ensure_origin(origin)?;

            ensure!(
                !<LinksOf<T>>::contains_key(&did, &crypto),
                Error::<T>::Exists
            );

            ensure!(address.len() >= 2, Error::<T>::InvalidAddress);

            let bytes = Self::generate_message(&did);

            let recovered = Self::recover_address(crypto, address.clone(), signature, bytes)?;

            ensure!(recovered == address, Error::<T>::UnexpectedAddress);

            <LinksOf<T>>::insert(&did, &crypto, address.clone());

            Self::deposit_event(Event::<T>::AccountLinked(did, crypto, address));

            Ok(())
        }

        #[pallet::weight(10000)]
        pub fn submit_link(
            origin: OriginFor<T>,
            did: DidOf<T>,
            site: types::AccountType,
            profile: Vec<u8>,
            ok: bool,
        ) -> DispatchResultWithPostInfo {
            if let Err(_) = ensure_none(origin.clone()) {
                let (registrar, _) = EnsureDid::<T>::ensure_origin(origin)?;

                ensure!(
                    <Registrar<T>>::get(&registrar) == Some(true),
                    Error::<T>::Blocked
                );
            }

            ensure!(!<LinksOf<T>>::contains_key(&did, &site), Error::<T>::Exists);

            let task = <PendingOf<T>>::get(&site, &did).ok_or(Error::<T>::NotExists)?;

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

            <PendingOf<T>>::remove(&site, &did);

            Ok(().into())
        }

        #[pallet::weight(10000)]
        pub fn submit_score(
            origin: OriginFor<T>,
            did: DidOf<T>,
            tag: Vec<u8>,
            score: i32,
        ) -> DispatchResultWithPostInfo {
            let (registrar, _) = EnsureDid::<T>::ensure_origin(origin)?;

            ensure!(
                <Registrar<T>>::get(&registrar) == Some(true),
                Error::<T>::Blocked
            );

            ensure!(T::Tags::get_score(&did, &tag) == 0, Error::<T>::Exists);

            T::Tags::influence(&did, &tag, score)?;

            Ok(().into())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn deposit(
            origin: OriginFor<T>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, who) = EnsureDid::<T>::ensure_origin(origin)?;

            ensure!(
                <Registrar<T>>::get(&did) != Some(false),
                Error::<T>::Blocked
            );

            let id = <T as Config>::PalletId::get();

            T::Currency::reserve_named(&id.0, &who, value)?;

            Self::deposit_event(Event::Deposited(did, value));

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn force_trust(origin: OriginFor<T>, did: DidOf<T>) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;

            let minimum = T::MinimumDeposit::get();

            let meta = Did::<T>::meta(&did).ok_or(Error::<T>::NotExists)?;

            let id = <T as Config>::PalletId::get();

            let reserved = T::Currency::reserved_balance_named(&id.0, &meta.account);

            ensure!(reserved >= minimum, Error::<T>::ExistentialDeposit);

            <Registrar<T>>::insert(&did, true);

            Self::deposit_event(Event::Trusted(did));

            Ok(())
        }

        #[pallet::weight(1_000_000_000)]
        pub fn force_block(origin: OriginFor<T>, registrar: DidOf<T>) -> DispatchResult {
            T::ForceOrigin::ensure_origin(origin)?;

            let meta = Did::<T>::meta(&registrar).ok_or(Error::<T>::NotExists)?;

            let id = <T as Config>::PalletId::get();

            let imb = T::Currency::slash_all_reserved_named(&id.0, &meta.account);

            T::Slash::on_unbalanced(imb);

            <Registrar<T>>::insert(&registrar, false);

            Self::deposit_event(Event::Blocked(registrar));

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T>
    where
        T: Config,
    {
        pub links: Vec<(DidOf<T>, types::AccountType, Vec<u8>)>,
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
                Call::submit_link { .. } => valid_tx(b"submit_link".to_vec()),
                _ => InvalidTransaction::Call.into(),
            }
        }
    }
}
