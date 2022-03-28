pub use abi::eth_abi;

mod abi;

use crate::{Call, Config, Error, Pallet, Porting};
use frame_support::dispatch::DispatchResult;
use frame_system::offchain::{SendTransactionTypes, SubmitTransaction};
use parami_ocw::{submit_unsigned, Pallet as Ocw};
use sp_std::prelude::Vec;

impl<T: Config + SendTransactionTypes<Call<T>>> Pallet<T> {
    pub fn ocw_begin_block(block_number: T::BlockNumber) -> DispatchResult {
        use parami_traits::types::Network::*;

        for network in [Ethereum] {
            let porting = <Porting<T>>::iter_prefix_values((network,));

            for task in porting {
                if task.deadline <= block_number {
                    // call to remove
                    Self::ocw_submit_port(
                        task.task.owner,
                        task.task.network,
                        task.task.namespace,
                        task.task.token,
                        false,
                    );

                    return Err(Error::<T>::Deadline)?;
                }

                if task.created < block_number {
                    // only start once (at created + 1)
                    continue;
                }

                // let profile = sp_std::str::from_utf8(&task.task) //
                //     .map_err(|_| Error::<T>::HttpFetchingError)?;

                // let result = match site {
                //     Telegram => Self::ocw_verify_telegram(did, profile),
                //     Twitter => Self::ocw_verify_twitter(did, profile),
                //     _ => {
                //         // drop unsupported sites
                //         Self::ocw_submit_link(did, site, task.task, false);

                //         continue;
                //     }
                // };

                // if let Ok(()) = result {
                //     Self::ocw_submit_link(did, site, task.task, true);
                // }
            }
        }

        Ok(())
    }

    pub(self) fn ocw_submit_port(
        did: T::DecentralizedId,
        network: parami_traits::types::Network,
        namespace: Vec<u8>,
        token: Vec<u8>,
        validated: bool,
    ) {
        let call = Call::submit_port {
            did,
            network,
            namespace,
            token,
            validated,
        };

        let _ = submit_unsigned!(call);
    }
}
