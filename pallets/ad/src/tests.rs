#![cfg(test)]

use super::{Event as AdEvent, *};
use frame_support::{assert_noop, assert_ok};
use crate::mock::{Event as MEvent, *};
use utils::test_helper::*;
use sp_core::Pair;

#[test]
fn create_advertiser_should_work() {
	ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Did::register(Origin::signed(ALICE), signer::<Runtime>(ALICE), None));

        let advertiser_id = NextId::<Runtime>::get();
        assert_ok!(Ad::create_advertiser(Origin::signed(ALICE), 0));
        assert_noop!(Ad::create_advertiser(Origin::signed(ALICE), 0), Error::<Runtime>::AdvertiserExists);
        let advertiser = Advertisers::<Runtime>::get(d!(ALICE)).unwrap();

        let deposit = AdvertiserDeposit::<Runtime>::get();
        assert!(deposit > 0);
        assert_eq!(free_balance::<Runtime>(&advertiser.deposit_account), deposit);

        assert_last_event::<Runtime>(MEvent::Ad(
            AdEvent::CreatedAdvertiser(ALICE, d!(ALICE), advertiser_id)
        ));
    });
}

#[test]
fn create_advertiser_should_fail() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(Ad::create_advertiser(Origin::signed(ALICE), 0), Error::<Runtime>::DIDNotExists);

        NextId::<Runtime>::put(GlobalId::MAX);
        assert_ok!(Did::register(Origin::signed(ALICE), signer::<Runtime>(ALICE), None));
        assert_noop!(Ad::create_advertiser(Origin::signed(ALICE), 0), Error::<Runtime>::NoAvailableId);
    });
}

#[test]
fn create_ad_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Did::register(Origin::signed(ALICE), signer::<Runtime>(ALICE), None));

        let advertiser_id = NextId::<Runtime>::get();
        assert_ok!(Ad::create_advertiser(Origin::signed(ALICE), 0));

        let ad_id = NextId::<Runtime>::get();
        assert_ok!(Ad::create_ad(Origin::signed(ALICE), ALICE, vec![(0,1), (1, 2),(2,3)], PerU16::from_percent(50)));

        let advertiser = Advertisers::<Runtime>::get(d!(ALICE)).unwrap();

        let deposit = AdDeposit::<Runtime>::get();
        assert!(deposit > 0);
        assert_eq!(reserved_balance::<Runtime>(&advertiser.deposit_account), deposit);

        let _ = Advertisements::<Runtime>::get(advertiser_id, ad_id).unwrap();
        assert_last_event::<Runtime>(MEvent::Ad(
            AdEvent::CreatedAd(d!(ALICE), advertiser_id, ad_id)
        ));
    });
}

#[test]
fn create_ad_should_fail() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(Ad::create_ad(Origin::signed(ALICE), ALICE, vec![(0,1), (1, 2),(2,3),(4,4)], PerU16::from_percent(50)), Error::<Runtime>::InvalidTagCoefficientCount);
        assert_noop!(Ad::create_ad(Origin::signed(ALICE), ALICE, vec![], PerU16::from_percent(50)), Error::<Runtime>::InvalidTagCoefficientCount);
        assert_noop!(Ad::create_ad(Origin::signed(ALICE), ALICE, vec![(0,1), (1, 2),(2,3)], PerU16::from_percent(50)), Error::<Runtime>::DIDNotExists);
        assert_noop!(Ad::create_ad(Origin::signed(ALICE), ALICE, vec![(0,1), (200, 2),(2,3)], PerU16::from_percent(50)), Error::<Runtime>::InvalidTagType);
        assert_noop!(Ad::create_ad(Origin::signed(ALICE), ALICE, vec![(0,1), (1, 2),(1,3)], PerU16::from_percent(50)), Error::<Runtime>::DuplicatedTagType);

        assert_ok!(Did::register(Origin::signed(ALICE), signer::<Runtime>(ALICE), None));
        assert_noop!(Ad::create_ad(Origin::signed(ALICE), ALICE, vec![(0,1), (1, 2),(2,3)], PerU16::from_percent(50)), Error::<Runtime>::AdvertiserNotExists);
    });
}

#[test]
fn ad_payout_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        // advertiser Alice
        assert_ok!(Did::register(Origin::signed(ALICE), signer::<Runtime>(ALICE), None));
        // user Charlie
        assert_ok!(Did::register(Origin::signed(CHARLIE), signer::<Runtime>(CHARLIE), None));
        // media Bob
        assert_ok!(Did::register(Origin::signed(BOB), signer::<Runtime>(BOB), None));

        let advertiser_id = NextId::<Runtime>::get();
        assert_ok!(Ad::create_advertiser(Origin::signed(ALICE), 10000 * UNIT));

        let (signer_pair, _) = sp_core::sr25519::Pair::generate();
        let signer: AccountId = signer_pair.public().0.clone().into();

        let ad_id = NextId::<Runtime>::get();
        assert_ok!(Ad::create_ad(Origin::signed(ALICE), signer.clone(), vec![(0,1), (1, 2),(2,3)], PerU16::from_percent(50)));

        assert_ok!(Ad::ad_payout(Origin::signed(ALICE), ad_id, d!(CHARLIE), d!(BOB), vec![1, 2, 3]));
        assert_last_event::<Runtime>(MEvent::Ad(
            AdEvent::AdReward(advertiser_id, ad_id, 30 * UNIT)
        ));
    });
}

#[test]
fn payout_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        // advertiser Alice
        assert_ok!(Did::register(Origin::signed(ALICE), signer::<Runtime>(ALICE), None));
        // user Charlie
        assert_ok!(Did::register(Origin::signed(CHARLIE), signer::<Runtime>(CHARLIE), None));
        // media Bob
        assert_ok!(Did::register(Origin::signed(BOB), signer::<Runtime>(BOB), None));

        let advertiser_id = NextId::<Runtime>::get();
        assert_ok!(Ad::create_advertiser(Origin::signed(ALICE), 10000 * UNIT));

        let signer_pair = sp_core::sr25519::Pair::from_string("//AliceSigner", None).unwrap();
        let signer: AccountId = signer_pair.public().0.clone().into();

        let ad_id = NextId::<Runtime>::get();
        assert_ok!(Ad::create_ad(Origin::signed(ALICE), signer.clone(), vec![(0,1), (1, 2),(2,3)], PerU16::from_percent(50)));

        pallet_timestamp::Now::<Runtime>::put(ADVERTISER_PAYMENT_WINDOW + 1);

        let (_, data_sign) = sign::<Runtime>(signer_pair, CHARLIE, BOB, ALICE, ad_id, 0);
        assert_ok!(Ad::payout(Origin::signed(DAVE), data_sign, d!(ALICE), ad_id, d!(CHARLIE), d!(BOB), 0));
        assert_last_event::<Runtime>(MEvent::Ad(
            AdEvent::AdReward(advertiser_id, ad_id, 30 * UNIT)
        ));

        assert_eq!(free_balance::<Runtime>(&DAVE), DAVE_INIT + EXTRA_REDEEM);
    });
}
