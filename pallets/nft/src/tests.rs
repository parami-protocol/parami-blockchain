use crate::{mock::*, Deposit, Deposits, Error};
use frame_support::{assert_noop, assert_ok};
use parami_did::Pallet as Did;
use sp_core::sr25519;

#[test]
fn should_back() {
    new_test_ext().execute_with(|| {
        let bob = sr25519::Public([2; 32]);
        let charlie = sr25519::Public([3; 32]);

        let did = DID::from_slice(&[0xee; 20]);
        let kol = DID::from_slice(&[0xff; 20]);

        let meta = Did::<Test>::meta(&kol).unwrap();

        assert_ok!(Nft::back(Origin::signed(bob), kol, 50));

        let maybe_deposit = <Deposit<Test>>::get(&kol);
        assert_ne!(maybe_deposit, None);
        let deposit = maybe_deposit.unwrap();
        assert_eq!(deposit, 50);

        let maybe_deposit = <Deposits<Test>>::get(&kol, &did);
        assert_ne!(maybe_deposit, None);
        let deposit = maybe_deposit.unwrap();
        assert_eq!(deposit, 50);

        assert_eq!(Balances::free_balance(meta.pot), 50);

        assert_ok!(Nft::back(Origin::signed(charlie), kol, 30));

        let maybe_deposit = <Deposit<Test>>::get(&kol);
        assert_ne!(maybe_deposit, None);
        let deposit = maybe_deposit.unwrap();
        assert_eq!(deposit, 80);

        assert_eq!(Balances::free_balance(meta.pot), 80);
    });
}

#[test]
fn should_fail_when_self() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        let kol = DID::from_slice(&[0xff; 20]);

        assert_noop!(
            Nft::back(Origin::signed(alice), kol, 50),
            Error::<Test>::YourSelf
        );
    });
}

#[test]
fn should_fail_when_minted() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);
        let bob = sr25519::Public([2; 32]);

        let kol = DID::from_slice(&[0xff; 20]);

        assert_ok!(Nft::back(Origin::signed(bob), kol, 2_000_100u128));

        assert_ok!(Nft::mint(
            Origin::signed(alice),
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        assert_noop!(
            Nft::mint(
                Origin::signed(alice),
                b"Test Token".to_vec(),
                b"XTT".to_vec()
            ),
            Error::<Test>::Minted
        );

        assert_noop!(
            Nft::back(Origin::signed(bob), kol, 50),
            Error::<Test>::Minted
        );
    });
}

#[test]
fn should_fail_when_insufficient_balance() {
    new_test_ext().execute_with(|| {
        let bob = sr25519::Public([2; 32]);

        let kol = DID::from_slice(&[0xff; 20]);

        assert_noop!(
            Nft::back(Origin::signed(bob), kol, 3_000_100u128),
            pallet_balances::Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn should_mint() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);
        let bob = sr25519::Public([2; 32]);

        let kol = DID::from_slice(&[0xff; 20]);

        assert_ok!(Nft::back(Origin::signed(bob), kol, 2_000_100u128));

        assert_ok!(Nft::mint(
            Origin::signed(alice),
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        let meta = Did::<Test>::meta(&kol).unwrap();
        assert_eq!(meta.nft, Some(0));
    });
}

#[test]
fn should_fail_when_insufficient() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        assert_noop!(
            Nft::mint(
                Origin::signed(alice),
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
        let alice = sr25519::Public([1; 32]);
        let bob = sr25519::Public([2; 32]);
        let charlie = sr25519::Public([3; 32]);

        let kol = DID::from_slice(&[0xff; 20]);
        let did2 = DID::from_slice(&[0xee; 20]);
        let did3 = DID::from_slice(&[0xdd; 20]);

        assert_ok!(Nft::back(Origin::signed(bob), kol, 2_000_000u128));
        assert_ok!(Nft::back(Origin::signed(charlie), kol, 1_000_000u128));

        assert_ok!(Nft::mint(
            Origin::signed(alice),
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        assert_ok!(Nft::claim(Origin::signed(bob), kol));
        assert_ok!(Nft::claim(Origin::signed(charlie), kol));

        assert_eq!(Assets::balance(0, &bob), 666_666);
        assert_eq!(Assets::balance(0, &charlie), 333_333);

        assert_eq!(<Deposits<Test>>::get(&kol, &did2), None);
        assert_eq!(<Deposits<Test>>::get(&kol, &did3), None);
    });
}
