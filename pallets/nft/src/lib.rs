#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[rustfmt::skip]
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod types;
pub use types::*;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{
        tokens::{
            fungibles::{
                metadata::Mutate as FungMetaMutate, Create as FungCreate, Inspect as _,
                InspectEnumerable as FungInspectEnumerable, Mutate as FungMutate,
                Transfer as FungTransfer,
            },
            nonfungibles::{Create as NftCreate, Mutate as NftMutate},
        },
        Currency, EnsureOrigin,
        ExistenceRequirement::KeepAlive,
        Get,
    },
    weights::Weight,
};
use parami_did::{EnsureDid};
use parami_traits::Swaps;
use sp_core::U512;
use sp_runtime::{
    traits::{One, Saturating},
    DispatchError, RuntimeDebug
};
use sp_std::{
    convert::{TryFrom, TryInto},
    prelude::*,
};

use weights::WeightInfo;

type BalanceOf<T> = <<T as parami_did::Config>::Currency as Currency<AccountOf<T>>>::Balance;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;

pub trait FarmingCurve<T: pallet::Config> {
    /// Calculate the farming value for a given block height
    ///
    /// # Arguments
    ///
    /// * `minted_height` - The block number of the initial minting
    /// * `started_supply` - the tokens amount of the initial minting
    /// * `maximum_tokens` - the maximum amount of tokens
    /// * `current_height` - the block number of current block
    /// * `current_supply` - the tokens amount before farming
    fn calculate_farming_reward(
        minted_height: HeightOf<T>,
        started_supply: BalanceOf<T>,
        maximum_tokens: BalanceOf<T>,
        current_height: HeightOf<T>,
        current_supply: BalanceOf<T>,
    ) -> BalanceOf<T>;
}

impl<T: Config> FarmingCurve<T> for () {
    fn calculate_farming_reward(
        _minted_height: HeightOf<T>,
        _started_supply: BalanceOf<T>,
        _maximum_tokens: BalanceOf<T>,
        _current_height: HeightOf<T>,
        _current_supply: BalanceOf<T>,
    ) -> BalanceOf<T> {
        Default::default()
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + parami_did::Config {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The assets trait to create, mint, and transfer fragments (fungible token)
        /// it uses parami_did::Config::AssetId as AssetId
        type Assets: FungCreate<AccountOf<Self>, AssetId = Self::AssetId>
            + FungInspectEnumerable<AccountOf<Self>>
            + FungMetaMutate<AccountOf<Self>, AssetId = Self::AssetId>
            + FungMutate<AccountOf<Self>, AssetId = Self::AssetId, Balance = BalanceOf<Self>>
            + FungTransfer<AccountOf<Self>, AssetId = Self::AssetId, Balance = BalanceOf<Self>>;

        /// The curve for seasoned orffering
        type FarmingCurve: FarmingCurve<Self>;

        /// The ICO baseline of donation for currency
        #[pallet::constant]
        type InitialMintingDeposit: Get<BalanceOf<Self>>;

        /// The ICO lockup period for fragments, KOL will not be able to claim before this period
        #[pallet::constant]
        type InitialMintingLockupPeriod: Get<HeightOf<Self>>;

        /// The ICO value base of fragments, system will mint triple of the value
        /// once for KOL, once to swaps, once to supporters
        /// The maximum value of fragments is decuple of this value
        #[pallet::constant]
        type InitialMintingValueBase: Get<BalanceOf<Self>>;

        /// The NFT trait to create, mint non-fungible token
        /// it uses parami_did::Config::AssetId as InstanceId and ClassId
        type Nft: NftCreate<AccountOf<Self>, InstanceId = Self::AssetId, ClassId = Self::AssetId>
            + NftMutate<AccountOf<Self>, InstanceId = Self::AssetId, ClassId = Self::AssetId>;

        /// The maximum length of a name or symbol stored on-chain.
        /// TODO(ironman_ch): Why define it as a Get<u32> instead of u32 ?
        #[pallet::constant]
        type StringLimit: Get<u32>;

        /// The swaps trait
        type Swaps: Swaps<
            AccountId = AccountOf<Self>,
            AssetId = Self::AssetId,
            QuoteBalance = BalanceOf<Self>,
            TokenBalance = BalanceOf<Self>,
        >;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    /// Total deposit in pot
    #[pallet::storage]
    #[pallet::getter(fn deposit)]
    pub(super) type Deposit<T: Config> = StorageMap<_, Identity, NftInstanceId<T>, BalanceOf<T>>;

    /// Nft's Metadata
    /// TODO: change hasher
    #[pallet::storage]
    pub(super) type NftMetaStore<T: Config> = StorageMap<
        _,
        Identity,
        NftInstanceId<T>,
        NftMetaFor<T>,
    >;

    /// Did's preferred Nft.
    #[pallet::storage]
    #[pallet::getter(fn preferred_nft_of)]
    pub(super) type PreferredNft<T: Config> =
        StorageMap<_, Identity, T::DecentralizedId, NftInstanceId<T>>;

    /// Deposits by supporter in pot
    /// TODO: change hasher
    #[pallet::storage]
    #[pallet::getter(fn deposits)]
    pub(super) type Deposits<T: Config> = StorageDoubleMap<
        _,
        Identity,
        NftInstanceId<T>,
        Identity,
        T::DecentralizedId, // Supporter
        BalanceOf<T>,
    >;

    /// Initial Minting date
    #[pallet::storage]
    #[pallet::getter(fn date)]
    pub(super) type Date<T: Config> = StorageMap<_, Twox64Concat, NftInstanceId<T>, HeightOf<T>>;

    /// Next available class ID
    #[pallet::storage]
    #[pallet::getter(fn next_cid)]
    pub(super) type NextClassId<T: Config> = StorageValue<_, T::AssetId, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// NFT fragments Minted \[did, kol, value\]
        Backed(
            T::DecentralizedId,
            T::DecentralizedId,
            NftInstanceId<T>,
            BalanceOf<T>,
        ),
        /// NFT fragments Claimed \[did, NftInstanceId, value\]
        Claimed(T::DecentralizedId, NftInstanceId<T>, BalanceOf<T>),
        /// NFT fragments Minted \[kol, instance, name, symbol, tokens\]
        Minted(
            T::DecentralizedId,
            NftInstanceId<T>,
            Vec<u8>,
            Vec<u8>,
            BalanceOf<T>,
        ),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<HeightOf<T>> for Pallet<T> {
        fn on_initialize(n: HeightOf<T>) -> Weight {
            let modu: u32 = n.try_into().map_or(100, |n: u32| n % 100);
            match modu {
                1 => Self::begin_block_for_farming_reward(n).unwrap_or_else(|e| {
                    sp_runtime::print(e);
                    0
                }),
                _ => 0,
            }
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        BadMetadata,
        InsufficientBalance,
        Minted,
        Overflow,
        NotExists,
        NoToken,
        YourSelf,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {

        /// generate a NftInstance and set it as the did's prefer to show nft in his personal page.
        pub fn gen_and_set_preferred_nft(
            origin: OriginFor<T>,
        ) -> DispatchError {
            todo!()
        }

        /// Back (support) the KOL.
        #[pallet::weight(<T as Config>::WeightInfo::back())]
        pub fn back(
            origin: OriginFor<T>,
            kol: T::DecentralizedId,
            instance_id: NftInstanceId<T>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, who) = EnsureDid::<T>::ensure_origin(origin)?;

            ensure!(kol != did, Error::<T>::YourSelf);

            let meta = <NftMetaStore<T>>::get(&instance_id).ok_or(Error::<T>::NotExists)?;

            ensure!(!meta.minted, Error::<T>::Minted);

            <T as parami_did::Config>::Currency::transfer(&who, &meta.pot, value, KeepAlive)?;

            <Deposit<T>>::mutate(&instance_id, |maybe| {
                if let Some(deposit) = maybe {
                    deposit.saturating_accrue(value);
                } else {
                    *maybe = Some(value);
                }
            });

            <Deposits<T>>::mutate(&instance_id, &did, |maybe| {
                if let Some(deposit) = maybe {
                    deposit.saturating_accrue(value);
                } else {
                    *maybe = Some(value);
                }
            });

            Self::deposit_event(Event::Backed(did, kol, instance_id, value));

            Ok(())
        }

        /// Fragment the NFT and mint token.
        #[pallet::weight(<T as Config>::WeightInfo::mint(name.len() as u32, symbol.len() as u32))]
        pub fn mint(
            origin: OriginFor<T>,
            instance_id: NftInstanceId<T>,
            name: Vec<u8>,
            symbol: Vec<u8>,
        ) -> DispatchResult {
            let limit = T::StringLimit::get() as usize - 4;

            ensure!(
                0 < name.len() && name.len() <= limit,
                Error::<T>::BadMetadata
            );
            ensure!(
                0 < name.len() && symbol.len() <= limit,
                Error::<T>::BadMetadata
            );

            let is_valid_char = |c: &u8| c.is_ascii_whitespace() || c.is_ascii_alphanumeric();

            ensure!(
                name[0].is_ascii_alphabetic() && name.iter().all(is_valid_char),
                Error::<T>::BadMetadata
            );
            ensure!(
                symbol[0].is_ascii_alphabetic() && symbol.iter().all(is_valid_char),
                Error::<T>::BadMetadata
            );

            let minted = <frame_system::Pallet<T>>::block_number();

            let (did, _) = EnsureDid::<T>::ensure_origin(origin)?;

            // 1. ensure funded

            let mut meta = <NftMetaStore<T>>::get(&instance_id).ok_or(Error::<T>::NotExists)?;
            ensure!(!meta.minted, Error::<T>::Minted);

            let deposit = <T as parami_did::Config>::Currency::free_balance(&meta.pot);

            ensure!(
                deposit >= T::InitialMintingDeposit::get(),
                Error::<T>::InsufficientBalance
            );

            // 2. create NFT token
            let tid = instance_id;

            T::Nft::create_class(&meta.class_id, &meta.pot, &meta.pot)?;
            T::Nft::mint_into(&meta.class_id, &tid, &meta.pot)?;

            // 3. initial minting

            let initial = T::InitialMintingValueBase::get();

            T::Assets::create(tid, meta.pot.clone(), true, One::one())?;
            T::Assets::set(tid, &meta.pot, name.clone(), symbol.clone(), 18)?;
            T::Assets::mint_into(tid, &meta.pot, initial.saturating_mul(3u32.into()))?;

            // 4. transfer third of initial minting to swap

            T::Swaps::new(&meta.pot, tid)?;
            T::Swaps::mint(&meta.pot, tid, deposit, deposit, initial, false)?;

            // 5. update local variable
            meta.minted = true;

            // 6. update storage
            <NftMetaStore<T>>::mutate(&tid, |maybe| {
                *maybe = Some(meta);
            });

            <Date<T>>::insert(&tid, minted);

            <Deposits<T>>::mutate(&tid, &did, |maybe| {
                *maybe = Some(deposit);
            });

            Self::deposit_event(Event::Minted(did, tid, name, symbol, initial));

            Ok(())
        }

        /// Claim the fragments.
        #[pallet::weight(<T as Config>::WeightInfo::claim())]
        pub fn claim(origin: OriginFor<T>, instance_id: NftInstanceId<T>) -> DispatchResult {
            let (did, who) = EnsureDid::<T>::ensure_origin(origin)?;

            let height = <frame_system::Pallet<T>>::block_number();

            let meta = NftMetaStore::<T>::get(&instance_id).ok_or(Error::<T>::NotExists)?;

            if meta.owner == did {
                let minted_block_number =
                    <Date<T>>::get(&instance_id).ok_or(Error::<T>::NotExists)?;
                ensure!(
                    height - minted_block_number >= T::InitialMintingLockupPeriod::get(),
                    Error::<T>::NoToken
                );
            }

            let total = <Deposit<T>>::get(&instance_id).ok_or(Error::<T>::NotExists)?;
            let deposit = <Deposits<T>>::get(&instance_id, &did).ok_or(Error::<T>::NoToken)?;
            let initial = T::InitialMintingValueBase::get();

            let total: U512 = Self::try_into(total)?;
            let deposit: U512 = Self::try_into(deposit)?;
            let initial: U512 = Self::try_into(initial)?;

            let tokens = initial * deposit / total;

            let tokens = Self::try_into(tokens)?;

            T::Assets::transfer(instance_id, &meta.pot, &who, tokens, false)?;

            <Deposits<T>>::remove(&instance_id, &did);

            Self::deposit_event(Event::Claimed(did, instance_id, tokens));

            Ok(())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub next_cid: T::AssetId,
        pub deposit: Vec<(NftInstanceId<T>, BalanceOf<T>)>,
        pub deposits: Vec<(NftInstanceId<T>, T::DecentralizedId, BalanceOf<T>)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                deposit: Default::default(),
                deposits: Default::default(),
                next_cid: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <NextClassId<T>>::put(self.next_class_id);

            let next_class_id: u32 = self.next_class_id.try_into().unwrap_or_default();
            if next_class_id > 0 {
                for token in 0u32..next_class_id {
                    let token: T::AssetId = token.into();
                    <Date<T>>::insert(token, T::InitialMintingLockupPeriod::get());
                }
            }

            for (instance_id, deposit) in &self.deposit {
                <Deposit<T>>::insert(instance_id, deposit);
            }

            for (instance_id, did, deposit) in &self.deposits {
                <Deposits<T>>::insert(instance_id, did, deposit);
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    fn begin_block_for_farming_reward(height: HeightOf<T>) -> Result<Weight, DispatchError> {
        let weight = 1_000_000_000;

        // TODO: weight benchmark

        let initial = T::InitialMintingValueBase::get();

        let started = initial.saturating_mul(3u32.into());
        let maximum = initial.saturating_mul(10u32.into());

        for swap in T::Swaps::iter() {
            let token_id = swap.0;
            let lp_token_id = swap.1;

            let minted_block_number = <Date<T>>::get(&token_id);
            if minted_block_number.is_none() {
                continue;
            }
            let minted_block_number = minted_block_number.unwrap();

            let supply = T::Assets::total_issuance(token_id);

            let amount = T::FarmingCurve::calculate_farming_reward(
                minted_block_number,
                started,
                maximum,
                height,
                supply,
            );

            let left = maximum - supply;
            let amount = if amount < left { amount } else { left };

            if amount < One::one() {
                continue;
            }

            let liquidity = T::Assets::total_issuance(lp_token_id);

            let amount: U512 = Self::try_into(amount)?;
            let liquidity: U512 = Self::try_into(liquidity)?;

            for holder in T::Assets::accounts(&lp_token_id) {
                let hold = T::Assets::balance(lp_token_id, &holder);

                let hold: U512 = Self::try_into(hold)?;

                let value = amount * hold / liquidity;

                let value = Self::try_into(value)?;

                if value < One::one() {
                    continue;
                }

                T::Assets::mint_into(token_id, &holder, value)?;
            }
        }

        Ok(weight)
    }

    fn try_into<S, D>(value: S) -> Result<D, Error<T>>
    where
        S: TryInto<u128>,
        D: TryFrom<u128>,
    {
        let value: u128 = value.try_into().map_err(|_| Error::<T>::Overflow)?;

        value.try_into().map_err(|_| Error::<T>::Overflow)
    }
}
