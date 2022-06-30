use codec::Codec;
use jsonrpsee::{
    core::{async_trait, Error, RpcResult},
    proc_macros::rpc,
    types::error::{CallError, ErrorObject, INTERNAL_ERROR_CODE},
};
use parami_primitives::BalanceWrapper;
pub use parami_swap_rpc_runtime_api::{ApiResult, SwapRuntimeApi};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{
    generic::BlockId,
    traits::{Block as BlockT, MaybeDisplay, MaybeFromStr},
};
use std::sync::Arc;

#[rpc(client, server)]
pub trait SwapApi<BlockHash, AssetId, Balance>
where
    Balance: MaybeDisplay + MaybeFromStr,
{
    /// Get dry-run result of add_liquidity
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `currency` - The currency to be involved in the swap
    /// * `max_tokens` - The maximum amount of tokens to be involved in the swap
    ///
    /// # Results
    ///
    /// tuple of (tokens, liquidity)
    ///
    /// * `tokens` - The amount of tokens to be involved in the swap
    /// * `liquidity` - The amount of liquidity to be minted
    #[method(name = "swap_drylyAddLiquidity")]
    fn dryly_add_liquidity(
        &self,
        token_id: AssetId,
        currency: BalanceWrapper<Balance>,
        max_tokens: BalanceWrapper<Balance>,
        at: Option<BlockHash>,
    ) -> RpcResult<(BalanceWrapper<Balance>, BalanceWrapper<Balance>)>;

    /// Get dry-run result of remove_liquidity
    ///
    /// # Arguments
    ///
    /// * `lp_token_id` - The Liquidity Provider Token ID
    ///
    /// # Results
    ///
    /// tuple of (token_id, liquidity, tokens, currency)
    ///
    /// * `token_id` - The Asset ID
    /// * `liquidity` - The amount of liquidity removed
    /// * `tokens` - The amount of tokens to be returned
    /// * `currency` - The currency to be returned
    #[method(name = "swap_drylyRemoveLiquidity")]
    fn dryly_remove_liquidity(
        &self,
        lp_token_id: AssetId,
        at: Option<BlockHash>,
    ) -> RpcResult<(
        AssetId,
        BalanceWrapper<Balance>,
        BalanceWrapper<Balance>,
        BalanceWrapper<Balance>,
    )>;

    /// Get dry-run result of buy_tokens
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `tokens` - The amount of tokens to be bought
    ///
    /// # Results
    ///
    /// The currency needed
    #[method(name = "swap_drylyBuyTokens")]
    fn dryly_buy_tokens(
        &self,
        token_id: AssetId,
        tokens: BalanceWrapper<Balance>,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;

    /// Get dry-run result of sell_tokens
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `tokens` - The amount of tokens to be sold
    ///
    /// # Results
    ///
    /// The currency to be gained
    #[method(name = "swap_drylySellTokens")]
    fn dryly_sell_tokens(
        &self,
        token_id: AssetId,
        tokens: BalanceWrapper<Balance>,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;

    /// Get dry-run result of sell_currency
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `currency` - The currency to be sold
    ///
    /// # Results
    ///
    /// The amount of tokens to be gained
    #[method(name = "swap_drylySellCurrency")]
    fn dryly_sell_currency(
        &self,
        token_id: AssetId,
        currency: BalanceWrapper<Balance>,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;

    /// Get dry-run result of buy_currency
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `currency` - The currency to be bought
    ///
    /// # Results
    ///
    /// The amount of tokens needed
    #[method(name = "swap_drylyBuyCurrency")]
    fn dryly_buy_currency(
        &self,
        token_id: AssetId,
        currency: BalanceWrapper<Balance>,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;

    /// Calculate staking reward
    ///
    /// # Arguments
    ///
    /// * `lp_token_id` - The Liquidity Provider Token ID
    ///
    /// # Results
    ///
    /// The amount of reward tokens
    #[method(name = "swap_calculateReward")]
    fn calculate_reward(
        &self,
        lp_token_id: AssetId,
        at: Option<BlockHash>,
    ) -> RpcResult<BalanceWrapper<Balance>>;
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

#[async_trait]
impl<C, Block, AssetId, Balance> SwapApiServer<<Block as BlockT>::Hash, AssetId, Balance>
    for SwapsRpcHandler<C, Block, AssetId, Balance>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: SwapRuntimeApi<Block, AssetId, Balance>,
    AssetId: Codec + Send + Sync + 'static,
    Balance: Codec + MaybeDisplay + MaybeFromStr + Send + Sync + 'static,
{
    fn dryly_add_liquidity(
        &self,
        token_id: AssetId,
        currency: BalanceWrapper<Balance>,
        max_tokens: BalanceWrapper<Balance>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<(BalanceWrapper<Balance>, BalanceWrapper<Balance>)> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let res = api
            .dryly_add_liquidity(&at, token_id, currency, max_tokens)
            .map_err(|e| {
                Error::Call(CallError::Custom(ErrorObject::owned(
                    INTERNAL_ERROR_CODE,
                    "Unable to dry-run burn.",
                    Some(format!("{:?}", e)),
                )))
            })?;

        res.map_err(|e| {
            Error::Call(CallError::Custom(ErrorObject::owned(
                INTERNAL_ERROR_CODE,
                "Unable to dry-run burn.",
                Some(format!("{:?}", e)),
            )))
        })
    }

    fn dryly_remove_liquidity(
        &self,
        lp_token_id: AssetId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<(
        AssetId,
        BalanceWrapper<Balance>,
        BalanceWrapper<Balance>,
        BalanceWrapper<Balance>,
    )> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let res = api.dryly_remove_liquidity(&at, lp_token_id).map_err(|e| {
            Error::Call(CallError::Custom(ErrorObject::owned(
                INTERNAL_ERROR_CODE,
                "Unable to dry-run burn.",
                Some(format!("{:?}", e)),
            )))
        })?;

        res.map_err(|e| {
            Error::Call(CallError::Custom(ErrorObject::owned(
                INTERNAL_ERROR_CODE,
                "Unable to dry-run burn.",
                Some(format!("{:?}", e)),
            )))
        })
    }

    fn dryly_buy_tokens(
        &self,
        token_id: AssetId,
        tokens: BalanceWrapper<Balance>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let res = api.dryly_buy_tokens(&at, token_id, tokens).map_err(|e| {
            Error::Call(CallError::Custom(ErrorObject::owned(
                INTERNAL_ERROR_CODE,
                "Unable to dry-run token_out.",
                Some(format!("{:?}", e)),
            )))
        })?;

        res.map_err(|e| {
            Error::Call(CallError::Custom(ErrorObject::owned(
                INTERNAL_ERROR_CODE,
                "Unable to dry-run token_out.",
                Some(format!("{:?}", e)),
            )))
        })
    }

    fn dryly_sell_tokens(
        &self,
        token_id: AssetId,
        tokens: BalanceWrapper<Balance>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let res = api.dryly_sell_tokens(&at, token_id, tokens).map_err(|e| {
            Error::Call(CallError::Custom(ErrorObject::owned(
                INTERNAL_ERROR_CODE,
                "Unable to dry-run token_in.",
                Some(format!("{:?}", e)),
            )))
        })?;

        res.map_err(|e| {
            Error::Call(CallError::Custom(ErrorObject::owned(
                INTERNAL_ERROR_CODE,
                "Unable to dry-run token_in.",
                Some(format!("{:?}", e)),
            )))
        })
    }

    fn dryly_sell_currency(
        &self,
        token_id: AssetId,
        currency: BalanceWrapper<Balance>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let res = api
            .dryly_sell_currency(&at, token_id, currency)
            .map_err(|e| {
                Error::Call(CallError::Custom(ErrorObject::owned(
                    INTERNAL_ERROR_CODE,
                    "Unable to dry-run quote_in.",
                    Some(format!("{:?}", e)),
                )))
            })?;

        res.map_err(|e| {
            Error::Call(CallError::Custom(ErrorObject::owned(
                INTERNAL_ERROR_CODE,
                "Unable to dry-run token_in.",
                Some(format!("{:?}", e)),
            )))
        })
    }

    fn dryly_buy_currency(
        &self,
        token_id: AssetId,
        currency: BalanceWrapper<Balance>,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let res = api
            .dryly_buy_currency(&at, token_id, currency)
            .map_err(|e| {
                Error::Call(CallError::Custom(ErrorObject::owned(
                    INTERNAL_ERROR_CODE,
                    "Unable to dry-run quote_out.",
                    Some(format!("{:?}", e)),
                )))
            })?;

        res.map_err(|e| {
            Error::Call(CallError::Custom(ErrorObject::owned(
                INTERNAL_ERROR_CODE,
                "Unable to dry-run quote_out.",
                Some(format!("{:?}", e)),
            )))
        })
    }

    fn calculate_reward(
        &self,
        lp_token_id: AssetId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> RpcResult<BalanceWrapper<Balance>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

        let res = api.calculate_reward(&at, lp_token_id).map_err(|e| {
            Error::Call(CallError::Custom(ErrorObject::owned(
                INTERNAL_ERROR_CODE,
                "Unable to calculate reward.",
                Some(format!("{:?}", e)),
            )))
        })?;

        res.map_err(|e| {
            Error::Call(CallError::Custom(ErrorObject::owned(
                INTERNAL_ERROR_CODE,
                "Unable to calculate reward.",
                Some(format!("{:?}", e)),
            )))
        })
    }
}
