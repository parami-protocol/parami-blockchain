use crate::{mock::*, AdsOf, Config, DeadlineOf, Did, EndtimeOf, Error, Metadata, SlotOf};
use frame_support::{
    assert_noop, assert_ok,
    traits::{Currency, Hooks},
};
use parami_traits::Tags;
use sp_core::{sr25519, H160};
use sp_runtime::traits::Hash;
use sp_std::collections::btree_map::BTreeMap;

#[test]
fn should_create() {
    new_test_ext().execute_with(|| {
        let tags = vec![
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8],
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
        ];

        let mut hashes = BTreeMap::new();
        for tag in &tags {
            let hash = Tag::key(tag);
            hashes.insert(hash, true);
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
        assert_eq!(Balances::free_balance(&meta.pot), meta.budget);
        assert_eq!(meta.creator, DID_ALICE);
        assert_eq!(meta.budget, 50);
        assert_eq!(meta.remain, 50);
        assert_eq!(meta.metadata, metadata);
        assert_eq!(meta.reward_rate, 1);
        assert_eq!(meta.created, 0);

        assert_eq!(<EndtimeOf<Test>>::get(&ad), Some(1));

        assert_eq!(Balances::free_balance(&ALICE), 100 - meta.budget);

        assert_eq!(<Test as Config>::Tags::tags_of(&ad), hashes);
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

        let mut hashes = BTreeMap::new();
        for tag in &tags {
            let hash = Tag::key(tag);
            hashes.insert(hash, true);
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
        assert_eq!(Balances::free_balance(&meta.pot), meta.budget);
        assert_eq!(meta.budget, 50 + 20);
        assert_eq!(meta.remain, 50 + 20);
    });
}

#[test]
fn should_bid() {
    new_test_ext().execute_with(|| {
        // 1. prepare

        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::back(Origin::signed(BOB), nft, 2_000_100u128));

        assert_ok!(Nft::mint(
            Origin::signed(ALICE),
            nft,
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        let nft_meta = Nft::meta(nft).unwrap();

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
        assert_eq!(Balances::free_balance(&meta1.pot), meta1.budget);
        assert_eq!(meta1.budget, 500);
        assert_eq!(meta1.remain, 500);

        // ad2

        assert_ok!(Ad::create(
            Origin::signed(CHARLIE),
            600,
            vec![],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad2 = <Metadata<Test>>::iter_keys().next().unwrap();

        let meta2 = <Metadata<Test>>::get(&ad2).unwrap();
        assert_eq!(Balances::free_balance(&meta2.pot), meta2.budget);
        assert_eq!(meta2.budget, 600);
        assert_eq!(meta2.remain, 600);

        // 2. bob bid for ad1

        assert_noop!(
            Ad::bid(Origin::signed(BOB), ad1, nft, 600, None, None),
            Error::<Test>::InsufficientBalance
        );

        assert_ok!(Ad::bid(Origin::signed(BOB), ad1, nft, 400, None, None));

        // ensure: deadline, slot, remain

        assert_eq!(<EndtimeOf<Test>>::get(&ad1), Some(43200));
        assert_eq!(<DeadlineOf<Test>>::get(nft, &ad1), Some(43200));

        let maybe_slot = <SlotOf<Test>>::get(nft);
        assert_ne!(maybe_slot, None);

        let meta1 = <Metadata<Test>>::get(&ad1).unwrap();
        assert_eq!(Balances::free_balance(&meta1.pot), meta1.budget - 40);
        assert_eq!(meta1.remain, 500 - 400);

        let slot = maybe_slot.unwrap();
        assert_eq!(
            Assets::balance(nft_meta.token_asset_id, &meta1.pot),
            slot.fractions_remain
        );
        assert_eq!(slot.ad_id, ad1);
        assert_eq!(slot.budget, 400);
        assert_eq!(slot.remain, 400 - 40);
        assert_eq!(slot.fractions_remain, 19);

        // 3. charlie bid for ad2

        assert_noop!(
            Ad::bid(Origin::signed(CHARLIE), ad2, nft, 400, None, None),
            Error::<Test>::Underbid
        );

        assert_ok!(Ad::bid(Origin::signed(CHARLIE), ad2, nft, 480, None, None));

        // ensure: deadline, slot, remain

        assert_eq!(<EndtimeOf<Test>>::get(&ad2), Some(1));
        assert_eq!(<DeadlineOf<Test>>::get(nft, &ad1), None);
        assert_eq!(<DeadlineOf<Test>>::get(nft, &ad2), Some(1));

        let maybe_slot = <SlotOf<Test>>::get(nft);
        assert_ne!(maybe_slot, None);

        let meta1 = <Metadata<Test>>::get(&ad1).unwrap();
        assert_eq!(Balances::free_balance(&meta1.pot), meta1.remain);
        assert_eq!(meta1.remain, 497);

        let meta2 = <Metadata<Test>>::get(&ad2).unwrap();
        assert_eq!(Balances::free_balance(&meta2.pot), meta2.budget - 48);
        assert_eq!(meta2.remain, 600 - 480);

        let slot = maybe_slot.unwrap();
        assert_eq!(Assets::balance(0, &meta1.pot), 0);

        assert_eq!(
            Assets::balance(nft_meta.token_asset_id, &meta2.pot),
            slot.fractions_remain
        );
        assert_eq!(slot.ad_id, ad2);
        assert_eq!(slot.budget, 480);
        assert_eq!(slot.remain, 480 - 48);
        assert_eq!(slot.fractions_remain, 23);
    });
}

#[test]
fn should_drawback() {
    new_test_ext().execute_with(|| {
        // 1. prepare

        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::back(Origin::signed(BOB), nft, 2_000_100u128));

        assert_ok!(Nft::mint(
            Origin::signed(ALICE),
            nft,
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

        assert_ok!(Ad::bid(Origin::signed(BOB), ad, nft, 400, None, None));

        // 2. step in

        System::set_block_number(43200);

        Ad::on_initialize(System::block_number());

        // ensure slot, remain

        assert_eq!(<SlotOf<Test>>::get(nft), None);

        let meta = <Metadata<Test>>::get(&ad).unwrap();
        assert_eq!(meta.remain, 497);

        assert_eq!(Balances::free_balance(&meta.pot), meta.remain);
        assert_eq!(Assets::balance(0, &meta.pot), 0);

        // 3. step in

        System::set_block_number(43200 * 2);

        Ad::on_initialize(System::block_number());

        // ensure remain

        let meta = <Metadata<Test>>::get(&ad).unwrap();

        assert_eq!(meta.remain, 0);
        assert_eq!(Balances::free_balance(&meta.pot), meta.remain);

        assert_eq!(
            Balances::free_balance(&BOB),
            3_000_000 - 2_000_100 - 500 + 497
        );
    });
}
type HashOf<T> = <<T as frame_system::Config>::Hashing as Hash>::Output;
type NftOf<T> = <T as parami_nft::Config>::AssetId;
fn prepare_pay() -> (HashOf<Test>, NftOf<Test>) {
    // 1. prepare

    let nft = Nft::preferred(DID_ALICE).unwrap();

    assert_ok!(Nft::back(Origin::signed(BOB), nft, 2_000_100u128));

    assert_ok!(Nft::mint(
        Origin::signed(ALICE),
        nft,
        b"Test Token".to_vec(),
        b"XTT".to_vec()
    ));
    // create ad

    assert_ok!(Ad::create(
        Origin::signed(BOB),
        500,
        vec![
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8]
        ],
        [0u8; 64].into(),
        1,
        1
    ));

    let ad = <Metadata<Test>>::iter_keys().next().unwrap();

    // bid

        assert_ok!(Ad::bid(Origin::signed(BOB), ad, nft, 400, None, None));

    return (ad, nft);
}

#[test]
fn should_pay() {
    new_test_ext().execute_with(|| {
        // 1. prepare
        let (ad, nft) = prepare_pay();

        // 2. pay

        assert_ok!(Ad::pay(
            Origin::signed(BOB),
            ad,
            nft,
            DID_CHARLIE,
            vec![(vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
            None
        ));

        let nft_meta = Nft::meta(nft).unwrap();
        let meta = <Metadata<Test>>::get(&ad).unwrap();
        let slot = <SlotOf<Test>>::get(nft).unwrap();
        assert_eq!(
            Assets::balance(nft_meta.token_asset_id, &meta.pot),
            slot.fractions_remain
        );
        assert_eq!(slot.remain, 400 - 40);
        assert_eq!(slot.fractions_remain, 19 - 2);

        assert_eq!(Assets::balance(nft_meta.token_asset_id, &CHARLIE), 2);

        assert_eq!(
            Tag::get_score(&DID_CHARLIE, vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8]),
            11
        );
    });
}

#[test]
fn should_pay_3_for_taga5_tagb2() {
    new_test_ext().execute_with(|| {
        // 1. prepare
        let (ad, nft) = prepare_pay();
        let nft_meta = Nft::meta(nft).unwrap();
        // 2 pay
        assert_ok!(Ad::pay(
            Origin::signed(BOB),
            ad,
            nft,
            DID_TAGA5_TAGB2,
            vec![(vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
            None
        ));

        assert_eq!(Assets::balance(nft_meta.token_asset_id, &TAGA5_TAGB2), 3);
    });
}

#[test]
fn should_pay_0_when_all_tags_score_are_zero() {
    new_test_ext().execute_with(|| {
        // 1. prepare
        let (ad, nft) = prepare_pay();
        let nft_meta = Nft::meta(nft).unwrap();
        // 2 pay
        assert_ok!(Ad::pay(
            Origin::signed(BOB),
            ad,
            nft,
            DID_TAGA0_TAGB0,
            vec![(vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
            None
        ));

        assert_eq!(Assets::balance(nft_meta.token_asset_id, &TAGA0_TAGB0), 0);
    });
}
#[test]
fn should_pay_10_when_all_tags_are_full_score() {
    new_test_ext().execute_with(|| {
        // 1. prepare
        let (ad, nft) = prepare_pay();
        let nft_meta = Nft::meta(nft).unwrap();
        // 2 pay
        assert_ok!(Ad::pay(
            Origin::signed(BOB),
            ad,
            nft,
            DID_TAGA100_TAGB100,
            vec![(vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
            None
        ));

        assert_eq!(
            Assets::balance(nft_meta.token_asset_id, &TAGA100_TAGB100),
            10
        );
    });
}
#[test]
fn should_pay_10_when_all_tags_are_fullscore_or_overflow() {
    new_test_ext().execute_with(|| {
        // 1. prepare
        let (ad, nft) = prepare_pay();
        let nft_meta = Nft::meta(nft).unwrap();
        // 2 pay
        assert_ok!(Ad::pay(
            Origin::signed(BOB),
            ad,
            nft,
            DID_TAGA120_TAGB0,
            vec![(vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
            None
        ));

        assert_eq!(Assets::balance(nft_meta.token_asset_id, &TAGA120_TAGB0), 10);
    });
}

#[test]
fn should_auto_swap_when_swapped_token_used_up() {
    new_test_ext().execute_with(|| {
        // 1. prepare

        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::back(Origin::signed(BOB), nft, 2_000_100u128));

        assert_ok!(Nft::mint(
            Origin::signed(ALICE),
            nft,
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        // create ad

        assert_ok!(Ad::create(
            Origin::signed(BOB),
            500,
            vec![vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8]],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        // bid

        assert_ok!(Ad::bid(Origin::signed(BOB), ad, nft, 400, None, None));

        // 2. pay to 9 users, 5 tokens each
        let viewer_dids = make_dids(9u8);
        for viewer_did in &viewer_dids {
            Tag::influence(viewer_did, vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5).unwrap();

            assert_ok!(Ad::pay(
                Origin::signed(BOB),
                ad,
                nft,
                *viewer_did,
                vec![(vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
                None
            ));
        }

        let slot = <SlotOf<Test>>::get(nft).unwrap();
        assert_eq!(slot.remain, 400 - 40 * 3);
    });
}

fn make_dids(num: u8) -> Vec<H160> {
    let mut res: Vec<H160> = Vec::new();
    for i in 0..num {
        let temp_account: sr25519::Public = sr25519::Public([i + 20; 32]);

        <Test as parami_did::Config>::Currency::make_free_balance_be(&temp_account, 2);
        assert_ok!(Did::<Test>::register(Origin::signed(temp_account), None));
        let temp_did = Did::<Test>::did_of(temp_account).unwrap();
        res.push(temp_did);
    }
    return res;
}
