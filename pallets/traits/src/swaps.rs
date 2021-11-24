use codec::MaxEncodedLen;
use frame_support::{traits::tokens::Balance, Parameter};
use sp_runtime::{
    traits::{
        AtLeast32BitUnsigned, Bounded, MaybeSerializeDeserialize, Member, UniqueSaturatedInto,
    },
    DispatchError,
};
use sp_std::boxed::Box;

pub trait Swaps {
    type AccountId: Parameter + Member + MaybeSerializeDeserialize + Ord + Default + MaxEncodedLen;

    type AssetId: Parameter
        + Member
        + MaybeSerializeDeserialize
        + AtLeast32BitUnsigned
        + Default
        + Bounded
        + Copy;

    type QuoteBalance: Balance
        + MaybeSerializeDeserialize
        + MaxEncodedLen
        + UniqueSaturatedInto<Self::TokenBalance>;

    type TokenBalance: Balance
        + MaybeSerializeDeserialize
        + MaxEncodedLen
        + UniqueSaturatedInto<Self::QuoteBalance>;

    /// Iterate over the swaps
    fn iter() -> Box<dyn Iterator<Item = (Self::AssetId, Self::AssetId, Self::AccountId)>>;

    /// Iterate over the holders
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    fn iter_holder(token_id: Self::AssetId) -> Box<dyn Iterator<Item = Self::AccountId>>;

    /// Create new swap pair
    ///
    /// # Arguments
    ///
    /// * `who` - The account ID of the operator
    /// * `token_id` - The Asset ID
    ///
    /// # Results
    ///
    /// The ID of the new swap pair
    fn new(who: &Self::AccountId, token_id: Self::AssetId) -> Result<Self::AssetId, DispatchError>;

    /// Get dry-run result of mint
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `currency` - The currency to be involved in the swap
    /// * `max_tokens` - The maximum amount of tokens to be involved in the swap
    ///
    /// # Results
    ///
    /// tuple of (token_id, tokens, lp_token_id, liquidity)
    ///
    /// * `token_id` - The Asset ID
    /// * `tokens` - The amount of tokens to be involved in the swap
    /// * `lp_token_id` - The Asset ID of the liquidity provider token
    /// * `liquidity` - The amount of liquidity to be minted
    fn mint_dry(
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        max_tokens: Self::TokenBalance,
    ) -> Result<
        (
            Self::AssetId,
            Self::TokenBalance,
            Self::AssetId,
            Self::TokenBalance,
        ),
        DispatchError,
    >;

    /// Add Liquidity
    ///
    /// # Arguments
    ///
    /// * `who` - The account ID of the operator
    /// * `token_id` - The Asset ID
    /// * `currency` - The currency to be involved in the swap
    /// * `min_liquidity` - The minimum amount of liquidity to be minted
    /// * `max_tokens` - The maximum amount of tokens to be involved in the swap
    /// * `keep_alive` - Whether to keep the account alive
    ///
    /// # Results
    ///
    /// tuple of (currency, tokens)
    ///
    /// * `currency` - The currency involved
    /// * `tokens` - The amount of tokens involved
    fn mint(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        min_liquidity: Self::TokenBalance,
        max_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError>;

    /// Get dry-run result of burn
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `liquidity` - The amount of liquidity to be removed
    ///
    /// # Results
    ///
    /// tuple of (token_id, tokens, lp_token_id, currency)
    ///
    /// * `token_id` - The Asset ID
    /// * `tokens` - The amount of tokens to be returned
    /// * `lp_token_id` - The Asset ID of the liquidity provider token
    /// * `currency` - The currency to be returned
    fn burn_dry(
        token_id: Self::AssetId,
        liquidity: Self::TokenBalance,
    ) -> Result<
        (
            Self::AssetId,
            Self::TokenBalance,
            Self::AssetId,
            Self::QuoteBalance,
        ),
        DispatchError,
    >;

    /// Remove Liquidity
    ///
    /// * `who` - The account ID of the operator
    /// * `token_id` - The Asset ID
    /// * `liquidity` - The amount of liquidity to be removed
    /// * `min_currency` - The minimum currency to be returned
    /// * `min_tokens` - The minimum amount of tokens to be returned
    ///
    /// # Results
    ///
    /// tuple of (currency, tokens)
    ///
    /// * `currency` - The currency returned
    /// * `tokens` - The amount of tokens returned
    fn burn(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        liquidity: Self::TokenBalance,
        min_currency: Self::QuoteBalance,
        min_tokens: Self::TokenBalance,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError>;

    /// Get dry-run result of token_out
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `tokens` - The amount of tokens to be bought
    ///
    /// # Results
    ///
    /// The currency needed
    fn token_out_dry(
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
    ) -> Result<Self::QuoteBalance, DispatchError>;

    /// Buy tokens
    ///
    /// * `who` - The account ID of the operator
    /// * `token_id` - The Asset ID
    /// * `tokens` - The amount of tokens to be bought
    /// * `max_currency` - The maximum currency to be spent
    /// * `keep_alive` - Whether to keep the account alive
    ///
    /// # Results
    ///
    /// tuple of (tokens, currency)
    ///
    /// * `tokens` - The amount of tokens bought
    /// * `currency` - The currency spent
    fn token_out(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
        max_currency: Self::QuoteBalance,
        keep_alive: bool,
    ) -> Result<(Self::TokenBalance, Self::QuoteBalance), DispatchError>;

    /// Get dry-run result of token_in
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `tokens` - The amount of tokens to be sold
    ///
    /// # Results
    ///
    /// The currency to be gained
    fn token_in_dry(
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
    ) -> Result<Self::QuoteBalance, DispatchError>;

    /// Sell tokens
    ///
    /// * `who` - The account ID of the operator
    /// * `token_id` - The Asset ID
    /// * `tokens` - The amount of tokens to be sold
    /// * `min_currency` - The maximum currency to be gained
    /// * `keep_alive` - Whether to keep the account alive
    ///
    /// # Results
    ///
    /// tuple of (tokens, currency)
    ///
    /// * `tokens` - The amount of tokens sold
    /// * `currency` - The currency gained
    fn token_in(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
        min_currency: Self::QuoteBalance,
        keep_alive: bool,
    ) -> Result<(Self::TokenBalance, Self::QuoteBalance), DispatchError>;

    /// Get dry-run result of quote_in
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `currency` - The currency to be sold
    ///
    /// # Results
    ///
    /// The amount of tokens to be gained
    fn quote_in_dry(
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
    ) -> Result<Self::TokenBalance, DispatchError>;

    /// Sell currency (buy tokens)
    ///
    /// * `who` - The account ID of the operator
    /// * `token_id` - The Asset ID
    /// * `currency` - The currency to be sold
    /// * `min_tokens` - The minimum amount of tokens to be gained
    /// * `keep_alive` - Whether to keep the account alive
    ///
    /// # Results
    ///
    /// tuple of (currency, tokens)
    ///
    /// * `currency` - The currency sold
    /// * `tokens` - The amount of tokens gained
    fn quote_in(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        min_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError>;

    /// Get dry-run result of quote_out
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `currency` - The currency to be bought
    ///
    /// # Results
    ///
    /// The amount of tokens needed
    fn quote_out_dry(
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
    ) -> Result<Self::TokenBalance, DispatchError>;

    /// Buy currency (sell tokens)
    ///
    /// * `who` - The account ID of the operator
    /// * `token_id` - The Asset ID
    /// * `currency` - The currency to be bought
    /// * `max_tokens` - The maximum amount of tokens to be spent
    /// * `keep_alive` - Whether to keep the account alive
    ///
    /// # Results
    ///
    /// tuple of (currency, tokens)
    ///
    /// * `currency` - The currency bought
    /// * `tokens` - The amount of tokens spent
    fn quote_out(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        max_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError>;
}
