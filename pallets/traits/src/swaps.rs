use codec::MaxEncodedLen;
use frame_support::{traits::tokens::Balance, Parameter};
use sp_runtime::{
    traits::{
        AtLeast32BitUnsigned, Bounded, MaybeSerializeDeserialize, Member, UniqueSaturatedInto,
    },
    DispatchError,
};

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

    /// create new swap pair
    fn new(who: &Self::AccountId, token_id: Self::AssetId) -> Result<Self::AssetId, DispatchError>;

    /// dry-run of mint
    /// returns: (token_id, token, lp_token_id, liquidity)
    fn mint_dry(
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        max_tokens: Self::TokenBalance,
    ) -> Result<
        (
            Self::AssetId,      // token_id
            Self::TokenBalance, // token
            Self::AssetId,      // lp_token_id
            Self::TokenBalance, // liquidity
        ),
        DispatchError,
    >;

    /// mint LP token
    fn mint(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        min_liquidity: Self::TokenBalance,
        max_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError>;

    /// dry-run of burn
    /// returns: (token_id, token, lp_token_id, currency)
    fn burn_dry(
        token_id: Self::AssetId,
        liquidity: Self::TokenBalance,
    ) -> Result<
        (
            Self::AssetId,      // token_id
            Self::TokenBalance, // token
            Self::AssetId,      // lp_token_id
            Self::QuoteBalance, // currency
        ),
        DispatchError,
    >;

    /// burn LP token
    fn burn(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        liquidity: Self::TokenBalance,
        min_currency: Self::QuoteBalance,
        min_tokens: Self::TokenBalance,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError>;

    /// dry-run of token_out
    /// returns: (currency, pot)
    fn token_out_dry(
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
    ) -> Result<Self::QuoteBalance, DispatchError>;

    /// buy tokens
    fn token_out(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
        max_currency: Self::QuoteBalance,
        keep_alive: bool,
    ) -> Result<(Self::TokenBalance, Self::QuoteBalance), DispatchError>;

    /// dry-run of token_in
    /// returns: (currency, pot)
    fn token_in_dry(
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
    ) -> Result<Self::QuoteBalance, DispatchError>;

    /// sell tokens
    fn token_in(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
        min_currency: Self::QuoteBalance,
        keep_alive: bool,
    ) -> Result<(Self::TokenBalance, Self::QuoteBalance), DispatchError>;

    /// dry-run of quote_in
    /// returns: (tokens, pot)
    fn quote_in_dry(
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
    ) -> Result<Self::TokenBalance, DispatchError>;

    /// sell currency (buy tokens)
    fn quote_in(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        min_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError>;

    /// dry-run of quote_out
    /// returns: (tokens, pot)
    fn quote_out_dry(
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
    ) -> Result<Self::TokenBalance, DispatchError>;

    /// buy currency (sell tokens)
    fn quote_out(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        max_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError>;
}
