#![cfg(test)]

use super::*;
use crate::mock::{Event, *};
use frame_support::{assert_noop, assert_ok};

pub fn free_balance(who: &AccountId) -> Balance {
    <Runtime as Config>::Currency::free_balance(who)
}

#[test]
fn redeem_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let tx_hash_redeem1 = || vec![121];
        let tx_hash_redeem2 = || vec![122];
        let tx_hash_redeem3 = || vec![123];
        let eth_addr = || vec![22];

        assert_ok!(Bridge::set_bridge_admin(Origin::root(), ALICE));
        assert_eq!(None, Bridge::erc20_txs(tx_hash_redeem1()));
        assert_eq!(None, Bridge::erc20_txs(tx_hash_redeem2()));
        assert_eq!(None, Bridge::erc20_txs(tx_hash_redeem3()));

        assert_eq!(0, free_balance(&BOB));
        assert_ok!(Bridge::redeem(
            Origin::signed(ALICE),
            tx_hash_redeem1(),
            eth_addr(),
            BOB,
            200
        ));
        assert_eq!(200, free_balance(&BOB));
        assert_ok!(Bridge::redeem(
            Origin::signed(ALICE),
            tx_hash_redeem1(),
            eth_addr(),
            BOB,
            200
        ));
        assert_eq!(200, free_balance(&BOB));

        assert_eq!(
            last_event(),
            Event::parami_bridge(crate::Event::Redeem(tx_hash_redeem1()))
        );

        assert_ok!(Bridge::redeem(
            Origin::signed(ALICE),
            tx_hash_redeem2(),
            eth_addr(),
            BOB,
            200
        ));
        assert_eq!(400, free_balance(&BOB));
        assert_eq!(
            last_event(),
            Event::parami_bridge(crate::Event::Redeem(tx_hash_redeem2()))
        );
    });
}

#[test]
fn redeem_should_fail() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            Bridge::redeem(Origin::signed(ALICE), vec![11], vec![22], BOB, 444),
            Error::<Runtime>::BridgeAdminNotSet,
        );

        assert_ok!(Bridge::set_bridge_admin(Origin::root(), ALICE));
        assert_noop!(
            Bridge::redeem(Origin::signed(BOB), vec![11], vec![22], ALICE, 444),
            Error::<Runtime>::NoPermission,
        );
    });
}

#[test]
fn withdraw_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let tx_hash_transfer = || vec![11];
        let tx_hash_withdraw1 = || vec![121];
        let tx_hash_withdraw2 = || vec![122];
        let tx_hash_withdraw3 = || vec![123];
        let eth_addr = || vec![22];
        let value: Balance = 444;
        assert_ok!(Bridge::set_bridge_admin(Origin::root(), ALICE));
        assert_eq!(None, Bridge::erc20_txs(tx_hash_withdraw1()));
        assert_eq!(None, Bridge::erc20_txs(tx_hash_withdraw2()));
        assert_ok!(Bridge::transfer(
            Origin::signed(ALICE),
            tx_hash_transfer(),
            eth_addr(),
            value
        ));

        assert_eq!(0, free_balance(&BOB));
        assert_ok!(Bridge::withdraw(
            Origin::signed(ALICE),
            tx_hash_withdraw1(),
            eth_addr(),
            BOB,
            200
        ));
        assert_ok!(Bridge::withdraw(
            Origin::signed(ALICE),
            tx_hash_withdraw1(),
            eth_addr(),
            BOB,
            200
        ));
        assert_eq!(200, free_balance(&BOB));
        assert_eq!(
            last_event(),
            Event::parami_bridge(crate::Event::Withdraw(tx_hash_withdraw1(), true))
        );

        assert_ok!(Bridge::withdraw(
            Origin::signed(ALICE),
            tx_hash_withdraw2(),
            eth_addr(),
            BOB,
            300
        ));
        assert_eq!(200, free_balance(&BOB));
        assert_eq!(
            last_event(),
            Event::parami_bridge(crate::Event::Withdraw(tx_hash_withdraw2(), false))
        );

        assert_ok!(Bridge::withdraw(
            Origin::signed(ALICE),
            tx_hash_withdraw3(),
            eth_addr(),
            BOB,
            value - 200
        ));
        assert_eq!(value, free_balance(&BOB));
        assert_eq!(
            last_event(),
            Event::parami_bridge(crate::Event::Withdraw(tx_hash_withdraw3(), true))
        );
    });
}

#[test]
fn withdraw_should_fail() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            Bridge::withdraw(Origin::signed(ALICE), vec![11], vec![22], BOB, 444),
            Error::<Runtime>::BridgeAdminNotSet,
        );

        assert_ok!(Bridge::set_bridge_admin(Origin::root(), ALICE));
        assert_noop!(
            Bridge::withdraw(Origin::signed(BOB), vec![11], vec![22], ALICE, 444),
            Error::<Runtime>::NoPermission,
        );

        assert_eq!(0, Bridge::erc20_balances(vec![22]));
        assert_eq!(0, free_balance(&BOB));
        assert_ok!(Bridge::withdraw(
            Origin::signed(ALICE),
            vec![11],
            vec![22],
            BOB,
            444
        ));
        assert_eq!(
            last_event(),
            Event::parami_bridge(crate::Event::Withdraw(vec![11], false))
        );
        assert_eq!(0, free_balance(&BOB));
        assert_eq!(0, Bridge::erc20_balances(vec![22]));
    });
}

#[test]
fn transfer_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let tx_hash = || vec![11];
        let tx_hash2 = || vec![12];
        let eth_addr = || vec![22];
        let value: Balance = 444;
        assert_ok!(Bridge::set_bridge_admin(Origin::root(), ALICE));

        assert_eq!(None, Bridge::erc20_txs(tx_hash()));
        assert_ok!(Bridge::transfer(
            Origin::signed(ALICE),
            tx_hash(),
            eth_addr(),
            value
        ));
        assert_eq!(
            last_event(),
            Event::parami_bridge(crate::Event::Transfer(tx_hash()))
        );
        assert_eq!(
            Some(crate::Erc20Event::Transfer {
                value,
                from: eth_addr(),
            }),
            Bridge::erc20_txs(tx_hash())
        );

        assert_eq!(value, Bridge::erc20_balances(eth_addr()));

        assert_ok!(Bridge::transfer(
            Origin::signed(ALICE),
            tx_hash2(),
            eth_addr(),
            value
        ));
        assert_eq!(
            last_event(),
            Event::parami_bridge(crate::Event::Transfer(tx_hash2()))
        );

        assert_ok!(Bridge::transfer(
            Origin::signed(ALICE),
            tx_hash2(),
            eth_addr(),
            value
        ));
        assert_ok!(Bridge::transfer(
            Origin::signed(ALICE),
            tx_hash2(),
            eth_addr(),
            value
        ));

        assert_eq!(value * 2, Bridge::erc20_balances(eth_addr()));
    });
}

#[test]
fn transfer_should_fail() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            Bridge::transfer(Origin::signed(ALICE), vec![11], vec![22], 444),
            Error::<Runtime>::BridgeAdminNotSet,
        );

        assert_ok!(Bridge::set_bridge_admin(Origin::root(), ALICE));
        assert_noop!(
            Bridge::transfer(Origin::signed(BOB), vec![11], vec![22], 444),
            Error::<Runtime>::NoPermission,
        );
    });
}

#[test]
fn set_bridge_admin_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(None, Bridge::bridge_admin());
        assert_ok!(Bridge::set_bridge_admin(Origin::root(), ALICE));
        assert_eq!(Some(ALICE), Bridge::bridge_admin());
    });
}

#[test]
fn set_bridge_admin_should_fail() {
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            Bridge::set_bridge_admin(Origin::signed(ALICE), BOB),
            DispatchError::BadOrigin,
        );
    });
}
