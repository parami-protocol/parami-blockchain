#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
pub use types::*;

#[rustfmt::skip]
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// #[cfg(feature = "runtime-benchmarks")]
// mod benchmarking;

mod types;

use frame_support::{
    dispatch::{DispatchError, DispatchResult, DispatchResultWithPostInfo},
    ensure,
    traits::{tokens::fungibles::Transfer, Get, IsType, ReservableCurrency},
    PalletId,
};
use orml_auction::Pallet as OrmlAuction;
use orml_traits::{Auction, AuctionHandler, Change, OnNewBidResult};
use parami_ad::{AdId, AdvertiserId, AdvertiserOf, Pallet as AdsPallet};
use parami_nft::{AssetHandler, Pallet as NftPallet};
use sp_runtime::traits::{AccountIdConversion, CheckedSub, Zero};

use weights::WeightInfo;

type BalanceOfAsset<T> = <T as orml_auction::Config>::Balance;

pub const PALLET_ID: PalletId = PalletId(*b"par/auct");

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config:
        frame_system::Config
        + pallet_assets::Config
        + parami_ad::Config
        + parami_nft::Config
        + orml_auction::Config
    {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency mechanism.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Minimum duration
        type MinimumAuctionDuration: Get<Self::BlockNumber>;

        /// auction handler
        type Handler: AuctionHandler<
            Self::AccountId,
            BalanceOfAsset<Self>,
            Self::BlockNumber,
            AuctionId,
        >;

        /// time to close auction
        type AuctionTimeToClose: Get<Self::BlockNumber>;

        /// ads list duration
        type AdsListDuration: Get<Self::BlockNumber>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    #[pallet::getter(fn assets_in_auction)]
    pub(super) type AssetsInAuction<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, bool, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn current_ads)]
    pub type CurrentAds<T: Config> =
        StorageMap<_, Twox64Concat, T::AuctionId, (AdvertiserId, AdId), OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn get_auction_item)]
    pub(super) type AuctionItems<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AuctionId,
        AuctionItem<T::AccountId, T::BlockNumber, BalanceOfAsset<T>, T::AssetId>,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// new auction created
        NewAuctionItem(
            T::AuctionId,
            T::AccountId,
            BalanceOfAsset<T>,
            BalanceOfAsset<T>,
            T::BlockNumber,
        ),
        Bid(T::AuctionId, T::AssetId, T::AccountId, BalanceOfAsset<T>),
        AuctionFinalized(T::AuctionId, T::AccountId, BalanceOfAsset<T>),
        AuctionFinalizedNoBid(T::AuctionId),
        AssetTransferFailed(T::AuctionId, T::AssetId),
        UpdateSlotFailed(T::AuctionId, T::AssetId),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        AuctionEndIsLessThanMinimumDuration,
        AssetIsNotExist,
        ClassNotFound,
        TokenInfoNotFound,
        BidderIsNotAdvertiser,
        BidderHasNoAdvertisement,
        AdsSlotNotExists,
        AdsIsListing,
        NoPermissionToCreateAuction,
        NotBounded,
        AssetAlreadyInAuction,
        AuctionTypeIsNotSupported,
        AuctionNotExist,
        InvalidAuctionType,
        ValueLessThanInitialAmount,
        SelfBidNotAccepted,
        InvalidBidPrice,
        BidNotAccepted,
        InsufficientFunds,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
        <T as orml_auction::Config>::Balance: From<u128> + Into<u128>,
    {
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn create_new_auction(
            origin: OriginFor<T>,
            item_id: ItemId<T::AssetId>,
            value: BalanceOfAsset<T>,
            end_time: T::BlockNumber,
        ) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin)?;

            let start_time: T::BlockNumber = <frame_system::Pallet<T>>::block_number();

            let remaining_time: T::BlockNumber =
                end_time.checked_sub(&start_time).ok_or("Overflow")?;

            ensure!(
                remaining_time >= T::MinimumAuctionDuration::get(),
                Error::<T>::AuctionEndIsLessThanMinimumDuration
            );

            Self::create_auction(
                AuctionType::Auction,
                item_id,
                Some(end_time),
                from.clone(),
                value.clone(),
                start_time,
            )?;

            Ok(().into())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn bid(
            origin: OriginFor<T>,
            id: T::AuctionId,
            value: BalanceOfAsset<T>,
            ad_id: AdId,
        ) -> DispatchResultWithPostInfo {
            let from = ensure_signed(origin.clone())?;

            let auction_item: AuctionItem<
                T::AccountId,
                T::BlockNumber,
                BalanceOfAsset<T>,
                T::AssetId,
            > = Self::get_auction_item(id.clone()).ok_or(Error::<T>::AuctionNotExist)?;
            ensure!(
                auction_item.auction_type == AuctionType::Auction,
                Error::<T>::InvalidAuctionType
            );
            ensure!(
                auction_item.recipient != from,
                Error::<T>::SelfBidNotAccepted
            );
            ensure!(
                value > auction_item.initial_amount,
                Error::<T>::ValueLessThanInitialAmount
            );

            // check bidder is an advertiser
            let bidder_did = AdsPallet::<T>::ensure_did(&from)?;
            let advertiser = <parami_ad::Advertisers<T>>::get(&bidder_did)
                .ok_or(Error::<T>::BidderIsNotAdvertiser)?;
            let _ads = <parami_ad::Advertisements<T>>::get(advertiser.advertiser_id, ad_id)
                .ok_or(Error::<T>::BidderHasNoAdvertisement)?;

            let block_number = <frame_system::Pallet<T>>::block_number();
            let auction = OrmlAuction::<T>::auction_info(id).ok_or(Error::<T>::AuctionNotExist)?;
            OrmlAuction::<T>::bid(origin.clone(), id, value)?;

            CurrentAds::<T>::insert(id, (advertiser.advertiser_id, ad_id));

            Self::auction_bid_handler(
                block_number,
                id,
                (from.clone(), value),
                auction.bid.clone(),
                advertiser,
            )?;

            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T>
where
    <T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
    <T as orml_auction::Config>::Balance: From<u128> + Into<u128>,
{
    pub fn auction_pool_id(asset_id: T::AssetId) -> T::AccountId {
        PALLET_ID.into_sub_account(asset_id)
    }

    pub fn create_auction(
        auction_type: AuctionType,
        item_id: ItemId<T::AssetId>,
        _end: Option<T::BlockNumber>,
        recipient: T::AccountId,
        initial_amount: BalanceOfAsset<T>,
        _start: T::BlockNumber,
    ) -> Result<T::AuctionId, DispatchError> {
        match item_id {
            ItemId::NFT(asset_id) => {
                //Get asset detail
                let asset =
                    NftPallet::<T>::get_asset(asset_id).ok_or(Error::<T>::AssetIsNotExist)?;

                //Check ownership
                let class_info =
                    orml_nft::Pallet::<T>::classes(asset.0).ok_or(Error::<T>::ClassNotFound)?;
                let class_info_data = class_info.data;
                let token_info = orml_nft::Pallet::<T>::tokens(asset.0, asset.1)
                    .ok_or(Error::<T>::TokenInfoNotFound)?;
                ensure!(
                    recipient == token_info.owner,
                    Error::<T>::NoPermissionToCreateAuction
                );
                ensure!(
                    !class_info_data.token_type.is_transferable(),
                    Error::<T>::NotBounded
                );
                ensure!(
                    Self::assets_in_auction(asset_id) == None,
                    Error::<T>::AssetAlreadyInAuction
                );

                let start_time = <frame_system::Pallet<T>>::block_number();

                // check ads is not listing
                let ads_slot =
                    NftPallet::<T>::get_ads_slot(asset_id).ok_or(Error::<T>::AdsSlotNotExists)?;
                ensure!(start_time > ads_slot.end_time, Error::<T>::AdsIsListing);

                let mut end_time = start_time + T::AuctionTimeToClose::get();
                if let Some(_end_block) = _end {
                    end_time = _end_block
                }
                let auction_id = OrmlAuction::<T>::new_auction(start_time, Some(end_time))?;

                let new_auction_item = AuctionItem {
                    item_id,
                    recipient: recipient.clone(),
                    initial_amount,
                    amount: initial_amount,
                    start_time,
                    end_time,
                    auction_type,
                };

                <AuctionItems<T>>::insert(auction_id, new_auction_item);

                <AssetsInAuction<T>>::insert(asset_id, true);

                Self::deposit_event(Event::NewAuctionItem(
                    auction_id,
                    recipient,
                    initial_amount,
                    initial_amount,
                    end_time,
                ));

                Ok(auction_id)
            }
            _ => Err(Error::<T>::AuctionTypeIsNotSupported.into()),
        }
    }

    pub fn remove_auction(id: T::AuctionId, item_id: ItemId<T::AssetId>) {
        OrmlAuction::<T>::remove_auction(id);

        <CurrentAds<T>>::remove(id);

        match item_id {
            ItemId::NFT(asset_id) => {
                <AssetsInAuction<T>>::remove(asset_id);
            }
            _ => {}
        }
    }

    fn auction_bid_handler(
        _now: T::BlockNumber,
        id: T::AuctionId,
        new_bid: (T::AccountId, BalanceOfAsset<T>),
        last_bid: Option<(T::AccountId, BalanceOfAsset<T>)>,
        advertiser: AdvertiserOf<T>,
    ) -> DispatchResult {
        let (new_bidder, new_bid_price) = new_bid;
        ensure!(!new_bid_price.is_zero(), Error::<T>::InvalidBidPrice);

        <AuctionItems<T>>::try_mutate_exists(id, |auction_item| -> DispatchResult {
            let mut auction_item = auction_item.as_mut().ok_or(Error::<T>::AuctionNotExist)?;

            let last_bid_price = last_bid.clone().map_or(Zero::zero(), |(_, price)| price);
            let last_bidder = last_bid.as_ref().map(|(who, _)| who);

            match auction_item.item_id {
                ItemId::NFT(asset_id) => {
                    let new_bid_amount =
                        <T as pallet_assets::Config>::Balance::from(new_bid_price.into());
                    ensure!(
                        <pallet_assets::Pallet<T>>::balance(asset_id, &new_bidder)
                            >= new_bid_amount,
                        Error::<T>::InsufficientFunds
                    );

                    if let Some(last_bidder) = last_bidder {
                        if !last_bid_price.is_zero() {
                            // last advertiser
                            let last_bidder_did = AdsPallet::<T>::ensure_did(&last_bidder).unwrap();
                            let last_advertiser =
                                <parami_ad::Advertisers<T>>::get(&last_bidder_did).unwrap();

                            // refund from pool to last bidder
                            <pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
                                asset_id,
                                &last_advertiser.reward_pool_account,
                                &last_bidder,
                                <T as pallet_assets::Config>::Balance::from(last_bid_price.into()),
                                true,
                            )?;
                        }
                    }

                    // transfer fund to pool
                    <pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
                        asset_id,
                        &new_bidder,
                        &advertiser.reward_pool_account,
                        new_bid_amount,
                        true,
                    )?;
                    auction_item.amount = new_bid_price.clone();

                    Self::deposit_event(Event::Bid(id, asset_id, new_bidder, new_bid_price));
                }
                _ => {}
            }

            Ok(())
        })
    }
}

impl<T: Config> AuctionHandler<T::AccountId, BalanceOfAsset<T>, T::BlockNumber, T::AuctionId>
    for Pallet<T>
where
    <T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
    <T as orml_auction::Config>::Balance: From<u128> + Into<u128>,
{
    fn on_new_bid(
        _now: T::BlockNumber,
        _id: T::AuctionId,
        _new_bid: (T::AccountId, BalanceOfAsset<T>),
        _last_bid: Option<(T::AccountId, BalanceOfAsset<T>)>,
    ) -> OnNewBidResult<T::BlockNumber> {
        OnNewBidResult {
            accept_bid: true,
            auction_end_change: Change::NoChange,
        }
    }

    fn on_auction_ended(
        auction_id: T::AuctionId,
        winner: Option<(T::AccountId, BalanceOfAsset<T>)>,
    ) {
        if let Some(auction_item) = <AuctionItems<T>>::get(&auction_id) {
            if let Some(current_bid) = winner {
                let (high_bidder, high_bid_price): (T::AccountId, BalanceOfAsset<T>) = current_bid;

                match auction_item.item_id {
                    ItemId::NFT(asset_id) => {
                        if let Some(current_ads) = <CurrentAds<T>>::get(&auction_id) {
                            let (advertiser_id, ad_id) = current_ads;
                            if let Some(ads) =
                                <parami_ad::Advertisements<T>>::get(advertiser_id, ad_id)
                            {
                                // update ads slot
                                let slot_update = NftPallet::<T>::update_ads_slot(
                                    &asset_id,
                                    auction_item.end_time,
                                    auction_item.end_time + T::AdsListDuration::get(),
                                    <T as pallet_assets::Config>::Balance::from(
                                        high_bid_price.into(),
                                    ),
                                    ads.metadata,
                                    high_bidder.clone(),
                                );
                                match slot_update {
                                    Err(_) => (),
                                    Ok(_) => {
                                        Self::remove_auction(
                                            auction_id.clone(),
                                            auction_item.item_id,
                                        );
                                        Self::deposit_event(Event::AuctionFinalized(
                                            auction_id,
                                            high_bidder,
                                            high_bid_price,
                                        ));
                                    }
                                }
                            }
                        }

                        // transfer funds to pool
                        // let asset_transfer = <pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
                        //     asset_id.clone(),
                        //     &bidder_advertiser.reward_pool_account,
                        //     &high_bidder,
                        //     <T as pallet_assets::Config>::Balance::from(
                        //         high_bid_price.into()
                        //     ),
                        //     true
                        // );

                        // match asset_transfer {
                        //     Err(_) => {
                        //         Self::deposit_event(Event::AssetTransferFailed(auction_id.clone(), asset_id));
                        //     },
                        //     Ok(_) => {
                        //         Self::deposit_event(Event::AuctionFinalized(auction_id, high_bidder, high_bid_price));
                        //     }
                        // }
                    }
                    _ => {}
                }
            } else {
                Self::deposit_event(Event::AuctionFinalizedNoBid(auction_id));
            }
        }
    }
}

impl<T: Config> AssetHandler<T::AssetId> for Pallet<T> {
    fn check_item_in_auction(asset_id: T::AssetId) -> bool {
        if Self::assets_in_auction(asset_id) == Some(true) {
            return true;
        }
        return false;
    }
}
