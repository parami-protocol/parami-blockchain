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

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{
        tokens::{
            fungibles::{
                metadata::Mutate as FungMetaMutate, Create as FungCreate, Mutate as FungMutate,
                Transfer as FungTransfer,
            },
            nonfungibles::{Create as NftCreate, Mutate as NftMutate},
        },
        Currency, EnsureOrigin,
        ExistenceRequirement::KeepAlive,
        Get,
    },
};
use parami_did::EnsureDid;
use parami_traits::Swaps;
use sp_core::U512;
use sp_runtime::{
    traits::{AccountIdConversion, AtLeast32BitUnsigned, Bounded, CheckedAdd, One, Saturating},
    DispatchError, RuntimeDebug,
};
use sp_std::{
    convert::{TryFrom, TryInto},
    prelude::*,
};

use types::*;
use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type AssetOf<T> = <T as Config>::AssetId;
type BalanceOf<T> = <<T as parami_did::Config>::Currency as Currency<AccountOf<T>>>::Balance;
type DidOf<T> = <T as parami_did::Config>::DecentralizedId;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
pub type NftIdOf<T> = AssetOf<T>;
pub type NftMetaFor<T> = NftMeta<DidOf<T>, AccountOf<T>, NftIdOf<T>, AssetOf<T>>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + parami_did::Config {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Fragments (fungible token) ID type
        type AssetId: Parameter
            + Member
            + MaybeSerializeDeserialize
            + AtLeast32BitUnsigned
            + Default
            + Bounded
            + Copy;

        /// The assets trait to create, mint, and transfer fragments (fungible token)
        type Assets: FungCreate<AccountOf<Self>, AssetId = AssetOf<Self>>
            + FungMetaMutate<AccountOf<Self>, AssetId = AssetOf<Self>>
            + FungMutate<AccountOf<Self>, AssetId = AssetOf<Self>, Balance = BalanceOf<Self>>
            + FungTransfer<AccountOf<Self>, AssetId = AssetOf<Self>, Balance = BalanceOf<Self>>;

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
        type Nft: NftCreate<AccountOf<Self>, InstanceId = NftIdOf<Self>, ClassId = NftIdOf<Self>>
            + NftMutate<AccountOf<Self>, InstanceId = NftIdOf<Self>, ClassId = NftIdOf<Self>>;

        /// The maximum length of a name or symbol stored on-chain.
        /// TODO(ironman_ch): Why define it as a Get<u32> instead of u32 ?
        #[pallet::constant]
        type StringLimit: Get<u32>;

        /// The swaps trait
        type Swaps: Swaps<
            AccountId = AccountOf<Self>,
            AssetId = AssetOf<Self>,
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
    pub(super) type Deposit<T: Config> = StorageMap<_, Twox64Concat, NftIdOf<T>, BalanceOf<T>>;

    /// Deposits by supporter in pot
    #[pallet::storage]
    #[pallet::getter(fn deposits)]
    pub(super) type Deposits<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        NftIdOf<T>,
        Identity,
        T::DecentralizedId, // Supporter
        BalanceOf<T>,
    >;

    /// Nft's Metadata
    #[pallet::storage]
    pub(super) type NftMetaStore<T: Config> = StorageMap<
        _,
        Twox64Concat,
        NftIdOf<T>, //
        NftMetaFor<T>,
    >;

    /// Did's preferred Nft.
    #[pallet::storage]
    #[pallet::getter(fn preferred_nft_of)]
    pub(super) type PreferredNft<T: Config> = StorageMap<
        _,
        Identity,
        T::DecentralizedId, //
        NftIdOf<T>,
    >;

    /// Initial Minting date
    #[pallet::storage]
    #[pallet::getter(fn date)]
    pub(super) type Date<T: Config> = StorageMap<_, Twox64Concat, NftIdOf<T>, HeightOf<T>>;

    #[pallet::type_value]
    pub(crate) fn InitNftId<T: Config>() -> NftIdOf<T> {
        NftIdOf::<T>::one()
    }

    /// Next available class ID
    #[pallet::storage]
    #[pallet::getter(fn next_cid)]
    pub(super) type NextNftId<T: Config> = StorageValue<_, NftIdOf<T>, ValueQuery, InitNftId<T>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// NFT fragments Minted \[did, kol, value\]
        Backed(
            T::DecentralizedId,
            T::DecentralizedId,
            NftIdOf<T>,
            BalanceOf<T>,
        ),
        /// NFT fragments Claimed \[did, NftInstanceId, value\]
        Claimed(T::DecentralizedId, NftIdOf<T>, BalanceOf<T>),
        /// NFT fragments Minted \[kol, instance, name, symbol, tokens\]
        Minted(
            T::DecentralizedId,
            NftIdOf<T>,
            Vec<u8>,
            Vec<u8>,
            BalanceOf<T>,
        ),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<HeightOf<T>> for Pallet<T> {}

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
        /// Back (support) the KOL.
        #[pallet::weight(<T as Config>::WeightInfo::back())]
        pub fn back(
            origin: OriginFor<T>,
            kol: T::DecentralizedId,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, who) = EnsureDid::<T>::ensure_origin(origin)?;

            ensure!(kol != did, Error::<T>::YourSelf);

            let instance_id = Self::get_or_create_preferred_nft(&kol)?;

            let meta = <NftMetaStore<T>>::get(instance_id).ok_or(Error::<T>::NotExists)?;

            ensure!(!meta.minted, Error::<T>::Minted);

            <T as parami_did::Config>::Currency::transfer(&who, &meta.pot, value, KeepAlive)?;

            <Deposit<T>>::mutate(instance_id, |maybe| {
                if let Some(deposit) = maybe {
                    deposit.saturating_accrue(value);
                } else {
                    *maybe = Some(value);
                }
            });

            <Deposits<T>>::mutate(instance_id, &did, |maybe| {
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
        /// TODO(ironman_ch): add tests for one creator mint multi nft.
        #[pallet::weight(<T as Config>::WeightInfo::mint(name.len() as u32, symbol.len() as u32))]
        pub fn mint(origin: OriginFor<T>, name: Vec<u8>, symbol: Vec<u8>) -> DispatchResult {
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
            let instance_id = Self::get_or_create_preferred_nft(&did)?;

            let mut meta = <NftMetaStore<T>>::get(instance_id).ok_or(Error::<T>::NotExists)?;
            ensure!(!meta.minted, Error::<T>::Minted);

            let deposit = <T as parami_did::Config>::Currency::free_balance(&meta.pot);

            let init = T::InitialMintingDeposit::get();
            ensure!(deposit >= init, Error::<T>::InsufficientBalance);

            // 2. create NFT token
            let tid = instance_id;

            T::Nft::create_class(&meta.class_id, &meta.pot, &meta.pot)?;
            T::Nft::mint_into(&meta.class_id, &instance_id, &meta.pot)?;

            // 3. initial minting

            let initial = T::InitialMintingValueBase::get();

            T::Assets::create(tid, meta.pot.clone(), true, One::one())?;
            T::Assets::set(tid, &meta.pot, name.clone(), symbol.clone(), 18)?;
            T::Assets::mint_into(tid, &meta.pot, initial.saturating_mul(3u32.into()))?;

            // 4. transfer third of initial minting to swap

            T::Swaps::new(tid)?;
            T::Swaps::mint(meta.pot.clone(), tid, deposit, deposit, initial, false)?;

            // 5. update local variable
            meta.minted = true;

            // 6. update storage
            <NftMetaStore<T>>::insert(instance_id, meta);

            <Date<T>>::insert(instance_id, minted);

            <Deposits<T>>::mutate(instance_id, &did, |maybe| {
                *maybe = Some(deposit);
            });

            Self::deposit_event(Event::Minted(did, instance_id, name, symbol, initial));

            Ok(())
        }

        /// Claim the fragments.
        #[pallet::weight(<T as Config>::WeightInfo::claim())]
        pub fn claim(origin: OriginFor<T>, kol: T::DecentralizedId) -> DispatchResult {
            let (did, who) = EnsureDid::<T>::ensure_origin(origin)?;

            let height = <frame_system::Pallet<T>>::block_number();

            let nft_id = Self::get_or_create_preferred_nft(&kol)?;
            let meta = <NftMetaStore<T>>::get(nft_id).ok_or(Error::<T>::NotExists)?;

            if meta.owner == did {
                let minted_block_number = <Date<T>>::get(nft_id).ok_or(Error::<T>::NotExists)?;
                ensure!(
                    height - minted_block_number >= T::InitialMintingLockupPeriod::get(),
                    Error::<T>::NoToken
                );
            }

            let total = <Deposit<T>>::get(nft_id).ok_or(Error::<T>::NotExists)?;
            let deposit = <Deposits<T>>::get(nft_id, &did).ok_or(Error::<T>::NoToken)?;
            let initial = T::InitialMintingValueBase::get();

            let total: U512 = Self::try_into(total)?;
            let deposit: U512 = Self::try_into(deposit)?;
            let initial: U512 = Self::try_into(initial)?;

            let tokens = initial * deposit / total;

            let tokens = Self::try_into(tokens)?;

            T::Assets::transfer(nft_id, &meta.pot, &who, tokens, false)?;

            <Deposits<T>>::remove(nft_id, &did);

            Self::deposit_event(Event::Claimed(did, nft_id, tokens));

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn get_or_create_preferred_nft(
            kol: &T::DecentralizedId,
        ) -> Result<NftIdOf<T>, DispatchError> {
            let preferred_nft_id_op = <PreferredNft<T>>::get(&kol);

            if let Some(nft_id) = preferred_nft_id_op {
                Ok(nft_id)
            } else {
                let nft_id =
                    NextNftId::<T>::try_mutate(|id| -> Result<NftIdOf<T>, DispatchError> {
                        let current_id = *id;
                        *id = id.checked_add(&One::one()).ok_or(Error::<T>::Overflow)?;
                        Ok(current_id)
                    })?;

                let meta = NftMetaFor::<T> {
                    owner: *kol,
                    pot: T::PalletId::get().into_sub_account(&kol),
                    class_id: nft_id,
                    token_asset_id: nft_id,
                    minted: false,
                };
                <NftMetaStore<T>>::insert(nft_id, meta);

                <PreferredNft<T>>::insert(&kol, nft_id);
                Ok(nft_id)
            }
        }

        /// get_preferred
        /// return preferred_instance_id of KOL if exists;
        /// return 0 otherwise;
        pub fn get_preferred(kol: T::DecentralizedId) -> Option<NftIdOf<T>> {
            <PreferredNft<T>>::get(&kol)
        }

        pub fn get_meta_of(nft_id: NftIdOf<T>) -> Option<NftMetaFor<T>> {
            <NftMetaStore<T>>::get(nft_id)
        }

        pub fn is_nft_minted(nft_id: NftIdOf<T>) -> bool {
            <NftMetaStore<T>>::get(nft_id)
                .map(|meta| meta.minted)
                .unwrap_or(false)
        }

        pub fn zero() -> NftIdOf<T> {
            Default::default()
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub deposit: Vec<(NftIdOf<T>, BalanceOf<T>)>,
        pub deposits: Vec<(NftIdOf<T>, T::DecentralizedId, BalanceOf<T>)>,
        pub next_instance_id: NftIdOf<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                deposit: Default::default(),
                deposits: Default::default(),
                next_instance_id: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <NextNftId<T>>::put(self.next_instance_id);

            let next_class_id: u32 = self.next_instance_id.try_into().unwrap_or_default();
            if next_class_id > 0 {
                for token in 0u32..next_class_id {
                    let token: NftIdOf<T> = token.into();
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
    fn try_into<S, D>(value: S) -> Result<D, DispatchError>
    where
        S: TryInto<u128>,
        D: TryFrom<u128>,
    {
        let value: u128 = value.try_into().map_err(|_| Error::<T>::Overflow)?;

        let value: D = value.try_into().map_err(|_| Error::<T>::Overflow)?;

        Ok(value)
    }
}
