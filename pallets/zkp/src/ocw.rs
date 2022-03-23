use frame_system::offchain::{CreateSignedTransaction, SubmitTransaction};
use sp_runtime::{
    offchain::{http, Duration},
    DispatchError,
};
use sp_runtime_interface::runtime_interface;
use sp_std::prelude::*;
use crate::{Config, Call, Error, Pallet, PendingOf, EkOf};
pub const USER_AGENT: &str =
    "GoogleBot (compatible; ParamiValidator/1.0; +http://parami.io/validator/)";


#[runtime_interface]
pub trait Zkp {
    fn encrypt_something(ek: &str, data: Vec<u8>) -> bool {
        #[cfg(feature = "std")]
        {
            // use paillier::*;
            true
        }
        #[cfg(not(feature = "std"))]
        {
            unimplemented!()
        }
    }

    fn verify(ek: Vec<u8>, json: Vec<u8>) -> (bool,Vec<u8>) {
        #[cfg(feature = "std")]
        {
            use paillier::*;
            use serde::{Serialize, Deserialize};
            use std::*;
            #[derive(Debug, Serialize, Deserialize)]
            pub struct ProofParams {
                pub encrypted_pairs: EncryptedPairs,
                pub challenge_bits: ChallengeBits,
                pub proof: Proof,
                #[serde(with = "paillier::serialize::bigint")]
                pub range: BigInt,
                #[serde(with = "paillier::serialize::bigint")]
                pub cipher_x: BigInt,
            }
            let json= match std::str::from_utf8(&json.as_slice()){
                Ok(s) => s,
                Err(e) => {
                    return (false, e.to_string().into_bytes());
                }
            };


            let proof_params: ProofParams = match serde_json::from_str(json) {
                Ok(data) => data,
                Err(err) => return  (false, err.to_string().into_bytes()),
            };
            let ek: EncryptionKey = match bincode::deserialize(&ek){
                Ok(data) => data,
                Err(err) => return (false, err.to_string().into_bytes()),
            };
            //true
            let result =
                Paillier::verifier(&ek, &proof_params.challenge_bits, &proof_params.encrypted_pairs, &proof_params.proof, &proof_params.range, proof_params.cipher_x.into());
            println!("zkp zkp zkp zkp zkp zkp ****** {}", result.is_ok());
            let range=std::vec::Vec::from(&proof_params.range);
            return (result.is_ok(),range);
        }

        #[cfg(not(feature = "std"))]
        {
            unimplemented!()
        }
    }
}


macro_rules! submit_unsigned {
    ($call:expr) => {
        SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction($call.into())
    };
}

impl<T: Config + CreateSignedTransaction<Call<T>>> Pallet<T> {
    pub fn ocw_begin_block(block_number: T::BlockNumber) -> Result<(), DispatchError> {
        let pending = <PendingOf<T>>::iter();

        for (ipfs, task) in pending {
            if task.deadline <= block_number {
                // call to remove
                Self::ocw_submit_verify(task.did.clone(), task.ipfs.clone(),  "deadline".as_bytes().to_vec(),false);
                Err(Error::<T>::Deadline)?
            }

            if task.created < block_number {
                // only start once (at created + 1)
                continue;
            }
            if let Some(ek)= EkOf::<T>::get(&task.did) {
                let (verify_result,range)= Self::ocw_verify_from_ipfs(ek,ipfs)?;
                Self::ocw_submit_verify(task.did, task.ipfs, range,verify_result);
            }else{
                Err(Error::<T>::NoEk)?
            }
        }
        Ok(())
    }

    pub(crate) fn ocw_submit_verify(
        did: <T as parami_did::Config>::DecentralizedId,
        ipfs: Vec<u8>,
        range: Vec<u8>,
        result: bool,
    ) {
        let call = Call::submit_verify {
            did,
            ipfs,
            range,
            result,
        };

        let _ = submit_unsigned!(call);
    }


    pub(crate) fn ocw_verify_from_ipfs(
        ek: Vec<u8>,
        ipfs: Vec<u8>,
    ) -> Result<(bool, Vec<u8>), DispatchError> {
        let res = Self::ocw_fetch(ipfs)?;
        Ok(zkp::verify(ek, res))
    }


    pub(crate) fn ocw_fetch(url: Vec<u8>) -> Result<Vec<u8>, DispatchError> {
        let url = sp_std::str::from_utf8(&url.as_slice()).map_err(|_| Error::<T>::IpfsError)?;

        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(3_000));

        let request = http::Request::get(url);

        let pending = request
            .add_header("User-Agent", USER_AGENT)
            .deadline(deadline)
            .send()
            .map_err(|_| Error::<T>::HttpFetchingError)?;

        let response = pending
            .try_wait(deadline)
            .map_err(|_| Error::<T>::HttpFetchingError)?
            .map_err(|_| Error::<T>::HttpFetchingError)?;

        if response.code != 200 {
            tracing::warn!("Unexpected status code: {}", response.code);
            Err(Error::<T>::HttpFetchingError)?
        }

        Ok(response.body().collect::<Vec<u8>>())
    }
}

