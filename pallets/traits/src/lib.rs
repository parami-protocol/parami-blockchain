#![cfg_attr(not(feature = "std"), no_std)]

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

    type AssetId: Parameter + Member + AtLeast32BitUnsigned + Default + Bounded + Copy;

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

    /// mint LP token
    fn mint(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        min_liquidity: Self::TokenBalance,
        max_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError>;

    /// burn LP token
    fn burn(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        liquidity: Self::TokenBalance,
        min_currency: Self::QuoteBalance,
        min_tokens: Self::TokenBalance,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError>;

    /// buy tokens
    fn token_out(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
        max_currency: Self::QuoteBalance,
        keep_alive: bool,
    ) -> Result<(Self::TokenBalance, Self::QuoteBalance), DispatchError>;

    /// sell tokens
    fn token_in(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        tokens: Self::TokenBalance,
        min_currency: Self::QuoteBalance,
        keep_alive: bool,
    ) -> Result<(Self::TokenBalance, Self::QuoteBalance), DispatchError>;

    /// sell currency (buy tokens)
    fn quote_in(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        min_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError>;

    /// buy currency (sell tokens)
    fn quote_out(
        who: &Self::AccountId,
        token_id: Self::AssetId,
        currency: Self::QuoteBalance,
        max_tokens: Self::TokenBalance,
        keep_alive: bool,
    ) -> Result<(Self::QuoteBalance, Self::TokenBalance), DispatchError>;

    fn price_buy(
        output_amount: Self::TokenBalance,
        input_reserve: Self::TokenBalance,
        output_reserve: Self::TokenBalance,
    ) -> Self::TokenBalance;

    fn price_sell(
        input_amount: Self::TokenBalance,
        input_reserve: Self::TokenBalance,
        output_reserve: Self::TokenBalance,
    ) -> Self::TokenBalance;
}
