//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use parami_dana_runtime::{
    opaque::Block, AccountId, AssetId, Balance, BlockNumber, DecentralizedId, Hash, Index as Nonce,
};
use sc_finality_grandpa::{
    FinalityProofProvider, GrandpaJustificationStream, SharedAuthoritySet, SharedVoterState,
};
use sc_finality_grandpa_rpc::GrandpaRpcHandler;
use sc_rpc::SubscriptionTaskExecutor;
pub use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};

/// Extra dependencies for GRANDPA
pub struct GrandpaDeps<B> {
    /// Voting round info.
    pub shared_voter_state: SharedVoterState,
    /// Authority set info.
    pub shared_authority_set: SharedAuthoritySet<Hash, BlockNumber>,
    /// Receives notifications about justification events from Grandpa.
    pub justification_stream: GrandpaJustificationStream<Block>,
    /// Executor to drive the subscription manager in the Grandpa RPC handler.
    pub subscription_executor: SubscriptionTaskExecutor,
    /// Finality proof provider.
    pub finality_provider: Arc<FinalityProofProvider<B, Block>>,
}

/// Full client dependencies.
pub struct FullDeps<C, P, B> {
    /// The backend instance to use.
    pub backend: Arc<B>,
    /// The client instance to use.
    pub client: Arc<C>,
    /// Transaction pool instance.
    pub pool: Arc<P>,
    /// Whether to deny unsafe calls
    pub deny_unsafe: DenyUnsafe,
    /// GRANDPA specific dependencies.
    pub grandpa: GrandpaDeps<B>,
}

/// Instantiate all full RPC extensions.
pub fn create_full<C, P, B>(deps: FullDeps<C, P, B>) -> jsonrpc_core::IoHandler<sc_rpc::Metadata>
where
    C: ProvideRuntimeApi<Block>,
    C: HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockChainError> + 'static,
    C: Send + Sync + 'static,
    C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
    C::Api: pallet_contracts_rpc::ContractsRuntimeApi<Block, AccountId, Balance, BlockNumber, Hash>,
    C::Api: pallet_mmr_rpc::MmrRuntimeApi<Block, <Block as sp_runtime::traits::Block>::Hash>,
    C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
    C::Api: parami_swap_rpc::SwapRuntimeApi<Block, AssetId, Balance>,
    C::Api: BlockBuilder<Block>,
    P: TransactionPool + 'static,
    B: sc_client_api::Backend<Block> + Send + Sync + 'static,
    B::State: sc_client_api::backend::StateBackend<sp_runtime::traits::HashFor<Block>>,
{
    use pallet_contracts_rpc::{Contracts, ContractsApi};
    use pallet_mmr_rpc::{Mmr, MmrApi};
    use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApi};
    use parami_did_rpc::{DidApi, DidRpcHandler};
    use parami_swap_rpc::{SwapApi, SwapsRpcHandler};
    use substrate_frame_rpc_system::{FullSystem, SystemApi};

    let mut io = jsonrpc_core::IoHandler::default();
    let FullDeps {
        backend,
        client,
        pool,
        deny_unsafe,
        grandpa,
    } = deps;

    let GrandpaDeps {
        shared_voter_state,
        shared_authority_set,
        justification_stream,
        subscription_executor,
        finality_provider,
    } = grandpa;

    io.extend_with(SystemApi::to_delegate(FullSystem::new(
        client.clone(),
        pool,
        deny_unsafe,
    )));
    io.extend_with(ContractsApi::to_delegate(Contracts::new(client.clone())));
    io.extend_with(MmrApi::to_delegate(Mmr::new(client.clone())));
    io.extend_with(TransactionPaymentApi::to_delegate(TransactionPayment::new(
        client.clone(),
    )));

    io.extend_with(sc_finality_grandpa_rpc::GrandpaApi::to_delegate(
        GrandpaRpcHandler::new(
            shared_authority_set,
            shared_voter_state,
            justification_stream,
            subscription_executor,
            finality_provider,
        ),
    ));

    if let Some(did_rpc) = backend
        .offchain_storage()
        .map(|storage| DidApi::<DecentralizedId>::to_delegate(DidRpcHandler::new(storage)))
    {
        io.extend_with(did_rpc);
    }
    io.extend_with(SwapApi::to_delegate(SwapsRpcHandler::new(client.clone())));
    io
}
