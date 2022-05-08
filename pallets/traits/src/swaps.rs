use codec::MaxEncodedLen;
use frame_support::{dispatch::DispatchResult, traits::tokens::Balance, Parameter};
use sp_runtime::{
    traits::{
        AtLeast32BitUnsigned, Bounded, MaybeSerializeDeserialize, Member, UniqueSaturatedInto,
    },
    DispatchError,
};
use sp_std::boxed::Box;

pub trait Swaps<AccountId> {
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
    fn iter() -> Box<dyn Iterator<Item = Self::AssetId>>;

    /// Iterate over the liquidity providers
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    fn iter_providers(token_id: Self::AssetId) -> Box<dyn Iterator<Item = AccountId>>;

    /// Get total liquidity for a given pair
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    ///
    /// # Returns
    ///
    /// total liquidity tokens issued for the pair
    fn total_liquidity(token_id: Self::AssetId) -> Self::TokenBalance;

    /// Get total liquidity for a given pair provided by a given account
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `who` - The account ID
    ///
    /// # Returns
    ///
    /// total liquidity tokens the account holds for the pair
    fn liquidity(token_id: Self::AssetId, who: &AccountId) -> Self::TokenBalance;

    /// Create new swap pair
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    ///
    /// # Returns
    ///
    /// wether the swap pair was created or not
    fn new(token_id: Self::AssetId) -> DispatchResult;

    /// Get pot account ID for a given pair
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    ///
    /// # Returns
    ///
    /// account ID of the pot account
    fn get_pool_account(token_id: Self::AssetId) -> AccountId;

    /// Get dry-run result of mint
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `currency` - The currency to be involved in the swap
    /// * `max_tokens` - The maximum amount of tokens to be involved in the swap
    ///
    /// # Returns
    ///
    /// tuple of (tokens, liquidity)
    ///
    /// * `tokens` - The amount of tokens to be involved in the swap
    /// * `liquidity` - The amount of liquidity to be minted
    fn mint_dry(
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        max_tokens: Self::TokenBalance,
    ) -> Result<(Self::TokenBalance, Self::TokenBalance), DispatchError>;

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
    /// # Returns
    ///
    /// tuple of (tokens, liquidity)
    ///
    /// * `tokens` - The amount of tokens involved
    /// * `liquidity` - The amount of liquidity minted
    fn mint(
        who: AccountId,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        min_liquidity: Self::TokenBalance,
        max_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> Result<(Self::TokenBalance, Self::TokenBalance), DispatchError>;

    /// Get dry-run result of burn
    ///
    /// # Arguments
    ///
    /// * `lp_token_id` - The Liquidity Provider Token ID
    ///
    /// # Returns
    ///
    /// tuple of (token_id, liquidity, tokens, currency)
    ///
    /// * `token_id` - The Asset ID
    /// * `liquidity` - The amount of liquidity removed
    /// * `tokens` - The amount of tokens returned
    /// * `currency` - The currency returned
    fn burn_dry(
        lp_token_id: Self::AssetId,
    ) -> Result<
        (
            Self::AssetId,
            Self::TokenBalance,
            Self::TokenBalance,
            Self::QuoteBalance,
        ),
        DispatchError,
    >;

    /// Remove Liquidity
    ///
    /// * `who` - The account ID of the operator
    /// * `lp_token_id` - The Liquidity Provider Token ID
    /// * `min_currency` - The minimum currency to be returned
    /// * `min_tokens` - The minimum amount of tokens to be returned
    ///
    /// # Returns
    ///
    /// tuple of (token_id, liquidity, tokens, currency)
    ///
    /// * `token_id` - The Asset ID
    /// * `liquidity` - The amount of liquidity removed
    /// * `tokens` - The amount of tokens returned
    /// * `currency` - The currency returned
    fn burn(
        who: AccountId,
        lp_token_id: Self::AssetId,
        min_currency: Self::QuoteBalance,
        min_tokens: Self::TokenBalance,
    ) -> Result<
        (
            Self::AssetId,
            Self::TokenBalance,
            Self::TokenBalance,
            Self::QuoteBalance,
        ),
        DispatchError,
    >;

    /// Get dry-run result of token_out
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `tokens` - The amount of tokens to be bought
    ///
    /// # Returns
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
    /// # Returns
    ///
    /// The currency spent
    fn token_out(
        who: AccountId,
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
        max_currency: Self::QuoteBalance,
        keep_alive: bool,
    ) -> Result<Self::QuoteBalance, DispatchError>;

    /// Get dry-run result of token_in
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `tokens` - The amount of tokens to be sold
    ///
    /// # Returns
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
    /// # Returns
    ///
    /// The currency gained
    fn token_in(
        who: AccountId,
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
        min_currency: Self::QuoteBalance,
        keep_alive: bool,
    ) -> Result<Self::QuoteBalance, DispatchError>;

    /// Get dry-run result of quote_in
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `currency` - The currency to be sold
    ///
    /// # Returns
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
    /// # Returns
    ///
    /// The amount of tokens gained
    fn quote_in(
        who: AccountId,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        min_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> Result<Self::TokenBalance, DispatchError>;

    /// Get dry-run result of quote_out
    ///
    /// # Arguments
    ///
    /// * `token_id` - The Asset ID
    /// * `currency` - The currency to be bought
    ///
    /// # Returns
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
    /// # Returns
    ///
    /// The amount of tokens spent
    fn quote_out(
        who: AccountId,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        max_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> Result<Self::TokenBalance, DispatchError>;
}
