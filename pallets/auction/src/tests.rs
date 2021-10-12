#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use parami_nft::{CollectionType, TokenType};

use orml_auction::Pallet as OrmlAuction;
use orml_nft::Pallet as OrmlNft;
use pallet_assets::Pallet as Assets;
use parami_ad::Pallet as AdsPallet;
use parami_did::Pallet as Did;
use sp_runtime::PerU16;

fn init_test(owner: Origin) {
    // Alice create NFT and fractionize it
    assert_ok!(NftPallet::<Runtime>::create_class(
        owner.clone(),
        vec![1],
        TokenType::BoundToAddress,
        CollectionType::Collectable,
    ));

    assert_ok!(NftPallet::<Runtime>::mint(
        owner.clone(),
        CLASS_ID,
        vec![1],
        vec![1],
        vec![1],
        vec![1],
        1
    ));
}

fn prepare_bid() {
    let owner = Origin::signed(ALICE);
    let bidder = Origin::signed(BOB);
    let bidder_two = Origin::signed(DAVE);

    // transfer some nft fraction to bidders
    assert_ok!(Assets::<Runtime>::transfer(owner.clone(), 0, BOB, 10000));
    assert_ok!(Assets::<Runtime>::transfer(owner.clone(), 0, DAVE, 10000));
    assert_eq!(Assets::<Runtime>::balance(0, BOB), 10000);
    assert_eq!(Assets::<Runtime>::balance(0, DAVE), 10000);

    // register did for bidders
    assert_ok!(Did::<Runtime>::register(
        bidder.clone(),
        signer::<Runtime>(BOB),
        None
    ));
    assert_ok!(Did::<Runtime>::register(
        bidder_two.clone(),
        signer::<Runtime>(DAVE),
        None
    ));

    // bider BOB creates advertiser and ads firstly for being qualified to bid the ads slot
    assert_ok!(AdsPallet::<Runtime>::create_advertiser(
        bidder.clone(),
        0,
        1000
    ));
    assert_ok!(AdsPallet::<Runtime>::create_ad(
        bidder.clone(),
        0,
        BOB,
        vec![(0, 1), (1, 2), (2, 3)],
        PerU16::from_percent(50),
        b"i am first bidder".to_vec()
    ));

    // bider DAVE creates advertiser and ads firstly for being qualified to bid the ads slot
    assert_ok!(AdsPallet::<Runtime>::create_advertiser(
        bidder_two.clone(),
        0,
        1000
    ));
    assert_ok!(AdsPallet::<Runtime>::create_ad(
        bidder_two.clone(),
        0,
        DAVE,
        vec![(0, 1), (1, 2), (2, 3)],
        PerU16::from_percent(50),
        b"i am second bidder".to_vec()
    ));
}

#[test]
fn create_new_auction_work() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = Origin::signed(ALICE);
        init_test(owner.clone());

        let nft_asset_data = OrmlNft::<Runtime>::tokens(CLASS_ID, TOKEN_ID);
        println!("nft data => {:?}", nft_asset_data);

        assert_ok!(AuctionPallet::create_new_auction(
            owner.clone(),
            ItemId::NFT(0),
            50,
            101
        ));

        let auction = <orml_auction::Auctions<Runtime>>::get(0);
        println!("auction => {:?}", auction);

        assert_eq!(AuctionPallet::assets_in_auction(0), Some(true));
    });
}

#[test]
fn create_auction_fail() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = Origin::signed(ALICE);
        let bob = Origin::signed(BOB);

        init_test(owner.clone());

        // generate another NFT
        assert_ok!(NftPallet::<Runtime>::create_class(
            bob.clone(),
            vec![1],
            TokenType::BoundToAddress,
            CollectionType::Collectable,
        ));

        assert_ok!(NftPallet::<Runtime>::mint(
            bob.clone(),
            1,
            vec![1],
            vec![1],
            vec![1],
            vec![1],
            1
        ));

        // have no permission to create auction
        assert_noop!(
            AuctionPallet::create_new_auction(owner.clone(), ItemId::NFT(1), 50, 101),
            Error::<Runtime>::NoPermissionToCreateAuction
        );

        assert_ok!(NftPallet::<Runtime>::create_class(
            owner.clone(),
            vec![1],
            TokenType::Transferable,
            CollectionType::Collectable,
        ));

        assert_ok!(NftPallet::<Runtime>::mint(
            owner.clone(),
            2,
            vec![1],
            vec![1],
            vec![1],
            vec![1],
            1
        ));

        // NFT not bound
        assert_noop!(
            AuctionPallet::create_new_auction(owner.clone(), ItemId::NFT(2), 50, 101),
            Error::<Runtime>::NotBounded
        );

        // Ads slot is already in auction
        assert_ok!(AuctionPallet::create_new_auction(
            owner.clone(),
            ItemId::NFT(0),
            50,
            101
        ));
        assert_noop!(
            AuctionPallet::create_new_auction(owner.clone(), ItemId::NFT(0), 50, 101),
            Error::<Runtime>::AssetAlreadyInAuction
        );
    });
}

#[test]
fn remove_auction_work() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = Origin::signed(ALICE);

        init_test(owner.clone());

        assert_ok!(AuctionPallet::create_new_auction(
            owner.clone(),
            ItemId::NFT(0),
            50,
            101
        ));

        AuctionPallet::remove_auction(0, ItemId::NFT(0));

        assert_eq!(OrmlAuction::<Runtime>::auctions(0), None);
        assert_eq!(AuctionPallet::assets_in_auction(0), None);
        assert_eq!(AuctionPallet::current_ads(0), None);
    });
}

#[test]
fn bid_works() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = Origin::signed(ALICE);
        let bidder = Origin::signed(BOB);
        let bidder_two = Origin::signed(DAVE);

        init_test(owner.clone());

        prepare_bid();

        // Alice creates ads slot auction
        assert_ok!(AuctionPallet::create_new_auction(
            owner.clone(),
            ItemId::NFT(0),
            50,
            101
        ));

        // Bob bids by 200 fraction
        assert_ok!(AuctionPallet::bid(bidder, 0, 200, 1));
        assert_eq!(
            last_event(),
            mock::Event::AuctionPallet(crate::Event::Bid(0, 0, BOB, 200))
        );

        let (_, reward_pool_account_1) = AdsPallet::<Runtime>::ad_accounts(0);
        assert_eq!(Assets::<Runtime>::balance(0, &reward_pool_account_1), 1200);

        // Dave bids again
        assert_ok!(AuctionPallet::bid(bidder_two, 0, 201, 3));
        let (_, reward_pool_account_2) = AdsPallet::<Runtime>::ad_accounts(2);
        assert_eq!(Assets::<Runtime>::balance(0, reward_pool_account_1), 1000);
        assert_eq!(Assets::<Runtime>::balance(0, reward_pool_account_2), 1201);
    });
}

#[test]
fn cannot_bid_on_non_existent_auction() {
    ExtBuilder::default().build().execute_with(|| {
        let bidder = Origin::signed(ALICE);

        assert_noop!(
            AuctionPallet::bid(bidder, 0, 100, 1),
            Error::<Runtime>::AuctionNotExist
        );
    });
}

#[test]
fn cannot_bid_with_insufficient_funds() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = Origin::signed(ALICE);
        let _bidder = Origin::signed(BOB);

        init_test(owner.clone());

        prepare_bid();

        assert_ok!(AuctionPallet::create_new_auction(
            owner.clone(),
            ItemId::NFT(0),
            50,
            101
        ));

        assert_eq!(Assets::<Runtime>::balance(0, BOB), 9000);

        // assert_noop!(AuctionPallet::bid(bidder, 0, 200000, 1), Error::<Runtime>::InsufficientFunds);
    });
}

#[test]
fn cannot_bid_on_own_auction() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = Origin::signed(ALICE);

        init_test(owner.clone());

        prepare_bid();

        assert_ok!(AuctionPallet::create_new_auction(
            owner.clone(),
            ItemId::NFT(0),
            50,
            101
        ));

        assert_noop!(
            AuctionPallet::bid(owner, 0, 50, 1),
            Error::<Runtime>::SelfBidNotAccepted
        );
    });
}

#[test]
fn ads_list_after_auction() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = Origin::signed(ALICE);
        let bidder = Origin::signed(BOB);

        init_test(owner.clone());
        prepare_bid();

        assert_eq!(NftPallet::<Runtime>::get_assets_by_owner(ALICE), [0]);

        assert_ok!(AuctionPallet::create_new_auction(
            owner.clone(),
            ItemId::NFT(0),
            50,
            101
        ));

        assert_ok!(AuctionPallet::bid(bidder, 0, 200, 1));
        assert_eq!(
            last_event(),
            mock::Event::AuctionPallet(crate::Event::Bid(0, 0, BOB, 200))
        );

        let (_, reward_pool_account) = AdsPallet::<Runtime>::ad_accounts(0);
        assert_eq!(Assets::<Runtime>::balance(0, &reward_pool_account), 1200);

        run_to_block(102);

        let ads_slot = parami_nft::AdsSlot::<AccountId, u128, u64> {
            start_time: 101,
            end_time: 201,
            deposit: 200,
            media: b"i am first bidder".to_vec(),
            owner: BOB,
        };
        assert_eq!(NftPallet::<Runtime>::get_ads_slot(0), Some(ads_slot));

        assert_eq!(Assets::<Runtime>::balance(0, reward_pool_account), 1200);

        // Auction Finalized
        assert_eq!(
            last_event(),
            mock::Event::AuctionPallet(crate::Event::AuctionFinalized(0, BOB, 200))
        );
    });
}

#[test]
fn cannot_create_auction_when_ads_listing() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = Origin::signed(ALICE);
        let bidder = Origin::signed(BOB);

        init_test(owner.clone());
        prepare_bid();

        assert_ok!(AuctionPallet::create_new_auction(
            owner.clone(),
            ItemId::NFT(0),
            50,
            101
        ));
        assert_ok!(AuctionPallet::bid(bidder, 0, 200, 1));

        run_to_block(102);

        assert_noop!(
            AuctionPallet::create_new_auction(owner.clone(), ItemId::NFT(0), 50, 202),
            Error::<Runtime>::AdsIsListing
        );

        System::set_block_number(202);

        assert_ok!(AuctionPallet::create_new_auction(
            owner.clone(),
            ItemId::NFT(0),
            50,
            502
        ));
    });
}

#[test]
fn cannot_bid_on_ended_auction() {
    ExtBuilder::default().build().execute_with(|| {
        let owner = Origin::signed(ALICE);
        let bidder = Origin::signed(BOB);

        init_test(owner.clone());
        prepare_bid();

        assert_ok!(AuctionPallet::create_new_auction(
            owner.clone(),
            ItemId::NFT(0),
            50,
            101
        ));

        System::set_block_number(101);

        assert_noop!(
            AuctionPallet::bid(bidder, 0, 200, 1),
            orml_auction::Error::<Runtime>::AuctionIsExpired
        );
    });
}
