#![cfg(test)]

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};

use orml_nft::Pallet as NftPallet;

fn free_balance(who: &u64) -> Balance {
    <Runtime as Config>::Currency::free_balance(who)
}

fn reserved_balance(who: &u64) -> Balance {
    <Runtime as Config>::Currency::reserved_balance(who)
}

fn class_id_account() -> u64 {
    <Runtime as Config>::PalletId::get().into_sub_account(CLASS_ID)
}

fn init_test_nft(owner: Origin) {
    assert_ok!(Nft::create_class(
        owner.clone(),
        vec![1],
        TokenType::Transferable,
        CollectionType::Collectable,
    ));
    assert_ok!(Nft::mint(
        owner.clone(),
        CLASS_ID,
        vec![1],
        vec![1],
        vec![1],
        vec![1],
        1
    ));
}

#[test]
fn create_class_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = Origin::signed(ALICE);
        assert_ok!(Nft::create_class(
            origin.clone(),
            vec![1],
            TokenType::Transferable,
            CollectionType::Collectable,
        ));

        let class_data = ClassData {
            deposit: 2,
            metadata: vec![1],
            token_type: TokenType::Transferable,
            collection_type: CollectionType::Collectable,
        };

        let class_info = orml_nft::ClassInfo::<u32, u64, ClassData<u128>> {
            metadata: vec![1],
            total_issuance: Default::default(),
            owner: ALICE,
            data: class_data,
        };

        assert_eq!(NftPallet::<Runtime>::classes(CLASS_ID), Some(class_info));

        let event = mock::Event::Nft(crate::Event::NftClassCreated(ALICE, CLASS_ID));
        assert_eq!(last_event(), event);

        assert_eq!(
            reserved_balance(&class_id_account()),
            <Runtime as Config>::CreateClassDeposit::get()
        );
    });
}

#[test]
fn destroy_class_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = Origin::signed(ALICE);
        assert_ok!(Nft::create_class(
            origin.clone(),
            vec![1],
            TokenType::Transferable,
            CollectionType::Collectable,
        ));

        let deposit = <Runtime as Config>::CreateClassDeposit::get();
        assert_eq!(free_balance(&ALICE), 100000 - deposit);
        assert_eq!(reserved_balance(&ALICE), 0);
        assert_eq!(free_balance(&class_id_account()), 0);
        assert_eq!(reserved_balance(&class_id_account()), deposit);

        assert_ok!(Nft::mint(
            origin.clone(),
            CLASS_ID,
            vec![2],
            vec![2],
            vec![2],
            vec![2],
            1
        ));
        let nft_deposit = <Runtime as Config>::CreateAssetDeposit::get();
        assert_eq!(reserved_balance(&class_id_account()), deposit + nft_deposit);

        assert_ok!(Nft::burn(origin.clone(), (CLASS_ID, NFT_TOKEN_ID)));
        assert_ok!(Nft::destroy_class(origin.clone(), CLASS_ID));
        assert_eq!(reserved_balance(&class_id_account()), 0);
        assert_eq!(free_balance(&ALICE), 100000);
    });
}

#[test]
fn destroy_class_should_fail() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = Origin::signed(ALICE);
        assert_ok!(Nft::create_class(
            origin.clone(),
            vec![1],
            TokenType::Transferable,
            CollectionType::Collectable,
        ));

        assert_ok!(Nft::mint(
            origin.clone(),
            CLASS_ID,
            vec![2],
            vec![2],
            vec![2],
            vec![2],
            1
        ));

        assert_noop!(
            Nft::destroy_class(origin.clone(), CLASS_ID_NOT_EXIST),
            Error::<Runtime>::ClassIdNotFound
        );
        assert_noop!(
            Nft::destroy_class(Origin::signed(BOB), CLASS_ID),
            Error::<Runtime>::NoPermission
        );
        assert_noop!(
            Nft::destroy_class(origin.clone(), CLASS_ID),
            Error::<Runtime>::CannotDestroyClass
        );

        assert_ok!(Nft::burn(origin.clone(), (CLASS_ID, NFT_TOKEN_ID)));
        assert_ok!(Nft::destroy_class(origin.clone(), CLASS_ID));
    });
}

#[test]
fn mint_nft_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = Origin::signed(ALICE);

        init_test_nft(origin.clone());

        assert_eq!(
            reserved_balance(&class_id_account()),
            <Runtime as Config>::CreateClassDeposit::get()
                + <Runtime as Config>::CreateAssetDeposit::get()
        );
        assert_eq!(Nft::next_asset_id(), 1);
        assert_eq!(Nft::get_assets_by_owner(ALICE), vec![0]);
        assert_eq!(Nft::get_asset(0), Some((CLASS_ID, NFT_TOKEN_ID)));

        let event = mock::Event::Nft(crate::Event::NftMinted(0, 0, ALICE, CLASS_ID, 1, 0));
        assert_eq!(last_event(), event);

        // mint second assets
        assert_ok!(Nft::mint(
            origin.clone(),
            CLASS_ID,
            vec![2],
            vec![2],
            vec![2],
            vec![2],
            2
        ));

        assert_eq!(Nft::next_asset_id(), 3);
        assert_eq!(Nft::get_assets_by_owner(ALICE), vec![0, 1, 2]);
        assert_eq!(Nft::get_asset(1), Some((CLASS_ID, 1)));
        assert_eq!(Nft::get_asset(2), Some((CLASS_ID, 2)));
    })
}

#[test]
fn mint_nft_and_fraction_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = Origin::signed(ALICE);

        assert_ok!(Nft::create_class(
            origin.clone(),
            vec![2],
            TokenType::BoundToAddress,
            CollectionType::Collectable,
        ));

        assert_ok!(Nft::mint(
            origin.clone(),
            CLASS_ID,
            vec![1],
            vec![1],
            vec![1],
            vec![1],
            1
        ));

        assert_eq!(
            reserved_balance(&class_id_account()),
            <Runtime as Config>::CreateClassDeposit::get()
                + <Runtime as Config>::CreateAssetDeposit::get()
        );
        assert_eq!(Nft::next_asset_id(), 1);
        assert_eq!(Nft::get_assets_by_owner(ALICE), vec![0]);
        assert_eq!(Nft::get_asset(0), Some((CLASS_ID, NFT_TOKEN_ID)));

        let event = mock::Event::Nft(crate::Event::NftMinted(0, 0, ALICE, CLASS_ID, 1, 0));
        assert_eq!(last_event(), event);

        let nft_asset_data = NftPallet::<Runtime>::tokens(CLASS_ID, NFT_TOKEN_ID);
        println!("nft data => {:?}", nft_asset_data);

        // check fractional
        let metadata = <pallet_assets::Metadata<Runtime>>::get(0);
        println!("metadata => {:?}", metadata);
        assert_eq!(<pallet_assets::Metadata::<Runtime>>::contains_key(0), true);

        let ads_slot = AdsSlots::<Runtime>::get(0);
        println!("ads slot => {:?}", ads_slot);

        init_test_nft(origin.clone());

        assert_eq!(Nft::next_asset_id(), 2);
        assert_eq!(Nft::get_assets_by_owner(ALICE), vec![0, 1]);
        assert_eq!(Nft::get_asset(0), Some((CLASS_ID, 0)));
        assert_eq!(Nft::get_asset(1), Some((CLASS_ID, 1)));
    })
}

#[test]
fn mint_nft_should_fail() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = Origin::signed(ALICE);
        let invalid_owner = Origin::signed(BOB);
        assert_ok!(Nft::create_class(
            origin.clone(),
            vec![1],
            TokenType::Transferable,
            CollectionType::Collectable,
        ));
        assert_noop!(
            Nft::mint(
                origin.clone(),
                CLASS_ID,
                vec![1],
                vec![1],
                vec![1],
                vec![1],
                0
            ),
            Error::<Runtime>::InvalidQuantity
        );
        assert_noop!(
            Nft::mint(origin.clone(), 1, vec![1], vec![1], vec![1], vec![1], 1),
            Error::<Runtime>::ClassIdNotFound
        );
        assert_noop!(
            Nft::mint(
                invalid_owner.clone(),
                CLASS_ID,
                vec![1],
                vec![1],
                vec![1],
                vec![1],
                1
            ),
            Error::<Runtime>::NoPermission
        );
        assert_ok!(Nft::create_class(
            origin.clone(),
            vec![1],
            TokenType::BoundToAddress,
            CollectionType::Collectable,
        ));
        assert_noop!(
            Nft::mint(
                origin.clone(),
                CLASS_ID_BOUND,
                vec![1],
                vec![1],
                vec![1],
                vec![1],
                2
            ),
            Error::<Runtime>::QuantityExceeds
        );
    })
}

#[test]
fn burn_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = Origin::signed(ALICE);

        assert_ok!(Nft::create_class(
            origin.clone(),
            vec![2],
            TokenType::Transferable,
            CollectionType::Collectable,
        ));

        assert_ok!(Nft::mint(
            origin.clone(),
            CLASS_ID,
            vec![1],
            vec![1],
            vec![1],
            vec![1],
            1
        ));

        assert_ok!(Nft::burn(origin.clone(), (CLASS_ID, NFT_TOKEN_ID)));
        let event = mock::Event::Nft(crate::Event::NftBurned(ALICE, CLASS_ID, NFT_TOKEN_ID));
        assert_eq!(last_event(), event);
        assert_eq!(reserved_balance(&class_id_account()), 2);
        assert_eq!(free_balance(&class_id_account()), 1);
    });
}

#[test]
fn burn_should_fail() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = Origin::signed(ALICE);
        let bob = Origin::signed(BOB);

        assert_ok!(Nft::create_class(
            origin.clone(),
            vec![2],
            TokenType::Transferable,
            CollectionType::Collectable,
        ));

        assert_ok!(Nft::mint(
            origin.clone(),
            CLASS_ID,
            vec![1],
            vec![1],
            vec![1],
            vec![1],
            1
        ));

        assert_noop!(
            Nft::burn(origin.clone(), (CLASS_ID, NFT_TOKEN_ID_NOT_EXIST)),
            Error::<Runtime>::AssetInfoNotFound
        );

        assert_noop!(
            Nft::burn(Origin::signed(BOB), (CLASS_ID, NFT_TOKEN_ID)),
            Error::<Runtime>::NoPermission
        );

        assert_ok!(Nft::create_class(
            bob.clone(),
            vec![2],
            TokenType::BoundToAddress,
            CollectionType::Collectable,
        ));

        assert_ok!(Nft::mint(
            bob.clone(),
            1,
            vec![1],
            vec![1],
            vec![1],
            vec![1],
            1
        ));

        assert_noop!(
            Nft::burn(bob.clone(), (1, NFT_TOKEN_ID)),
            Error::<Runtime>::CannotBeBurned
        );
    });
}

#[test]
fn transfer_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = Origin::signed(ALICE);
        init_test_nft(origin.clone());
        assert_ok!(Nft::transfer(origin, BOB, 0));
        let event = mock::Event::Nft(crate::Event::NftTransfered(1, 2, 0));
        assert_eq!(last_event(), event);
    })
}

#[test]
fn transfer_batch_should_work() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = Origin::signed(ALICE);
        init_test_nft(origin.clone());
        assert_ok!(Nft::create_class(
            origin.clone(),
            vec![1],
            TokenType::Transferable,
            CollectionType::Collectable,
        ));
        assert_ok!(Nft::mint(
            origin.clone(),
            1,
            vec![1],
            vec![1],
            vec![1],
            vec![1],
            1
        ));
        assert_ok!(Nft::transfer_batch(origin, vec![(BOB, 0), (BOB, 1)]));
        let event = mock::Event::Nft(crate::Event::NftTransfered(1, 2, 0));
        assert_eq!(last_event(), event);
    })
}

#[test]
fn transfer_batch_should_fail() {
    ExtBuilder::default().build().execute_with(|| {
        let origin = Origin::signed(ALICE);
        init_test_nft(origin.clone());
        assert_ok!(Nft::create_class(
            origin.clone(),
            vec![1],
            TokenType::Transferable,
            CollectionType::Collectable,
        ));
        assert_ok!(Nft::mint(
            origin.clone(),
            1,
            vec![1],
            vec![1],
            vec![1],
            vec![1],
            1
        ));
        assert_noop!(
            Nft::transfer_batch(origin.clone(), vec![(BOB, 3), (BOB, 4)]),
            Error::<Runtime>::AssetIdNotFound
        );
    })
}

#[test]
fn handle_ownership_transfer_should_work() {
    let origin = Origin::signed(ALICE);
    ExtBuilder::default().build().execute_with(|| {
        init_test_nft(origin.clone());
        assert_ok!(Nft::handle_ownership_transfer(&ALICE, &BOB, 0));
        assert_eq!(Nft::get_assets_by_owner(ALICE), Vec::<u32>::new());
        assert_eq!(Nft::get_assets_by_owner(BOB), vec![0]);
    })
}

#[test]
fn do_transfer_should_work() {
    let origin = Origin::signed(ALICE);
    ExtBuilder::default().build().execute_with(|| {
        init_test_nft(origin.clone());
        assert_ok!(Nft::do_transfer(&ALICE, &BOB, 0));
        assert_eq!(Nft::get_assets_by_owner(ALICE), Vec::<u32>::new());
        assert_eq!(Nft::get_assets_by_owner(BOB), vec![0]);
    })
}

#[test]
fn do_transfer_should_fail() {
    let origin = Origin::signed(ALICE);
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            Nft::do_transfer(&ALICE, &BOB, 0),
            Error::<Runtime>::AssetIdNotFound
        );
        init_test_nft(origin.clone());
        assert_noop!(
            Nft::do_transfer(&BOB, &ALICE, 0),
            Error::<Runtime>::NoPermission
        );
        assert_ok!(Nft::create_class(
            origin.clone(),
            vec![1],
            TokenType::BoundToAddress,
            CollectionType::Collectable,
        ));
        assert_ok!(Nft::mint(
            origin.clone(),
            1,
            vec![1],
            vec![1],
            vec![1],
            vec![1],
            1
        ));
        assert_noop!(
            Nft::do_transfer(&ALICE, &BOB, 1),
            Error::<Runtime>::NonTransferable
        );
    })
}

#[test]
fn check_ownership_should_work() {
    let origin = Origin::signed(ALICE);
    ExtBuilder::default().build().execute_with(|| {
        init_test_nft(origin.clone());
        assert_ok!(Nft::check_ownership(&ALICE, &NFT_TOKEN_ID), true);
        assert_ok!(Nft::check_ownership(&BOB, &NFT_TOKEN_ID), false);
    })
}

#[test]
fn check_ownership_should_fail() {
    let _origin = Origin::signed(ALICE);
    ExtBuilder::default().build().execute_with(|| {
        assert_noop!(
            Nft::check_ownership(&ALICE, &NFT_TOKEN_ID),
            Error::<Runtime>::AssetIdNotFound
        );
    })
}
