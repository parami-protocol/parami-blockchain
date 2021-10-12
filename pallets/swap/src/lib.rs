#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use codec::{Decode, Encode, HasCompact};
use frame_support::{
    dispatch::DispatchResult,
    traits::{
        tokens::fungibles::{Inspect, Mutate, Transfer},
        Currency, ExistenceRequirement,
    },
    PalletId, RuntimeDebug,
};
use scale_info::TypeInfo;
use sp_runtime::traits::{
    AccountIdConversion, AtLeast32BitUnsigned, IntegerSquareRoot, Saturating, StaticLookup, Zero,
};

const PALLET_ID: PalletId = PalletId(*b"paraswap");
const MINIMUM_LIQUIDITY: u128 = 1_000;

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
pub struct SwapPair<AccountId> {
    account: AccountId,
    // reserveA
    native_reserve: u128,
    // reserveB
    asset_reserve: u128,
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config:
        frame_system::Config + pallet_assets::Config + pallet_balances::Config
    {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The currency trait.
        type Currency: Currency<Self::AccountId>;

        // Go to hell, your types sucks
        type NativeBalance: IsType<<Self as pallet_balances::Config>::Balance>
            + Parameter
            + Member
            + Copy
            + AtLeast32BitUnsigned
            + HasCompact
            + IsType<u128>
            + IsType<<<Self as Config>::Currency as Currency<<Self as frame_system::Config>::AccountId>>::Balance>;
        type SwapAssetBalance: IsType<<Self as pallet_assets::Config>::Balance>
            + Parameter
            + Member
            + Copy
            + AtLeast32BitUnsigned
            + HasCompact
            + IsType<u128>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::storage]
    pub type Swap<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, SwapPair<T::AccountId>, OptionQuery>;

    // (asset-id, account-id) => amount of LP-token
    // serve as the fake lp-token balance
    #[pallet::storage]
    pub type LiquidityProvider<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::AssetId,
        Blake2_128Concat,
        T::AccountId,
        u128,
        ValueQuery,
    >;

    /*
    #[pallet::storage]
    #[pallet::getter(fn total_liquidity_of)]
    pub type TotalLiquidity<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AssetId, T::Balance, ValueQuery>;
     */

    #[pallet::event]
    #[pallet::generate_deposit(pub fn deposit_event)]
    pub enum Event<T: Config> {
        /// New swap pair created
        Created(T::AssetId),
        /// Add liquidity
        LiquidityAdded(T::AccountId, T::AssetId),
        /// Remove liquidity
        LiquidityRemoved(T::AccountId, T::AssetId),
        /// Buy tokens
        SwapBuy(T::AccountId, T::AssetId, T::SwapAssetBalance),
        /// Sell tokens
        SwapSell(T::AccountId, T::AssetId, T::NativeBalance),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        Exists,
        AssetNotFound,
        SwapNotFound,
        NativeAmountRequired,
        AssetAmountRequired,
        Overflow,
        /// remaining token/native token balance is too low
        InsufficientLiquidity,
        InsufficientNativeToken,
        InsufficientAssetToken,
        InsufficientNativeBalance,
        InsufficientAssetBalance,
        NativeAmountIsZero,
        AssetAmountIsZero,
        MintedLiquidityIsZero,
        LiquidityAmountIsZero,

        InsufficientPoolAssetAmount,
        InsufficientPoolNativeAmount,
        /// Minimum liquidity for add liquidity first time.
        MinimumLiquidity,
        /// INSUFFICIENT_INPUT_AMOUNT
        InsufficientInputAmount,
        InsufficientOutputAmount,
        /// INSUFFICIENT_LIQUIDITY_BURNED
        InsufficientLiquidityBurned,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T>
    where
        <<T as Config>::Currency as frame_support::traits::Currency<
            <T as frame_system::Config>::AccountId,
        >>::Balance: From<u128> + Into<u128>,
        <T as pallet_assets::Config>::Balance: From<u128> + Into<u128>,
        <T as pallet_assets::Config>::AssetId: AtLeast32BitUnsigned,
    {
        #[pallet::weight(0)]
        pub fn create(
            origin: OriginFor<T>,
            #[pallet::compact] asset_id: T::AssetId,
        ) -> DispatchResult {
            let who = ensure_signed(origin.clone())?;

            let sender = who;

            // constrait on asset_id
            ensure!(
                asset_id < T::AssetId::from(1_000_000_u32),
                Error::<T>::AssetNotFound
            );

            let total_supply = <pallet_assets::Pallet<T>>::total_supply(asset_id);
            log::info!("!! create total supply => {:?}", total_supply);
            ensure!(total_supply > Zero::zero(), Error::<T>::AssetNotFound);

            log::debug!(
                "!! min balance => {:?}",
                <pallet_assets::Pallet<T>>::minimum_balance(asset_id)
            );

            ensure!(!Swap::<T>::contains_key(asset_id), Error::<T>::Exists);

            // create pool account
            let pool_account_id = Self::asset_account_id(asset_id);
            <T as Config>::Currency::transfer(
                &sender,
                &pool_account_id,
                <T as Config>::Currency::minimum_balance(),
                ExistenceRequirement::KeepAlive,
            )?;

            <pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
                asset_id,
                &sender,
                &pool_account_id,
                <pallet_assets::Pallet<T>>::minimum_balance(asset_id),
                true,
            )?;

            // create lp-token
            // offset asset_id by 1_000_000, constraint, won't save
            let lp_asset_id = asset_id + T::AssetId::from(1_000_000_u32);
            <pallet_assets::Pallet<T>>::create(
                origin.clone(),
                lp_asset_id,
                <T::Lookup as StaticLookup>::unlookup(pool_account_id.clone()),
                MINIMUM_LIQUIDITY.into(),
            )?;

            // TODO: get symbols from assets
            // blocked by paritytech/substrate#9757

            // let (_, asset_symbol, _) = <pallet_assets::Pallet<T>>::metadata(asset_id);
            // let mut lp_name = asset_symbol.clone();
            // lp_name.extend_from_slice(b"/AD3 LP");
            // let mut lp_symbol = asset_symbol.clone();
            // lp_symbol.extend_from_slice(b"-AD3");

            log::info!("issure LP token {:?}", asset_id);

            <pallet_assets::Pallet<T>>::set_metadata(
                origin.clone(),
                lp_asset_id,
                "AD3 LP".into(),
                "LP".into(),
                0,
            )?;
            <pallet_assets::Pallet<T>>::transfer_ownership(
                origin.clone(),
                lp_asset_id,
                <T::Lookup as StaticLookup>::unlookup(pool_account_id),
            )?;

            Swap::<T>::insert(
                asset_id,
                SwapPair {
                    account: Self::asset_account_id(asset_id),
                    native_reserve: 0,
                    asset_reserve: 0,
                },
            );

            // TotalLiquidity::insert(asset_id, Zero::zero());
            Self::deposit_event(Event::Created(asset_id));

            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn add_liquidity(
            origin: OriginFor<T>,
            #[pallet::compact] asset_id: T::AssetId,
            #[pallet::compact] native_amount: T::NativeBalance,
            maybe_asset_amount: Option<T::SwapAssetBalance>,
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;

            let sender = origin;
            let mut pair = Swap::<T>::get(asset_id).ok_or(Error::<T>::SwapNotFound)?;

            log::info!("trade pair => {:?}", pair);
            let lp_asset_id = asset_id + T::AssetId::from(1_000_000_u32);

            // sender's native balance check
            ensure!(native_amount > Zero::zero(), Error::<T>::NativeAmountIsZero);
            ensure!(
                <pallet_balances::Pallet<T> as Currency<_>>::free_balance(&sender)
                    > native_amount.into(),
                Error::<T>::InsufficientNativeBalance
            );

            let (pool_account_id, pool_native_amount) = Self::native_pool(asset_id);
            let (_, pool_asset_amount) = Self::asset_pool(asset_id);

            // unit conversion
            let native_amount: u128 = native_amount.into();
            let asset_amount: u128;
            let minted_liquidity;

            log::info!(
                "add liquidity native={:?} asset={:?}",
                native_amount,
                maybe_asset_amount
            );

            if pair.native_reserve == 0 && pair.asset_reserve == 0 {
                // initial add liquidity
                ensure!(
                    maybe_asset_amount.is_some(),
                    Error::<T>::AssetAmountRequired
                );
                asset_amount = maybe_asset_amount.unwrap().into();
                // MINIMUM_LIQUIDITY = 1_000
                ensure!(
                    asset_amount > MINIMUM_LIQUIDITY,
                    Error::<T>::MinimumLiquidity
                );
                ensure!(
                    native_amount > MINIMUM_LIQUIDITY,
                    Error::<T>::MinimumLiquidity
                );

                minted_liquidity =
                    (asset_amount * native_amount).integer_sqrt() - MINIMUM_LIQUIDITY;
            } else {
                let pool_asset_amount: u128 = pool_asset_amount.into();
                asset_amount = pool_asset_amount
                    .checked_mul(native_amount)
                    .and_then(|v| v.checked_div(pool_native_amount.into()))
                    .ok_or(Error::<T>::Overflow)?;

                if let Some(provide_amount) = maybe_asset_amount {
                    ensure!(
                        provide_amount >= asset_amount.into(),
                        Error::<T>::InsufficientAssetToken
                    );
                }

                // total liquidity
                let total_supply: u128 =
                    <pallet_assets::Pallet<T>>::total_supply(lp_asset_id).into();

                minted_liquidity = u128::min(
                    native_amount * total_supply / pair.native_reserve,
                    asset_amount * total_supply / pair.asset_reserve,
                );

                log::info!(
                    "calculated asset={:?} liquidity={:?}",
                    asset_amount,
                    minted_liquidity
                );
            }

            ensure!(asset_amount > Zero::zero(), Error::<T>::AssetAmountIsZero);
            ensure!(
                minted_liquidity > Zero::zero(),
                Error::<T>::MintedLiquidityIsZero
            );

            // Equation:
            // pool_native_amount *asset_amount == pool_asset_amount * native_amount

            // sender asset amount check
            {
                let sender_asset_balance: T::SwapAssetBalance = T::SwapAssetBalance::from(
                    <pallet_assets::Pallet<T>>::balance(asset_id, &sender),
                );
                ensure!(
                    sender_asset_balance > asset_amount.into(),
                    Error::<T>::InsufficientAssetBalance
                );
            }

            // TODO: liquidity token check?

            // native token inject
            <T as Config>::Currency::transfer(
                &sender,
                &pool_account_id,
                native_amount.into(),
                ExistenceRequirement::KeepAlive,
            )?;
            // asset token inject
            <pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
                asset_id,
                &sender,
                &pool_account_id,
                asset_amount.into(),
                true,
            )?;

            // LP handling
            LiquidityProvider::<T>::try_mutate_exists(
                asset_id,
                sender.clone(),
                |maybe_lp| -> DispatchResult {
                    let mut lp = maybe_lp.take().unwrap_or_default();
                    lp = lp
                        .checked_add(minted_liquidity)
                        .ok_or(Error::<T>::Overflow)?;
                    *maybe_lp = Some(lp);
                    Ok(())
                },
            )?;
            // mint LP token, won't trigger event
            <pallet_assets::Pallet<T>>::mint_into(lp_asset_id, &sender, minted_liquidity.into())?;

            pair.native_reserve = Self::native_pool(asset_id).1.into();
            pair.asset_reserve = Self::asset_pool(asset_id).1.into();
            /*
            pair.issued_liquidity = pair
                .issued_liquidity
                .checked_add(minted_liquidity)
                .ok_or(Error::<T>::Overflow)?;
                */
            Swap::<T>::insert(asset_id, pair);

            Self::deposit_event(Event::LiquidityAdded(sender, asset_id));

            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn remove_liquidity(
            origin: OriginFor<T>,
            #[pallet::compact] asset_id: T::AssetId,
            #[pallet::compact] liquidity: T::SwapAssetBalance,
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;

            let sender = origin;
            let liquidity: u128 = liquidity.into();
            let mut pair = Swap::<T>::get(asset_id).ok_or(Error::<T>::SwapNotFound)?;

            let lp_asset_id = asset_id + T::AssetId::from(1_000_000_u32);
            // total liquidity
            let total_supply: u128 = <pallet_assets::Pallet<T>>::total_supply(lp_asset_id).into();

            ensure!(liquidity <= total_supply, Error::<T>::InsufficientLiquidity);
            // FIXME: the error might be inarguments, or pair info
            ensure!(
                liquidity > Zero::zero() && total_supply > 0,
                Error::<T>::LiquidityAmountIsZero
            );

            let (pool_account_id, pool_native_amount) = Self::native_pool(asset_id);
            let (_, pool_asset_amount) = Self::asset_pool(asset_id);

            ensure!(
                pool_native_amount > Zero::zero(),
                Error::<T>::InsufficientPoolNativeAmount
            );
            ensure!(
                pool_asset_amount > Zero::zero(),
                Error::<T>::InsufficientPoolAssetAmount
            );

            let pool_native_amount: u128 = pool_native_amount.into();
            let pool_asset_amount: u128 = pool_asset_amount.into();

            let native_amount = liquidity * pool_native_amount / total_supply;
            let asset_amount = liquidity * pool_asset_amount / total_supply;

            log::info!(
                "remove liquidity, native={:?}, asset={:?}",
                native_amount,
                asset_amount
            );

            ensure!(
                native_amount > Zero::zero() && asset_amount > Zero::zero(),
                Error::<T>::InsufficientLiquidityBurned
            );

            // free balance check
            ensure!(
                pool_native_amount > native_amount,
                Error::<T>::InsufficientPoolNativeAmount
            );
            ensure!(
                pool_asset_amount > asset_amount,
                Error::<T>::InsufficientPoolAssetAmount
            );
            ensure!(
                LiquidityProvider::<T>::get(asset_id, sender.clone()) >= liquidity,
                Error::<T>::InsufficientLiquidity
            );

            // remove liquidity
            <T as Config>::Currency::transfer(
                &pool_account_id,
                &sender,
                native_amount.into(),
                ExistenceRequirement::AllowDeath,
            )?;
            <pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
                asset_id,
                &pool_account_id,
                &sender,
                asset_amount.into(),
                true,
            )?;

            // LP handling
            LiquidityProvider::<T>::try_mutate_exists(
                asset_id,
                sender.clone(),
                |maybe_lp| -> DispatchResult {
                    let mut lp = maybe_lp.take().unwrap_or_default();
                    lp = lp
                        .checked_sub(liquidity.into())
                        .ok_or(Error::<T>::Overflow)?;
                    *maybe_lp = Some(lp);
                    Ok(())
                },
            )?;
            // burn LP token
            <pallet_assets::Pallet<T>>::burn_from(lp_asset_id, &sender, liquidity.into())?;

            // _update
            pair.native_reserve = Self::native_pool(asset_id).1.into();
            pair.asset_reserve = Self::asset_pool(asset_id).1.into();
            // total supply
            /*
            pair.issued_liquidity = pair
                .issued_liquidity
                .checked_sub(liquidity.into())
                .ok_or(Error::<T>::Overflow)?;
            */
            Swap::<T>::insert(asset_id, pair);

            Self::deposit_event(Event::LiquidityRemoved(sender, asset_id));

            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn swap_native(
            origin: OriginFor<T>,
            #[pallet::compact] asset_id: T::AssetId,
            #[pallet::compact] native_amount_in: T::NativeBalance,
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;
            let sender = origin;

            let mut pair = Swap::<T>::get(asset_id).ok_or(Error::<T>::SwapNotFound)?;

            let pool_account_id = Self::asset_account_id(asset_id);

            // UniswapV2Library.getAmountOut
            // given an input amount of an asset and pair reserves, returns the maximum output amount of the other asset
            ensure!(
                native_amount_in > Zero::zero(),
                Error::<T>::InsufficientInputAmount
            );
            ensure!(
                pair.native_reserve > 0 && pair.asset_reserve > 0,
                Error::<T>::InsufficientLiquidity
            );
            let asset_amount_out;
            {
                let native_amount_in: u128 = native_amount_in.into();
                let native_amount_in_with_fee: u128 = native_amount_in
                    .checked_mul(997)
                    .ok_or(Error::<T>::Overflow)?;
                let numerator = native_amount_in_with_fee
                    .checked_mul(pair.asset_reserve)
                    .ok_or(Error::<T>::Overflow)?;
                let denominator = pair.native_reserve * 1000 + native_amount_in_with_fee;
                asset_amount_out = numerator / denominator;
            }

            // check balance
            ensure!(
                <pallet_balances::Pallet<T> as Currency<_>>::free_balance(&sender)
                    > native_amount_in.into(),
                Error::<T>::InsufficientNativeBalance
            );

            // UniswapV2Pair.swap
            ensure!(asset_amount_out > 0, Error::<T>::InsufficientOutputAmount);
            // FIXME: check asset_amount_out > minimum_balance
            ensure!(
                asset_amount_out < pair.asset_reserve,
                Error::<T>::InsufficientLiquidity
            );

            // do transfer
            <T as Config>::Currency::transfer(
                &sender,
                &pool_account_id,
                native_amount_in.into(),
                ExistenceRequirement::KeepAlive,
            )?;
            <pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
                asset_id,
                &pool_account_id,
                &sender,
                asset_amount_out.into(),
                true,
            )?;

            pair.native_reserve = Self::native_pool(asset_id).1.into();
            pair.asset_reserve = Self::asset_pool(asset_id).1.into();
            Swap::<T>::insert(asset_id, pair);

            log::debug!(
                "swap native={:?} for asset={:?}",
                native_amount_in,
                asset_amount_out
            );

            Self::deposit_event(Event::SwapBuy(sender, asset_id, asset_amount_out.into()));
            Ok(().into())
        }

        #[pallet::weight(0)]
        pub fn swap_asset(
            origin: OriginFor<T>,
            #[pallet::compact] asset_id: T::AssetId,
            #[pallet::compact] asset_amount_in: T::SwapAssetBalance,
        ) -> DispatchResult {
            let origin = ensure_signed(origin)?;
            let sender = origin;

            let mut pair = Swap::<T>::get(asset_id).ok_or(Error::<T>::SwapNotFound)?;

            let pool_account_id = Self::asset_account_id(asset_id);

            // UniswapV2Library.getAmountOut
            // given an input amount of an asset and pair reserves, returns the maximum output amount of the other asset
            ensure!(
                asset_amount_in > Zero::zero(),
                Error::<T>::InsufficientInputAmount
            );
            ensure!(
                pair.native_reserve > 0 && pair.asset_reserve > 0,
                Error::<T>::InsufficientLiquidity
            );
            let native_amount_out;
            {
                let asset_amount_in: u128 = asset_amount_in.into();
                let asset_amount_in_with_fee: u128 = asset_amount_in
                    .checked_mul(997)
                    .ok_or(Error::<T>::Overflow)?;
                let numerator = asset_amount_in_with_fee
                    .checked_mul(pair.native_reserve)
                    .ok_or(Error::<T>::Overflow)?;
                let denominator = pair.asset_reserve * 1000 + asset_amount_in_with_fee;
                native_amount_out = numerator / denominator;
            }

            // check balance
            {
                let sender_asset_balance: T::SwapAssetBalance = T::SwapAssetBalance::from(
                    <pallet_assets::Pallet<T>>::balance(asset_id, &sender),
                );
                ensure!(
                    sender_asset_balance > asset_amount_in.into(),
                    Error::<T>::InsufficientAssetBalance
                );
            }

            // UniswapV2Pair.swap
            ensure!(native_amount_out > 0, Error::<T>::InsufficientOutputAmount);
            ensure!(
                native_amount_out < pair.native_reserve,
                Error::<T>::InsufficientLiquidity
            );

            // do transfer
            <pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
                asset_id,
                &sender,
                &pool_account_id,
                asset_amount_in.into(),
                true,
            )?;
            <T as Config>::Currency::transfer(
                &pool_account_id,
                &sender,
                native_amount_out.into(),
                ExistenceRequirement::KeepAlive,
            )?;

            pair.native_reserve = Self::native_pool(asset_id).1.into();
            pair.asset_reserve = Self::asset_pool(asset_id).1.into();
            Swap::<T>::insert(asset_id, pair);

            log::debug!(
                "swap asset={:?} for native={:?}",
                asset_amount_in,
                native_amount_out,
            );

            Self::deposit_event(Event::SwapSell(sender, asset_id, native_amount_out.into()));
            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    /// The account ID of the swap pool.
    ///
    /// This actually does computation. If you need to keep using it, then make sure you cache the
    /// value and only call this once.
    pub fn account_id() -> T::AccountId {
        PALLET_ID.into_account()
    }

    /// Return the pool account and amount of money in the pool.
    // The existential deposit is not part of the pool so airdrop account never gets deleted.
    fn _pool() -> (T::AccountId, T::NativeBalance) {
        let account_id = Self::account_id();
        let balance = <T as Config>::Currency::free_balance(&account_id)
            .saturating_sub(<T as Config>::Currency::minimum_balance());

        (account_id, balance.into())
    }

    pub fn asset_account_id(asset_id: T::AssetId) -> T::AccountId {
        PALLET_ID.into_sub_account(asset_id)
    }

    fn asset_pool(asset_id: T::AssetId) -> (T::AccountId, T::SwapAssetBalance) {
        let account_id = Self::asset_account_id(asset_id);

        let asset_balance = <pallet_assets::Pallet<T>>::balance(asset_id, &account_id)
            - <pallet_assets::Pallet<T>>::minimum_balance(asset_id);
        (account_id, T::SwapAssetBalance::from(asset_balance))
    }

    fn native_pool(asset_id: T::AssetId) -> (T::AccountId, T::NativeBalance) {
        let account_id = Self::asset_account_id(asset_id);

        let balance = <T as Config>::Currency::free_balance(&account_id)
            .saturating_sub(<T as Config>::Currency::minimum_balance());
        (account_id, balance.into())
    }
}

impl<T: Config> Pallet<T>
where
    <<T as Config>::Currency as frame_support::traits::Currency<
        <T as frame_system::Config>::AccountId,
    >>::Balance: From<u128> + Into<u128>,
    <T as pallet_assets::Config>::Balance: From<u128>,
{
}
