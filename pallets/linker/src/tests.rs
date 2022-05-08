use crate::{mock::*, Config, Error, Linked, LinksOf, PendingOf, Registrar};
use codec::Decode;
use frame_support::{assert_noop, assert_ok, traits::Hooks};
use parami_ocw::USER_AGENT;
use parami_traits::types::Network;
use sp_core::offchain::{testing, OffchainWorkerExt, TransactionPoolExt};

macro_rules! assert_ok_eq {
    ($left:expr, $right:expr) => {
        assert_eq!($left.ok(), Some($right));
    };
}

macro_rules! assert_tx {
    ($tx:tt, $call:expr) => {
        let tx = Extrinsic::decode(&mut &*$tx).unwrap();

        assert_eq!(tx.signature, None);

        assert_eq!(tx.call, $call);
    };
}

const MESSAGE: &[u8] = b"Link: did:ad3:hwtGPq42GojPtyx5ngtSRSpJfjN";

#[test]
fn should_generate_message() {
    new_test_ext().execute_with(|| {
        assert_eq!(Linker::generate_message(&DID_ALICE), MESSAGE.to_vec());
    });
}

#[test]
fn should_link() {
    new_test_ext().execute_with(|| {
        let profile = b"https://t.me/AmeliaParami".to_vec();

        assert_ok!(Linker::insert_pending(
            DID_ALICE,
            Network::Telegram,
            profile.clone(),
        ));

        let maybe_pending = <PendingOf<Test>>::get(Network::Telegram, &DID_ALICE);
        assert_ne!(maybe_pending, None);

        let pending = maybe_pending.unwrap();
        assert_eq!(pending.task, profile);
        assert_eq!(pending.deadline, <Test as Config>::PendingLifetime::get());

        assert_ok!(Linker::insert_link(
            DID_ALICE,
            Network::Telegram,
            profile.clone(),
            DID_ALICE,
        ));

        assert_eq!(<PendingOf<Test>>::get(Network::Telegram, &DID_ALICE), None);

        assert!(<Linked<Test>>::get(Network::Telegram, &profile));

        assert_eq!(
            <LinksOf<Test>>::get(&DID_ALICE, Network::Telegram),
            Some(profile)
        );
    })
}

#[test]
fn should_fail_when_exists() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Linker::insert_pending(DID_ALICE, Network::Polkadot, POLKA.to_vec()),
            Error::<Test>::Exists
        );

        assert_noop!(
            Linker::insert_pending(DID_BOB, Network::Polkadot, POLKA.to_vec()),
            Error::<Test>::Exists
        );
    })
}

#[test]
fn should_ocw_submit() {
    let (offchain, _) = testing::TestOffchainExt::new();
    let (pool, state) = testing::TestTransactionPoolExt::new();

    let mut t = new_test_ext();
    t.register_extension(OffchainWorkerExt::new(offchain));
    t.register_extension(TransactionPoolExt::new(pool));

    t.execute_with(|| {
        Linker::ocw_submit_link(DID_ALICE, Network::Telegram, Vec::<u8>::new(), false);

        let tx = state.write().transactions.pop().unwrap();

        assert_tx!(
            tx,
            Call::Linker(crate::Call::submit_link {
                did: DID_ALICE,
                site: Network::Telegram,
                profile: Vec::<u8>::new(),
                validated: false,
            })
        );
    });
}

#[test]
fn should_register() {
    new_test_ext().execute_with(|| {
        assert_ok!(Linker::submit_register(Origin::signed(ALICE), CHARLIE));

        assert_ne!(Did::did_of(CHARLIE), None);
    })
}

#[test]
fn should_submit() {
    new_test_ext().execute_with(|| {
        let profile = b"https://t.me/AmeliaParami".to_vec();

        assert_ok!(Linker::submit_link(
            Origin::none(),
            DID_ALICE,
            Network::Telegram,
            profile.clone(),
            true,
        ));

        assert!(<Linked<Test>>::get(Network::Telegram, &profile));

        assert_eq!(
            <LinksOf<Test>>::get(&DID_ALICE, Network::Telegram),
            Some(profile)
        );
    })
}

#[test]
fn should_submit_when_pending() {
    new_test_ext().execute_with(|| {
        let profile = b"https://t.me/AmeliaParami".to_vec();

        assert_ok!(Linker::link_sociality(
            Origin::signed(ALICE),
            Network::Telegram,
            profile.clone(),
        ));

        assert_ne!(<PendingOf<Test>>::get(Network::Telegram, &DID_ALICE), None);

        assert_ok!(Linker::submit_link(
            Origin::none(),
            DID_ALICE,
            Network::Telegram,
            profile.clone(),
            false,
        ));

        assert_eq!(<PendingOf<Test>>::get(Network::Telegram, &DID_ALICE), None);
    })
}

#[test]
fn should_link_sociality() {
    let profile: String = "https://t.me/AmeliaParami".into();

    let (offchain, state) = testing::TestOffchainExt::new();
    let (pool, tx) = testing::TestTransactionPoolExt::new();

    {
        let mut state = state.write();
        state.expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: profile.clone(),
            headers: vec![("User-Agent".into(), USER_AGENT.into())],
            response: Some(Vec::new()),
            sent: true,
            ..Default::default()
        });
    }

    let profile = profile.as_bytes().to_vec();

    let mut t = new_test_ext();
    t.register_extension(OffchainWorkerExt::new(offchain));
    t.register_extension(TransactionPoolExt::new(pool));

    t.execute_with(|| {
        assert_ok!(Linker::link_sociality(
            Origin::signed(ALICE),
            Network::Telegram,
            profile.clone(),
        ));

        Linker::offchain_worker(0);

        Linker::offchain_worker(5);

        let tx = tx.write().transactions.pop().unwrap();

        assert_tx!(
            tx,
            Call::Linker(crate::Call::submit_link {
                did: DID_ALICE,
                site: Network::Telegram,
                profile: profile,
                validated: false,
            })
        );
    });
}

#[test]
fn should_verify_telegram() {
    const HTM: &[u8] = include_bytes!("../artifacts/telegram.html");
    const JPG: &[u8] = include_bytes!("../artifacts/did.jpg");

    let htm = HTM.to_vec();
    let jpg = JPG.to_vec();

    let profile: String = "https://t.me/AmeliaParami".into();
    let avatar: String = "https://cdn5.telesco.pe/file/did.jpg".into();

    let (offchain, state) = testing::TestOffchainExt::new();

    {
        let mut state = state.write();
        state.expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: profile.clone(),
            headers: vec![("User-Agent".into(), USER_AGENT.into())],
            response: Some(htm),
            sent: true,
            ..Default::default()
        });
        state.expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: avatar.clone(),
            headers: vec![("User-Agent".into(), USER_AGENT.into())],
            response: Some(jpg),
            sent: true,
            ..Default::default()
        });
    }

    let mut t = new_test_ext();
    t.register_extension(OffchainWorkerExt::new(offchain));

    t.execute_with(|| {
        assert_ok!(Linker::ocw_verify_telegram(DID_ALICE, profile));
    });
}

#[test]
fn should_verify_twitter() {
    const HTM: &[u8] = include_bytes!("../artifacts/twitter.html");
    const JPG: &[u8] = include_bytes!("../artifacts/did.jpg");

    let htm = HTM.to_vec();
    let jpg = JPG.to_vec();

    let profile: String = "https://twitter.com/ParamiProtocol".into();
    let avatar: String = "https://pbs.twimg.com/profile_images/1380053132760125441/did.jpg".into();

    let (offchain, state) = testing::TestOffchainExt::new();

    {
        let mut state = state.write();
        state.expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: profile.clone(),
            headers: vec![("User-Agent".into(), USER_AGENT.into())],
            response: Some(htm),
            sent: true,
            ..Default::default()
        });
        state.expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: avatar.clone(),
            headers: vec![("User-Agent".into(), USER_AGENT.into())],
            response: Some(jpg),
            sent: true,
            ..Default::default()
        });
    }

    let mut t = new_test_ext();
    t.register_extension(OffchainWorkerExt::new(offchain));

    t.execute_with(|| {
        assert_ok!(Linker::ocw_verify_twitter(DID_ALICE, profile));
    });
}

#[test]
fn should_link_crypto() {
    new_test_ext().execute_with(|| {
        let address = vec![0u8; 20];
        let signature = [0u8; 65];

        assert_ok!(Linker::link_crypto(
            Origin::signed(ALICE),
            Network::Unknown,
            address.clone(),
            signature,
        ));

        assert!(<Linked<Test>>::get(Network::Unknown, &address));

        assert_eq!(
            <LinksOf<Test>>::get(&DID_ALICE, Network::Unknown),
            Some(address)
        );
    });
}

#[test]
fn should_recover_btc() {
    new_test_ext().execute_with(|| {
        // PK: 5KYZdUEo39z3FPrtuX2QbbwGnNP5zTd7yyr2SC1j299sBCnWjss
        let address = b"1F3sAm6ZtwLAUnj7d38pGFxtP3RVEvtsbV".to_vec();

        // 1cb5b2e3b269cd9b78c2fec806cf667f35ccbf8934bdcb8b125ed70d64e48c8b9c81853b2c0e905184e57add15874b0cffcc5fe0cce33f3a31f508bb569b19b4a2
        let signature = [
            0x1c, 0xb5, 0xb2, 0xe3, 0xb2, 0x69, 0xcd, 0x9b, 0x78, 0xc2, 0xfe, 0xc8, 0x06, 0xcf,
            0x66, 0x7f, 0x35, 0xcc, 0xbf, 0x89, 0x34, 0xbd, 0xcb, 0x8b, 0x12, 0x5e, 0xd7, 0x0d,
            0x64, 0xe4, 0x8c, 0x8b, 0x9c, 0x81, 0x85, 0x3b, 0x2c, 0x0e, 0x90, 0x51, 0x84, 0xe5,
            0x7a, 0xdd, 0x15, 0x87, 0x4b, 0x0c, 0xff, 0xcc, 0x5f, 0xe0, 0xcc, 0xe3, 0x3f, 0x3a,
            0x31, 0xf5, 0x08, 0xbb, 0x56, 0x9b, 0x19, 0xb4, 0xa2,
        ];

        let mut sig = [0u8; 65];
        sig.copy_from_slice(&signature);

        assert_ok_eq!(
            Linker::recover_address(Network::Bitcoin, address.clone(), sig, MESSAGE.to_vec()),
            address
        );
    });
}

#[test]
fn should_recover_btc_segwit() {
    new_test_ext().execute_with(|| {
        // PK: p2wpkh:Kzbv1fJbGs24LpWjdPNgvtBEdkVF9w1urLiqbfrvTt2YGqQS6SSC
        let address = b"bc1qug9quswyl8pxalrfudfr9p34mmjvj2f6tx6f0k".to_vec();

        // 203b166d7adfe349fdae5b36e1262a979c70b1e041228df149d8b7d0d5278b6aad4b027693ae2eda794f13da93b928505c4b1d23da572ebeba9696edc4af57cf58
        let signature = [
            0x20, 0x3b, 0x16, 0x6d, 0x7a, 0xdf, 0xe3, 0x49, 0xfd, 0xae, 0x5b, 0x36, 0xe1, 0x26,
            0x2a, 0x97, 0x9c, 0x70, 0xb1, 0xe0, 0x41, 0x22, 0x8d, 0xf1, 0x49, 0xd8, 0xb7, 0xd0,
            0xd5, 0x27, 0x8b, 0x6a, 0xad, 0x4b, 0x02, 0x76, 0x93, 0xae, 0x2e, 0xda, 0x79, 0x4f,
            0x13, 0xda, 0x93, 0xb9, 0x28, 0x50, 0x5c, 0x4b, 0x1d, 0x23, 0xda, 0x57, 0x2e, 0xbe,
            0xba, 0x96, 0x96, 0xed, 0xc4, 0xaf, 0x57, 0xcf, 0x58,
        ];

        let mut sig = [0u8; 65];
        sig.copy_from_slice(&signature);

        assert_ok_eq!(
            Linker::recover_address(Network::Bitcoin, address.clone(), sig, MESSAGE.to_vec()),
            address
        );
    });
}

#[test]
fn should_recover_dot() {
    new_test_ext().execute_with(|| {
        // URI: //Alice
        let address = b"5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY".to_vec();

        // 00b019009d196eb10f3d7f46309b591d21950fa617ced4f0b01b908b472bf0566610691636fde1088383b2b8134e5aee1bf48b2f4b46056709f8f0d81f79ebe58b
        let signature = [
            0x00, 0xb0, 0x19, 0x00, 0x9d, 0x19, 0x6e, 0xb1, 0x0f, 0x3d, 0x7f, 0x46, 0x30, 0x9b,
            0x59, 0x1d, 0x21, 0x95, 0x0f, 0xa6, 0x17, 0xce, 0xd4, 0xf0, 0xb0, 0x1b, 0x90, 0x8b,
            0x47, 0x2b, 0xf0, 0x56, 0x66, 0x10, 0x69, 0x16, 0x36, 0xfd, 0xe1, 0x08, 0x83, 0x83,
            0xb2, 0xb8, 0x13, 0x4e, 0x5a, 0xee, 0x1b, 0xf4, 0x8b, 0x2f, 0x4b, 0x46, 0x05, 0x67,
            0x09, 0xf8, 0xf0, 0xd8, 0x1f, 0x79, 0xeb, 0xe5, 0x8b,
        ];

        let mut sig = [0u8; 65];
        sig.copy_from_slice(&signature);

        assert_ok_eq!(
            Linker::recover_address(Network::Polkadot, address.clone(), sig, MESSAGE.to_vec()),
            address
        );
    });
}

#[test]
fn should_recover_eth() {
    new_test_ext().execute_with(|| {
        // PK: be6383dad004f233317e46ddb46ad31b16064d14447a95cc1d8c8d4bc61c3728
        // eb014f8c8b418db6b45774c326a0e64c78914dc0
        let address = vec![
            0xeb, 0x01, 0x4f, 0x8c, 0x8b, 0x41, 0x8d, 0xb6, 0xb4, 0x57, 0x74, 0xc3, 0x26, 0xa0,
            0xe6, 0x4c, 0x78, 0x91, 0x4d, 0xc0,
        ];

        // 193883369b84888e1dded1e83a8fd92cdde41b9a9c977be5ddbbb259783a69d060d120704760eb82671889c664be25d6cf6f25b9efe781fb637bbd6097da0e621c
        let signature = [
            0x19, 0x38, 0x83, 0x36, 0x9b, 0x84, 0x88, 0x8e, 0x1d, 0xde, 0xd1, 0xe8, 0x3a, 0x8f,
            0xd9, 0x2c, 0xdd, 0xe4, 0x1b, 0x9a, 0x9c, 0x97, 0x7b, 0xe5, 0xdd, 0xbb, 0xb2, 0x59,
            0x78, 0x3a, 0x69, 0xd0, 0x60, 0xd1, 0x20, 0x70, 0x47, 0x60, 0xeb, 0x82, 0x67, 0x18,
            0x89, 0xc6, 0x64, 0xbe, 0x25, 0xd6, 0xcf, 0x6f, 0x25, 0xb9, 0xef, 0xe7, 0x81, 0xfb,
            0x63, 0x7b, 0xbd, 0x60, 0x97, 0xda, 0x0e, 0x62, 0x1c,
        ];

        let mut sig = [0u8; 65];
        sig.copy_from_slice(&signature);

        assert_ok_eq!(
            Linker::recover_address(Network::Ethereum, address.clone(), sig, MESSAGE.to_vec()),
            address
        );
    });
}

#[test]
fn should_recover_sol() {
    new_test_ext().execute_with(|| {
        // PK: 4c696e6b3a206469643a6164333a6877744750713432476f6a50747978356e6774535253704a666a4e
        let address = b"2q7pyhPwAwZ3QMfZrnAbDhnh9mDUqycszcpf86VgQxhF".to_vec();

        // 00f94c93e56f6a07540ac21f95449eb308495048904ccfbdc9fe9a49b890da942ec3b1cd8cad30eef7e28437afa3463d389d75e0451d715997302cc2aaaa65630e
        let signature = [
            0x00, 0xf9, 0x4c, 0x93, 0xe5, 0x6f, 0x6a, 0x07, 0x54, 0x0a, 0xc2, 0x1f, 0x95, 0x44,
            0x9e, 0xb3, 0x08, 0x49, 0x50, 0x48, 0x90, 0x4c, 0xcf, 0xbd, 0xc9, 0xfe, 0x9a, 0x49,
            0xb8, 0x90, 0xda, 0x94, 0x2e, 0xc3, 0xb1, 0xcd, 0x8c, 0xad, 0x30, 0xee, 0xf7, 0xe2,
            0x84, 0x37, 0xaf, 0xa3, 0x46, 0x3d, 0x38, 0x9d, 0x75, 0xe0, 0x45, 0x1d, 0x71, 0x59,
            0x97, 0x30, 0x2c, 0xc2, 0xaa, 0xaa, 0x65, 0x63, 0x0e,
        ];

        let mut sig = [0u8; 65];
        sig.copy_from_slice(&signature);

        assert_ok_eq!(
            Linker::recover_address(Network::Solana, address.clone(), sig, MESSAGE.to_vec()),
            address
        );
    });
}

#[test]
fn should_recover_trx() {
    new_test_ext().execute_with(|| {
        // PK: da146374a75310b9666e834ee4ad0866d6f4035967bfc76217c5a495fff9f0d0
        let address = b"TPL66VK2gCXNCD7EJg9pgJRfqcRazjhUZY".to_vec();

        // 4d423812706f526a546adc810968a87b361664097bfb9bf8c768089493eecb2d1cfdc4fcbc9705da2c3f0c81b6c52c3a3c334db6656e7671194647e0628f7deb1b
        let signature = [
            0x4d, 0x42, 0x38, 0x12, 0x70, 0x6f, 0x52, 0x6a, 0x54, 0x6a, 0xdc, 0x81, 0x09, 0x68,
            0xa8, 0x7b, 0x36, 0x16, 0x64, 0x09, 0x7b, 0xfb, 0x9b, 0xf8, 0xc7, 0x68, 0x08, 0x94,
            0x93, 0xee, 0xcb, 0x2d, 0x1c, 0xfd, 0xc4, 0xfc, 0xbc, 0x97, 0x05, 0xda, 0x2c, 0x3f,
            0x0c, 0x81, 0xb6, 0xc5, 0x2c, 0x3a, 0x3c, 0x33, 0x4d, 0xb6, 0x65, 0x6e, 0x76, 0x71,
            0x19, 0x46, 0x47, 0xe0, 0x62, 0x8f, 0x7d, 0xeb, 0x1b,
        ];

        let mut sig = [0u8; 65];
        sig.copy_from_slice(&signature);

        assert_ok_eq!(
            Linker::recover_address(Network::Tron, address.clone(), sig, MESSAGE.to_vec()),
            address
        );
    });
}

#[test]
fn should_deposit_and_trust() {
    new_test_ext().execute_with(|| {
        assert_ok!(Linker::deposit(Origin::signed(BOB), 10));

        assert_eq!(Balances::free_balance(&BOB), 90);
        assert_eq!(Balances::reserved_balance(BOB), 10);

        assert_ok!(Linker::force_trust(Origin::root(), DID_BOB));

        assert_eq!(<Registrar<Test>>::get(&DID_BOB), Some(true));

        assert_ok!(Linker::force_block(Origin::root(), DID_BOB));

        assert_eq!(<Registrar<Test>>::get(&DID_BOB), Some(false));

        assert_eq!(Balances::free_balance(&BOB), 90);
        assert_eq!(Balances::reserved_balance(BOB), 0);
    });
}

#[test]
fn should_link_via_registrar() {
    new_test_ext().execute_with(|| {
        let profile = b"https://t.me/AmeliaParami".to_vec();

        assert_noop!(
            Linker::submit_link(
                Origin::signed(BOB),
                DID_ALICE,
                Network::Telegram,
                profile.clone(),
                true
            ),
            Error::<Test>::Blocked
        );

        assert_ok!(Linker::deposit(Origin::signed(ALICE), 10));

        assert_ok!(Linker::force_trust(Origin::root(), DID_ALICE));

        assert_ok!(Linker::submit_link(
            Origin::signed(ALICE),
            DID_ALICE,
            Network::Telegram,
            profile.clone(),
            true
        ));

        assert_ok!(Linker::submit_score(
            Origin::signed(ALICE),
            DID_ALICE,
            b"telegram".to_vec(),
            50
        ));
    });
}

#[test]
fn should_force_unlink() {
    new_test_ext().execute_with(|| {
        assert_ok!(Linker::force_unlink(
            Origin::root(),
            DID_ALICE,
            Network::Polkadot,
        ));
    })
}
