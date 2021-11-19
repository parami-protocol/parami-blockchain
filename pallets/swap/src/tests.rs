use crate::{mock::*, Config, Error, Metadata};
use frame_support::{
    assert_noop, assert_ok,
    traits::{tokens::fungibles::Mutate as FungMutate, Currency},
};

macro_rules! ensure_balance {
    ($meta:tt, $quote:expr, $token: expr) => {
        assert_eq!($meta.quote, $quote);
        assert_eq!(Balances::free_balance(&$meta.pot), $quote);

        assert_eq!($meta.token, $token);
        assert_eq!(Assets::balance($meta.token_id, &$meta.pot), $token);
    };
}

#[test]
fn should_create() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token));

        let maybe_meta = <Metadata<Test>>::get(token);
        assert_ne!(maybe_meta, None);

        let meta = maybe_meta.unwrap();

        assert_eq!(meta.token_id, token);
        assert_eq!(
            meta.lp_token_id,
            <Test as Config>::AssetId::max_value() - token
        );

        let token = 9;

        assert_ok!(Swap::create(Origin::signed(ALICE), token));

        let maybe_meta = <Metadata<Test>>::get(token);
        assert_ne!(maybe_meta, None);

        let meta = maybe_meta.unwrap();

        assert_eq!(meta.token_id, token);
        assert_eq!(
            meta.lp_token_id,
            <Test as Config>::AssetId::max_value() - token
        );
    });
}

#[test]
fn should_fail_when_exists() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token));

        assert_noop!(
            Swap::create(Origin::signed(ALICE), token),
            Error::<Test>::Exists
        );
    });
}

#[test]
fn should_add_liquidity() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(ALICE),
            token,
            200,
            200,
            20,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 200, 20);

        assert_eq!(Balances::free_balance(&ALICE), 10000 - 200);
        assert_eq!(Assets::balance(token, &ALICE), 24);
        assert_eq!(Assets::balance(meta.lp_token_id, &ALICE), 200);

        assert_noop!(
            Swap::add_liquidity(Origin::signed(ALICE), token, 100, 101, 10, 100),
            Error::<Test>::TooLowLiquidity
        );

        assert_ok!(Swap::add_liquidity(
            Origin::signed(ALICE),
            token,
            100,
            100,
            10,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 300, 30);

        assert_eq!(Balances::free_balance(&ALICE), 10000 - 300);
        assert_eq!(Assets::balance(token, &ALICE), 14);
        assert_eq!(Assets::balance(meta.lp_token_id, &ALICE), 300);
    });
}

#[test]
fn should_remove_liquidity() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token));

        assert_noop!(
            Swap::remove_liquidity(Origin::signed(ALICE), token, 200, 200, 20, 100),
            Error::<Test>::NoLiquidity
        );

        assert_ok!(Swap::add_liquidity(
            Origin::signed(ALICE),
            token,
            200,
            200,
            20,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 200, 20);

        assert_noop!(
            Swap::remove_liquidity(Origin::signed(ALICE), token, 0, 0, 0, 100),
            Error::<Test>::ZeroLiquidity
        );

        assert_noop!(
            Swap::remove_liquidity(Origin::signed(ALICE), token, 200, 2000, 0, 100),
            Error::<Test>::TooLowCurrency
        );

        assert_noop!(
            Swap::remove_liquidity(Origin::signed(ALICE), token, 200, 0, 2000, 100),
            Error::<Test>::TooLowTokens
        );

        assert_ok!(Swap::remove_liquidity(
            Origin::signed(ALICE),
            token,
            200,
            200,
            20,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 0, 0);

        assert_eq!(Balances::free_balance(&ALICE), 10000);
        assert_eq!(Assets::balance(token, &ALICE), 44);
        assert_eq!(Assets::balance(meta.lp_token_id, &ALICE), 0);
    });
}

#[test]
fn should_buy_tokens() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(ALICE),
            token,
            420,
            420,
            42,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420, 42);

        assert_noop!(
            Swap::buy_tokens(Origin::signed(ALICE), token, 17, 200, 100),
            Error::<Test>::TooExpensiveCurrency
        );

        assert_ok!(Swap::buy_tokens(Origin::signed(ALICE), token, 17, 300, 100));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420 + 290, 42 - 17);

        assert_eq!(Balances::free_balance(&ALICE), 10000 - 420 - 290);
        assert_eq!(Assets::balance(token, &ALICE), 2 + 17);
        assert_eq!(Assets::balance(meta.lp_token_id, &ALICE), 420);
    });
}

#[test]
fn should_sell_tokens() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(ALICE),
            token,
            420,
            420,
            42,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420, 42);

        assert_ok!(Assets::mint_into(token, &ALICE, 42));

        assert_noop!(
            Swap::sell_tokens(Origin::signed(ALICE), token, 20, 1000, 100),
            Error::<Test>::TooLowCurrency,
        );

        assert_ok!(Swap::sell_tokens(Origin::signed(ALICE), token, 20, 1, 100));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420 - 133, 42 + 20);

        assert_eq!(Balances::free_balance(&ALICE), 10000 - 420 + 133);
        assert_eq!(Assets::balance(token, &ALICE), 2 + 42 - 20);
        assert_eq!(Assets::balance(meta.lp_token_id, &ALICE), 420);
    });
}

#[test]
fn should_sell_currency() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(ALICE),
            token,
            420,
            420,
            42,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420, 42);

        assert_noop!(
            Swap::sell_currency(Origin::signed(ALICE), token, 300, 20, 100),
            Error::<Test>::TooExpensiveTokens
        );

        assert_ok!(Swap::sell_currency(
            Origin::signed(ALICE),
            token,
            300,
            1,
            100
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420 + 300, 42 - 14);

        assert_eq!(Balances::free_balance(&ALICE), 10000 - 420 - 300);
        assert_eq!(Assets::balance(token, &ALICE), 2 + 14);
        assert_eq!(Assets::balance(meta.lp_token_id, &ALICE), 420);
    });
}

#[test]
fn should_buy_currency() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(ALICE),
            token,
            420,
            420,
            42,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420, 42);

        assert_ok!(Assets::mint_into(token, &ALICE, 42));

        assert_noop!(
            Swap::buy_currency(Origin::signed(ALICE), token, 135, 1, 100),
            Error::<Test>::TooLowTokens,
        );

        assert_ok!(Swap::buy_currency(
            Origin::signed(ALICE),
            token,
            135,
            1000,
            100
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420 - 135, 42 + 22);

        assert_eq!(Balances::free_balance(&ALICE), 10000 - 420 + 135);
        assert_eq!(Assets::balance(token, &ALICE), 2 + 42 - 22);
        assert_eq!(Assets::balance(meta.lp_token_id, &ALICE), 420);
    });
}

#[test]
fn should_swap_in_piecewisely() {
    use sp_core::U512;

    let mut input_reserve = U512::from(1_000_000_000_000_000_000_000_000u128);
    let mut output_reserve = U512::from(1_000_000_000_000_000_000_000_000u128);

    let input = U512::from(300_000_000_000_000_000_000_000u128);
    let obtain_once = Swap::calculate_price_sell(input, input_reserve, output_reserve);

    let mut obtain_quintuple = U512::zero();
    {
        let input = U512::from(100_000_000_000_000_000_000_000u128);
        let obtain = Swap::calculate_price_sell(input, input_reserve, output_reserve);

        input_reserve += input;
        output_reserve -= obtain;

        obtain_quintuple += obtain;
    }
    {
        let input = U512::from(110_000_000_000_000_000_000_000u128);
        let obtain = Swap::calculate_price_sell(input, input_reserve, output_reserve);

        input_reserve += input;
        output_reserve -= obtain;

        obtain_quintuple += obtain;
    }
    {
        let input = U512::from(90_000_000_000_000_000_000_000u128);
        let obtain = Swap::calculate_price_sell(input, input_reserve, output_reserve);

        input_reserve += input;
        output_reserve -= obtain;

        obtain_quintuple += obtain;
    }

    assert_eq!(obtain_once, obtain_quintuple);
}

#[test]
fn should_swap_out_piecewisely() {
    use sp_core::U512;

    let mut input_reserve = U512::from(1_000_000_000_000_000_000_000_000u128);
    let mut output_reserve = U512::from(1_000_000_000_000_000_000_000_000u128);

    let output = U512::from(300_000_000_000_000_000_000_000u128);
    let coast_once = Swap::calculate_price_buy(output, input_reserve, output_reserve);

    let mut coast_quintuple = U512::zero();
    {
        let output = U512::from(100_000_000_000_000_000_000_000u128);
        let coast = Swap::calculate_price_buy(output, input_reserve, output_reserve);

        input_reserve += coast;
        output_reserve -= output;

        coast_quintuple += coast;
    }
    {
        let output = U512::from(90_000_000_000_000_000_000_000u128);
        let coast = Swap::calculate_price_buy(output, input_reserve, output_reserve);

        input_reserve += coast;
        output_reserve -= output;

        coast_quintuple += coast;
    }
    {
        let output = U512::from(81_000_000_000_000_000_000_000u128);
        let coast = Swap::calculate_price_buy(output, input_reserve, output_reserve);

        input_reserve += coast;
        output_reserve -= output;

        coast_quintuple += coast;
    }
    {
        let output = U512::from(29_000_000_000_000_000_000_000u128);
        let coast = Swap::calculate_price_buy(output, input_reserve, output_reserve);

        input_reserve += coast;
        output_reserve -= output;

        coast_quintuple += coast;
    }

    assert_eq!(coast_once, coast_quintuple);
}

#[test]
fn should_not_overflow_when_calculating_price() {
    new_test_ext().execute_with(|| {
        let token = 0;

        assert_ok!(Assets::create(Origin::signed(ALICE), token, ALICE, 1));

        Balances::make_free_balance_be(&ALICE, 3_000_000_000_000_000_000_000_000_000u128);
        assert_ok!(Assets::mint_into(
            token,
            &ALICE,
            3_000_000_000_000_000_000_000_000_000u128
        ));

        assert_ok!(Swap::create(Origin::signed(ALICE), token));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(ALICE),
            token,
            2_000_000_000_000_000_000_000_000_000u128,
            2_000_000_000_000_000_000_000_000_000u128,
            200_000_000_000_000_000_000_000_000u128,
            100,
        ));

        assert_ok!(Swap::remove_liquidity(
            Origin::signed(ALICE),
            token,
            2_000_000_000_000_000_000_000_000_000u128,
            2_000_000_000_000_000_000_000_000_000u128,
            200_000_000_000_000_000_000_000_000u128,
            100,
        ));

        assert_eq!(
            Balances::free_balance(&ALICE),
            3_000_000_000_000_000_000_000_000_000u128
        );
        assert_eq!(
            Assets::balance(token, &ALICE),
            3_000_000_000_000_000_000_000_000_000u128
        );
    });
}
