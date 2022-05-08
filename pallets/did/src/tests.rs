use crate::{mock::*, DidOf, EnsureDid, Error, Metadata, ReferrerOf};
use frame_support::{assert_noop, assert_ok};
use parami_did_utils::derive_storage_key;
use sp_core::offchain::{
    testing::{TestOffchainExt, TestPersistentOffchainDB},
    OffchainDbExt,
};

#[test]
fn should_register() {
    new_test_ext().execute_with(|| {
        System::set_block_number(0);

        assert_ok!(Did::register(Origin::signed(BOB), None));

        System::set_block_number(1);

        let r = Did::register(Origin::signed(BOB), None);

        assert_noop!(r, Error::<Test>::Exists);

        let maybe_did = <DidOf<Test>>::get(&BOB);
        assert_ne!(maybe_did, None);

        let maybe_meta = <Metadata<Test>>::get(maybe_did.unwrap());
        assert_ne!(maybe_meta, None);
        assert_eq!(maybe_meta.unwrap().revoked, false);
    });
}

#[test]
fn should_fail_when_exist() {
    new_test_ext().execute_with(|| {
        let r = Did::register(Origin::signed(ALICE), None);
        assert_noop!(r, Error::<Test>::Exists);
    });
}

#[test]
fn should_register_with_referer() {
    new_test_ext().execute_with(|| {
        assert_ok!(Did::register(Origin::signed(BOB), Some(DID_ALICE)));

        let maybe_did = <DidOf<Test>>::get(&BOB);
        assert_ne!(maybe_did, None);

        let maybe_referrer = <ReferrerOf<Test>>::get(maybe_did.unwrap());
        assert_eq!(maybe_referrer, Some(DID_ALICE));
    });
}

#[test]
fn should_fail_when_referer_not_exist() {
    new_test_ext().execute_with(|| {
        let r = Did::register(Origin::signed(BOB), Some(DID_BOB));
        assert_noop!(r, Error::<Test>::ReferrerNotExists);
    });
}

#[test]
fn should_revoke() {
    new_test_ext().execute_with(|| {
        assert_ok!(Did::revoke(Origin::signed(ALICE)));

        assert!(!<DidOf<Test>>::contains_key(&ALICE));

        let meta = <Metadata<Test>>::get(&DID_ALICE).unwrap();

        assert_eq!(meta.revoked, true);
    });
}

#[test]
fn should_fail_when_not_exist() {
    new_test_ext().execute_with(|| {
        assert_noop!(Did::revoke(Origin::signed(BOB)), Error::<Test>::NotExists);
    });
}

#[test]
fn should_reassign() {
    new_test_ext().execute_with(|| {
        System::set_block_number(0);

        assert_ok!(Did::register(Origin::signed(BOB), None));

        System::set_block_number(1);

        let did = <DidOf<Test>>::get(&BOB).unwrap();

        assert_ok!(Did::revoke(Origin::signed(BOB)));

        System::set_block_number(2);

        assert_ok!(Did::register(Origin::signed(BOB), None));

        assert_ne!(<DidOf<Test>>::get(&BOB), Some(did));
    });
}

#[test]
fn should_transfer() {
    new_test_ext().execute_with(|| {
        System::set_block_number(2);

        assert_ok!(Did::transfer(Origin::signed(ALICE), BOB));

        assert_eq!(<DidOf<Test>>::get(&ALICE), None);

        let maybe_did = <DidOf<Test>>::get(&BOB);
        assert_ne!(maybe_did, None);

        let maybe_meta = <Metadata<Test>>::get(maybe_did.unwrap());
        assert_ne!(maybe_meta, None);

        let meta = maybe_meta.unwrap();
        assert_eq!(meta.account, BOB);
        assert_eq!(meta.created, 2);
        assert_eq!(meta.revoked, false);
    });
}

#[test]
fn should_fail_when_already_have_did() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Did::transfer(Origin::signed(ALICE), ALICE),
            Error::<Test>::Exists
        );
    });
}

#[test]
fn should_fail_when_revoked() {
    new_test_ext().execute_with(|| {
        assert_ok!(Did::revoke(Origin::signed(ALICE)));

        assert_noop!(
            Did::transfer(Origin::signed(ALICE), BOB),
            Error::<Test>::NotExists
        );
    });
}

#[test]
fn should_set_metadata() {
    const KEY: &[u8] = b"avatar";
    const VALUE: &[u8] = b"ipfs://QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG";

    let db = TestPersistentOffchainDB::new();
    let (offchain, _state) = TestOffchainExt::with_offchain_db(db);
    let mut t = new_test_ext();
    t.register_extension(OffchainDbExt::new(offchain.clone()));

    t.execute_with(|| {
        assert_ok!(Did::set_metadata(
            Origin::signed(ALICE),
            KEY.to_vec(),
            VALUE.to_vec()
        ));
    });

    t.persist_offchain_overlay();

    let db = t.offchain_db();
    let avatar = db.get(&derive_storage_key(KEY, &DID_ALICE));
    assert_eq!(avatar, Some(VALUE.to_vec()));
}

#[test]
fn should_ensure() {
    new_test_ext().execute_with(|| {
        use frame_support::traits::EnsureOrigin;

        let ensure = EnsureDid::<Test>::try_origin(Origin::signed(ALICE));
        assert!(ensure.is_ok());
        assert_eq!(ensure.unwrap(), (DID_ALICE, ALICE));

        assert!(EnsureDid::<Test>::try_origin(Origin::signed(BOB)).is_err());
    });
}
