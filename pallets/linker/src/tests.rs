use crate::{
    mock::*, ocw::USER_AGENT, types::AccountType, Config, Error, Linked, LinksOf, PendingOf,
    Registrar,
};
use codec::Decode;
use frame_support::{assert_noop, assert_ok, traits::Hooks};
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
fn should_fetch() {
    let url: String = "https://example.com".into();

    let (offchain, state) = testing::TestOffchainExt::new();
    let mut t = new_test_ext();
    t.register_extension(OffchainWorkerExt::new(offchain));

    {
        let mut state = state.write();
        state.expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: url.clone(),
            headers: vec![("User-Agent".into(), USER_AGENT.into())],
            response: Some(b"Example Domain".to_vec()),
            sent: true,
            ..Default::default()
        });
    }

    t.execute_with(|| {
        let result = Linker::ocw_fetch(url).unwrap();

        assert_eq!(result, b"Example Domain".to_vec());
    });
}

#[test]
fn should_submit() {
    let (offchain, _) = testing::TestOffchainExt::new();
    let (pool, state) = testing::TestTransactionPoolExt::new();

    let mut t = new_test_ext();
    t.register_extension(OffchainWorkerExt::new(offchain));
    t.register_extension(TransactionPoolExt::new(pool));

    t.execute_with(|| {
        Linker::ocw_submit_link(DID_ALICE, AccountType::Unknown, Vec::<u8>::new(), false);

        let tx = state.write().transactions.pop().unwrap();

        assert_tx!(
            tx,
            Call::Linker(crate::Call::submit_link {
                did: DID_ALICE,
                site: AccountType::Unknown,
                profile: Vec::<u8>::new(),
                ok: false,
            })
        );

        // TODO: test that the transaction is actually submitted
    });
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
            AccountType::Telegram,
            profile.clone(),
        ));

        let maybe_pending = <PendingOf<Test>>::get(&AccountType::Telegram, &DID_ALICE);
        assert_ne!(maybe_pending, None);

        let pending = maybe_pending.unwrap();
        assert_eq!(pending.profile, profile);
        assert_eq!(pending.deadline, <Test as Config>::PendingLifetime::get());

        Linker::offchain_worker(0);

        Linker::offchain_worker(5);

        let tx = tx.write().transactions.pop().unwrap();

        assert_tx!(
            tx,
            Call::Linker(crate::Call::submit_link {
                did: DID_ALICE,
                site: AccountType::Telegram,
                profile: Vec::new(),
                ok: false,
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
            AccountType::Unknown,
            address.clone(),
            signature,
        ));

        assert!(<Linked<Test>>::get(&AccountType::Unknown, &address));

        assert_eq!(
            <LinksOf<Test>>::get(&DID_ALICE, &AccountType::Unknown),
            Some(address)
        );
    });
}

#[test]
fn should_recover_btc() {
    new_test_ext().execute_with(|| {
        // PK: 5KYZdUEo39z3FPrtuX2QbbwGnNP5zTd7yyr2SC1j299sBCnWjss
        let address = b"1F3sAm6ZtwLAUnj7d38pGFxtP3RVEvtsbV".to_vec();

        let signature = "1cb5b2e3b269cd9b78c2fec806cf667f35ccbf8934bdcb8b125ed70d64e48c8b9c81853b2c0e905184e57add15874b0cffcc5fe0cce33f3a31f508bb569b19b4a2";
        let signature = decode_hex(signature).unwrap();

        let mut sig = [0u8; 65];
        sig.copy_from_slice(&signature);

        assert_ok_eq!(Linker::recover_address(
            AccountType::Bitcoin,
            address.clone(),
            sig,
            MESSAGE.to_vec()
        ), address);
    });
}

#[test]
fn should_recover_btc_segwit() {
    new_test_ext().execute_with(|| {
        // PK: p2wpkh:Kzbv1fJbGs24LpWjdPNgvtBEdkVF9w1urLiqbfrvTt2YGqQS6SSC
        let address = b"bc1qug9quswyl8pxalrfudfr9p34mmjvj2f6tx6f0k".to_vec();

        let signature = "203b166d7adfe349fdae5b36e1262a979c70b1e041228df149d8b7d0d5278b6aad4b027693ae2eda794f13da93b928505c4b1d23da572ebeba9696edc4af57cf58";
        let signature = decode_hex(signature).unwrap();

        let mut sig = [0u8; 65];
        sig.copy_from_slice(&signature);

        assert_ok_eq!(Linker::recover_address(
            AccountType::Bitcoin,
            address.clone(),
            sig,
            MESSAGE.to_vec()
        ), address);
    });
}

#[test]
fn should_recover_dot() {
    new_test_ext().execute_with(|| {
        // URI: //Alice
        let address = b"5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY".to_vec();

        let signature = "00b019009d196eb10f3d7f46309b591d21950fa617ced4f0b01b908b472bf0566610691636fde1088383b2b8134e5aee1bf48b2f4b46056709f8f0d81f79ebe58b";
        let signature = decode_hex(signature).unwrap();

        let mut sig = [0u8; 65];
        sig.copy_from_slice(&signature);

        assert_ok_eq!(Linker::recover_address(
            AccountType::Polkadot,
            address.clone(),
            sig,
            MESSAGE.to_vec()
        ), address);
    });
}

#[test]
fn should_recover_eth() {
    new_test_ext().execute_with(|| {
        // PK: b63a08b47e2476870812e98896d696c6662018091f34f537e29359270c98d474
        let address = "a42e6A7529BC92011B1ffac36B451C485aa2DfaC";
        let address = decode_hex(address).unwrap();

        let signature = "9e89a2fc056b12ed8da4c95f3832e94558a624d63a5508301ef2d430860b41d123e33f5e56383097f08e7f042f5ac8a73517533462dc8fbb6c8cdde2c51a03171b";
        let signature = decode_hex(signature).unwrap();

        let mut sig = [0u8; 65];
        sig.copy_from_slice(&signature);

        assert_ok_eq!(Linker::recover_address(
            AccountType::Ethereum,
            address.clone(),
            sig,
            MESSAGE.to_vec()
        ), address);
    });
}

#[test]
fn should_recover_sol() {
    new_test_ext().execute_with(|| {
        // PK: 4c696e6b3a206469643a6164333a6877744750713432476f6a50747978356e6774535253704a666a4e
        let address = b"2q7pyhPwAwZ3QMfZrnAbDhnh9mDUqycszcpf86VgQxhF".to_vec();

        let signature = "00f94c93e56f6a07540ac21f95449eb308495048904ccfbdc9fe9a49b890da942ec3b1cd8cad30eef7e28437afa3463d389d75e0451d715997302cc2aaaa65630e";
        let signature = decode_hex(signature).unwrap();

        let mut sig = [0u8; 65];
        sig.copy_from_slice(&signature);

        assert_ok_eq!(Linker::recover_address(
            AccountType::Solana,
            address.clone(),
            sig,
            MESSAGE.to_vec()
        ), address);
    });
}

#[test]
fn should_recover_trx() {
    new_test_ext().execute_with(|| {
        // PK: da146374a75310b9666e834ee4ad0866d6f4035967bfc76217c5a495fff9f0d0
        let address = b"TPL66VK2gCXNCD7EJg9pgJRfqcRazjhUZY".to_vec();

        let signature = "4d423812706f526a546adc810968a87b361664097bfb9bf8c768089493eecb2d1cfdc4fcbc9705da2c3f0c81b6c52c3a3c334db6656e7671194647e0628f7deb1b";
        let signature = decode_hex(signature).unwrap();

        let mut sig = [0u8; 65];
        sig.copy_from_slice(&signature);

        assert_ok_eq!(Linker::recover_address(
            AccountType::Tron,
            address.clone(),
            sig,
            MESSAGE.to_vec()
        ), address);
    });
}

#[test]
fn should_deposit() {
    new_test_ext().execute_with(|| {
        assert_ok!(Linker::deposit(Origin::signed(ALICE), 10));

        assert_eq!(Balances::free_balance(&ALICE), 90);
        assert_eq!(Balances::reserved_balance(ALICE), 10);
    });
}

#[test]
fn should_trust() {
    new_test_ext().execute_with(|| {
        assert_ok!(Linker::deposit(Origin::signed(ALICE), 10));

        assert_ok!(Linker::force_trust(Origin::root(), DID_ALICE));

        assert_eq!(<Registrar<Test>>::get(&DID_ALICE), Some(true));
    });
}

#[test]
fn should_block() {
    new_test_ext().execute_with(|| {
        assert_ok!(Linker::deposit(Origin::signed(ALICE), 10));

        assert_ok!(Linker::force_trust(Origin::root(), DID_ALICE));

        assert_ok!(Linker::force_block(Origin::root(), DID_ALICE));

        assert_eq!(<Registrar<Test>>::get(&DID_ALICE), Some(false));

        assert_eq!(Balances::free_balance(&ALICE), 90);
        assert_eq!(Balances::reserved_balance(ALICE), 0);

        assert_noop!(
            Linker::deposit(Origin::signed(ALICE), 10),
            Error::<Test>::Blocked
        );
    });
}

#[test]
fn should_link_via_registrar() {
    new_test_ext().execute_with(|| {
        let profile = b"https://mastodon.social/@ParamiProtocol".to_vec();

        assert_noop!(
            Linker::submit_link(
                Origin::signed(ALICE),
                DID_ALICE,
                AccountType::Mastodon,
                profile.clone(),
                true
            ),
            Error::<Test>::Blocked
        );

        assert_ok!(Linker::deposit(Origin::signed(ALICE), 10));

        assert_ok!(Linker::force_trust(Origin::root(), DID_ALICE));

        assert_eq!(<Registrar<Test>>::get(&DID_ALICE), Some(true));

        assert_ok!(Linker::link_sociality(
            Origin::signed(ALICE),
            AccountType::Mastodon,
            profile.clone(),
        ));

        assert_ok!(Linker::submit_link(
            Origin::signed(ALICE),
            DID_ALICE,
            AccountType::Mastodon,
            profile.clone(),
            true
        ));

        assert_ok!(Linker::submit_score(
            Origin::signed(ALICE),
            DID_ALICE,
            b"mastodon".to_vec(),
            50
        ));

        assert_noop!(
            Linker::submit_link(
                Origin::signed(ALICE),
                DID_ALICE,
                AccountType::Mastodon,
                profile.clone(),
                true
            ),
            Error::<Test>::Exists
        );

        assert_noop!(
            Linker::submit_link(
                Origin::signed(ALICE),
                DID_BOB,
                AccountType::Mastodon,
                profile.clone(),
                true
            ),
            Error::<Test>::Exists
        );
    });
}

#[test]
fn should_reject_duplicate() {
    new_test_ext().execute_with(|| {
        let address = vec![0u8; 20];
        let signature = [0u8; 65];

        assert_ok!(Linker::link_crypto(
            Origin::signed(ALICE),
            AccountType::Unknown,
            address.clone(),
            signature,
        ));

        assert_noop!(
            Linker::link_crypto(
                Origin::signed(ALICE),
                AccountType::Unknown,
                address.clone(),
                signature,
            ),
            Error::<Test>::Exists
        );

        assert_noop!(
            Linker::link_crypto(
                Origin::signed(BOB),
                AccountType::Unknown,
                address.clone(),
                signature,
            ),
            Error::<Test>::Exists
        );
    });
}
