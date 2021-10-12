use crate::mock::*;
use frame_support::{assert_ok, traits::Currency};

#[test]
fn create_swap() {
    new_test_ext().execute_with(|| {
        let id = 1;
        // id = 1
        // min_balance = 1
        assert_ok!(Assets::create(Origin::signed(A), id, A, 1));
        // decimals = 0
        assert_ok!(Assets::set_metadata(
            Origin::signed(A),
            id,
            b"A Token".to_vec(),
            b"AAA".to_vec(),
            0
        ));

        assert_ok!(Assets::mint(Origin::signed(A), id, A, 10_00000000));
        assert_ok!(Assets::mint(Origin::signed(A), id, B, 10_00000));

        assert_ok!(Swap::create(Origin::signed(A), id));

        // create twice
        // assert!(Swap::create(Origin::signed(A), 1).is_err());
        // 1 native for 100 asset
        assert_ok!(Swap::add_liquidity(
            Origin::signed(A),
            id,
            10_000,
            Some(1000_000)
        ));

        println!("native bal => {:?}", Balances::total_balance(&A));
        println!("asset bal => {:?}", Assets::balance(id, &A));

        assert_ok!(Swap::add_liquidity(Origin::signed(B), id, 1, None));

        println!("native bal => {:?}", Balances::total_balance(&B));
        println!("asset bal => {:?}", Assets::balance(id, &B));

        assert_ok!(Swap::swap_native(Origin::signed(A), id, 20));
        println!("asset bal => {:?}", Assets::balance(id, &A));
    });
}
