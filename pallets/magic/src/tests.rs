use crate::{mock::*, ControllerAccountOf, Error, StableAccountOf};
use frame_support::{assert_noop, assert_ok};
use sp_core::sr25519;

#[test]
fn should_create() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([0x1; 32]);
        let magic = sr25519::Public([0xe; 32]);

        assert_eq!(Balances::free_balance(alice), 100);
        assert_eq!(Balances::total_issuance(), 100);

        assert_ok!(Magic::create_stable_account(
            Origin::signed(alice),
            magic,
            1
        ));

        assert_eq!(Balances::free_balance(alice), 98);
        assert_eq!(Balances::total_issuance(), 100);

        let maybe_stash = <StableAccountOf<Test>>::get(alice);
        assert_ne!(maybe_stash, None);

        let stash = maybe_stash.unwrap();
        assert_eq!(stash.controller_account, alice);
        assert_eq!(stash.magic_account, magic);

        assert_eq!(Balances::free_balance(stash.stash_account), 1);
        assert_eq!(Balances::free_balance(magic), 1);

        assert_eq!(<ControllerAccountOf<Test>>::get(magic), Some(alice));
    });
}

#[test]
fn should_fail_when_insufficient() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([0x1; 32]);
        let magic = sr25519::Public([0xe; 32]);

        assert_noop!(
            Magic::create_stable_account(Origin::signed(alice), magic, 100),
            Error::<Test>::InsufficientBalance
        );
    });
}

#[test]
fn should_fail_when_magic_is_controller() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([0x1; 32]);
        let magic = sr25519::Public([0x1; 32]);

        assert_noop!(
            Magic::create_stable_account(Origin::signed(alice), magic, 1),
            Error::<Test>::ControllerEqualToMagic
        );
    });
}

#[test]
fn should_transfer() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([0x1; 32]);
        let bob = sr25519::Public([0x2; 32]);

        assert_eq!(Balances::free_balance(bob), 0);

        let magic = sr25519::Public([0xe; 32]);

        assert_ok!(Magic::create_stable_account(
            Origin::signed(alice),
            magic,
            1
        ));

        assert_ok!(Magic::change_controller(Origin::signed(magic), bob));

        assert_eq!(<StableAccountOf<Test>>::get(alice), None);

        let maybe_stash = <StableAccountOf<Test>>::get(bob);
        assert_ne!(maybe_stash, None);

        let stash = maybe_stash.unwrap();
        assert_eq!(stash.controller_account, bob);
        assert_eq!(stash.magic_account, magic);

        assert_eq!(Balances::free_balance(stash.stash_account), 1);
        assert_eq!(Balances::free_balance(magic), 1);
        assert_ne!(Balances::free_balance(bob), 0);

        assert_eq!(<ControllerAccountOf<Test>>::get(magic), Some(bob));
    });
}
