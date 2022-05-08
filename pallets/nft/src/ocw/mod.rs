pub use abi::eth_abi;

mod abi;

use crate::{Call, Config, Error, Pallet, Porting};
use frame_support::dispatch::DispatchResult;
use frame_system::offchain::{SendTransactionTypes, SubmitTransaction};
use parami_ocw::{submit_unsigned, Pallet as Ocw};
use parami_traits::Links;
use sp_core::U256;
use sp_std::prelude::Vec;

impl<T: Config + SendTransactionTypes<Call<T>>> Pallet<T> {
    pub fn ocw_begin_block(block_number: T::BlockNumber) -> DispatchResult {
        use parami_traits::types::Network::*;

        for network in [Ethereum] {
            let porting = <Porting<T>>::iter_prefix_values((network,));

            for task in porting {
                if task.deadline <= block_number {
                    // call to remove
                    Self::ocw_submit_porting(
                        task.task.owner,
                        task.task.network,
                        task.task.namespace,
                        task.task.token,
                        false,
                    );

                    continue;
                }

                if task.created < block_number {
                    // only start once (at created)
                    continue;
                }

                let links = T::Links::links(&task.task.owner, task.task.network);

                let result = match task.task.network {
                    Ethereum => Self::ocw_port_erc721(
                        &links,
                        "https://rinkeby.infura.io/v3/cffb10a5fde442cb80af59a65783c296",
                        &task.task.namespace,
                        &task.task.token,
                    ),
                    _ => {
                        // drop unsupported sites
                        Self::ocw_submit_porting(
                            task.task.owner,
                            task.task.network,
                            task.task.namespace,
                            task.task.token,
                            false,
                        );

                        continue;
                    }
                };

                if let Ok(()) = result {
                    Self::ocw_submit_porting(
                        task.task.owner,
                        task.task.network,
                        task.task.namespace,
                        task.task.token,
                        true,
                    );
                }
            }
        }

        Ok(())
    }

    pub(self) fn ocw_submit_porting(
        did: T::DecentralizedId,
        network: parami_traits::types::Network,
        namespace: Vec<u8>,
        token: Vec<u8>,
        validated: bool,
    ) {
        let call = Call::submit_porting {
            did,
            network,
            namespace,
            token,
            validated,
        };

        let _ = submit_unsigned!(call);
    }

    pub(super) fn ocw_port_erc721(
        links: &[Vec<u8>],
        rpc: &str,
        namespace: &[u8],
        token: &[u8],
    ) -> DispatchResult {
        use parami_ocw::JsonValue;

        let body = r#"{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "eth_call",
    "params": [
        {
            "from": "0x0000000000000000000000000000000000000000",
            "data": "<data>",
            "to": "<contract>"
        },
        "latest"
    ]
}"#;
        let encoded = eth_abi::encode_input(
            "ownerOf".as_bytes(),
            &[abi::ParamType::Uint(256)],
            &[abi::Token::Uint(U256::from(token))],
        );
        let body = body
            .replace("<data>", &hex::encode(&encoded))
            .replace("<contract>", &hex::encode(namespace));

        let res = Ocw::<T>::ocw_post(rpc, body.into())?;

        let res: Vec<u8> = match res.json() {
            JsonValue::Object(res) => {
                let v = res
                    .into_iter()
                    .find(|(k, _)| k.iter().copied().eq("result".chars()));
                match v {
                    Some((_, JsonValue::String(str))) => str.iter().map(|s| *s as u8).collect(),
                    _ => return Err(Error::<T>::InvalidExternalToken)?,
                }
            }
            _ => return Err(Error::<T>::InvalidExternalToken)?,
        };

        let res = eth_abi::decode(&[abi::ParamType::Address], &res);
        let res = res.first();
        match res {
            Some(abi::Token::Address(addr)) if links.contains(&addr.as_bytes().to_vec()) => Ok(()),
            _ => Err(Error::<T>::InvalidExternalToken)?,
        }
    }
}
