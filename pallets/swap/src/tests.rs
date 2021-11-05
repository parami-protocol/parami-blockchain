use crate::{mock::*, Config, Error, Metadata};
use frame_support::{
    assert_noop, assert_ok,
    traits::{tokens::fungibles::Mutate as FungMutate, Currency},
};
use sp_core::sr25519;

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

        let alice = sr25519::Public([1; 32]);

        assert_ok!(Swap::create(Origin::signed(alice), token));

        let maybe_meta = <Metadata<Test>>::get(token);
        assert_ne!(maybe_meta, None);

        let meta = maybe_meta.unwrap();

        assert_eq!(meta.token_id, token);
        assert_eq!(meta.lp_token_id, <Test as Config>::AssetId::max_value());
    });
}

#[test]
fn should_fail_when_exists() {
    new_test_ext().execute_with(|| {
        let token = 1;

        let alice = sr25519::Public([1; 32]);

        assert_ok!(Swap::create(Origin::signed(alice), token));

        assert_noop!(
            Swap::create(Origin::signed(alice), token),
            Error::<Test>::Exists
        );
    });
}

#[test]
fn should_add_liquidity() {
    new_test_ext().execute_with(|| {
        let token = 1;

        let alice = sr25519::Public([1; 32]);

        assert_ok!(Swap::create(Origin::signed(alice), token));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(alice),
            token,
            200,
            200,
            20,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 200, 20);

        assert_eq!(Balances::free_balance(&alice), 10000 - 200);
        assert_eq!(Assets::balance(token, &alice), 24);
        assert_eq!(Assets::balance(meta.lp_token_id, &alice), 200);

        assert_noop!(
            Swap::add_liquidity(Origin::signed(alice), token, 100, 101, 10, 100),
            Error::<Test>::TooLowLiquidity
        );

        assert_ok!(Swap::add_liquidity(
            Origin::signed(alice),
            token,
            100,
            100,
            10,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 300, 30);

        assert_eq!(Balances::free_balance(&alice), 10000 - 300);
        assert_eq!(Assets::balance(token, &alice), 14);
        assert_eq!(Assets::balance(meta.lp_token_id, &alice), 300);
    });
}

#[test]
fn should_remove_liquidity() {
    new_test_ext().execute_with(|| {
        let token = 1;

        let alice = sr25519::Public([1; 32]);

        assert_ok!(Swap::create(Origin::signed(alice), token));

        assert_noop!(
            Swap::remove_liquidity(Origin::signed(alice), token, 200, 200, 20, 100),
            Error::<Test>::NoLiquidity
        );

        assert_ok!(Swap::add_liquidity(
            Origin::signed(alice),
            token,
            200,
            200,
            20,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 200, 20);

        assert_noop!(
            Swap::remove_liquidity(Origin::signed(alice), token, 0, 0, 0, 100),
            Error::<Test>::ZeroLiquidity
        );

        assert_noop!(
            Swap::remove_liquidity(Origin::signed(alice), token, 200, 2000, 0, 100),
            Error::<Test>::TooLowCurrency
        );

        assert_noop!(
            Swap::remove_liquidity(Origin::signed(alice), token, 200, 0, 2000, 100),
            Error::<Test>::TooLowTokens
        );

        assert_ok!(Swap::remove_liquidity(
            Origin::signed(alice),
            token,
            200,
            200,
            20,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 0, 0);

        assert_eq!(Balances::free_balance(&alice), 10000);
        assert_eq!(Assets::balance(token, &alice), 44);
        assert_eq!(Assets::balance(meta.lp_token_id, &alice), 0);
    });
}

#[test]
fn should_buy_tokens() {
    new_test_ext().execute_with(|| {
        let token = 1;

        let alice = sr25519::Public([1; 32]);

        assert_ok!(Swap::create(Origin::signed(alice), token));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(alice),
            token,
            420,
            420,
            42,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420, 42);

        assert_noop!(
            Swap::buy_tokens(Origin::signed(alice), token, 17, 200, 100),
            Error::<Test>::TooExpensiveCurrency
        );

        assert_ok!(Swap::buy_tokens(Origin::signed(alice), token, 17, 300, 100));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420 + 287, 42 - 17);

        assert_eq!(Balances::free_balance(&alice), 10000 - 420 - 287);
        assert_eq!(Assets::balance(token, &alice), 2 + 17);
        assert_eq!(Assets::balance(meta.lp_token_id, &alice), 420);
    });
}

#[test]
fn should_sell_tokens() {
    new_test_ext().execute_with(|| {
        let token = 1;

        let alice = sr25519::Public([1; 32]);

        assert_ok!(Swap::create(Origin::signed(alice), token));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(alice),
            token,
            420,
            420,
            42,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420, 42);

        assert_ok!(Assets::mint_into(token, &alice, 42));

        assert_noop!(
            Swap::sell_tokens(Origin::signed(alice), token, 20, 1000, 100),
            Error::<Test>::TooLowCurrency,
        );

        assert_ok!(Swap::sell_tokens(Origin::signed(alice), token, 20, 1, 100));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420 - 135, 42 + 20);

        assert_eq!(Balances::free_balance(&alice), 10000 - 420 + 135);
        assert_eq!(Assets::balance(token, &alice), 2 + 42 - 20);
        assert_eq!(Assets::balance(meta.lp_token_id, &alice), 420);
    });
}

#[test]
fn should_sell_currency() {
    new_test_ext().execute_with(|| {
        let token = 1;

        let alice = sr25519::Public([1; 32]);

        assert_ok!(Swap::create(Origin::signed(alice), token));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(alice),
            token,
            420,
            420,
            42,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420, 42);

        assert_noop!(
            Swap::sell_currency(Origin::signed(alice), token, 300, 20, 100),
            Error::<Test>::TooExpensiveTokens
        );

        assert_ok!(Swap::sell_currency(
            Origin::signed(alice),
            token,
            300,
            1,
            100
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420 + 300, 42 - 17);

        assert_eq!(Balances::free_balance(&alice), 10000 - 420 - 300);
        assert_eq!(Assets::balance(token, &alice), 2 + 17);
        assert_eq!(Assets::balance(meta.lp_token_id, &alice), 420);
    });
}

#[test]
fn should_buy_currency() {
    new_test_ext().execute_with(|| {
        let token = 1;

        let alice = sr25519::Public([1; 32]);

        assert_ok!(Swap::create(Origin::signed(alice), token));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(alice),
            token,
            420,
            420,
            42,
            100,
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420, 42);

        assert_ok!(Assets::mint_into(token, &alice, 42));

        assert_noop!(
            Swap::buy_currency(Origin::signed(alice), token, 135, 1, 100),
            Error::<Test>::TooLowTokens,
        );

        assert_ok!(Swap::buy_currency(
            Origin::signed(alice),
            token,
            135,
            1000,
            100
        ));

        let meta = <Metadata<Test>>::get(token).unwrap();
        ensure_balance!(meta, 420 - 135, 42 + 20);

        assert_eq!(Balances::free_balance(&alice), 10000 - 420 + 135);
        assert_eq!(Assets::balance(token, &alice), 2 + 42 - 20);
        assert_eq!(Assets::balance(meta.lp_token_id, &alice), 420);
    });
}

#[test]
fn should_not_overflow() {
    new_test_ext().execute_with(|| {
        let token = 0;

        let alice = sr25519::Public([1; 32]);

        assert_ok!(Assets::create(Origin::signed(alice), token, alice, 1));

        Balances::make_free_balance_be(&alice, 3_000_000_000_000_000_000_000_000_000u128);
        assert_ok!(Assets::mint_into(
            token,
            &alice,
            3_000_000_000_000_000_000_000_000_000u128
        ));

        assert_ok!(Swap::create(Origin::signed(alice), token));

        assert_ok!(Swap::add_liquidity(
            Origin::signed(alice),
            token,
            2_000_000_000_000_000_000_000_000_000u128,
            2_000_000_000_000_000_000_000_000_000u128,
            200_000_000_000_000_000_000_000_000u128,
            100,
        ));

        assert_ok!(Swap::remove_liquidity(
            Origin::signed(alice),
            token,
            2_000_000_000_000_000_000_000_000_000u128,
            2_000_000_000_000_000_000_000_000_000u128,
            200_000_000_000_000_000_000_000_000u128,
            100,
        ));

        assert_eq!(
            Balances::free_balance(&alice),
            3_000_000_000_000_000_000_000_000_000u128
        );
        assert_eq!(
            Assets::balance(token, &alice),
            3_000_000_000_000_000_000_000_000_000u128
        );
    });
}
