use crate::{mock::*, Deposit, Deposits, Error};
use frame_support::{assert_noop, assert_ok, traits::Hooks};

#[test]
fn should_back() {
    new_test_ext().execute_with(|| {
        let meta = Did::meta(&DID_ALICE).unwrap();

        assert_ok!(Nft::back(Origin::signed(BOB), DID_ALICE, 50));

        let deposit = <Deposit<Test>>::get(&DID_ALICE);
        assert_eq!(deposit, Some(50));

        let deposit = <Deposits<Test>>::get(&DID_ALICE, &DID_BOB);
        assert_eq!(deposit, Some(50));

        assert_eq!(Balances::free_balance(&meta.pot), 50);

        assert_ok!(Nft::back(Origin::signed(CHARLIE), DID_ALICE, 30));

        let deposit = <Deposit<Test>>::get(&DID_ALICE);
        assert_eq!(deposit, Some(80));

        assert_eq!(Balances::free_balance(&meta.pot), 80);
    });
}

#[test]
fn should_fail_when_self() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Nft::back(Origin::signed(ALICE), DID_ALICE, 50),
            Error::<Test>::YourSelf
        );
    });
}

#[test]
fn should_fail_when_minted() {
    new_test_ext().execute_with(|| {
        assert_ok!(Nft::back(Origin::signed(BOB), DID_ALICE, 2_000_100u128));

        assert_ok!(Nft::mint(
            Origin::signed(ALICE),
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        assert_noop!(
            Nft::mint(
                Origin::signed(ALICE),
                b"Test Token".to_vec(),
                b"XTT".to_vec()
            ),
            Error::<Test>::Minted
        );

        assert_noop!(
            Nft::back(Origin::signed(BOB), DID_ALICE, 50),
            Error::<Test>::Minted
        );
    });
}

#[test]
fn should_fail_when_insufficient_balance() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Nft::back(Origin::signed(BOB), DID_ALICE, 3_000_100u128),
            pallet_balances::Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn should_mint() {
    new_test_ext().execute_with(|| {
        assert_ok!(Nft::back(Origin::signed(BOB), DID_ALICE, 2_000_100u128));

        assert_ok!(Nft::mint(
            Origin::signed(ALICE),
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        let meta = Did::meta(&DID_ALICE).unwrap();
        assert_eq!(meta.nft, Some(0));

        let deposit = <Deposit<Test>>::get(&DID_ALICE);
        assert_eq!(deposit, Some(2_000_100u128));

        let deposit_bob = <Deposits<Test>>::get(&DID_ALICE, &DID_BOB);
        assert_eq!(deposit_bob, deposit);

        let deposit_kol = <Deposits<Test>>::get(&DID_ALICE, &DID_ALICE);
        assert_eq!(deposit_kol, deposit);
    });
}

#[test]
fn should_fail_when_insufficient() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Nft::mint(
                Origin::signed(ALICE),
                b"Test Token".to_vec(),
                b"XTT".to_vec()
            ),
            Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn should_claim() {
    new_test_ext().execute_with(|| {
        assert_ok!(Nft::back(Origin::signed(BOB), DID_ALICE, 2_000_000u128));
        assert_ok!(Nft::back(Origin::signed(CHARLIE), DID_ALICE, 1_000_000u128));

        assert_ok!(Nft::mint(
            Origin::signed(ALICE),
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        assert_ok!(Nft::claim(Origin::signed(BOB), DID_ALICE));
        assert_ok!(Nft::claim(Origin::signed(CHARLIE), DID_ALICE));

        assert_eq!(Assets::balance(0, &BOB), 666_666);
        assert_eq!(Assets::balance(0, &CHARLIE), 333_333);

        assert_eq!(<Deposits<Test>>::get(&DID_ALICE, &DID_BOB), None);
        assert_eq!(<Deposits<Test>>::get(&DID_ALICE, &DID_CHARLIE), None);

        assert_noop!(
            Nft::claim(Origin::signed(ALICE), DID_ALICE),
            Error::<Test>::NoToken
        );

        System::set_block_number(5);

        assert_ok!(Nft::claim(Origin::signed(ALICE), DID_ALICE));

        assert_eq!(Assets::balance(0, &ALICE), 1_000_000);
        assert_eq!(<Deposits<Test>>::get(&DID_ALICE, &DID_ALICE), None);
    });
}

#[test]
fn should_farming() {
    new_test_ext().execute_with(|| {
        assert_ok!(Nft::back(Origin::signed(BOB), DID_ALICE, 2_000_000u128));
        assert_ok!(Nft::back(Origin::signed(CHARLIE), DID_ALICE, 1_000_000u128));

        assert_ok!(Nft::mint(
            Origin::signed(ALICE),
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        assert_ok!(Nft::claim(Origin::signed(BOB), DID_ALICE));
        assert_ok!(Nft::claim(Origin::signed(CHARLIE), DID_ALICE));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(BOB),
            0,
            500_000,
            1,
            300_000,
            2
        ));
        assert_ok!(Swap::add_liquidity(
            Origin::signed(CHARLIE),
            0,
            400_000,
            1,
            300_000,
            2
        ));

        System::set_block_number(1);

        Nft::on_initialize(System::block_number());

        assert_eq!(Assets::balance(0, &BOB), 500_000 + 12);
        assert_eq!(Assets::balance(0, &CHARLIE), 200_000 + 10);
    });
}
