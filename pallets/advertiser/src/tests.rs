use crate::{mock::*, Blocked, EnsureAdvertiser, Error};
use frame_support::{assert_noop, assert_ok};
use sp_core::sr25519;

#[test]
fn should_deposit() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        assert_eq!(Balances::free_balance(&alice), 100);
        assert_eq!(Balances::reserved_balance(alice), 0);

        assert_ok!(Advertiser::deposit(Origin::signed(alice), 10));

        assert_eq!(Balances::free_balance(&alice), 90);
        assert_eq!(Balances::reserved_balance(alice), 10);
    });
}

#[test]
fn should_fail_when_insufficient() {
    new_test_ext().execute_with(|| {
        let bob = sr25519::Public([2; 32]);

        assert_noop!(
            Advertiser::deposit(Origin::signed(bob), 10),
            pallet_balances::Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn should_fail_when_existential() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        assert_noop!(
            Advertiser::deposit(Origin::signed(alice), 9),
            Error::<Test>::ExistentialDeposit
        );

        assert_ok!(Advertiser::deposit(Origin::signed(alice), 10));
        assert_ok!(Advertiser::deposit(Origin::signed(alice), 1));
    });
}

#[test]
fn should_block() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);
        let did = DID::from_slice(&[0xff; 20]);

        assert_ok!(Advertiser::deposit(Origin::signed(alice), 10));

        assert_eq!(Balances::free_balance(&alice), 90);
        assert_eq!(Balances::reserved_balance(alice), 10);

        assert_ok!(Advertiser::force_block(Origin::root(), did));

        assert_eq!(Balances::free_balance(&alice), 90);
        assert_eq!(Balances::reserved_balance(alice), 0);

        assert_eq!(<Blocked<Test>>::get(&did), Some(true));

        assert_noop!(
            Advertiser::deposit(Origin::signed(alice), 10),
            Error::<Test>::Blocked
        );
    });
}

#[test]
fn should_ensure() {
    new_test_ext().execute_with(|| {
        use frame_support::traits::EnsureOrigin;

        type Ensure = EnsureAdvertiser<Test>;

        let alice = sr25519::Public([1; 32]);
        let did = DID::from_slice(&[0xff; 20]);

        assert!(Ensure::try_origin(Origin::signed(alice)).is_err());

        assert_ok!(Advertiser::deposit(Origin::signed(alice), 10));

        let ensure = Ensure::try_origin(Origin::signed(alice));
        assert!(ensure.is_ok());
        assert_eq!(ensure.unwrap(), (did, alice));
    });
}
