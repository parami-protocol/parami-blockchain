use codec::Codec;
use jsonrpsee::{
    core::{async_trait, RpcResult},
    proc_macros::rpc,
};
use parami_did_utils::derive_storage_key;
use parking_lot::RwLock;
use sp_core::offchain::OffchainStorage;
use std::sync::Arc;

#[rpc(client, server)]
pub trait DidApi<DecentralizedId> {
    /// Get metadata of a DID
    ///
    /// # Arguments
    ///
    /// * `did` - The DID
    /// * `key` - The metadata key
    ///
    /// # Results
    ///
    /// the requested metadata
    #[method(name = "did_getMetadata")]
    fn get_metadata(&self, did: DecentralizedId, key: String) -> RpcResult<String>;

    /// Batch get metadata of a DID
    ///
    /// # Arguments
    ///
    /// * `did` - The DID
    /// * `keys` - The metadata keys
    ///
    /// # Results
    ///
    /// the requested metadata
    #[method(name = "did_batchGetMetadata")]
    fn batch_get_metadata(&self, did: DecentralizedId, keys: Vec<String>)
        -> RpcResult<Vec<String>>;
}

pub struct DidRpcHandler<T: OffchainStorage, DecentralizedId> {
    storage: Arc<RwLock<T>>,
    _marker: std::marker::PhantomData<DecentralizedId>,
}

impl<T, DecentralizedId> DidRpcHandler<T, DecentralizedId>
where
    T: OffchainStorage,
    DecentralizedId: Codec,
{
    pub fn new(storage: T) -> Self {
        Self {
            storage: Arc::new(RwLock::new(storage)),
            _marker: Default::default(),
        }
    }
}

#[async_trait]
impl<T, DecentralizedId> DidApiServer<DecentralizedId> for DidRpcHandler<T, DecentralizedId>
where
    T: OffchainStorage + 'static,
    DecentralizedId: Codec + Send + Sync + 'static,
{
    fn get_metadata(&self, did: DecentralizedId, key: String) -> RpcResult<String> {
        let metadata = self
            .storage
            .read()
            .get(
                sp_offchain::STORAGE_PREFIX,
                &*derive_storage_key(key.as_bytes(), &did),
            )
            .map(from_utf8)
            .unwrap_or_default();

        Ok(metadata)
    }

    fn batch_get_metadata(
        &self,
        did: DecentralizedId,
        keys: Vec<String>,
    ) -> RpcResult<Vec<String>> {
        let mut result = Vec::new();

        for key in keys {
            let metadata = self
                .storage
                .read()
                .get(
                    sp_offchain::STORAGE_PREFIX,
                    &*derive_storage_key(key.as_bytes(), &did),
                )
                .map(from_utf8)
                .unwrap_or_default();

            result.push(metadata);
        }

        Ok(result)
    }
}

fn from_utf8<S: AsRef<[u8]>>(s: S) -> String {
    String::from_utf8_lossy(s.as_ref()).into_owned()
}
