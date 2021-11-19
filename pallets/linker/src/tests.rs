use crate::{mock::*, types::AccountType, Config, Error, LinksOf, PendingOf};
use codec::Decode;
use frame_support::{assert_noop, assert_ok};
use sp_core::{
    offchain::{testing, OffchainWorkerExt, TransactionPoolExt},
    sr25519, H160,
};

#[test]
fn should_generate_message() {
    new_test_ext().execute_with(|| {
        let did = "32ac799d35de72a2ae57a46ca975319fbbb125a9";
        let did = H160::from_slice(&decode_hex(did).unwrap());

        assert_eq!(
            Linker::generate_message(&did),
            b"Link: did:ad3:hwtGPq42GojPtyx5ngtSRSpJfjN".to_vec()
        );
    });
}

#[test]
fn should_fetch() {
    let (offchain, state) = testing::TestOffchainExt::new();
    let mut t = new_test_ext();
    t.register_extension(OffchainWorkerExt::new(offchain));

    {
        let mut state = state.write();
        state.expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: "https://example.com".into(),
            headers: vec![("User-Agent".into(), "ParamiLinker/1.0".into())],
            response: Some(b"Example Domain".to_vec()),
            sent: true,
            ..Default::default()
        });
    }

    t.execute_with(|| {
        let result = Linker::ocw_fetch(b"https://example.com".to_vec()).unwrap();

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

    let did = "32ac799d35de72a2ae57a46ca975319fbbb125a9";
    let did = H160::from_slice(&decode_hex(did).unwrap());

    t.execute_with(|| {
        Linker::ocw_submit_link(did, AccountType::Unknown, Vec::<u8>::new(), false);

        let tx = state.write().transactions.pop().unwrap();
        let tx = Extrinsic::decode(&mut &*tx).unwrap();

        assert_eq!(tx.signature, None);

        assert_eq!(
            tx.call,
            Call::Linker(crate::Call::submit_link_unsigned {
                did,
                site: AccountType::Unknown,
                profile: Vec::<u8>::new(),
                ok: false,
            })
        );
    });
}

#[test]
fn should_link_telegram() {
    const HTM: &[u8] = include_bytes!("../artifacts/telegram.html");
    const JPG: &[u8] = include_bytes!("../artifacts/did.jpg");

    let htm = HTM.to_vec();
    let jpg = JPG.to_vec();

    let alice = sr25519::Public([1; 32]);

    let did = "32ac799d35de72a2ae57a46ca975319fbbb125a9";
    let did = H160::from_slice(&decode_hex(did).unwrap());

    let profile = b"https://t.me/AmeliaParami".to_vec();

    let (offchain, state) = testing::TestOffchainExt::new();
    let (pool, _) = testing::TestTransactionPoolExt::new();

    let mut t = new_test_ext();
    t.register_extension(OffchainWorkerExt::new(offchain));
    t.register_extension(TransactionPoolExt::new(pool));

    {
        let mut state = state.write();
        state.expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: String::from_utf8(profile.clone()).unwrap(),
            headers: vec![("User-Agent".into(), "ParamiLinker/1.0".into())],
            response: Some(htm),
            sent: true,
            ..Default::default()
        });
        state.expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: "https://cdn5.telesco.pe/file/did.jpg".into(),
            headers: vec![("User-Agent".into(), "ParamiLinker/1.0".into())],
            response: Some(jpg),
            sent: true,
            ..Default::default()
        });
    }

    t.execute_with(|| {
        assert_ok!(Linker::link_sociality(
            Origin::signed(alice),
            AccountType::Telegram,
            profile.clone(),
        ));

        let maybe_link = <PendingOf<Test>>::get(&did, AccountType::Telegram);
        assert_ne!(maybe_link, None);

        let link = maybe_link.unwrap();

        assert_eq!(link.profile, profile.clone());
        assert_eq!(link.deadline, <Test as Config>::PendingLifetime::get());

        assert_ok!(Linker::ocw_link_telegram(did, profile));
    });
}

#[test]
fn should_link_eth() {
    new_test_ext().execute_with(|| {
        let alice = sr25519::Public([1; 32]);

        let did = "32ac799d35de72a2ae57a46ca975319fbbb125a9";
        let did = H160::from_slice(&decode_hex(did).unwrap());

        // PK: b63a08b47e2476870812e98896d696c6662018091f34f537e29359270c98d474
        let address = "a42e6A7529BC92011B1ffac36B451C485aa2DfaC";
        let address = decode_hex(address).unwrap();

        let signature = "9e89a2fc056b12ed8da4c95f3832e94558a624d63a5508301ef2d430860b41d123e33f5e56383097f08e7f042f5ac8a73517533462dc8fbb6c8cdde2c51a03171b";
        let signature = decode_hex(signature).unwrap();

        let mut sig = [0u8; 65];
        sig.copy_from_slice(&signature);

        assert_ok!(Linker::link_eth(
            Origin::signed(alice),
            address.clone(),
            sig,
        ));

        assert_eq!(<LinksOf<Test>>::get(&did, AccountType::Ethereum), Some(address));
    });
}
