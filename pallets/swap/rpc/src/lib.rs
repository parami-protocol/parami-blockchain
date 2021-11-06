pub use self::gen_client::Client as SwapClient;
use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
pub use parami_swap_rpc_runtime_api::{BalanceWrapper, SwapRuntimeApi};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
};
use std::sync::Arc;

#[rpc]
pub trait SwapApi<BlockHash, AssetId, Balance>
where
    Balance: MaybeDisplay + MaybeFromStr,
{
    /// Get dry-run result of mint
    #[rpc(name = "swaps_drylyMint")]
    fn dryly_mint(
        &self,
        token_id: AssetId,
        currency: BalanceWrapper<Balance>,
        tokens: BalanceWrapper<Balance>,
        at: Option<BlockHash>,
    ) -> Result<(
        AssetId,
        BalanceWrapper<Balance>,
        AssetId,
        BalanceWrapper<Balance>,
    )>;

    /// Get dry-run result of burn
    #[rpc(name = "swaps_drylyBurn")]
    fn dryly_burn(
        &self,
        token_id: AssetId,
        liquidity: BalanceWrapper<Balance>,
        at: Option<BlockHash>,
    ) -> Result<(
        AssetId,
        BalanceWrapper<Balance>,
        AssetId,
        BalanceWrapper<Balance>,
    )>;

    /// Get dry-run result of token_out
    #[rpc(name = "swaps_drylyTokenOut")]
    fn dryly_token_out(
        &self,
        token_id: AssetId,
        tokens: BalanceWrapper<Balance>,
        at: Option<BlockHash>,
    ) -> Result<BalanceWrapper<Balance>>;

    /// Get dry-run result of token_in
    #[rpc(name = "swaps_drylyTokenIn")]
    fn dryly_token_in(
        &self,
        token_id: AssetId,
        tokens: BalanceWrapper<Balance>,
        at: Option<BlockHash>,
    ) -> Result<BalanceWrapper<Balance>>;

    /// Get dry-run result of quote_in
    #[rpc(name = "swaps_drylyQuoteIn")]
    fn dryly_quote_in(
        &self,
        token_id: AssetId,
        currency: BalanceWrapper<Balance>,
        at: Option<BlockHash>,
    ) -> Result<BalanceWrapper<Balance>>;

    /// Get dry-run result of quote_out
    #[rpc(name = "swaps_drylyQuoteOut")]
    fn dryly_quote_out(
        &self,
        token_id: AssetId,
        currency: BalanceWrapper<Balance>,
        at: Option<BlockHash>,
    ) -> Result<BalanceWrapper<Balance>>;
}

pub struct SwapsRpcHandler<C, Block, AssetId, Balance> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<(Block, AssetId, Balance)>,
}

impl<C, Block, AssetId, Balance> SwapsRpcHandler<C, Block, AssetId, Balance> {
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, AssetId, Balance> SwapApi<<Block as BlockT>::Hash, AssetId, Balance>
    for SwapsRpcHandler<C, Block, AssetId, Balance>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: SwapRuntimeApi<Block, AssetId, Balance>,
    AssetId: Codec + Send + Sync + 'static,
    Balance: Codec + MaybeDisplay + MaybeFromStr + Send + Sync + 'static,
{
    fn dryly_mint(
        &self,
        token_id: AssetId,
        currency: BalanceWrapper<Balance>,
        tokens: BalanceWrapper<Balance>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<(
        AssetId,
        BalanceWrapper<Balance>,
        AssetId,
        BalanceWrapper<Balance>,
    )> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let res = api
            .dryly_mint(&at, token_id, currency, tokens)
            .map_err(|e| RpcError {
                code: ErrorCode::InternalError,
                message: "Unable to dry-run mint.".into(),
                data: Some(format!("{:?}", e).into()),
            })?;

        res.ok_or(RpcError {
            code: ErrorCode::ServerError(1),
            message: "Unable to dry-run mint.".into(),
            data: None,
        })
    }

    fn dryly_burn(
        &self,
        token_id: AssetId,
        liquidity: BalanceWrapper<Balance>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<(
        AssetId,
        BalanceWrapper<Balance>,
        AssetId,
        BalanceWrapper<Balance>,
    )> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let res = api
            .dryly_burn(&at, token_id, liquidity)
            .map_err(|e| RpcError {
                code: ErrorCode::InternalError,
                message: "Unable to dry-run burn.".into(),
                data: Some(format!("{:?}", e).into()),
            })?;

        res.ok_or(RpcError {
            code: ErrorCode::ServerError(1),
            message: "Unable to dry-run burn.".into(),
            data: None,
        })
    }

    fn dryly_token_out(
        &self,
        token_id: AssetId,
        tokens: BalanceWrapper<Balance>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let res = api
            .dryly_token_out(&at, token_id, tokens)
            .map_err(|e| RpcError {
                code: ErrorCode::InternalError,
                message: "Unable to dry-run token_out.".into(),
                data: Some(format!("{:?}", e).into()),
            })?;

        res.ok_or(RpcError {
            code: ErrorCode::ServerError(1),
            message: "Unable to dry-run token_out.".into(),
            data: None,
        })
    }

    fn dryly_token_in(
        &self,
        token_id: AssetId,
        tokens: BalanceWrapper<Balance>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let res = api
            .dryly_token_in(&at, token_id, tokens)
            .map_err(|e| RpcError {
                code: ErrorCode::InternalError,
                message: "Unable to dry-run token_in.".into(),
                data: Some(format!("{:?}", e).into()),
            })?;

        res.ok_or(RpcError {
            code: ErrorCode::ServerError(1),
            message: "Unable to dry-run token_in.".into(),
            data: None,
        })
    }

    fn dryly_quote_in(
        &self,
        token_id: AssetId,
        currency: BalanceWrapper<Balance>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let res = api
            .dryly_quote_in(&at, token_id, currency)
            .map_err(|e| RpcError {
                code: ErrorCode::InternalError,
                message: "Unable to dry-run quote_in.".into(),
                data: Some(format!("{:?}", e).into()),
            })?;

        res.ok_or(RpcError {
            code: ErrorCode::ServerError(1),
            message: "Unable to dry-run quote_in.".into(),
            data: None,
        })
    }

    fn dryly_quote_out(
        &self,
        token_id: AssetId,
        currency: BalanceWrapper<Balance>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let res = api
            .dryly_quote_out(&at, token_id, currency)
            .map_err(|e| RpcError {
                code: ErrorCode::InternalError,
                message: "Unable to dry-run quote_out.".into(),
                data: Some(format!("{:?}", e).into()),
            })?;

        res.ok_or(RpcError {
            code: ErrorCode::ServerError(1),
            message: "Unable to dry-run quote_out.".into(),
            data: None,
        })
    }
}