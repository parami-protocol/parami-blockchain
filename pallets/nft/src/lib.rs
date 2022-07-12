#![cfg_attr(not(feature = "std"), no_std)]

pub use ocw::eth_abi;
pub use pallet::*;

#[rustfmt::skip]
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod migrations;
mod ocw;
mod types;

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    traits::{
        tokens::{
            fungibles::{
                metadata::Mutate as FungMetaMutate, Create as FungCreate, Inspect,
                Mutate as FungMutate, Transfer as FungTransfer,
            },
            nonfungibles::{Create as NftCreate, Mutate as NftMutate},
        },
        Currency, EnsureOrigin,
        ExistenceRequirement::KeepAlive,
        Get, StorageVersion,
    },
    PalletId,
};
use frame_system::offchain::SendTransactionTypes;
use parami_did::EnsureDid;
use parami_traits::{
    types::{Network, Task},
    Links, Nfts, Swaps,
};
use sp_core::U512;
use sp_runtime::{
    traits::{
        AccountIdConversion, AtLeast32BitUnsigned, Bounded, CheckedAdd, One, Saturating, Zero,
    },
    DispatchError, RuntimeDebug,
};
use sp_std::{
    convert::{TryFrom, TryInto},
    prelude::*,
};

use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type AssetOf<T> = <T as Config>::AssetId;
type BalanceOf<T> = <<T as parami_did::Config>::Currency as Currency<AccountOf<T>>>::Balance;
type DidOf<T> = <T as parami_did::Config>::DecentralizedId;
type ExternalOf<T> = types::External<DidOf<T>>;
type HeightOf<T> = <T as frame_system::Config>::BlockNumber;
type MetaOf<T> = types::Metadata<DidOf<T>, AccountOf<T>, NftOf<T>, AssetOf<T>>;
type NftOf<T> = <T as Config>::AssetId;
type TaskOf<T> = Task<ExternalOf<T>, HeightOf<T>>;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + parami_did::Config
        + parami_ocw::Config
        + SendTransactionTypes<Call<Self>>
    {
        /// The overarching event type
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Fragments (fungible token) ID type
        type AssetId: Parameter
            + Member
            + MaybeSerializeDeserialize
            + AtLeast32BitUnsigned
            + Default
            + Bounded
            + Copy
            + MaxEncodedLen;

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

        /// Unsigned Call Priority
        #[pallet::constant]
        type UnsignedPriority: Get<TransactionPriority>;

        /// The links trait
        type Links: Links<DidOf<Self>>;

        /// The NFT trait to create, mint non-fungible token
        type Nft: NftCreate<AccountOf<Self>, ItemId = NftOf<Self>, CollectionId = NftOf<Self>>
            + NftMutate<AccountOf<Self>, ItemId = NftOf<Self>, CollectionId = NftOf<Self>>;

        /// The pallet id, used for deriving "pot" accounts to receive donation
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Lifetime of a pending task
        #[pallet::constant]
        type PendingLifetime: Get<HeightOf<Self>>;

        /// The maximum length of a name or symbol stored on-chain.
        #[pallet::constant]
        type StringLimit: Get<u32>;

        /// The swaps trait
        type Swaps: Swaps<
            AccountOf<Self>,
            AssetId = AssetOf<Self>,
            QuoteBalance = BalanceOf<Self>,
            TokenBalance = BalanceOf<Self>,
        >;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    #[pallet::generate_store(pub(super) trait Store)]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// Total deposit in pot
    #[pallet::storage]
    pub(super) type Deposit<T: Config> = StorageMap<_, Twox64Concat, NftOf<T>, BalanceOf<T>>;

    /// Deposits by supporter in pot
    #[pallet::storage]
    pub(super) type Deposits<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat,
        NftOf<T>,
        Identity,
        T::DecentralizedId, // Supporter
        BalanceOf<T>,
    >;

    /// Importing in progress
    #[pallet::storage]
    pub(super) type Porting<T: Config> = StorageNMap<
        _,
        (
            NMapKey<Twox64Concat, Network>,
            NMapKey<Blake2_128, Vec<u8>>, // Namespace
            NMapKey<Blake2_128, Vec<u8>>, // Token
        ),
        TaskOf<T>,
    >;

    /// Ported NFTs
    #[pallet::storage]
    pub(super) type Ported<T: Config> = StorageNMap<
        _,
        (
            NMapKey<Twox64Concat, Network>,
            NMapKey<Blake2_128, Vec<u8>>, // Namespace
            NMapKey<Blake2_128, Vec<u8>>, // Token
        ),
        NftOf<T>,
    >;

    /// Imported NFTs
    #[pallet::storage]
    pub(super) type External<T: Config> = StorageMap<_, Twox64Concat, NftOf<T>, ExternalOf<T>>;

    /// Metadata
    #[pallet::storage]
    #[pallet::getter(fn meta)]
    pub(super) type Metadata<T: Config> = StorageMap<_, Twox64Concat, NftOf<T>, MetaOf<T>>;

    /// Did's preferred Nft.
    #[pallet::storage]
    #[pallet::getter(fn preferred)]
    pub(super) type Preferred<T: Config> = StorageMap<_, Identity, T::DecentralizedId, NftOf<T>>;

    /// Initial Minting date
    #[pallet::storage]
    pub(super) type Date<T: Config> = StorageMap<_, Twox64Concat, NftOf<T>, HeightOf<T>>;

    #[pallet::type_value]
    pub(crate) fn DefaultId<T: Config>() -> NftOf<T> {
        One::one()
    }

    /// Next available class ID
    #[pallet::storage]
    pub(super) type NextClassId<T: Config> = StorageValue<_, NftOf<T>, ValueQuery, DefaultId<T>>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// NFT Created \[did, instance\]
        Created(T::DecentralizedId, NftOf<T>),
        /// NFT fragments Minted \[did, instance, value\]
        Backed(T::DecentralizedId, NftOf<T>, BalanceOf<T>),
        /// NFT fragments Claimed \[did, instance, value\]
        Claimed(T::DecentralizedId, NftOf<T>, BalanceOf<T>),
        /// NFT fragments Minted \[kol, instance, token, name, symbol, tokens\]
        Minted(
            T::DecentralizedId,
            NftOf<T>,
            AssetOf<T>,
            Vec<u8>,
            Vec<u8>,
            BalanceOf<T>,
        ),
        /// Import NFT Failed \[did, network, namespace, token_id\]
        ImportFailed(T::DecentralizedId, Network, Vec<u8>, Vec<u8>),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn offchain_worker(block_number: BlockNumberFor<T>) {
            match Self::ocw_begin_block(block_number) {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("An error occurred in OCW: {:?}", e);
                }
            }
        }

        fn on_runtime_upgrade() -> Weight {
            migrations::migrate::<T>()
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        BadMetadata,
        Deadline,
        Exists,
        InsufficientBalance,
        Minted,
        NotExists,
        Overflow,
        YourSelf,
        NetworkNotLinked,
        OcwParseError,
        NotTokenOwner,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Import an existing NFT for crowdfunding.
        #[pallet::weight(<T as Config>::WeightInfo::port())]
        pub fn port(
            origin: OriginFor<T>,
            network: Network,
            namespace: Vec<u8>,
            token: Vec<u8>,
        ) -> DispatchResult {
            let (owner, _) = EnsureDid::<T>::ensure_origin(origin)?;

            ensure!(
                !<Porting<T>>::contains_key((network, &namespace, &token)),
                Error::<T>::Exists
            );

            ensure!(
                !<Ported<T>>::contains_key((network, &namespace, &token)),
                Error::<T>::Exists
            );

            // user should link network first
            ensure!(
                T::Links::all_links(&owner).contains_key(&network),
                Error::<T>::NetworkNotLinked
            );

            let created = <frame_system::Pallet<T>>::block_number();
            let lifetime = T::PendingLifetime::get();
            let deadline = created.saturating_add(lifetime);

            <Porting<T>>::insert(
                (network, &namespace.clone(), &token.clone()),
                Task {
                    task: types::External {
                        owner,
                        network,
                        namespace,
                        token,
                    },
                    deadline,
                    created,
                },
            );

            Ok(())
        }

        /// Create a new NFT for crowdfunding.
        #[pallet::weight(<T as Config>::WeightInfo::kick())]
        pub fn kick(origin: OriginFor<T>) -> DispatchResult {
            let (owner, _) = EnsureDid::<T>::ensure_origin(origin)?;

            Self::create(owner)?;

            Ok(())
        }

        /// Back (support) the KOL.
        #[pallet::weight(<T as Config>::WeightInfo::back())]
        pub fn back(
            origin: OriginFor<T>,
            nft: NftOf<T>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResult {
            let (did, who) = EnsureDid::<T>::ensure_origin(origin)?;

            let meta = <Metadata<T>>::get(nft).ok_or(Error::<T>::NotExists)?;

            ensure!(meta.owner != did, Error::<T>::YourSelf);

            ensure!(!meta.minted, Error::<T>::Minted);

            T::Currency::transfer(&who, &meta.pot, value, KeepAlive)?;

            <Deposit<T>>::mutate(nft, |maybe| {
                if let Some(deposit) = maybe {
                    deposit.saturating_accrue(value);
                } else {
                    *maybe = Some(value);
                }
            });

            <Deposits<T>>::mutate(nft, &did, |maybe| {
                if let Some(deposit) = maybe {
                    deposit.saturating_accrue(value);
                } else {
                    *maybe = Some(value);
                }
            });

            Self::deposit_event(Event::Backed(did, nft, value));

            Ok(())
        }

        /// Fragment the NFT and mint token.
        /// TODO(ironman_ch): add tests for one creator mint multi nft.
        #[pallet::weight(<T as Config>::WeightInfo::mint(name.len() as u32, symbol.len() as u32))]
        pub fn mint(
            origin: OriginFor<T>,
            nft: NftOf<T>,
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
            let mut meta = <Metadata<T>>::get(nft).ok_or(Error::<T>::NotExists)?;
            ensure!(!meta.minted, Error::<T>::Minted);

            let deposit = T::Currency::free_balance(&meta.pot);

            let init = T::InitialMintingDeposit::get();
            ensure!(deposit >= init, Error::<T>::InsufficientBalance);

            // 2. create NFT token
            let tid = nft;

            T::Nft::create_collection(&meta.class_id, &meta.pot, &meta.pot)?;
            T::Nft::mint_into(&meta.class_id, &nft, &meta.pot)?;

            // 3. initial minting

            let initial = T::InitialMintingValueBase::get();
            let supply = initial.saturating_mul(3u32.into());

            T::Assets::create(tid, meta.pot.clone(), true, One::one())?;
            T::Assets::set(tid, &meta.pot, name.clone(), symbol.clone(), 18)?;
            T::Assets::mint_into(tid, &meta.pot, supply)?;

            // 4. transfer third of initial minting to swap

            T::Swaps::new(tid)?;
            T::Swaps::mint(meta.pot.clone(), tid, deposit, deposit, initial, false)?;

            // 5. update local variable
            meta.minted = true;

            // 6. update storage
            <Metadata<T>>::insert(nft, meta);

            <Date<T>>::insert(nft, minted);

            <Deposits<T>>::mutate(nft, &did, |maybe| {
                *maybe = Some(deposit);
            });

            Self::deposit_event(Event::Minted(did, nft, tid, name, symbol, supply));

            Ok(())
        }

        /// Claim the fragments.
        #[pallet::weight(<T as Config>::WeightInfo::claim())]
        pub fn claim(origin: OriginFor<T>, nft: NftOf<T>) -> DispatchResult {
            let (did, who) = EnsureDid::<T>::ensure_origin(origin)?;

            let height = <frame_system::Pallet<T>>::block_number();

            let meta = <Metadata<T>>::get(nft).ok_or(Error::<T>::NotExists)?;

            if meta.owner == did {
                let minted_block_number = <Date<T>>::get(nft).ok_or(Error::<T>::NotExists)?;
                ensure!(
                    height - minted_block_number >= T::InitialMintingLockupPeriod::get(),
                    Error::<T>::NotExists
                );
            }

            let total = <Deposit<T>>::get(nft).ok_or(Error::<T>::NotExists)?;
            let deposit = <Deposits<T>>::get(nft, &did).ok_or(Error::<T>::NotExists)?;
            let initial = T::InitialMintingValueBase::get();

            let total: U512 = Self::try_into(total)?;
            let deposit: U512 = Self::try_into(deposit)?;
            let initial: U512 = Self::try_into(initial)?;

            let tokens = initial * deposit / total;

            let tokens = Self::try_into(tokens)?;

            T::Assets::transfer(meta.token_asset_id, &meta.pot, &who, tokens, false)?;

            <Deposits<T>>::remove(nft, &did);

            Self::deposit_event(Event::Claimed(did, nft, tokens));

            Ok(())
        }

        #[pallet::weight(<T as Config>::WeightInfo::submit_porting())]
        pub fn submit_porting(
            origin: OriginFor<T>,
            did: DidOf<T>,
            network: Network,
            namespace: Vec<u8>,
            token: Vec<u8>,
            validated: bool,
        ) -> DispatchResultWithPostInfo {
            ensure_none(origin)?;

            let task = <Porting<T>>::get((network, &namespace, &token));

            ensure!(task.is_some(), Error::<T>::NotExists);

            let task = task.unwrap();

            if validated {
                let id = Self::create(task.task.owner)?;

                <Ported<T>>::insert((network, namespace.clone(), token.clone()), id);

                <External<T>>::insert(
                    id,
                    types::External {
                        network,
                        namespace: namespace.clone(),
                        token: token.clone(),
                        owner: task.task.owner,
                    },
                );
            } else {
                Self::deposit_event(Event::ImportFailed(
                    did,
                    network,
                    namespace.clone(),
                    token.clone(),
                ));
            }

            <Porting<T>>::remove((network, namespace, token));
            Ok(().into())
        }
    }

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub deposit: Vec<(NftOf<T>, BalanceOf<T>)>,
        pub deposits: Vec<(NftOf<T>, T::DecentralizedId, BalanceOf<T>)>,
        pub next_instance_id: NftOf<T>,
        pub nfts: Vec<(NftOf<T>, T::DecentralizedId, bool)>,
        pub externals: Vec<(NftOf<T>, Network, Vec<u8>, Vec<u8>, T::DecentralizedId)>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self {
                deposit: Default::default(),
                deposits: Default::default(),
                next_instance_id: Default::default(),
                nfts: Default::default(),
                externals: Default::default(),
            }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <NextClassId<T>>::put(self.next_instance_id);

            for (instance_id, deposit) in &self.deposit {
                <Deposit<T>>::insert(instance_id, deposit);
            }

            for (instance_id, did, deposit) in &self.deposits {
                <Deposits<T>>::insert(instance_id, did, deposit);
            }

            for (id, owner, minted) in &self.nfts {
                let id = *id;
                let minted = *minted;

                if id >= self.next_instance_id {
                    panic!("NFT ID must be less than next_instance_id");
                }

                let pot: AccountOf<T> = T::PalletId::get().into_sub_account_truncating(owner);

                <Metadata<T>>::insert(
                    id,
                    types::Metadata {
                        owner: owner.clone(),
                        pot: pot.clone(),
                        class_id: id,
                        token_asset_id: id,
                        minted,
                    },
                );

                <Preferred<T>>::insert(owner, id);

                if minted {
                    // MARK: pallet_uniques does not support genesis
                    T::Nft::create_collection(&id, &pot, &pot).unwrap();
                    T::Nft::mint_into(&id, &id, &pot).unwrap();

                    <Date<T>>::insert(id, HeightOf::<T>::zero());
                }
            }

            for (id, network, namespace, token, owner) in &self.externals {
                let id = *id;
                let network = *network;
                let owner = *owner;

                <Ported<T>>::insert((network, namespace.clone(), token.clone()), id);

                <External<T>>::insert(
                    id,
                    types::External {
                        network,
                        namespace: namespace.clone(),
                        token: token.clone(),
                        owner,
                    },
                );
            }
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
                ValidTransaction::with_tag_prefix("nft")
                    .priority(T::UnsignedPriority::get())
                    .and_provides([&provide])
                    .longevity(3)
                    .propagate(false)
                    .build()
            };

            match call {
                Call::submit_porting { .. } => valid_tx(b"submit_porting".to_vec()),
                _ => InvalidTransaction::Call.into(),
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    fn create(owner: DidOf<T>) -> Result<NftOf<T>, DispatchError> {
        let id = <NextClassId<T>>::try_mutate(|id| -> Result<NftOf<T>, DispatchError> {
            let current_id = *id;
            *id = id.checked_add(&One::one()).ok_or(Error::<T>::Overflow)?;
            Ok(current_id)
        })?;

        let pot = T::PalletId::get().into_sub_account_truncating(&owner);

        ensure!(!<Metadata<T>>::contains_key(id), Error::<T>::Exists);

        <Metadata<T>>::insert(
            id,
            types::Metadata {
                owner,
                pot,
                class_id: id,
                token_asset_id: id,
                minted: false,
            },
        );

        if !<Preferred<T>>::contains_key(&owner) {
            <Preferred<T>>::insert(&owner, id);
        }

        Self::deposit_event(Event::Created(owner, id));

        Ok(id)
    }

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

impl<T: Config> Nfts<T::AccountId> for Pallet<T> {
    fn force_transfer_all_fractions(
        src: &T::AccountId,
        dest: &T::AccountId,
    ) -> Result<(), DispatchError> {
        for (_nft_id, nft_meta) in <Metadata<T>>::iter() {
            let balance = T::Assets::balance(nft_meta.token_asset_id, &src);
            T::Assets::transfer(nft_meta.token_asset_id, src, dest, balance, false)?;
        }

        Ok(())
    }
}
