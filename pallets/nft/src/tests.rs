use crate::{mock::*, Deposit, Deposits, Error, Metadata, Porting};
use frame_support::{assert_noop, assert_ok};
use parami_primitives::constants::DOLLARS;
use parami_traits::{types::Network, Swaps};
use sp_core::offchain::{testing, OffchainWorkerExt};
use sp_core::U256;
use sp_runtime::offchain::testing::OffchainState;
use sp_std::prelude::*;

#[test]
fn should_import() {
    new_test_ext().execute_with(|| {
        let namespace = NAMESPACE.to_vec();
        let token = vec![0x02];
        let did = DID_BOB;

        let _result = Linker::insert_link(did, Network::Ethereum, "something".into(), did);

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
        let did = DID_BOB;

        let _result = Linker::insert_link(did, Network::Ethereum, "something".into(), did);

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
fn should_fail_when_did_not_linked_network() {
    new_test_ext().execute_with(|| {
        let namespace = NAMESPACE.to_vec();
        let token = vec![0x02];

        assert_noop!(
            Nft::port(
                Origin::signed(BOB),
                Network::Ethereum,
                namespace.clone(),
                token.clone(),
            ),
            Error::<Test>::NetworkNotLinked
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

        let free_balance_of_backer = Balances::free_balance(BOB);

        let r = Nft::back(Origin::signed(BOB), nft, free_balance_of_backer + 30000);

        assert_noop!(r, pallet_balances::Error::<Test>::InsufficientBalance);
    });
}

#[test]
fn should_mint() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::back(Origin::signed(BOB), nft, 1000 * DOLLARS));

        assert_ok!(Nft::mint(
            Origin::signed(ALICE),
            nft,
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        let deposit = <Deposit<Test>>::get(&nft);
        assert_eq!(deposit, Some(1000 * DOLLARS));

        let deposit_kol = <Deposits<Test>>::get(nft, &DID_ALICE);
        assert_eq!(deposit_kol, deposit);
    });
}

#[test]
fn pay_1000_ad3_should_elevate_token_price_by_1x() {
    new_test_ext().execute_with(|| {
        let required_ad3_amount = elevate_token_price_to_target(2 * DOLLARS);
        log::info!("required_ad3_amount is {}", required_ad3_amount);
        assert!(required_ad3_amount < 1000 * DOLLARS);
    });
}

//return required ad3 amount
fn elevate_token_price_to_target(target_ad3_amount_per_1000_token: u128) -> u128 {
    let nft = Nft::preferred(DID_ALICE).unwrap();

    assert_ok!(Nft::back(Origin::signed(BOB), nft, 1000 * DOLLARS));

    assert_ok!(Nft::mint(
        Origin::signed(ALICE),
        nft,
        b"Test Token".to_vec(),
        b"XTT".to_vec()
    ));

    let ad3_balance_of_bob_before_buying_token = Balances::free_balance(BOB);

    let mut ad3_amount_per_1000_token = Swap::token_out_dry(nft, 1000 * DOLLARS).unwrap();
    while ad3_amount_per_1000_token < target_ad3_amount_per_1000_token {
        Swap::buy_tokens(
            Origin::signed(BOB),
            nft,
            100_000 * DOLLARS,
            1000 * DOLLARS,
            100,
        )
        .unwrap();
        ad3_amount_per_1000_token = Swap::token_out_dry(nft, 1000 * DOLLARS).unwrap();
    }

    let ad3_balance_of_bob_after_buying_token = Balances::free_balance(BOB);

    ad3_balance_of_bob_before_buying_token - ad3_balance_of_bob_after_buying_token
}

#[test]
fn should_fail_when_minted() {
    new_test_ext().execute_with(|| {
        let nft = Nft::preferred(DID_ALICE).unwrap();

        assert_ok!(Nft::back(Origin::signed(BOB), nft, 2000 * DOLLARS));

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

        assert_ok!(Nft::back(Origin::signed(BOB), nft, 2000 * DOLLARS));
        assert_ok!(Nft::back(Origin::signed(CHARLIE), nft, 1000 * DOLLARS));

        assert_ok!(Nft::mint(
            Origin::signed(ALICE),
            nft,
            b"Test Token".to_vec(),
            b"XTT".to_vec()
        ));

        assert_ok!(Nft::claim(Origin::signed(BOB), nft));
        assert_ok!(Nft::claim(Origin::signed(CHARLIE), nft));

        assert_eq!(Assets::balance(nft, &BOB), 666666666666666666666666);
        assert_eq!(Assets::balance(nft, &CHARLIE), 333333333333333333333333);

        assert_eq!(<Deposits<Test>>::get(nft, &DID_BOB), None);
        assert_eq!(<Deposits<Test>>::get(nft, &DID_CHARLIE), None);

        assert_noop!(
            Nft::claim(Origin::signed(BOB), nft),
            Error::<Test>::NotExists
        );

        System::set_block_number(5);

        assert_ok!(Nft::claim(Origin::signed(ALICE), nft));

        assert_eq!(Assets::balance(nft, &ALICE), 1_000_000 * DOLLARS);
        assert_eq!(<Deposits<Test>>::get(nft, &DID_ALICE), None);
    });
}
fn mock_validate_request(ether_endpoint: &str, body: String) -> testing::PendingRequest {
    let res = r#"{"jsonrpc":"2.0","id":1,"result":"0x000000000000000000000000dbd04424318d1e06b34259add64bf10a8eb45a87"}"#;
    testing::PendingRequest {
        method: "POST".into(),
        uri: ether_endpoint.into(),
        sent: true,
        headers: vec![(
            "User-Agent".into(),
            "GoogleBot (compatible; ParamiWorker/1.0; +http://parami.io/worker/)".into(),
        )],
        body: body.into(),
        response: Some(res.into()),
        ..Default::default()
    }
}

#[test]
fn should_success_when_validate_etherum_token_owner() {
    let (offchain, state) = testing::TestOffchainExt::new();
    let mut t = new_test_ext();
    t.register_extension(OffchainWorkerExt::new(offchain));

    let ether_endpoint = "http://etherum.endpoint/example";
    let links: &[Vec<u8>] = &[vec![
        219, 208, 68, 36, 49, 141, 30, 6, 179, 66, 89, 173, 214, 75, 241, 10, 142, 180, 90, 135,
    ]];
    let contract_address = b"contractaddress";
    let token = 546u64;

    let body = Nft::construct_request_body(contract_address, &token.to_be_bytes());

    {
        let mut state = state.write();
        state.expect_request(mock_validate_request(ether_endpoint.into(), body));
    }

    t.execute_with(|| {
        let result = Nft::ocw_validate_etherum_token_owner(
            links,
            ether_endpoint,
            b"contractaddress",
            &token.to_be_bytes(),
        );

        assert_ok!(result);
    });
}
