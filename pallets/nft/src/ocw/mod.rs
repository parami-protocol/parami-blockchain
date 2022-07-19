pub use abi::eth_abi;

mod abi;
mod types;

use crate::{Call, Config, Error, Pallet, Porting, ValidateEndpoint};
use ethabi::ethereum_types::U256;
use frame_support::dispatch::DispatchError;
use frame_support::dispatch::DispatchResult;
use frame_system::offchain::{SendTransactionTypes, SubmitTransaction};
use log;
use parami_ocw::JsonValue;
use parami_ocw::{submit_unsigned, Pallet as Ocw};
use parami_traits::Links;
use scale_info::prelude::string::String;
use sp_std::prelude::Vec;
use sp_std::str;

impl<T: Config + SendTransactionTypes<Call<T>>> Pallet<T> {
    pub fn ocw_begin_block(block_number: T::BlockNumber) -> DispatchResult {
        use parami_traits::types::Network::*;

        for network in [Ethereum] {
            let porting = <Porting<T>>::iter_prefix_values((network,));

            let endpoint = <ValidateEndpoint<T>>::get(network);
            if endpoint.is_none() {
                log::error!("network {:?} endpoint not found, skip import", network);
                continue;
            }
            let endpoint = endpoint.unwrap().into_inner();
            let endpoint = str::from_utf8(&endpoint);
            if endpoint.is_err() {
                log::error!(
                    "Convert endpoint to str failed, err = {:?}",
                    endpoint.unwrap_err(),
                );
            }

            let endpoint = endpoint.unwrap();

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

                let links = T::Links::links(&task.task.owner, task.task.network);

                let result = match task.task.network {
                    Ethereum => Self::ocw_validate_etherum_token_owner(
                        &links,
                        endpoint,
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

    pub(super) fn construct_request_body(namespace: &[u8], token: &[u8]) -> String {
        let body = r#"{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "eth_call",
    "params": [
        {
            "from": "0x0000000000000000000000000000000000000000",
            "data": "0x<data>",
            "to": "0x<contract>"
        },
        "latest"
    ]
}"#;
        let encoded = eth_abi::encode_input(
            "ownerOf".as_bytes(),
            &[types::ParamType::Uint(256)],
            &[types::Token::Uint(U256::from(token))],
        );
        let body = body
            .replace("<data>", &hex::encode(&encoded))
            .replace("<contract>", &hex::encode(&namespace));
        return body;
    }

    pub(super) fn ocw_validate_etherum_token_owner(
        links: &[Vec<u8>],
        rpc: &str,
        namespace: &[u8],
        token: &[u8],
    ) -> DispatchResult {
        let token_owner = Self::ocw_fetch_etherum_token_owner(rpc, namespace, token)?;

        Self::ocw_validate_token_owner(links, &token_owner)
    }

    pub(super) fn ocw_fetch_etherum_token_owner(
        rpc: &str,
        contract: &[u8],
        token: &[u8],
    ) -> Result<U256, DispatchError> {
        let body = Self::construct_request_body(contract, token);
        let res = Ocw::<T>::ocw_post(rpc, body.into())?;

        let json = res.json();
        match json {
            JsonValue::Object(res) => {
                let v = res
                    .into_iter()
                    .find(|(k, _)| k.iter().copied().eq("result".chars()));
                match v {
                    Some((_, JsonValue::String(chars))) => {
                        let str: String = chars.into_iter().collect();
                        Ok(U256::from_str_radix(str.as_str(), 16)
                            .map_err(|_e| Error::<T>::OcwParseError)?)
                    }
                    _ => return Err(Error::<T>::OcwParseError)?,
                }
            }
            _ => return Err(Error::<T>::OcwParseError)?,
        }
    }

    pub(super) fn ocw_validate_token_owner(
        links: &[Vec<u8>],
        token_owner: &U256,
    ) -> DispatchResult {
        let links: Vec<U256> = links.into_iter().map(|l| U256::from(l as &[u8])).collect();
        if links.contains(token_owner) {
            Ok(())
        } else {
            Err(Error::<T>::NotTokenOwner)?
        }
    }
}
