use crate::{mock::*, Error, InfluencesOf, Metadata, PersonasOf};
use frame_support::{assert_noop, assert_ok};
use sp_core::sr25519;
use sp_runtime::DispatchError;

#[test]
fn should_create() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        assert_eq!(Balances::free_balance(alice), 100);
        assert_eq!(Balances::total_issuance(), 100);

        let tag = vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8];

        assert_ok!(Tag::create(Origin::signed(alice), tag.clone()));

        let maybe_tag = <Metadata<Test>>::get(&tag);
        assert_ne!(maybe_tag, None);

        assert_eq!(Balances::free_balance(alice), 99);
        assert_eq!(Balances::total_issuance(), 99);
    });
}

#[test]
fn should_fail_when_did_not_exists() {
    new_test_ext().execute_with(|| {
        let charlie = sr25519::Public([3; 32]);

        let tag = vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8];

        assert_noop!(
            Tag::create(Origin::signed(charlie), tag),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn should_fail_when_insufficient() {
    new_test_ext().execute_with(|| {
        let bob = sr25519::Public([2; 32]);

        assert_eq!(Balances::free_balance(bob), 0);
        assert_eq!(Balances::total_issuance(), 100);

        let tag = vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8];

        assert_noop!(
            Tag::create(Origin::signed(bob), tag.clone()),
            Error::<Test>::InsufficientBalance
        );

        let maybe_tag = <Metadata<Test>>::get(&tag);
        assert_eq!(maybe_tag, None);

        assert_eq!(Balances::free_balance(bob), 0);
        assert_eq!(Balances::total_issuance(), 100);
    });
}

#[test]
fn should_force_create() {
    new_test_ext().execute_with(|| {
        assert_eq!(Balances::total_issuance(), 100);

        let tag = vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8];

        assert_ok!(Tag::force_create(Origin::root(), tag.clone()));

        let maybe_tag = <Metadata<Test>>::get(&tag);
        assert_ne!(maybe_tag, None);

        assert_eq!(Balances::total_issuance(), 100);
    });
}

#[test]
fn should_scoring() {
    new_test_ext().execute_with(|| {
        let tag1 = vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8];
        let tag2 = vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8];

        assert_ok!(Tag::force_create(Origin::root(), tag1.clone()));

        let hash1 = <Metadata<Test>>::hashed_key_for(&tag1);
        let hash2 = <Metadata<Test>>::hashed_key_for(&tag2);

        let did = DID::from_slice(&[0xff; 20]);

        assert_ok!(Tag::influence(did, tag1.clone(), 5));
        assert_eq!(<PersonasOf<Test>>::get(&did, &hash1), Some(5));
        assert_ok!(Tag::influence(did, tag1.clone(), 3));
        assert_eq!(<PersonasOf<Test>>::get(&did, &hash1), Some(8));

        assert_noop!(
            Tag::influence(did, tag2.clone(), 5),
            Error::<Test>::NotExists
        );
        assert_eq!(<PersonasOf<Test>>::get(&did, &hash2), None);

        assert_ok!(Tag::impact(did, tag1.clone(), 5));
        assert_eq!(<InfluencesOf<Test>>::get(&did, &hash1), Some(5));
        assert_ok!(Tag::impact(did, tag1.clone(), 3));
        assert_eq!(<InfluencesOf<Test>>::get(&did, &hash1), Some(8));
        assert_noop!(Tag::impact(did, tag2.clone(), 5), Error::<Test>::NotExists);
        assert_eq!(<InfluencesOf<Test>>::get(&did, &hash2), None);
    });
}
