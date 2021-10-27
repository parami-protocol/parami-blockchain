use crate::{mock::*, DidOf, EnsureDid, Error, Metadata, ReferrerOf};
use frame_support::{assert_noop, assert_ok};
use sp_core::sr25519;

#[test]
fn should_register() {
    new_test_ext().execute_with(|| {
        System::set_block_number(0);

        let bob = sr25519::Public([2; 32]);
        assert_ok!(Did::register(Origin::signed(bob), None));

        System::set_block_number(1);

        assert_noop!(
            Did::register(Origin::signed(bob), None),
            Error::<Test>::Exists
        );

        let maybe_did = <DidOf<Test>>::get(&bob);
        assert_ne!(maybe_did, None);

        let maybe_meta = <Metadata<Test>>::get(maybe_did.unwrap());
        assert_ne!(maybe_meta, None);
        assert_eq!(maybe_meta.unwrap().revoked, false);
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

        let maybe_did = <DidOf<Test>>::get(&bob);
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

        assert!(!<DidOf<Test>>::contains_key(&alice));

        let meta = <Metadata<Test>>::get(&did).unwrap();

        assert_eq!(meta.revoked, true);
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
        System::set_block_number(0);

        let bob = sr25519::Public([2; 32]);
        assert_ok!(Did::register(Origin::signed(bob), None));

        System::set_block_number(1);

        let did = <DidOf<Test>>::get(&bob).unwrap();

        assert_ok!(Did::revoke(Origin::signed(bob)));

        System::set_block_number(2);

        assert_ok!(Did::register(Origin::signed(bob), None));

        assert_ne!(<DidOf<Test>>::get(&bob), Some(did));
    });
}

#[test]
fn should_transfer() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);
        let bob = sr25519::Public([2; 32]);

        Timestamp::set_timestamp(2);

        assert_ok!(Did::transfer(Origin::signed(alice), bob));

        assert_eq!(<DidOf<Test>>::get(&alice), None);

        let maybe_did = <DidOf<Test>>::get(&bob);
        assert_ne!(maybe_did, None);

        let maybe_meta = <Metadata<Test>>::get(maybe_did.unwrap());
        assert_ne!(maybe_meta, None);

        let meta = maybe_meta.unwrap();
        assert_eq!(meta.account, bob);
        assert_eq!(meta.created, 0);
        assert_eq!(meta.revoked, false);
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

#[test]
fn should_ensure() {
    new_test_ext().execute_with(|| {
        use frame_support::traits::EnsureOrigin;

        let alice = sr25519::Public([1; 32]);
        let bob = sr25519::Public([2; 32]);

        let did = DID::from_slice(&[0xff; 20]);

        let ensure = EnsureDid::<Test>::try_origin(Origin::signed(alice));
        assert!(ensure.is_ok());
        assert_eq!(ensure.unwrap(), (did, alice));

        assert!(EnsureDid::<Test>::try_origin(Origin::signed(bob)).is_err());
    });
}
