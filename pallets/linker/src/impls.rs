use crate::{
    btc,
    types::{AccountType, Signature},
    witness::WitnessProgram,
    Config, Error, Pallet,
};

use base58::ToBase58;
use codec::Encode;
use sp_std::prelude::*;

impl<T: Config> Pallet<T> {
    pub fn generate_message(did: &T::DecentralizedId) -> Vec<u8> {
        let mut bytes = b"Link: ".to_vec();

        let did = did.as_ref();
        let did = did.to_base58();
        let mut did = did.as_bytes().to_vec();

        let mut prefix = b"did:ad3:".to_vec();

        bytes.append(&mut prefix);
        bytes.append(&mut did);
        bytes
    }

    pub fn recover_address(
        crypto: AccountType,
        address: Vec<u8>,
        signature: Signature,
        bytes: Vec<u8>,
    ) -> Result<Vec<u8>, Error<T>> {
        match crypto {
            AccountType::Binance => Self::recover_address_eth(address, signature, bytes),
            AccountType::Bitcoin => Self::recover_address_btc(address, signature, bytes),
            AccountType::Ethereum => Self::recover_address_eth(address, signature, bytes),
            AccountType::Polkadot => Self::recover_address_dot(address, signature, bytes),
            AccountType::Solana => Self::recover_address_sol(address, signature, bytes),
            AccountType::Tron => Self::recover_address_trx(address, signature, bytes),
            _ => Err(Error::<T>::UnsupportedSite),
        }
    }

    fn recover_address_btc(
        address: Vec<u8>,
        signature: Signature,
        mut bytes: Vec<u8>,
    ) -> Result<Vec<u8>, Error<T>> {
        let mut length = (bytes.len() as u8).encode();
        let mut data = b"\x18Bitcoin Signed Message:\n".encode();
        data.append(&mut length);
        data.append(&mut bytes);
        let hash = btc::sha256d(&data);

        let mut sig: Signature = [0u8; 65];
        sig[64] = (signature[0] - 27) & 3;
        sig[..64].copy_from_slice(&signature[1..65]);

        let pk = sp_io::crypto::secp256k1_ecdsa_recover_compressed(&sig, &hash)
            .map_err(|_| Error::<T>::InvalidSignature)?;

        let recovered = if address[0] == b'1' {
            let mut result = [0u8; 25];

            result[0] = 0;
            result[1..21].copy_from_slice(&btc::ripemd160(&pk));
            let cs = btc::checksum(&result[0..21]);
            result[21..25].copy_from_slice(&cs);

            result.to_base58().as_bytes().to_vec()
        } else if address[0] == b'b' && address[1] == b'c' {
            // Native P2WPKH is a scriptPubKey of 22 bytes.
            // It starts with a OP_0, followed by a canonical push of the keyhash (i.e. 0x0014{20-byte keyhash})
            // keyhash is RIPEMD160(SHA256) of a compressed public key
            // https://bitcoincore.org/en/segwit_wallet_dev/

            let pk_hash = btc::ripemd160(&pk);
            let mut pk = [0u8; 22];
            pk[0] = 0;
            pk[1] = 20;
            pk[2..].copy_from_slice(&pk_hash);
            let wp = WitnessProgram::from_scriptpubkey(&pk.to_vec())
                .map_err(|_| Error::<T>::InvalidAddress)?;

            wp.to_address(b"bc".to_vec())
                .map_err(|_| Error::<T>::InvalidAddress)?
        } else {
            Err(Error::<T>::InvalidAddress)?
        };

        Ok(recovered)
    }

    fn recover_address_dot(
        raw: Vec<u8>,
        signature: Signature,
        bytes: Vec<u8>,
    ) -> Result<Vec<u8>, Error<T>> {
        use base58::FromBase58;
        use sp_core::sr25519;
        use sp_std::str;

        let address = str::from_utf8(&raw).map_err(|_| Error::<T>::InvalidAddress)?;
        let address = address
            .from_base58()
            .map_err(|_| Error::<T>::InvalidAddress)?;

        let mut nonce = [0u8; 32];
        nonce.copy_from_slice(&address[1..33]);

        let address = sr25519::Public::from_raw(nonce);
        let signature = sr25519::Signature::from_slice(&signature[1..]);

        if sp_io::crypto::sr25519_verify(&signature, &bytes, &address) {
            Ok(raw)
        } else {
            Err(Error::<T>::InvalidSignature)?
        }
    }

    fn recover_address_eth(
        _address: Vec<u8>,
        signature: Signature,
        mut bytes: Vec<u8>,
    ) -> Result<Vec<u8>, Error<T>> {
        let mut length = Self::usize_to_u8_array(bytes.len());
        let mut data = b"\x19Ethereum Signed Message:\n".encode();
        data.append(&mut length);
        data.append(&mut bytes);
        let hash = sp_io::hashing::keccak_256(&data);

        let pubkey = sp_io::crypto::secp256k1_ecdsa_recover(&signature, &hash)
            .map_err(|_| Error::<T>::InvalidSignature)?;
        let pk = sp_io::hashing::keccak_256(&pubkey);

        Ok(pk[12..32].to_vec())
    }

    fn recover_address_sol(
        raw: Vec<u8>,
        signature: Signature,
        bytes: Vec<u8>,
    ) -> Result<Vec<u8>, Error<T>> {
        use base58::FromBase58;
        use sp_core::ed25519;
        use sp_std::str;

        let address = str::from_utf8(&raw).map_err(|_| Error::<T>::InvalidAddress)?;
        let address = address
            .from_base58()
            .map_err(|_| Error::<T>::InvalidAddress)?;

        let mut nonce = [0u8; 32];
        nonce.copy_from_slice(&address[0..32]);

        let address = ed25519::Public::from_raw(nonce);
        let signature = ed25519::Signature::from_slice(&signature[1..]);

        if sp_io::crypto::ed25519_verify(&signature, &bytes, &address) {
            Ok(raw)
        } else {
            Err(Error::<T>::InvalidSignature)?
        }
    }

    fn recover_address_trx(
        _address: Vec<u8>,
        signature: Signature,
        mut bytes: Vec<u8>,
    ) -> Result<Vec<u8>, Error<T>> {
        let mut data = b"\x19TRON Signed Message:\n32".encode();
        data.append(&mut bytes);
        let hash = sp_io::hashing::keccak_256(&data);

        let pubkey = sp_io::crypto::secp256k1_ecdsa_recover(&signature, &hash)
            .map_err(|_| Error::<T>::InvalidSignature)?;
        let pk = sp_io::hashing::keccak_256(&pubkey);

        let mut result = [0u8; 25];

        result[0] = 0x41;
        result[1..21].copy_from_slice(&pk[12..32]);
        let cs = btc::checksum(&result[0..21]);
        result[21..25].copy_from_slice(&cs);

        let pk = result.to_base58().as_bytes().to_vec();

        Ok(pk)
    }

    fn usize_to_u8_array(length: usize) -> Vec<u8> {
        if length > 100 {
            return b"0".to_vec();
        }

        let digits = b"0123456789".encode();
        let tens = length / 10;
        let ones = length % 10;

        let mut vec_res: Vec<u8> = Vec::new();
        if tens != 0 {
            vec_res.push(digits[tens]);
        }
        vec_res.push(digits[ones]);

        vec_res
    }
}
