#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub use types::*;

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
    dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo},
    ensure,
    traits::{
        Currency,
        ExistenceRequirement::{AllowDeath, KeepAlive},
        Get, ReservableCurrency,
    },
    PalletId,
};
use orml_nft::Pallet as OrmlNft;
use sp_runtime::traits::{
    AccountIdConversion, AtLeast32BitUnsigned, CheckedAdd, StaticLookup, Zero,
};
use sp_std::prelude::*;

use weights::WeightInfo;

pub trait AssetHandler<AssetId> {
    fn check_item_in_auction(asset_id: AssetId) -> bool;
}

type ClassIdOf<T> = <T as orml_nft::Config>::ClassId;
type TokenIdOf<T> = <T as orml_nft::Config>::TokenId;
type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type BalanceOfAsset<T> = <T as pallet_assets::Config>::Balance;

const MIN_BALANCE: u128 = 1;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + pallet_assets::Config
        + orml_nft::Config<
            TokenData = AssetData<BalanceOf<Self>>,
            ClassData = ClassData<BalanceOf<Self>>,
        >
    {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Minimum deposit to create class
        #[pallet::constant]
        type CreateClassDeposit: Get<BalanceOf<Self>>;

        /// Minimum deposit to create a NFT
        #[pallet::constant]
        type CreateAssetDeposit: Get<BalanceOf<Self>>;

        /// The currency trait
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

        /// NFT Pallet Id
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// Asset handler
        type AssetsHandler: AssetHandler<<Self as pallet_assets::Config>::AssetId>;

        /// Weight info
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    #[pallet::getter(fn get_ads_slot)]
    pub(super) type AdsSlots<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AssetId,
        AdsSlot<T::AccountId, BalanceOfAsset<T>, T::BlockNumber>,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn next_asset_id)]
    pub(super) type NextAssetId<T: Config> = StorageValue<_, T::AssetId, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_asset)]
    pub(super) type Assets<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, (ClassIdOf<T>, TokenIdOf<T>), OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_assets_by_owner)]
    pub(super) type AssetsByOwner<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, Vec<T::AssetId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_asset_supporters)]
    pub(super) type AssetSupporters<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, Vec<T::AccountId>, OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// NFT Class created
        NftClassCreated(T::AccountId, ClassIdOf<T>),

        /// Class destroyed
        ClassDestroyed(T::AccountId, ClassIdOf<T>),

        /// NFT minted
        NftMinted(
            T::AssetId,
            T::AssetId,
            T::AccountId,
            ClassIdOf<T>,
            u32,
            TokenIdOf<T>,
        ),

        /// NFT transferred
        NftTransfered(T::AccountId, T::AccountId, TokenIdOf<T>),

        /// NFT burned
        NftBurned(T::AccountId, ClassIdOf<T>, TokenIdOf<T>),

        /// NFT signed
        SignedNft(TokenIdOf<T>, T::AccountId),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        /// No permission
        NoPermission,
        /// Class Id not found
        ClassIdNotFound,
        /// Can not destroy class
        CannotDestroyClass,
        /// Asset Id not found
        AssetIdNotFound,
        /// Asset Info not found
        AssetInfoNotFound,
        /// Asset Id already exists
        AssetIdAlreadyExists,
        /// Asset Id is currently in an auction
        AssetAlreadyInAuction,
        /// Non Transferable
        NonTransferable,
        /// Invalid quantity
        InvalidQuantity,
        /// exceeds quantity
        QuantityExceeds,
        /// No available asset id
        NoAvailableAssetId,
        /// Cannot be burned
        CannotBeBurned,
        /// Sign your own Asset
        SignOwnAsset,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <<T as pallet::Config>::Currency as frame_support::traits::Currency<
            <T as frame_system::Config>::AccountId,
        >>::Balance: From<u128> + Into<u128>,
        <T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
        <T as pallet_assets::Config>::AssetId: AtLeast32BitUnsigned,
    {
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn create_class(
            origin: OriginFor<T>,
            metadata: Vec<u8>,
            token_type: TokenType,
            collection_type: CollectionType,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let next_class_id = OrmlNft::<T>::next_class_id();

            let class_pool_account: T::AccountId =
                T::PalletId::get().into_sub_account(next_class_id);
            let class_deposit = T::CreateClassDeposit::get();

            <T as Config>::Currency::transfer(
                &sender,
                &class_pool_account,
                class_deposit,
                KeepAlive,
            )?;
            <T as Config>::Currency::reserve(
                &class_pool_account,
                <T as Config>::Currency::free_balance(&class_pool_account),
            )?;

            let class_data = ClassData {
                deposit: class_deposit,
                token_type,
                collection_type,
                metadata: metadata.clone(),
            };

            OrmlNft::<T>::create_class(&sender, metadata, class_data)?;

            Self::deposit_event(Event::<T>::NftClassCreated(sender, next_class_id));

            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn destroy_class(
            origin: OriginFor<T>,
            class_id: ClassIdOf<T>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;
            let class_detail =
                OrmlNft::<T>::classes(class_id).ok_or(Error::<T>::ClassIdNotFound)?;

            ensure!(sender == class_detail.owner, Error::<T>::NoPermission);
            ensure!(
                class_detail.total_issuance == Zero::zero(),
                Error::<T>::CannotDestroyClass
            );

            let data = class_detail.data;
            ensure!(
                data.token_type != TokenType::BoundToAddress,
                Error::<T>::CannotDestroyClass
            );

            let class_pool_account: T::AccountId = T::PalletId::get().into_sub_account(class_id);
            <T as Config>::Currency::unreserve(&class_pool_account, data.deposit);

            OrmlNft::<T>::destroy_class(&sender, class_id)?;

            // refund reserved tokens
            <T as Config>::Currency::transfer(
                &class_pool_account,
                &sender,
                <T as Config>::Currency::free_balance(&class_pool_account),
                AllowDeath,
            )?;

            Self::deposit_event(Event::ClassDestroyed(sender, class_id));
            Ok(().into())
        }

        #[pallet::weight(< T as Config >::WeightInfo::mint(* quantity))]
        pub fn mint(
            origin: OriginFor<T>,
            class_id: ClassIdOf<T>,
            name: Vec<u8>,
            symbol: Vec<u8>,
            description: Vec<u8>,
            metadata: Vec<u8>,
            quantity: u32,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin.clone())?;

            ensure!(quantity >= 1, Error::<T>::InvalidQuantity);
            let class_detail =
                OrmlNft::<T>::classes(class_id).ok_or(Error::<T>::ClassIdNotFound)?;
            ensure!(sender == class_detail.owner, Error::<T>::NoPermission);

            let deposit = T::CreateAssetDeposit::get();
            let class_pool_account: T::AccountId = T::PalletId::get().into_sub_account(class_id);
            let total_deposit = deposit * Into::<BalanceOf<T>>::into(quantity);

            let data = class_detail.data;
            if data.token_type == TokenType::BoundToAddress {
                ensure!(quantity < 2, Error::<T>::QuantityExceeds);
            }

            <T as Config>::Currency::transfer(
                &sender,
                &class_pool_account,
                total_deposit,
                KeepAlive,
            )?;
            <T as Config>::Currency::reserve(&class_pool_account, total_deposit)?;

            let new_nft_data = AssetData {
                deposit,
                name: name.clone(),
                description,
            };

            let mut new_asset_ids: Vec<T::AssetId> = Vec::new();
            let mut last_token_id: TokenIdOf<T> = Default::default();

            for _ in 0..quantity {
                let asset_id =
                    NextAssetId::<T>::try_mutate(|id| -> Result<T::AssetId, DispatchError> {
                        let current_id = *id;
                        *id = id
                            .checked_add(&(Into::into(1u32)))
                            .ok_or(Error::<T>::NoAvailableAssetId)?;

                        Ok(current_id)
                    })?;

                new_asset_ids.push(asset_id);

                if AssetsByOwner::<T>::contains_key(&sender) {
                    AssetsByOwner::<T>::try_mutate(&sender, |asset_ids| -> DispatchResult {
                        ensure!(
                            !asset_ids.iter().any(|i| asset_id == *i),
                            Error::<T>::AssetIdAlreadyExists
                        );
                        asset_ids.push(asset_id);
                        Ok(())
                    })?;
                } else {
                    let mut assets = Vec::<T::AssetId>::new();
                    assets.push(asset_id);
                    AssetsByOwner::<T>::insert(&sender, assets)
                }

                let nft_token_id =
                    OrmlNft::<T>::mint(&sender, class_id, metadata.clone(), new_nft_data.clone())?;
                Assets::<T>::insert(asset_id, (class_id, nft_token_id));
                last_token_id = nft_token_id;

                // generate nft fraction
                if data.token_type == TokenType::BoundToAddress {
                    // set ads slot
                    let ads_slot = AdsSlot {
                        start_time: Zero::zero(),
                        end_time: Zero::zero(),
                        deposit: Zero::zero(),
                        media: Vec::new(),
                        owner: Default::default(),
                    };
                    AdsSlots::<T>::insert(asset_id, ads_slot);

                    // create fraction
                    <pallet_assets::Pallet<T>>::create(
                        origin.clone(),
                        asset_id,
                        <T::Lookup as StaticLookup>::unlookup(sender.clone()),
                        MIN_BALANCE.into(),
                    )?;
                    // set metadata
                    <pallet_assets::Pallet<T>>::set_metadata(
                        origin.clone(),
                        asset_id,
                        name.clone(),
                        symbol.clone(),
                        18,
                    )?;
                    // initial mint
                    <pallet_assets::Pallet<T>>::mint(
                        origin.clone(),
                        asset_id,
                        <T::Lookup as StaticLookup>::unlookup(sender.clone()),
                        100000000000000000000000000u128.into(),
                    )?;

                    // log::info!("name => {:?}", name);
                }
            }

            Self::deposit_event(Event::<T>::NftMinted(
                *new_asset_ids.first().unwrap(),
                *new_asset_ids.last().unwrap(),
                sender,
                class_id,
                quantity,
                last_token_id,
            ));

            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn burn(origin: OriginFor<T>, token: (ClassIdOf<T>, TokenIdOf<T>)) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let nft_info =
                OrmlNft::<T>::tokens(token.0, token.1).ok_or(Error::<T>::AssetInfoNotFound)?;
            ensure!(sender == nft_info.owner, Error::<T>::NoPermission);

            // check class type
            let class_detail = OrmlNft::<T>::classes(token.0).ok_or(Error::<T>::ClassIdNotFound)?;
            ensure!(
                class_detail.data.token_type != TokenType::BoundToAddress,
                Error::<T>::CannotBeBurned
            );

            OrmlNft::<T>::burn(&sender, token)?;

            let data = nft_info.data;
            let class_pool_account: T::AccountId = T::PalletId::get().into_sub_account(token.0);
            <T as Config>::Currency::unreserve(&class_pool_account, data.deposit);

            Self::deposit_event(Event::NftBurned(sender, token.0, token.1));

            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn transfer(
            origin: OriginFor<T>,
            to: T::AccountId,
            asset_id: T::AssetId,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            ensure!(
                !T::AssetsHandler::check_item_in_auction(asset_id),
                Error::<T>::AssetAlreadyInAuction
            );

            let nft_token_id = Self::do_transfer(&sender, &to, asset_id)?;

            Self::deposit_event(Event::<T>::NftTransfered(sender, to, nft_token_id));

            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn transfer_batch(
            origin: OriginFor<T>,
            tos: Vec<(T::AccountId, T::AssetId)>,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            for (_i, x) in tos.iter().enumerate() {
                let item = &x;
                let owner = &sender.clone();

                let nft_info = Assets::<T>::get(item.1).ok_or(Error::<T>::AssetIdNotFound)?;
                let class_detail =
                    OrmlNft::<T>::classes(nft_info.0).ok_or(Error::<T>::ClassIdNotFound)?;

                match class_detail.data.token_type {
                    TokenType::Transferable => {
                        let asset_detail = OrmlNft::<T>::tokens(nft_info.0, nft_info.1)
                            .ok_or(Error::<T>::AssetInfoNotFound)?;
                        ensure!(
                            owner.clone() == asset_detail.owner,
                            Error::<T>::NoPermission
                        );

                        Self::handle_ownership_transfer(&owner, &item.0, item.1)?;
                        OrmlNft::<T>::transfer(&owner, &item.0, (nft_info.0, nft_info.1))?;

                        Self::deposit_event(Event::<T>::NftTransfered(
                            owner.clone(),
                            item.0.clone(),
                            nft_info.1.clone(),
                        ));
                    }
                    _ => (),
                };
            }

            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn sign_asset(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            let asset_by_owner: Vec<T::AssetId> = Self::get_assets_by_owner(&sender);
            ensure!(
                !asset_by_owner.contains(&asset_id),
                Error::<T>::SignOwnAsset
            );

            if AssetSupporters::<T>::contains_key(&asset_id) {
                AssetSupporters::<T>::try_mutate(asset_id, |supporters| -> DispatchResult {
                    let supporters = supporters.as_mut().ok_or("Empty supporters")?;
                    supporters.push(sender);
                    Ok(())
                })?;
                Ok(().into())
            } else {
                let mut new_supporters = Vec::new();
                new_supporters.push(sender);
                AssetSupporters::<T>::insert(asset_id, new_supporters);
                Ok(().into())
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    pub fn handle_ownership_transfer(
        sender: &T::AccountId,
        to: &T::AccountId,
        asset_id: T::AssetId,
    ) -> DispatchResult {
        AssetsByOwner::<T>::try_mutate(&sender, |asset_ids| -> DispatchResult {
            let asset_index = asset_ids.iter().position(|x| *x == asset_id).unwrap();
            asset_ids.remove(asset_index);

            Ok(())
        })?;

        if AssetsByOwner::<T>::contains_key(to) {
            AssetsByOwner::<T>::try_mutate(&to, |asset_ids| -> DispatchResult {
                ensure!(
                    !asset_ids.iter().any(|i| asset_id == *i),
                    Error::<T>::AssetIdAlreadyExists
                );
                asset_ids.push(asset_id);
                Ok(())
            })?;
        } else {
            let mut asset_ids = Vec::<T::AssetId>::new();
            asset_ids.push(asset_id);
            AssetsByOwner::<T>::insert(&to, asset_ids);
        }

        Ok(())
    }

    pub fn do_transfer(
        sender: &T::AccountId,
        to: &T::AccountId,
        asset_id: T::AssetId,
    ) -> Result<<T as orml_nft::Config>::TokenId, DispatchError> {
        let nft_info = Assets::<T>::get(asset_id).ok_or(Error::<T>::AssetIdNotFound)?;

        let class_detail = OrmlNft::<T>::classes(nft_info.0).ok_or(Error::<T>::ClassIdNotFound)?;

        match class_detail.data.token_type {
            TokenType::Transferable => {
                let check_ownership = Self::check_ownership(&sender, &asset_id)?;
                ensure!(check_ownership, Error::<T>::NoPermission);

                Self::handle_ownership_transfer(&sender, &to, asset_id)?;

                OrmlNft::<T>::transfer(&sender, &to, nft_info.clone())?;
                Ok(nft_info.1)
            }
            TokenType::BoundToAddress => Err(Error::<T>::NonTransferable.into()),
        }
    }

    pub fn check_ownership(
        sender: &T::AccountId,
        asset_id: &T::AssetId,
    ) -> Result<bool, DispatchError> {
        let nft_info = Assets::<T>::get(asset_id).ok_or(Error::<T>::AssetIdNotFound)?;

        let asset_detail =
            OrmlNft::<T>::tokens(nft_info.0, nft_info.1).ok_or(Error::<T>::AssetInfoNotFound)?;
        if sender == &asset_detail.owner {
            return Ok(true);
        }

        return Ok(false);
    }

    pub fn update_ads_slot(
        asset_id: &T::AssetId,
        start_time: T::BlockNumber,
        end_time: T::BlockNumber,
        deposit: BalanceOfAsset<T>,
        media: Vec<u8>,
        owner: T::AccountId,
    ) -> DispatchResult {
        let ads_slot = AdsSlot {
            start_time,
            end_time,
            deposit,
            media,
            owner,
        };
        AdsSlots::<T>::insert(asset_id, ads_slot);
        // AdsSlots::<T>::try_mutate(asset_id, |slot| -> DispatchResult{
        //     let ads_slot = slot.as_mut().ok_or("empty slot")?;

        //     ads_slot.start_time = <frame_system::Pallet<T>>::block_number();

        //     Ok(())
        // })?;

        Ok(())
    }
}
