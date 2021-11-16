use crate::{mock::*, types::AccountType, Error, LinksOf};
use frame_support::{assert_noop, assert_ok};
use sp_core::{sr25519, H160};

#[test] 
fn should_generate_message() {
    new_test_ext().execute_with(|| {
        let did = "32ac799d35de72a2ae57a46ca975319fbbb125a9";
        let did = H160::from_slice(&decode_hex(did).unwrap());

        assert_eq!(Linker::generate_message(&did), b"Link: did:ad3:hwtGPq42GojPtyx5ngtSRSpJfjN".to_vec());
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
    
        assert_eq!(LinksOf::<Test>::get(&did, AccountType::Ethereum), Some(address));    
    });
}
