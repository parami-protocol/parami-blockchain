use std::sync::Arc;
pub use self::gen_client::Client as ZkpClient;
use jsonrpc_core::Result;
use jsonrpc_derive::rpc;
use parami_zkp::ocw;

#[rpc]
pub trait ZkpApi {
    /// Verify a proof
    ///
    /// # Arguments
    ///
    /// * `ek` - The encrypt key in json
    /// * `challenge` - The challenge in json
    /// * `proof` - The proof in json
    /// * `range` - A BigInt(gmp_impl) in BigEndian array
    /// * `cipher_x` The encrypted data
    /// # Results
    ///
    /// the proof result
    #[rpc(name = "zkp_verifier")]
    fn verify_it(&self, ek: Vec<u8>, json: Vec<u8>) -> Result<bool>;

    /// Encrypt something
    ///
    /// # Arguments
    ///
    /// * `ek` - The encrypt key in json
    /// * `data` - plain text in u8 array
    ///
    /// # Results
    ///
    /// the encrypted data in u8 array
    #[rpc(name = "zkp_encrypt")]
    fn encrypt(&self, ek: Vec<u8>, data: Vec<u8>) -> Result<Vec<u8>>;
}

pub struct ZkpRpcHandler<C> {
    _client: Arc<C>,
    _marker: std::marker::PhantomData<i32>,
}

impl<C> ZkpRpcHandler<C>
{
    pub fn new(_client: Arc<C>) -> Self {
        Self {
            _client,
            _marker: Default::default(),
        }
    }
}

impl<C> ZkpApi for ZkpRpcHandler<C>
    where C: Send + Sync + 'static
{
    fn verify_it(&self, ek: Vec<u8>, json: Vec<u8>) -> Result<bool> {
        let (result, _range) = ocw::zkp::verify(ek, json);
        Ok(result)
    }

    fn encrypt(&self, ek: Vec<u8>, data: Vec<u8>) -> Result<Vec<u8>> {
        Ok([1u8].into())
    }
}
