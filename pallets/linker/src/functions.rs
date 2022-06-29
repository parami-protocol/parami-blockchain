use crate::{
    btc, types, witness::WitnessProgram, Config, DidOf, Error, Event, Linked, LinksOf, Pallet,
    PendingOf,
};

use base58::ToBase58;
use codec::Encode;
use frame_support::ensure;
use parami_traits::types::{Network, Task};
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::prelude::*;

macro_rules! is_task {
    ($profile:expr, $prefix:expr) => {
        $profile.starts_with($prefix) && $profile.len() > $prefix.len()
    };
}

impl<T: Config> Pallet<T> {
    fn ensure_profile(did: &DidOf<T>, site: Network, profile: &[u8]) -> DispatchResult {
        use Network::*;

        ensure!(!<LinksOf<T>>::contains_key(did, site), Error::<T>::Exists);
        ensure!(
            !<Linked<T>>::contains_key(site, profile),
            Error::<T>::Exists
        );

        match site {
            Binance | Bitcoin | Eosio | Ethereum | Kusama | Polkadot | Solana | Tron | Near
            | Unknown => {}

            Discord if is_task!(profile, b"https://discordapp.com/users/") => {}
            Facebook if is_task!(profile, b"https://www.facebook.com/") => {}
            Github if is_task!(profile, b"https://github.com/") => {}
            HackerNews if is_task!(profile, b"https://news.ycombinator.com/user?id=") => {}
            Mastodon => {}
            Reddit if is_task!(profile, b"https://www.reddit.com/user/") => {}
            Telegram if is_task!(profile, b"https://t.me/") => {}
            Twitter if is_task!(profile, b"https://twitter.com/") => {}

            _ => Err(Error::<T>::UnsupportedSite)?,
        };

        Ok(())
    }

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

    pub fn veto_pending(did: DidOf<T>, site: Network, profile: Vec<u8>) -> DispatchResult {
        <PendingOf<T>>::remove(site, &did);

        Self::deposit_event(Event::<T>::ValidationFailed(did, site, profile));

        Ok(())
    }

    pub fn insert_link(
        did: DidOf<T>,
        site: Network,
        profile: Vec<u8>,
        registrar: DidOf<T>,
    ) -> DispatchResult {
        Self::ensure_profile(&did, site, &profile)?;

        <PendingOf<T>>::remove(site, &did);

        <Linked<T>>::insert(site, &profile, true);

        <LinksOf<T>>::insert(&did, site, profile.clone());

        Self::deposit_event(Event::<T>::AccountLinked(did, site, profile, registrar));

        Ok(())
    }

    pub fn insert_pending(did: DidOf<T>, site: Network, profile: Vec<u8>) -> DispatchResult {
        use frame_support::traits::Get;
        use sp_runtime::traits::Saturating;

        Self::ensure_profile(&did, site, &profile)?;

        ensure!(
            !<PendingOf<T>>::contains_key(site, &did),
            Error::<T>::Exists
        );

        let created = <frame_system::Pallet<T>>::block_number();
        let lifetime = T::PendingLifetime::get();
        let deadline = created.saturating_add(lifetime);

        <PendingOf<T>>::insert(
            site,
            &did,
            Task {
                task: profile,
                deadline,
                created,
            },
        );

        Ok(())
    }

    pub fn recover_address(
        crypto: Network,
        address: Vec<u8>,
        signature: types::Signature,
        bytes: Vec<u8>,
    ) -> Result<Vec<u8>, DispatchError> {
        use Network::*;

        match crypto {
            Unknown => Ok(address),
            Binance => Self::recover_address_eth(address, signature, bytes),
            Bitcoin => Self::recover_address_btc(address, signature, bytes),
            Ethereum => Self::recover_address_eth(address, signature, bytes),
            Polkadot => Self::recover_address_dot(address, signature, bytes),
            Solana => Self::recover_address_sol(address, signature, bytes),
            Tron => Self::recover_address_trx(address, signature, bytes),
            _ => Err(Error::<T>::UnsupportedSite)?,
        }
    }

    fn recover_address_btc(
        address: Vec<u8>,
        signature: types::Signature,
        mut bytes: Vec<u8>,
    ) -> Result<Vec<u8>, DispatchError> {
        let mut length = (bytes.len() as u8).encode();
        let mut data = b"\x18Bitcoin Signed Message:\n".encode();
        data.append(&mut length);
        data.append(&mut bytes);
        let hash = btc::sha256d(&data);

        let mut sig: types::Signature = [0u8; 65];
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
        signature: types::Signature,
        bytes: Vec<u8>,
    ) -> Result<Vec<u8>, DispatchError> {
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
        let signature =
            sr25519::Signature::from_slice(&signature[1..]).ok_or(Error::<T>::InvalidAddress)?;

        if sp_io::crypto::sr25519_verify(&signature, &bytes, &address) {
            Ok(raw)
        } else {
            Err(Error::<T>::InvalidSignature)?
        }
    }

    fn recover_address_eth(
        _address: Vec<u8>,
        signature: types::Signature,
        mut bytes: Vec<u8>,
    ) -> Result<Vec<u8>, DispatchError> {
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
        signature: types::Signature,
        bytes: Vec<u8>,
    ) -> Result<Vec<u8>, DispatchError> {
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
        let signature =
            ed25519::Signature::from_slice(&signature[1..]).ok_or(Error::<T>::InvalidAddress)?;

        if sp_io::crypto::ed25519_verify(&signature, &bytes, &address) {
            Ok(raw)
        } else {
            Err(Error::<T>::InvalidSignature)?
        }
    }

    fn recover_address_trx(
        _address: Vec<u8>,
        signature: types::Signature,
        mut bytes: Vec<u8>,
    ) -> Result<Vec<u8>, DispatchError> {
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
