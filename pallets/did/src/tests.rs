use crate::{mock::*, DidOf, Metadata, TotalDids};
use frame_support::{assert_ok, traits::Currency};
use sp_core::sr25519;

#[test]
fn register_did_should_work() {
    new_test_ext().execute_with(|| {
        let acct1 = sr25519::Public([1; 32]);
        assert_ok!(Did::register(
            Origin::signed(sr25519::Public([1; 32])),
            sr25519::Public([1; 32]),
            None
        ));
        assert_eq!(<TotalDids<Test>>::get(), Some(1));
        assert_eq!(Balances::total_balance(&acct1), 10);
        //assert_eq!(Balances::total_balance(&2), 8);
        // should have a did
        let maybe_did = <DidOf<Test>>::get(acct1);
        assert!(maybe_did.is_some());
        // should have metadata
        let maybe_metadata = <Metadata<Test>>::get(maybe_did.unwrap());
        assert!(maybe_metadata.is_some());
        // not revoked
        assert!(!maybe_metadata.unwrap().3);

        // referrer should work
        let did1 = maybe_did.unwrap();
        assert_ok!(Did::register(
            Origin::signed(sr25519::Public([2; 32])),
            sr25519::Public([2; 32]),
            Some(did1)
        ));
        assert_eq!(<TotalDids<Test>>::get(), Some(2));

        // register for on-ex account on chain
        // 0.you cannot register before deposit
        assert!(Did::register_for(
            Origin::signed(sr25519::Public([1; 32])),
            sr25519::Public([3; 32]),
        )
        .is_err());
        // 1.first, lock amount
        assert_ok!(Did::lock(Origin::signed(sr25519::Public([1; 32])), 5));
        // 2.then, register
        assert_ok!(Did::register_for(
            Origin::signed(sr25519::Public([1; 32])),
            sr25519::Public([3; 32]),
        ));
    });
}

#[test]
fn refuse_wrong_public() {
    new_test_ext().execute_with(|| {
        assert!(Did::register(
            Origin::signed(sr25519::Public([2; 32])),
            sr25519::Public([1; 32]),
            None
        )
        .is_err());
    });
}

#[test]
fn refuse_nonex_referrer() {
    new_test_ext().execute_with(|| {
        assert!(Did::register(
            Origin::signed(sr25519::Public([1; 32])),
            sr25519::Public([1; 32]),
            Some([0xee; 20])
        )
        .is_err());
    });
}

#[test]
fn refuse_multiple_registrations() {
    new_test_ext().execute_with(|| {
        assert_ok!(Did::register(
            Origin::signed(sr25519::Public([1; 32])),
            sr25519::Public([1; 32]),
            None
        ));

        assert!(Did::register(
            Origin::signed(sr25519::Public([1; 32])),
            sr25519::Public([1; 32]),
            None
        )
        .is_err());
    });
}
