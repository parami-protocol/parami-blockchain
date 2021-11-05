use crate::{mock::*, AdsOf, Config, DeadlineOf, Error, Metadata, SlotOf};
use frame_support::{
    assert_noop, assert_ok,
    traits::{Hooks, StoredMap},
};
use sp_core::sr25519;

macro_rules! ensure_remain {
    ($meta:tt, $currency:expr, $tokens: expr) => {
        assert_eq!($meta.remain, $currency);
        assert_eq!(Balances::free_balance(&$meta.pot), $currency);

        assert_eq!(Assets::balance(0, &$meta.pot), $tokens);
    };
}

#[test]
fn should_create() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);
        let did = DID::from_slice(&[0xff; 20]);

        assert_eq!(Balances::free_balance(alice), 100);

        let tags = vec![
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8],
        ];

        let mut hashes = vec![];
        for tag in &tags {
            let hash = Tag::key(tag);
            hashes.push(hash);
        }

        let metadata = vec![0u8; 64];

        assert_ok!(Ad::create(
            Origin::signed(alice),
            50,
            tags.clone(),
            metadata.clone(),
            1,
            1
        ));

        assert_eq!(<AdsOf<Test>>::get(&did).unwrap().len(), 1);

        let maybe_ad = <Metadata<Test>>::iter_values().next();
        assert_ne!(maybe_ad, None);

        let ad = maybe_ad.unwrap();
        assert_eq!(ad.creator, did);
        assert_eq!(ad.budget, 50);
        assert_eq!(ad.remain, 50);
        assert_eq!(ad.metadata, metadata);
        assert_eq!(ad.reward_rate, 1);
        assert_eq!(ad.created, 0);
        assert_eq!(ad.deadline, 1);

        assert_eq!(Balances::free_balance(alice), 50);

        let pool = ad.pot;

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        assert_eq!(<Test as Config>::TagsStore::get(&ad), hashes);

        assert_eq!(Balances::free_balance(pool), 50);
    });
}

#[test]
fn should_fail_when_insufficient() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        assert_noop!(
            Ad::create(Origin::signed(alice), 200, vec![], [0u8; 64].into(), 1, 1),
            pallet_balances::Error::<Test>::InsufficientBalance
        );

        assert_eq!(Balances::free_balance(alice), 100);
    });
}

#[test]
fn should_fail_when_tag_not_exists() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        let tags = vec![
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8],
            vec![0u8; 6],
        ];

        assert_noop!(
            Ad::create(Origin::signed(alice), 200, tags, [0u8; 64].into(), 1, 1),
            Error::<Test>::TagNotExists
        );

        assert_eq!(Balances::free_balance(alice), 100);
    });
}

#[test]
fn should_update_reward_rate() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        assert_ok!(Ad::create(
            Origin::signed(alice),
            50,
            vec![],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        assert_ok!(Ad::update_reward_rate(Origin::signed(alice), ad, 2));

        assert_eq!(<Metadata<Test>>::get(&ad).unwrap().reward_rate, 2);
    });
}

#[test]
fn should_fail_when_not_exists_or_not_owned() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        assert_noop!(
            Ad::update_reward_rate(Origin::signed(alice), Default::default(), 2),
            Error::<Test>::NotExists
        );

        assert_ok!(Ad::create(
            Origin::signed(alice),
            50,
            vec![],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        let bob = sr25519::Public([2; 32]);

        assert_noop!(
            Ad::update_reward_rate(Origin::signed(bob), ad, 2),
            Error::<Test>::NotOwned
        );
    });
}

#[test]
fn should_update_tags() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        let tags = vec![
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8],
        ];

        let mut hashes = vec![];
        for tag in &tags {
            let hash = Tag::key(tag);
            hashes.push(hash);
        }

        assert_ok!(Ad::create(
            Origin::signed(alice),
            50,
            vec![vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        assert_ok!(Ad::update_tags(Origin::signed(alice), ad, tags));

        assert_eq!(<Test as Config>::TagsStore::get(&ad), hashes);
    });
}

#[test]
fn should_bid() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);
        let kol = DID::from_slice(&[0xff; 20]);

        let bob = sr25519::Public([2; 32]);
        let charlie = sr25519::Public([3; 32]);

        // 1. prepare

        assert_ok!(Nft::back(Origin::signed(bob), kol, 2_000_100u128));

        assert_ok!(Nft::mint(
            Origin::signed(alice),
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        // ad1

        assert_ok!(Ad::create(
            Origin::signed(bob),
            500,
            vec![],
            [0u8; 64].into(),
            1,
            43200
        ));

        let ad1 = <Metadata<Test>>::iter_keys().next().unwrap();
        let meta1 = <Metadata<Test>>::get(&ad1).unwrap();
        ensure_remain!(meta1, 500, 0);

        // ad2

        assert_ok!(Ad::create(
            Origin::signed(charlie),
            500,
            vec![],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad2 = <Metadata<Test>>::iter_keys().next().unwrap();
        let meta2 = <Metadata<Test>>::get(&ad2).unwrap();
        ensure_remain!(meta2, 500, 0);

        // 2. bob bid for ad1

        assert_noop!(
            Ad::bid(Origin::signed(bob), ad1, kol, 600),
            pallet_balances::Error::<Test>::InsufficientBalance
        );

        assert_ok!(Ad::bid(Origin::signed(bob), ad1, kol, 400));

        // ensure: deadline, slot, remain

        assert_eq!(<DeadlineOf<Test>>::get(&kol, &ad1), Some(43200));

        let maybe_slot = <SlotOf<Test>>::get(&kol);
        assert_ne!(maybe_slot, None);
        let slot = maybe_slot.unwrap();
        assert_eq!(slot.ad, ad1);
        assert_eq!(slot.budget, 199);
        assert_eq!(slot.remain, 199);
        assert_eq!(slot.deadline, 43200);

        let meta1 = <Metadata<Test>>::get(&ad1).unwrap();
        ensure_remain!(meta1, 100, slot.remain);

        // 3. charlie bid for ad2

        assert_noop!(
            Ad::bid(Origin::signed(charlie), ad2, kol, 400),
            Error::<Test>::Underbid
        );

        assert_ok!(Ad::bid(Origin::signed(charlie), ad2, kol, 480));

        // ensure: deadline, slot, remain

        assert_eq!(<DeadlineOf<Test>>::get(&kol, &ad1), None);
        assert_eq!(<DeadlineOf<Test>>::get(&kol, &ad2), Some(1));

        let maybe_slot = <SlotOf<Test>>::get(&kol);
        assert_ne!(maybe_slot, None);
        let slot = maybe_slot.unwrap();
        assert_eq!(slot.ad, ad2);
        assert_eq!(slot.budget, 239);
        assert_eq!(slot.remain, 239);
        assert_eq!(slot.deadline, 1);

        let meta1 = <Metadata<Test>>::get(&ad1).unwrap();
        ensure_remain!(meta1, 496, 0);

        let meta2 = <Metadata<Test>>::get(&ad2).unwrap();
        ensure_remain!(meta2, 20, slot.remain);
    });
}

#[test]
fn should_drawback() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);
        let kol = DID::from_slice(&[0xff; 20]);

        let bob = sr25519::Public([2; 32]);

        // 1. prepare

        assert_ok!(Nft::back(Origin::signed(bob), kol, 2_000_100u128));

        assert_ok!(Nft::mint(
            Origin::signed(alice),
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        // create ad

        assert_ok!(Ad::create(
            Origin::signed(bob),
            500,
            vec![],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        // bid

        assert_ok!(Ad::bid(Origin::signed(bob), ad, kol, 400));

        let meta = <Metadata<Test>>::get(&ad).unwrap();
        let slot = <SlotOf<Test>>::get(&kol).unwrap();
        assert_eq!(slot.budget, 199);
        assert_eq!(slot.remain, 199);

        ensure_remain!(meta, 100, slot.remain);

        // 2. step in

        System::set_block_number(1);
        Ad::on_initialize(System::block_number());

        // ensure slot, remain

        assert_eq!(<SlotOf<Test>>::get(&kol), None);

        let meta = <Metadata<Test>>::get(&ad).unwrap();
        ensure_remain!(meta, 496, 0);
    });
}

#[test]
fn should_deposit() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        assert_ok!(Ad::create(
            Origin::signed(alice),
            50,
            vec![],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        assert_ok!(Ad::deposit(Origin::signed(alice), ad, 20));

        let meta = <Metadata<Test>>::get(&ad).unwrap();

        assert_eq!(meta.budget, 70);
        assert_eq!(meta.remain, 70);

        assert_eq!(Balances::free_balance(meta.pot), 70);
    });
}

#[test]
fn should_pay() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);
        let kol = DID::from_slice(&[0xff; 20]);

        let bob = sr25519::Public([2; 32]);

        let charlie = sr25519::Public([3; 32]);
        let did = DID::from_slice(&[0xdd; 20]);

        // 1. prepare

        assert_ok!(Nft::back(Origin::signed(bob), kol, 2_000_100u128));

        assert_ok!(Nft::mint(
            Origin::signed(alice),
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        // create ad

        assert_ok!(Ad::create(
            Origin::signed(bob),
            500,
            vec![],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        // bid

        assert_ok!(Ad::bid(Origin::signed(bob), ad, kol, 400));

        let meta = <Metadata<Test>>::get(&ad).unwrap();
        let slot = <SlotOf<Test>>::get(&kol).unwrap();
        assert_eq!(slot.budget, 199);
        assert_eq!(slot.remain, 199);

        ensure_remain!(meta, 100, slot.remain);

        // 2. pay

        assert_ok!(Ad::pay(Origin::signed(bob), ad, kol, did, vec![], None));

        let slot = <SlotOf<Test>>::get(&kol).unwrap();
        assert_eq!(slot.remain, 198);
        assert_eq!(Assets::balance(0, &meta.pot), 198);

        assert_eq!(Assets::balance(0, &charlie), 1);
    });
}
