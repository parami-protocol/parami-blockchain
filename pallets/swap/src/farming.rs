use crate::{
    Account, AssetOf, BalanceOf, Config, Error, HeightOf, Liquidity, LiquidityOf, Metadata, Pallet,
};

use frame_support::traits::{tokens::fungibles::Inspect, Currency, Get};
use sp_core::U512;
use sp_runtime::{
    traits::{Saturating, Zero},
    DispatchError,
};
use sp_std::marker::PhantomData;

pub trait FarmingCurve<T: Config> {
    /// Calculate the farming value for a given block height
    ///
    /// # Arguments
    ///
    /// * `created_height` - The block number at which the swap was created
    /// * `staked_height` - The block number at which the liquidity was staked
    /// * `current_height` - the block number of current block
    /// * `total_supply` - the tokens issued
    fn calculate_farming_reward(
        created_height: HeightOf<T>,
        staked_height: HeightOf<T>,
        current_height: HeightOf<T>,
        total_supply: BalanceOf<T>,
        initial_quote: BalanceOf<T>,
    ) -> BalanceOf<T>;
}

impl<T: Config> FarmingCurve<T> for () {
    fn calculate_farming_reward(
        _created_height: HeightOf<T>,
        _staked_height: HeightOf<T>,
        _current_height: HeightOf<T>,
        _total_supply: BalanceOf<T>,
        _initial_mint: BalanceOf<T>,
    ) -> BalanceOf<T> {
        Zero::zero()
    }
}

pub struct LinearFarmingCurve<T, I>(PhantomData<(T, I)>);
impl<T, InitialFarmingReward> FarmingCurve<T>
    for LinearFarmingCurve<T, InitialFarmingReward>
where
    T: Config,
    T::BlockNumber: From<u32> + Into<U512>,
    <T::Currency as Currency<T::AccountId>>::Balance: From<u32> + Into<U512> + TryFrom<U512>,
    InitialFarmingReward: Get<BalanceOf<T>>,
{
    ///TODO(ironman_ch): change this calculate algorithm into percentage calculation.
    fn calculate_farming_reward(
        created_height: HeightOf<T>,
        staked_height: HeightOf<T>,
        current_height: HeightOf<T>,
        total_supply: BalanceOf<T>,
        initial_quote: BalanceOf<T>,
    ) -> BalanceOf<T> {
        let multiplier = BalanceOf::<T>::from(10u32);
        //TODO(ironman_ch): if total_supply >= initial_mint* 10, what will happen ?
        if total_supply >= initial_quote.saturating_mul(multiplier) {
            return Zero::zero();
        }

        let x_lower = staked_height - created_height;
        let x_upper = current_height - created_height;

        let x_lower: U512 = x_lower.into();
        let x_upper: U512 = x_upper.into();

        // we use a linear curve for farming reward
        // y = a * x + b

        // we will issue 100 dollars in the first block
        let base: U512 = InitialFarmingReward::get().into();
        // b = 100DOLLARS

        // our goal is to issue 7,000,000 dollars in 3 years
        // DAYS is the block number of a day
        // const PERIOD: f64 = 3f64 * 365.25f64 * DAYS as f64;
        // PERIOD = 3 * 365.25 * (60000 / 12000 * 60 * 24) = 7889400

        // to calculate the total supply, we use integral
        // Y = Integrate[-ax + 100DOLLARS]

        // ∵ Integrate[-ax + 100DOLLARS, {x, 0, PERIOD}] = 7_000_000DOLLARS
        // ∴ a = 39_097_000_000_000_000_000_000 / 1556065809
        // ∴ Y = 100DOLLARS x - 19_548_500_000_000_000_000_000 * Power[x,2] / 1_556_065_809
        // Y ≈ 100DOLLARS x - 12_562_772_015_768 * Power[x,2]
        let c = U512::from(12_562_772_015_768u128);

        // reward = Integrate[-ax + b, {x, staked_height, current_height}]
        // cuz Newton-Leibniz formula
        // reward = Y(x_upper) - Y(x_lower)

        let reward = (base * x_upper - c * x_upper.pow(U512::from(2u32)))
            - (base * x_lower - c * x_lower.pow(U512::from(2u32)));

        reward.try_into().unwrap_or_default()
    }
}

impl<T: Config> Pallet<T> {
    pub fn calculate_reward(
        lp_token_id: AssetOf<T>,
    ) -> Result<(LiquidityOf<T>, BalanceOf<T>), DispatchError> {
        let liquidity = <Liquidity<T>>::get(lp_token_id).ok_or(Error::<T>::NotExists)?;

        let meta = <Metadata<T>>::get(liquidity.token_id).ok_or(Error::<T>::NotExists)?;

        let height = <frame_system::Pallet<T>>::block_number();
        let supply = T::Assets::total_issuance(liquidity.token_id);

        let claimed = match <Account<T>>::get(&liquidity.owner, lp_token_id) {
            Some(claimed) => {
                if claimed > liquidity.minted {
                    claimed
                } else {
                    liquidity.minted
                }
            }
            None => liquidity.minted,
        };

        // calculate the reward from the height when
        // the liquidity was staked or last claimed
        // so that we will always have a positive reward
        let reward = T::FarmingCurve::calculate_farming_reward(
            meta.created,
            claimed, // last claimed
            height,
            supply,
            meta.initial_quote,
        );

        let reward: U512 = Self::try_into(reward)?;
        let numerator: U512 = Self::try_into(liquidity.amount)?;
        let denominator: U512 = Self::try_into(meta.liquidity)?;

        let reward = reward * numerator / denominator;

        let reward: BalanceOf<T> = Self::try_into(reward)?;

        Ok((liquidity, reward))
    }
}
