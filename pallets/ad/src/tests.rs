use crate::{mock::*, AdsOf, Config, DeadlineOf, Error, Metadata, SlotOf};
use frame_support::{assert_noop, assert_ok, traits::Hooks};
use parami_did::Pallet as Did;
use parami_traits::Tags;

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
        assert_eq!(Balances::free_balance(ALICE), 100);

        let tags = vec![
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8],
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
        ];

        let mut hashes = vec![];
        for tag in &tags {
            let hash = Tag::key(tag);
            hashes.push(hash);
        }

        let metadata = vec![0u8; 64];

        assert_ok!(Ad::create(
            Origin::signed(ALICE),
            50,
            tags,
            metadata.clone(),
            1,
            1
        ));

        assert_eq!(<AdsOf<Test>>::get(&DID_ALICE).unwrap().len(), 1);

        let maybe_ad = <Metadata<Test>>::iter().next();
        assert_ne!(maybe_ad, None);

        let (ad, meta) = maybe_ad.unwrap();
        assert_eq!(meta.creator, DID_ALICE);
        assert_eq!(meta.budget, 50);
        assert_eq!(meta.remain, 50);
        assert_eq!(meta.metadata, metadata);
        assert_eq!(meta.reward_rate, 1);
        assert_eq!(meta.created, 0);

        assert_eq!(<DeadlineOf<Test>>::get(&Did::<Test>::zero(), &ad), Some(1));

        assert_eq!(Balances::free_balance(ALICE), 50);

        let pool = meta.pot;

        assert_eq!(<Test as Config>::Tags::tags_of(&ad), hashes);

        assert_eq!(Balances::free_balance(pool), 50);
    });
}

#[test]
fn should_fail_when_insufficient() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Ad::create(Origin::signed(ALICE), 200, vec![], [0u8; 64].into(), 1, 1),
            pallet_balances::Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn should_fail_when_tag_not_exists() {
    new_test_ext().execute_with(|| {
        let tags = vec![
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8],
            vec![0u8; 6],
        ];

        assert_noop!(
            Ad::create(Origin::signed(ALICE), 200, tags, [0u8; 64].into(), 1, 1),
            Error::<Test>::TagNotExists
        );
    });
}

#[test]
fn should_update_reward_rate() {
    new_test_ext().execute_with(|| {
        assert_ok!(Ad::create(
            Origin::signed(ALICE),
            50,
            vec![],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        assert_ok!(Ad::update_reward_rate(Origin::signed(ALICE), ad, 2));

        assert_eq!(<Metadata<Test>>::get(&ad).unwrap().reward_rate, 2);
    });
}

#[test]
fn should_fail_when_not_exists_or_not_owned() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Ad::update_reward_rate(Origin::signed(ALICE), Default::default(), 2),
            Error::<Test>::NotExists
        );

        assert_ok!(Ad::create(
            Origin::signed(ALICE),
            50,
            vec![],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        assert_noop!(
            Ad::update_reward_rate(Origin::signed(BOB), ad, 2),
            Error::<Test>::NotOwned
        );
    });
}

#[test]
fn should_update_tags() {
    new_test_ext().execute_with(|| {
        let tags = vec![
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8],
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
        ];

        let mut hashes = vec![];
        for tag in &tags {
            let hash = Tag::key(tag);
            hashes.push(hash);
        }

        assert_ok!(Ad::create(
            Origin::signed(ALICE),
            50,
            vec![vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        assert_ok!(Ad::update_tags(Origin::signed(ALICE), ad, tags));

        assert_eq!(<Test as Config>::Tags::tags_of(&ad), hashes);
    });
}

#[test]
fn should_add_budget() {
    new_test_ext().execute_with(|| {
        assert_ok!(Ad::create(
            Origin::signed(ALICE),
            50,
            vec![],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        assert_ok!(Ad::add_budget(Origin::signed(ALICE), ad, 20));

        let meta = <Metadata<Test>>::get(&ad).unwrap();

        assert_eq!(meta.budget, 70);
        assert_eq!(meta.remain, 70);

        assert_eq!(Balances::free_balance(meta.pot), 70);
    });
}

#[test]
fn should_bid() {
    new_test_ext().execute_with(|| {
        // 1. prepare

        assert_ok!(Nft::back(Origin::signed(BOB), DID_ALICE, 2_000_100u128));

        assert_ok!(Nft::mint(
            Origin::signed(ALICE),
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        // ad1

        assert_ok!(Ad::create(
            Origin::signed(BOB),
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
            Origin::signed(CHARLIE),
            500,
            vec![],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad2 = <Metadata<Test>>::iter_keys().next().unwrap();
        let meta2 = <Metadata<Test>>::get(&ad2).unwrap();
        ensure_remain!(meta2, 500, 0);

        // 2. BOB bid for ad1

        assert_noop!(
            Ad::bid(Origin::signed(BOB), ad1, DID_ALICE, 600),
            pallet_balances::Error::<Test>::InsufficientBalance
        );

        assert_ok!(Ad::bid(Origin::signed(BOB), ad1, DID_ALICE, 400));

        // ensure: deadline, slot, remain

        assert_eq!(<DeadlineOf<Test>>::get(&DID_ALICE, &ad1), Some(43200));

        let maybe_slot = <SlotOf<Test>>::get(&DID_ALICE);
        assert_ne!(maybe_slot, None);
        let slot = maybe_slot.unwrap();
        assert_eq!(slot.ad, ad1);
        assert_eq!(slot.budget, 199);
        assert_eq!(slot.remain, 199);

        let meta1 = <Metadata<Test>>::get(&ad1).unwrap();
        ensure_remain!(meta1, 100, slot.remain);

        // 3. CHARLIE bid for ad2

        assert_noop!(
            Ad::bid(Origin::signed(CHARLIE), ad2, DID_ALICE, 400),
            Error::<Test>::Underbid
        );

        assert_ok!(Ad::bid(Origin::signed(CHARLIE), ad2, DID_ALICE, 480));

        // ensure: deadline, slot, remain

        assert_eq!(<DeadlineOf<Test>>::get(&DID_ALICE, &ad1), None);
        assert_eq!(<DeadlineOf<Test>>::get(&DID_ALICE, &ad2), Some(1));

        let maybe_slot = <SlotOf<Test>>::get(&DID_ALICE);
        assert_ne!(maybe_slot, None);
        let slot = maybe_slot.unwrap();
        assert_eq!(slot.ad, ad2);
        assert_eq!(slot.budget, 239);
        assert_eq!(slot.remain, 239);

        let meta1 = <Metadata<Test>>::get(&ad1).unwrap();
        ensure_remain!(meta1, 496, 0);

        let meta2 = <Metadata<Test>>::get(&ad2).unwrap();
        ensure_remain!(meta2, 20, slot.remain);
    });
}

#[test]
fn should_drawback() {
    new_test_ext().execute_with(|| {
        // 1. prepare

        assert_ok!(Nft::back(Origin::signed(BOB), DID_ALICE, 2_000_100u128));

        assert_ok!(Nft::mint(
            Origin::signed(ALICE),
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        // create ad

        assert_ok!(Ad::create(
            Origin::signed(BOB),
            500,
            vec![],
            [0u8; 64].into(),
            1,
            43200 * 2
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        // bid

        assert_ok!(Ad::bid(Origin::signed(BOB), ad, DID_ALICE, 400));

        // 2. step in

        System::set_block_number(43200);

        Ad::on_initialize(System::block_number());

        // ensure slot, remain

        assert_eq!(<SlotOf<Test>>::get(&DID_ALICE), None);

        let meta = <Metadata<Test>>::get(&ad).unwrap();
        ensure_remain!(meta, 496, 0);

        // 3. step in

        System::set_block_number(43200 * 2);

        Ad::on_initialize(System::block_number());

        // ensure remain

        let meta = <Metadata<Test>>::get(&ad).unwrap();
        ensure_remain!(meta, 0, 0);

        assert_eq!(
            Balances::free_balance(&BOB),
            3_000_000 - 2_000_100 - 500 + 496
        );
    });
}

#[test]
fn should_pay() {
    new_test_ext().execute_with(|| {
        // 1. prepare

        assert_ok!(Tag::create(Origin::signed(ALICE), b"Test".to_vec()));

        assert_ok!(Nft::back(Origin::signed(BOB), DID_ALICE, 2_000_100u128));

        assert_ok!(Nft::mint(
            Origin::signed(ALICE),
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        // create ad

        assert_ok!(Ad::create(
            Origin::signed(BOB),
            500,
            vec![b"Test".to_vec()],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();
        let meta = <Metadata<Test>>::get(&ad).unwrap();

        // bid

        assert_ok!(Ad::bid(Origin::signed(BOB), ad, DID_ALICE, 400));

        // 2. pay

        assert_ok!(Ad::pay(
            Origin::signed(BOB),
            ad,
            DID_ALICE,
            DID_CHARLIE,
            vec![(b"Test".to_vec(), 5)],
            None
        ));

        let slot = <SlotOf<Test>>::get(&DID_ALICE).unwrap();
        assert_eq!(slot.remain, 194);
        assert_eq!(Assets::balance(0, &meta.pot), 194);

        assert_eq!(Assets::balance(0, &CHARLIE), 5);

        assert_eq!(Tag::get_score(&DID_CHARLIE, b"Test".to_vec()), 5);
    });
}
