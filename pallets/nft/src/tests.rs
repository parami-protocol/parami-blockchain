use crate::{mock::*, Account, Deposit, Deposits, Error, Metadata, Porting};
use frame_support::{assert_noop, assert_ok};
use parami_traits::types::Network;

#[test]
fn should_import() {
    new_test_ext().execute_with(|| {
        let namespace = NAMESPACE.to_vec();
        let token = vec![0x02];

        assert_ok!(Nft::port(
            Origin::signed(BOB),
            Network::Ethereum,
            namespace.clone(),
            token.clone()
        ));

        let maybe_porting = <Porting<Test>>::get((Network::Ethereum, &namespace, &token));
        assert_ne!(maybe_porting, None);

        let porting = maybe_porting.unwrap();
        assert_eq!(porting.task.owner, DID_BOB);
        assert_eq!(porting.task.network, Network::Ethereum);
        assert_eq!(porting.task.namespace, namespace);
        assert_eq!(porting.task.token, token);
        assert_eq!(porting.deadline, 5);
        assert_eq!(porting.created, 0);
    });
}

#[test]
fn should_fail_when_imported() {
    new_test_ext().execute_with(|| {
        let namespace = NAMESPACE.to_vec();
        let token = vec![0x01];

        assert_noop!(
            Nft::port(
                Origin::signed(BOB),
                Network::Ethereum,
                namespace,
                token.clone()
            ),
            Error::<Test>::Exists
        );
    });
}

#[test]
fn should_fail_when_importing() {
    new_test_ext().execute_with(|| {
        let namespace = NAMESPACE.to_vec();
        let token = vec![0x02];

        assert_ok!(Nft::port(
            Origin::signed(BOB),
            Network::Ethereum,
            namespace.clone(),
            token.clone(),
        ));

        assert_noop!(
            Nft::port(
                Origin::signed(ALICE),
                Network::Ethereum,
                namespace,
                token.clone()
            ),
            Error::<Test>::Exists
        );
    });
}

#[test]
fn should_create() {
    new_test_ext().execute_with(|| {
        assert_ok!(Nft::kick(Origin::signed(BOB)));

        let maybe_nft = Nft::preferred(DID_BOB);
        assert_ne!(maybe_nft, None);

        let nft = maybe_nft.unwrap();

        let maybe_meta = <Metadata<Test>>::get(nft);
        assert_ne!(maybe_meta, None);

        let meta = maybe_meta.unwrap();
        assert_eq!(meta.owner, DID_BOB);
        assert_eq!(meta.class_id, NEXT_INSTANCE_ID);
        assert_eq!(meta.minted, false);
        assert_eq!(meta.token_asset_id, NEXT_INSTANCE_ID);

        assert_eq!(<Account<Test>>::get(&DID_BOB, nft), Some(true));
    });
}

#[test]
fn should_back() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::back(Origin::signed(BOB), nft, 50));

        let deposit = <Deposit<Test>>::get(nft);
        assert_eq!(deposit, Some(50));

        let deposit = <Deposits<Test>>::get(nft, &DID_BOB);
        assert_eq!(deposit, Some(50));

        let meta = <Metadata<Test>>::get(nft).unwrap();
        assert_eq!(Balances::free_balance(&meta.pot), 50);

        assert_ok!(Nft::back(Origin::signed(CHARLIE), nft, 30));

        let deposit = <Deposits<Test>>::get(nft, &DID_CHARLIE);
        assert_eq!(deposit, Some(30));

        let deposit = <Deposit<Test>>::get(nft);
        assert_eq!(deposit, Some(50 + 30));
        assert_eq!(Balances::free_balance(&meta.pot), 50 + 30);
    });
}

#[test]
fn should_fail_when_self() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Nft::back(Origin::signed(ALICE), 0, 50),
            Error::<Test>::YourSelf
        );
    });
}

#[test]
fn should_fail_when_insufficient_balance() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        let r = Nft::back(Origin::signed(BOB), nft, 3_000_100u128);

        assert_noop!(r, pallet_balances::Error::<Test>::InsufficientBalance);
    });
}

#[test]
fn should_mint() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::back(Origin::signed(BOB), nft, 2_000_100u128));

        assert_ok!(Nft::mint(
            Origin::signed(ALICE),
            nft,
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        let deposit = <Deposit<Test>>::get(&nft);
        assert_eq!(deposit, Some(2_000_100u128));

        let deposit_kol = <Deposits<Test>>::get(nft, &DID_ALICE);
        assert_eq!(deposit_kol, deposit);
    });
}

#[test]
fn should_fail_when_minted() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::back(Origin::signed(BOB), nft, 2_000_100u128));

        assert_ok!(Nft::mint(
            Origin::signed(ALICE),
            nft,
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        assert_noop!(
            Nft::mint(
                Origin::signed(ALICE),
                nft,
                b"Test Token".to_vec(),
                b"XTT".to_vec()
            ),
            Error::<Test>::Minted
        );

        assert_noop!(
            Nft::back(Origin::signed(BOB), nft, 50),
            Error::<Test>::Minted
        );
    });
}

#[test]
fn should_fail_when_insufficient() {
    new_test_ext().execute_with(|| {
        let r = Nft::mint(
            Origin::signed(ALICE),
            0,
            b"Test Token".to_vec(),
            b"XTT".to_vec(),
        );

        assert_noop!(r, Error::<Test>::InsufficientBalance);
    });
}

#[test]
fn should_claim() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::back(Origin::signed(BOB), nft, 2_000_000u128));
        assert_ok!(Nft::back(Origin::signed(CHARLIE), nft, 1_000_000u128));

        assert_ok!(Nft::mint(
            Origin::signed(ALICE),
            nft,
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        assert_ok!(Nft::claim(Origin::signed(BOB), nft));
        assert_ok!(Nft::claim(Origin::signed(CHARLIE), nft));

        assert_eq!(Assets::balance(nft, &BOB), 666_666);
        assert_eq!(Assets::balance(nft, &CHARLIE), 333_333);

        assert_eq!(<Deposits<Test>>::get(nft, &DID_BOB), None);
        assert_eq!(<Deposits<Test>>::get(nft, &DID_CHARLIE), None);

        assert_noop!(
            Nft::claim(Origin::signed(BOB), nft),
            Error::<Test>::NotExists
        );

        System::set_block_number(5);

        assert_ok!(Nft::claim(Origin::signed(ALICE), nft));

        assert_eq!(Assets::balance(nft, &ALICE), 1_000_000);
        assert_eq!(<Deposits<Test>>::get(nft, &DID_ALICE), None);
    });
}
