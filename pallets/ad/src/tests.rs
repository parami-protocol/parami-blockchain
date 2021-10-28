use crate::{mock::*, Config, Error, Metadata};
use frame_support::{assert_noop, assert_ok, traits::StoredMap};
use sp_core::sr25519;

#[test]
fn should_create() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);
        let did = DID::from_slice(&[0xff; 20]);

        assert_eq!(Balances::free_balance(alice), 100);

        let tags = vec![
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8],
        ];

        let mut hashes = vec![];
        for tag in &tags {
            let hash = Tag::key(tag);
            hashes.push(hash);
        }

        let meta = [0u8; 64].into();

        assert_ok!(Ad::create(
            Origin::signed(alice),
            50,
            tags.clone(),
            meta,
            1,
            1
        ));

        let maybe_ad = <Metadata<Test>>::iter_values().next();
        assert_ne!(maybe_ad, None);

        let ad = maybe_ad.unwrap();
        assert_eq!(ad.creator, did);
        assert_eq!(ad.budget, 50);
        assert_eq!(ad.remain, 50);
        assert_eq!(ad.metadata, meta);
        assert_eq!(ad.reward_rate, 1);
        assert_eq!(ad.created, 0);
        assert_eq!(ad.deadline, 1);

        assert_eq!(Balances::free_balance(alice), 50);

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        assert_eq!(<Test as Config>::TagsStore::get(&ad), hashes);
    });
}

#[test]
fn should_fail_when_insufficient() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        assert_noop!(
            Ad::create(Origin::signed(alice), 200, vec![], [0u8; 64].into(), 1, 1),
            pallet_balances::Error::<Test>::InsufficientBalance
        );

        assert_eq!(Balances::free_balance(alice), 100);
    });
}

#[test]
fn should_fail_when_tag_not_exists() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        let tags = vec![
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8],
            vec![0u8; 6],
        ];

        assert_noop!(
            Ad::create(Origin::signed(alice), 200, tags, [0u8; 64].into(), 1, 1),
            Error::<Test>::TagNotExists
        );

        assert_eq!(Balances::free_balance(alice), 100);
    });
}

#[test]
fn should_update_reward_rate() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        assert_ok!(Ad::create(
            Origin::signed(alice),
            50,
            vec![],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        assert_ok!(Ad::update_reward_rate(Origin::signed(alice), ad, 2));

        assert_eq!(<Metadata<Test>>::get(&ad).unwrap().reward_rate, 2);
    });
}

#[test]
fn should_fail_when_not_exists_or_not_owned() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        assert_noop!(
            Ad::update_reward_rate(Origin::signed(alice), Default::default(), 2),
            Error::<Test>::NotExists
        );

        assert_ok!(Ad::create(
            Origin::signed(alice),
            50,
            vec![],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        let bob = sr25519::Public([2; 32]);

        assert_noop!(
            Ad::update_reward_rate(Origin::signed(bob), ad, 2),
            Error::<Test>::NotOwned
        );
    });
}

#[test]
fn should_update_tags() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        let tags = vec![
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8],
        ];

        let mut hashes = vec![];
        for tag in &tags {
            let hash = Tag::key(tag);
            hashes.push(hash);
        }

        assert_ok!(Ad::create(
            Origin::signed(alice),
            50,
            vec![vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],],
            [0u8; 64].into(),
            1,
            1
        ));

        let ad = <Metadata<Test>>::iter_keys().next().unwrap();

        assert_ok!(Ad::update_tags(Origin::signed(alice), ad, tags));

        assert_eq!(<Test as Config>::TagsStore::get(&ad), hashes);
    });
}
