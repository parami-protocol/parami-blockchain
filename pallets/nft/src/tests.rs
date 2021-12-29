use crate::*;
use crate::{mock::*, Deposit, Deposits, Error};
use frame_support::{assert_noop, assert_ok};

#[test]
fn should_back() {
    new_test_ext().execute_with(|| {
        assert_ok!(Nft::back(Origin::signed(BOB), DID_ALICE, 50));

        let nft_id = Nft::get_preferred(DID_ALICE).unwrap();

        let meta = <NftMetaStore<Test>>::get(nft_id).unwrap();

        let deposit = <Deposit<Test>>::get(nft_id);
        assert_eq!(deposit, Some(50));

        let deposit = <Deposits<Test>>::get(nft_id, &DID_BOB);
        assert_eq!(deposit, Some(50));

        assert_eq!(Balances::free_balance(&meta.pot), 50);

        assert_ok!(Nft::back(Origin::signed(CHARLIE), DID_ALICE, 30));

        let deposit = <Deposit<Test>>::get(nft_id);
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
        let r = Nft::back(Origin::signed(BOB), DID_ALICE, 3_000_100u128);
        assert_noop!(r, pallet_balances::Error::<Test>::InsufficientBalance);
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

        let nft_id: NftIdOf<Test> = Nft::get_preferred(DID_ALICE).unwrap();
        let deposit = <Deposit<Test>>::get(&nft_id);
        assert_eq!(deposit, Some(2_000_100u128));

        let deposit_bob = <Deposits<Test>>::get(nft_id, &DID_BOB);
        assert_eq!(deposit_bob, deposit);

        let deposit_kol = <Deposits<Test>>::get(nft_id, &DID_ALICE);
        assert_eq!(deposit_kol, deposit);
    });
}

#[test]
fn should_fail_when_insufficient() {
    new_test_ext().execute_with(|| {
        let r = Nft::mint(
            Origin::signed(ALICE),
            b"Test Token".to_vec(),
            b"XTT".to_vec(),
        );

        assert_noop!(r, Error::<Test>::InsufficientBalance);
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

        let nft_id: NftIdOf<Test> = Nft::get_preferred(DID_ALICE).unwrap();

        assert_eq!(Assets::balance(nft_id, &BOB), 666_666);
        assert_eq!(Assets::balance(nft_id, &CHARLIE), 333_333);

        assert_eq!(<Deposits<Test>>::get(nft_id, &DID_BOB), None);
        assert_eq!(<Deposits<Test>>::get(nft_id, &DID_CHARLIE), None);

        assert_noop!(
            Nft::claim(Origin::signed(BOB), DID_ALICE),
            Error::<Test>::NoToken
        );

        System::set_block_number(5);

        assert_ok!(Nft::claim(Origin::signed(ALICE), DID_ALICE));

        assert_eq!(Assets::balance(nft_id, &ALICE), 1_000_000);
        assert_eq!(<Deposits<Test>>::get(nft_id, &DID_ALICE), None);
    });
}
