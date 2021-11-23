use sp_runtime_interface::runtime_interface;

pub use hashing::{checksum, ripemd160, sha256d};

#[runtime_interface]
pub trait Hashing {
    fn checksum(input: &[u8]) -> [u8; 4] {
        let mut result = [0u8; 4];
        result.copy_from_slice(&sha256d(input)[0..4]);
        result
    }

    fn sha256d(bytes: &[u8]) -> [u8; 32] {
        let digest = sp_io::hashing::sha2_256(&bytes);

        sp_io::hashing::sha2_256(&digest)
    }

    fn ripemd160(bytes: &[u8]) -> [u8; 20] {
        #[cfg(feature = "std")]
        {
            use ripemd160::{Digest, Ripemd160};

            let digest = sp_io::hashing::sha2_256(&bytes);

            let mut hasher_ripemd = Ripemd160::new();
            hasher_ripemd.update(digest);
            let mut ret = [0; 20];
            ret.copy_from_slice(&hasher_ripemd.finalize()[..]);
            ret
        }

        #[cfg(not(feature = "std"))]
        {
            unimplemented!()
        }
    }
}
