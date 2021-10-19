#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[rustfmt::skip]
pub mod weights;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod btc;
mod eth;

use btc::{base58::ToBase58, witness::WitnessProgram};
use codec::Encode;
use frame_support::dispatch::DispatchResultWithPostInfo;
use frame_system::ensure_signed;
use sp_io::crypto::secp256k1_ecdsa_recover_compressed;
use sp_std::prelude::*;

use weights::WeightInfo;

type Signature = [u8; 65];

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    pub const EXPIRING_BLOCK_NUMBER_MAX: u32 = 10 * 60 * 24 * 30; // 30 days for 6s per block
    pub const MAX_ETH_LINKS: usize = 3;
    pub const MAX_BTC_LINKS: usize = 3;

    enum BTCAddressType {
        Legacy,
        SegWit,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    #[pallet::getter(fn eth_addresses)]
    pub(super) type EthereumLink<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, Vec<Vec<u8>>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn btc_addresses)]
    pub(super) type BitcoinLink<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, Vec<Vec<u8>>, ValueQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        EthAddressLinked(T::AccountId, Vec<u8>),
        BtcAddressLinked(T::AccountId, Vec<u8>),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        // Cannot recover the signature
        EcdsaRecoverFailure,
        // Link request expired
        LinkRequestExpired,
        // Provided address mismatch the address recovered from signature recovery
        UnexpectedAddress,
        // Unexpected ethereum message length error
        UnexpectedEthMsgLength,
        // Invalid BTC address to link
        InvalidBTCAddress,
        // Expiration block number is too far away from now
        InvalidExpiringBlockNumber,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Link an Ethereum address.
        /// providing a proof signature from the private key of that Ethereum address.
        ///
        /// The runtime needs to ensure that a malicious index can be handled correctly.
        /// Currently, when vec.len > MAX_ETH_LINKS, replacement will always happen at the final index.
        /// Otherwise it will use the next new slot unless index is valid against a currently available slot.
        ///
        /// Parameters:
        /// - `account`: The address that is to be linked
        /// - `index`: The index of the linked Ethereum address that the user wants to replace with
        /// - `address`: The intended Ethereum address to link
        /// - `expiring_block_number`: The block number after which this link request will expire
        /// - `sig`: The rsv-signature generated by the private key of the address
        ///
        /// Emits `EthAddressLinked` event when successful.
        #[pallet::weight(T::WeightInfo::link_eth())]
        pub fn link_eth(
            origin: OriginFor<T>,
            account: T::AccountId,
            index: u32,
            address: Vec<u8>,
            expiring_block_number: T::BlockNumber,
            sig: Signature,
        ) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin)?;

            let current_block_number = <frame_system::Pallet<T>>::block_number();
            ensure!(
                expiring_block_number > current_block_number,
                Error::<T>::LinkRequestExpired
            );
            ensure!(
                (expiring_block_number - current_block_number)
                    < T::BlockNumber::from(EXPIRING_BLOCK_NUMBER_MAX),
                Error::<T>::InvalidExpiringBlockNumber
            );

            let bytes = Self::generate_raw_message(&account, expiring_block_number);

            let hash = eth::eth_data_hash(bytes).map_err(|_| Error::<T>::UnexpectedEthMsgLength)?;

            let mut msg = [0u8; 32];
            msg[..32].copy_from_slice(&hash[..32]);

            let addr =
                eth::address_from_sig(msg, sig).map_err(|_| Error::<T>::EcdsaRecoverFailure)?;
            ensure!(addr == address, Error::<T>::UnexpectedAddress);

            EthereumLink::<T>::mutate(&account, |addrs| {
                let index = index as usize;
                // NOTE: allow linking `MAX_ETH_LINKS` eth addresses.
                if (index >= addrs.len()) && (addrs.len() != MAX_ETH_LINKS) {
                    addrs.push(addr.clone());
                } else if (index >= addrs.len()) && (addrs.len() == MAX_ETH_LINKS) {
                    addrs[MAX_ETH_LINKS - 1] = addr.clone();
                } else {
                    addrs[index] = addr.clone();
                }
            });

            Self::deposit_event(Event::EthAddressLinked(account, addr.to_vec()));

            Ok(().into())
        }

        /// Link a BTC address.
        /// providing a proof signature from the private key of that BTC address.
        /// The BTC address may either be a legacy P2PK one (started with b'1') or a SegWit P2PK one (started with b'bc').
        ///
        /// The runtime needs to ensure that a malicious index can be handled correctly.
        /// Currently, when vec.len > MAX_ETH_LINKS, replacement will always happen at the final index.
        /// Otherwise it will use the next new slot unless index is valid against a currently available slot.
        ///
        /// Parameters:
        /// - `account`: The address that is to be linked
        /// - `index`: The index of the linked BTC address that the user wants to replace with
        /// - `address`: The intended BTC address to link
        /// - `expiring_block_number`: The block number after which this link request will expire
        /// - `sig`: The rsv-signature generated by the private key of the address
        ///
        /// Emits `BtcAddressLinked` event when successful.
        #[pallet::weight(T::WeightInfo::link_btc())]
        pub fn link_btc(
            origin: OriginFor<T>,
            account: T::AccountId,
            index: u32,
            address: Vec<u8>,
            expiring_block_number: T::BlockNumber,
            sig: Signature,
        ) -> DispatchResultWithPostInfo {
            let _ = ensure_signed(origin)?;

            let current_block_number = <frame_system::Pallet<T>>::block_number();
            ensure!(
                expiring_block_number > current_block_number,
                Error::<T>::LinkRequestExpired
            );
            ensure!(
                (expiring_block_number - current_block_number)
                    < T::BlockNumber::from(EXPIRING_BLOCK_NUMBER_MAX),
                Error::<T>::InvalidExpiringBlockNumber
            );

            if address.len() < 2 {
                Err(Error::<T>::InvalidBTCAddress)?
            }

            let address_type = if address[0] == b'1' {
                BTCAddressType::Legacy
            } else if address[0] == b'b' && address[1] == b'c' {
                BTCAddressType::SegWit
            } else {
                Err(Error::<T>::InvalidBTCAddress)?
            };

            let bytes = Self::generate_raw_message(&account, expiring_block_number);

            let hash = sp_io::hashing::keccak_256(&bytes);

            let mut msg = [0u8; 32];
            msg[..32].copy_from_slice(&hash[..32]);

            let pk = secp256k1_ecdsa_recover_compressed(&sig, &msg)
                .map_err(|_| Error::<T>::EcdsaRecoverFailure)?;

            let addr = match address_type {
                BTCAddressType::Legacy => btc::legacy::btc_address_from_pk(&pk).to_base58(),
                // Native P2WPKH is 22 bytes, starts with a OP_0, followed by a canonical push of the keyhash (i.e. 0x0014{20-byte keyhash})
                // keyhash is RIPEMD160(SHA256) of a compressed public key
                // https://bitcoincore.org/en/SegWit_wallet_dev/
                BTCAddressType::SegWit => {
                    let pk_hash = btc::legacy::hash160(&pk);
                    let mut pk = [0u8; 22];
                    pk[0] = 0;
                    pk[1] = 20;
                    pk[2..].copy_from_slice(&pk_hash);
                    let wp = WitnessProgram::from_scriptpubkey(&pk.to_vec())
                        .map_err(|_| Error::<T>::InvalidBTCAddress)?;
                    wp.to_address(b"bc".to_vec())
                        .map_err(|_| Error::<T>::InvalidBTCAddress)?
                }
            };

            ensure!(addr == address, Error::<T>::UnexpectedAddress);

            BitcoinLink::<T>::mutate(&account, |addrs| {
                let index = index as usize;
                // NOTE: allow linking `MAX_BTC_LINKS` btc addresses.
                if (index >= addrs.len()) && (addrs.len() != MAX_BTC_LINKS) {
                    addrs.push(addr.clone());
                } else if (index >= addrs.len()) && (addrs.len() == MAX_BTC_LINKS) {
                    addrs[MAX_BTC_LINKS - 1] = addr.clone();
                } else {
                    addrs[index] = addr.clone();
                }
            });

            Self::deposit_event(Event::BtcAddressLinked(account, addr));

            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Assemble the message that the user has signed
    /// Format: "Link Parami: " + Parami account + expiring block number
    fn generate_raw_message(
        account: &T::AccountId,
        expiring_block_number: T::BlockNumber,
    ) -> Vec<u8> {
        let mut bytes = b"Link Parami: ".encode();
        let mut account_vec = account.encode();
        let mut expiring_block_number_vec = expiring_block_number.encode();

        bytes.append(&mut account_vec);
        bytes.append(&mut expiring_block_number_vec);
        bytes
    }
}
