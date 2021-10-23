use crate::{mock::*, DidOf, Error, Metadata, ReferrerOf};
use frame_support::{assert_noop, assert_ok};
use sp_core::sr25519;

#[test]
fn should_register() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(0);

        let bob = sr25519::Public([2; 32]);
        assert_ok!(Did::register(Origin::signed(bob), None));

        frame_system::Pallet::<Test>::set_block_number(1);

        assert_noop!(
            Did::register(Origin::signed(bob), None),
            Error::<Test>::Exists
        );

        let maybe_did = <DidOf<Test>>::get(bob);
        assert_ne!(maybe_did, None);

        let maybe_metadata = <Metadata<Test>>::get(maybe_did.unwrap());
        assert_ne!(maybe_metadata, None);
        assert_eq!(maybe_metadata.unwrap().revoked, false);
    });
}

#[test]
fn should_fail_when_exist() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);
        assert_noop!(
            Did::register(Origin::signed(alice), None),
            Error::<Test>::Exists
        );
    });
}

#[test]
fn should_register_with_referer() {
    new_test_ext().execute_with(|| {
        let did = DID::from_slice(&[0xff; 20]);

        let bob = sr25519::Public([2; 32]);
        assert_ok!(Did::register(Origin::signed(bob), Some(did)));

        let maybe_did = <DidOf<Test>>::get(bob);
        assert_ne!(maybe_did, None);

        let maybe_referrer = <ReferrerOf<Test>>::get(maybe_did.unwrap());
        assert_eq!(maybe_referrer, Some(did));
    });
}

#[test]
fn should_fail_when_referer_not_exist() {
    new_test_ext().execute_with(|| {
        let did = DID::from_slice(&[0xee; 20]);

        let bob = sr25519::Public([2; 32]);
        assert_noop!(
            Did::register(Origin::signed(bob), Some(did)),
            Error::<Test>::ReferrerNotExists
        );
    });
}

#[test]
fn should_revoke() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);
        let did = DID::from_slice(&[0xff; 20]);

        assert_ok!(Did::revoke(Origin::signed(alice)));

        assert!(!<DidOf<Test>>::contains_key(alice));

        let metadata = <Metadata<Test>>::get(did).unwrap();

        assert_eq!(metadata.revoked, true);
    });
}

#[test]
fn should_fail_when_not_exist() {
    new_test_ext().execute_with(|| {
        let bob = sr25519::Public([2; 32]);
        assert_noop!(Did::revoke(Origin::signed(bob)), Error::<Test>::NotExists);
    });
}

#[test]
fn should_reassign() {
    new_test_ext().execute_with(|| {
        frame_system::Pallet::<Test>::set_block_number(0);

        let bob = sr25519::Public([2; 32]);
        assert_ok!(Did::register(Origin::signed(bob), None));

        frame_system::Pallet::<Test>::set_block_number(1);

        let did = <DidOf<Test>>::get(bob).unwrap();

        assert_ok!(Did::revoke(Origin::signed(bob)));

        frame_system::Pallet::<Test>::set_block_number(2);

        assert_ok!(Did::register(Origin::signed(bob), None));

        assert_ne!(<DidOf<Test>>::get(bob), Some(did));
    });
}

#[test]
fn should_transfer() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);
        let bob = sr25519::Public([2; 32]);

        pallet_timestamp::Pallet::<Test>::set_timestamp(2);

        assert_ok!(Did::transfer(Origin::signed(alice), bob));

        assert_eq!(<DidOf<Test>>::get(alice), None);

        let maybe_did = <DidOf<Test>>::get(bob);
        assert_ne!(maybe_did, None);

        let maybe_metadata = <Metadata<Test>>::get(maybe_did.unwrap());
        assert_ne!(maybe_metadata, None);

        let metadata = maybe_metadata.unwrap();
        assert_eq!(metadata.account, bob);
        assert_eq!(metadata.created, 0);
        assert_eq!(metadata.revoked, false);
    });
}

#[test]
fn should_fail_when_already_have_did() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        assert_noop!(
            Did::transfer(Origin::signed(alice), alice),
            Error::<Test>::Exists
        );
    });
}

#[test]
fn should_fail_when_revoked() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);
        let bob = sr25519::Public([2; 32]);

        assert_ok!(Did::revoke(Origin::signed(alice)));

        assert_noop!(
            Did::transfer(Origin::signed(alice), bob),
            Error::<Test>::NotExists
        );
    });
}
