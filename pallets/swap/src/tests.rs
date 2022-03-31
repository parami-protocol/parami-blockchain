use crate::{mock::*, Account, Error, Liquidity, Metadata, Provider};
use frame_support::{
    assert_noop, assert_ok,
    traits::{tokens::fungibles::Mutate as FungMutate, Currency},
};
use parami_traits::Swaps;

#[test]
fn should_create() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token, 1_000_000, 7_000_000));

        let maybe_meta = <Metadata<Test>>::get(&token);
        assert_ne!(maybe_meta, None);

        assert_eq!(token, token);
    });
}

#[test]
fn should_fail_when_exists() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token, 1_000_000, 7_000_000));

        assert_noop!(
            Swap::create(Origin::signed(ALICE), token, 1_000_000, 7_000_000),
            Error::<Test>::Exists
        );
    });
}

#[test]
fn should_add_liquidity() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token, 1_000_000, 7_000_000));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(ALICE),
            token,
            200,
            200,
            20,
            100,
        ));

        let meta = <Metadata<Test>>::get(&token).unwrap();
        assert_eq!(meta.liquidity, 200);

        let pot = Swap::get_pool_account(token);
        assert_eq!(Balances::free_balance(&pot), 200);
        assert_eq!(Assets::balance(token, &pot), 20);

        assert_eq!(Balances::free_balance(&ALICE), 10000 - 200);
        assert_eq!(Assets::balance(token, &ALICE), 44 - 20);

        assert_eq!(<Provider<Test>>::get(token, &ALICE), 200);
        assert_eq!(<Account<Test>>::get(&ALICE, 0), Some(0));

        let liquidity = <Liquidity<Test>>::get(0);
        assert_ne!(liquidity, None);
        let liquidity = liquidity.unwrap();
        assert_eq!(liquidity.token_id, token);
        assert_eq!(liquidity.amount, 200);

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

        let meta = <Metadata<Test>>::get(&token).unwrap();
        assert_eq!(meta.liquidity, 200 + 100);

        assert_eq!(Balances::free_balance(&pot), 200 + 100);
        assert_eq!(Assets::balance(token, &pot), 20 + 10);

        assert_eq!(Balances::free_balance(&ALICE), 10000 - 200 - 100);
        assert_eq!(Assets::balance(token, &ALICE), 44 - 20 - 10);

        assert_eq!(<Provider<Test>>::get(token, &ALICE), 200 + 100);
        assert_eq!(<Account<Test>>::get(&ALICE, 1), Some(0));

        let liquidity = <Liquidity<Test>>::get(1);
        assert_ne!(liquidity, None);
        let liquidity = liquidity.unwrap();
        assert_eq!(liquidity.token_id, token);
        assert_eq!(liquidity.amount, 100);
    });
}

#[test]
fn should_remove_liquidity() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token, 1_000_000, 7_000_000));

        assert_noop!(
            Swap::remove_liquidity(Origin::signed(ALICE), 0, 200, 20, 100),
            Error::<Test>::NotExists
        );

        assert_ok!(Swap::add_liquidity(
            Origin::signed(ALICE),
            token,
            200,
            200,
            20,
            100,
        ));

        assert_noop!(
            Swap::remove_liquidity(Origin::signed(ALICE), 0, 2000, 0, 100),
            Error::<Test>::TooLowCurrency
        );

        assert_noop!(
            Swap::remove_liquidity(Origin::signed(ALICE), 0, 0, 2000, 100),
            Error::<Test>::TooLowTokens
        );

        assert_ok!(Swap::remove_liquidity(
            Origin::signed(ALICE),
            0,
            200,
            20,
            100,
        ));

        let meta = <Metadata<Test>>::get(&token).unwrap();
        assert_eq!(meta.liquidity, 0);

        let pot = Swap::get_pool_account(token);
        assert_eq!(Balances::free_balance(&pot), 0);
        assert_eq!(Assets::balance(token, &pot), 0);

        assert_eq!(Balances::free_balance(&ALICE), 10000);
        assert_eq!(Assets::balance(token, &ALICE), 44);

        assert_eq!(<Provider<Test>>::get(token, &ALICE), 0);
        assert_eq!(<Account<Test>>::get(&ALICE, 0), None);
        assert_eq!(<Liquidity<Test>>::get(0), None);
    });
}

#[test]
fn should_buy_tokens() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token, 1_000_000, 7_000_000));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(ALICE),
            token,
            420,
            420,
            42,
            100,
        ));

        let pot = Swap::get_pool_account(token);
        assert_eq!(Balances::free_balance(&pot), 420);
        assert_eq!(Assets::balance(token, &pot), 42);

        assert_noop!(
            Swap::buy_tokens(Origin::signed(ALICE), token, 17, 200, 100),
            Error::<Test>::TooExpensiveCurrency
        );

        assert_ok!(Swap::buy_tokens(Origin::signed(ALICE), token, 17, 300, 100));

        assert_eq!(Balances::free_balance(&pot), 420 + 290);
        assert_eq!(Assets::balance(token, &pot), 42 - 17);

        assert_eq!(Balances::free_balance(&ALICE), 10000 - 420 - 290);
        assert_eq!(Assets::balance(token, &ALICE), 44 - 42 + 17);
    });
}

#[test]
fn should_sell_tokens() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token, 1_000_000, 7_000_000));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(ALICE),
            token,
            420,
            420,
            42,
            100,
        ));

        let pot = Swap::get_pool_account(token);
        assert_eq!(Balances::free_balance(&pot), 420);
        assert_eq!(Assets::balance(token, &pot), 42);

        assert_ok!(Assets::mint_into(token, &ALICE, 42));

        assert_noop!(
            Swap::sell_tokens(Origin::signed(ALICE), token, 20, 1000, 100),
            Error::<Test>::TooLowCurrency,
        );

        assert_ok!(Swap::sell_tokens(Origin::signed(ALICE), token, 20, 1, 100));

        assert_eq!(Balances::free_balance(&pot), 420 - 133);
        assert_eq!(Assets::balance(token, &pot), 42 + 20);

        assert_eq!(Balances::free_balance(&ALICE), 10000 - 420 + 133);
        assert_eq!(Assets::balance(token, &ALICE), 44 - 42 + 42 - 20);
    });
}

#[test]
fn should_sell_currency() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token, 1_000_000, 7_000_000));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(ALICE),
            token,
            420,
            420,
            42,
            100,
        ));

        let pot = Swap::get_pool_account(token);
        assert_eq!(Balances::free_balance(&pot), 420);
        assert_eq!(Assets::balance(token, &pot), 42);

        assert_noop!(
            Swap::sell_currency(Origin::signed(ALICE), token, 300, 40, 100),
            Error::<Test>::TooExpensiveTokens
        );

        assert_ok!(Swap::sell_currency(
            Origin::signed(ALICE),
            token,
            300,
            1,
            100
        ));

        assert_eq!(Balances::free_balance(&pot), 420 + 300);
        assert_eq!(Assets::balance(token, &pot), 42 - 14);

        assert_eq!(Balances::free_balance(&ALICE), 10000 - 420 - 300);
        assert_eq!(Assets::balance(token, &ALICE), 44 - 42 + 14);
    });
}

#[test]
fn should_buy_currency() {
    new_test_ext().execute_with(|| {
        let token = 1;

        assert_ok!(Swap::create(Origin::signed(ALICE), token, 1_000_000, 7_000_000));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(ALICE),
            token,
            420,
            420,
            42,
            100,
        ));

        let pot = Swap::get_pool_account(token);
        assert_eq!(Balances::free_balance(&pot), 420);
        assert_eq!(Assets::balance(token, &pot), 42);

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

        assert_eq!(Balances::free_balance(&pot), 420 - 135);
        assert_eq!(Assets::balance(token, &pot), 42 + 22);

        assert_eq!(Balances::free_balance(&ALICE), 10000 - 420 + 135);
        assert_eq!(Assets::balance(token, &ALICE), 44 - 42 + 42 - 22);
    });
}

#[test]
fn should_swap_in_piecewisely() {
    let mut input_reserve = 1_000_000_000_000_000_000_000_000u128;
    let mut output_reserve = 1_000_000_000_000_000_000_000_000u128;

    let input = 300_000_000_000_000_000_000_000u128;
    let obtain_once = Swap::price_sell(input, input_reserve, output_reserve).unwrap();

    let mut obtain_quintuple = 0;
    {
        let input = 100_000_000_000_000_000_000_000u128;
        let obtain = Swap::price_sell(input, input_reserve, output_reserve).unwrap();

        input_reserve += input;
        output_reserve -= obtain;

        obtain_quintuple += obtain;
    }
    {
        let input = 110_000_000_000_000_000_000_000u128;
        let obtain = Swap::price_sell(input, input_reserve, output_reserve).unwrap();

        input_reserve += input;
        output_reserve -= obtain;

        obtain_quintuple += obtain;
    }
    {
        let input = 90_000_000_000_000_000_000_000u128;
        let obtain = Swap::price_sell(input, input_reserve, output_reserve).unwrap();

        obtain_quintuple += obtain;
    }

    assert_eq!(obtain_once, obtain_quintuple);
}

#[test]
fn should_swap_out_piecewisely() {
    let mut input_reserve = 1_000_000_000_000_000_000_000_000u128;
    let mut output_reserve = 1_000_000_000_000_000_000_000_000u128;

    let output = 300_000_000_000_000_000_000_000u128;
    let coast_once = Swap::price_buy(output, input_reserve, output_reserve).unwrap();

    let mut coast_quintuple = 0;
    {
        let output = 100_000_000_000_000_000_000_000u128;
        let coast = Swap::price_buy(output, input_reserve, output_reserve).unwrap();

        input_reserve += coast;
        output_reserve -= output;

        coast_quintuple += coast;
    }
    {
        let output = 90_000_000_000_000_000_000_000u128;
        let coast = Swap::price_buy(output, input_reserve, output_reserve).unwrap();

        input_reserve += coast;
        output_reserve -= output;

        coast_quintuple += coast;
    }
    {
        let output = 81_000_000_000_000_000_000_000u128;
        let coast = Swap::price_buy(output, input_reserve, output_reserve).unwrap();

        input_reserve += coast;
        output_reserve -= output;

        coast_quintuple += coast;
    }
    {
        let output = 29_000_000_000_000_000_000_000u128;
        let coast = Swap::price_buy(output, input_reserve, output_reserve).unwrap();

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

        assert_ok!(Swap::create(Origin::signed(ALICE), token, 1_000_000, 7_000_000));

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
            0,
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
