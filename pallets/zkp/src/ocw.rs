use frame_system::offchain::CreateSignedTransaction;
use sp_runtime::DispatchError;
use sp_runtime_interface::runtime_interface;
use sp_std::prelude::*;
use crate::{Config, Error, Pallet,Call};
macro_rules! super_match {
        ($aa:expr) => {
            match $aa {
                    Ok(v)=>v,
                    Err(_)=>return false
                }
        };
}
#[runtime_interface]
pub trait Zkp {
    fn encrypt_something(ek: &str, data: Vec<u8>) -> bool {
        #[cfg(feature = "std")]
            {
                use paillier::*;
                true
            }
        #[cfg(not(feature = "std"))]
            {
                unimplemented!()
            }
    }

    fn verify(ek: Vec<u8>, challenge: Vec<u8>, encrypted_pairs: Vec<u8>, proof: Vec<u8>, range: Vec<u8>, cipher_x: Vec<u8>) -> bool {
        #[cfg(feature = "std")]
            {
                use paillier::*;
                use bincode::*;
                let encrypted_pairs:EncryptedPairs=super_match!(bincode::deserialize(&encrypted_pairs));
                let ek:EncryptionKey= super_match!(bincode::deserialize(&ek));
                let challenge: ChallengeBits = super_match!(bincode::deserialize(&challenge));
                let proof:Proof= super_match!(bincode::deserialize(&proof));
                let result =
                    Paillier::verifier(&ek, &challenge, &encrypted_pairs, &proof, &BigInt::from(range.as_slice()), BigInt::from(cipher_x.as_slice()).into());
                println!("zkp zkp zkp zkp zkp zkp ****** {}", result.is_ok());
                return result.is_ok();
            }

        #[cfg(not(feature = "std"))]
            {
                unimplemented!()
            }
    }
}
