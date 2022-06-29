use crate::ocw::types;
use ethabi::{ParamType, Token};
use sp_runtime_interface::runtime_interface;
use sp_std::prelude::*;

#[runtime_interface]
pub trait EthAbi {
    fn encode_input(name: &[u8], types: &[types::ParamType], tokens: &[types::Token]) -> Vec<u8> {
        #[cfg(feature = "std")]
        {
            let name = String::from_utf8_lossy(name);
            let types: Vec<ethabi::ParamType> = types.iter().map(|i| i.into()).collect();
            let signed = ethabi::short_signature(&name, &types).to_vec();
            let tokens: Vec<ethabi::Token> = tokens.iter().map(|i| i.into()).collect();
            let encoded = ethabi::encode(tokens.as_slice());
            signed.into_iter().chain(encoded.into_iter()).collect()
        }

        #[cfg(not(feature = "std"))]
        {
            unimplemented!()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use sp_core::U256;

    #[test]
    fn test_encode_decode() {
        let encoded = eth_abi::encode_input(
            "ownerOf".as_bytes(),
            &[ParamType::Uint(256)],
            &[Token::Uint(U256::from(1919810u64))],
        );

        assert_eq!(
            encoded,
            vec![
                0x63, 0x52, 0x21, 0x1e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x1d, 0x4b, 0x42
            ]
        );
    }
}
