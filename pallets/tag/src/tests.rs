use crate::{mock::*, Error, HashOf, Metadata};
use frame_support::{assert_noop, assert_ok, traits::Currency};
use sp_core::sr25519;
use sp_runtime::DispatchError;

#[test]
fn should_create() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        assert_eq!(Balances::free_balance(&alice), 100);
        assert_eq!(Balances::total_issuance(), 100);

        let tag = vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8];

        assert_ok!(Tag::create(Origin::signed(alice), tag.clone()));

        let maybe_tag = <Metadata<Test>>::get(&tag);
        assert_ne!(maybe_tag, None);

        assert_eq!(Balances::free_balance(&alice), 99);
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

        Balances::make_free_balance_be(&bob, 1);

        assert_eq!(Balances::free_balance(&bob), 1);
        assert_eq!(Balances::total_issuance(), 101);

        let tag = vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8];

        assert_noop!(
            Tag::create(Origin::signed(bob), tag.clone()),
            Error::<Test>::InsufficientBalance
        );

        let maybe_tag = <Metadata<Test>>::get(&tag);
        assert_eq!(maybe_tag, None);

        assert_eq!(Balances::free_balance(&bob), 1);
        assert_eq!(Balances::total_issuance(), 101);
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
fn tags_trait() {
    use parami_traits::Tags;

    new_test_ext().execute_with(|| {
        let tag1 = vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8];
        let tag2 = vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8];

        let hash1 = Tag::key(&tag1);
        let hash2 = Tag::key(&tag2);

        let ad = <HashOf<Test>>::default();

        assert_ok!(Tag::add_tag(&ad, tag1.clone()));
        assert_eq!(Tag::tags_of(&ad), vec![hash1.clone()]);
        assert_eq!(Tag::has_tag(&ad, &tag1), true);

        assert_ok!(Tag::add_tag(&ad, tag2.clone()));
        assert_eq!(Tag::tags_of(&ad), vec![hash2.clone(), hash1.clone()]);

        assert_ok!(Tag::del_tag(&ad, &tag2));
        assert_eq!(Tag::tags_of(&ad), vec![hash1.clone()]);

        assert_ok!(Tag::clr_tag(&ad));
        assert_eq!(Tag::tags_of(&ad), Vec::<Vec<u8>>::new());

        let did = DID::from_slice(&[0xff; 20]);

        assert_ok!(Tag::influence(&did, &tag1, 5));
        assert_eq!(Tag::personas_of(&did), vec![(hash1.clone(), 5)]);
        assert_eq!(Tag::get_score(&did, &tag1), 5);

        let did = DID::from_slice(&[0xff; 20]);

        assert_ok!(Tag::impact(&did, &tag1, 3));
        assert_eq!(Tag::influences_of(&did), vec![(hash1.clone(), 3)]);
        assert_eq!(Tag::get_influence(&did, &tag1), 3);
    });
}
