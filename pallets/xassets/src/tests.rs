use crate::{self as parami_xassets, mock::*, *};

use codec::Encode;

use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};

use sp_core::{blake2_256, H256};

pub type HashId = <mock::MockRuntime as parami_xassets::Config>::HashId;

fn make_remark_proposal(hash: H256) -> mock::Call {
    let resource_id = HashId::get();
    mock::Call::XAssets(crate::Call::remark {
        hash,
        r_id: resource_id,
    })
}

fn make_transfer_proposal(to: u64, amount: u64) -> mock::Call {
    let resource_id = HashId::get();
    mock::Call::XAssets(crate::Call::transfer {
        to,
        amount: amount.into(),
        r_id: resource_id,
    })
}

// ----------------------------------------------------------------------------
// Test cases
// ----------------------------------------------------------------------------

#[test]
fn transfer_hash() {
    TestExternalitiesBuilder::default()
        .build()
        .execute_with(|| {
            let dest_chain = 0;
            let resource_id = HashId::get();
            let hash: H256 = "ABC".using_encoded(blake2_256).into();

            assert_ok!(mock::ChainBridge::set_threshold(
                Origin::root(),
                TEST_THRESHOLD
            ));

            assert_ok!(mock::ChainBridge::whitelist_chain(
                Origin::root(),
                dest_chain.clone()
            ));
            assert_ok!(mock::XAssets::transfer_hash(
                Origin::signed(1),
                hash.clone(),
                dest_chain,
            ));

            expect_event(parami_chainbridge::Event::GenericTransfer(
                dest_chain,
                1,
                resource_id,
                hash.as_ref().to_vec(),
            ));
        })
}

#[test]
fn transfer_native() {
    TestExternalitiesBuilder::default()
        .build()
        .execute_with(|| {
            let dest_chain = 0;
            let resource_id = NativeTokenId::get();
            let amount: u64 = 100;
            let recipient = vec![99];

            assert_ok!(mock::ChainBridge::whitelist_chain(
                Origin::root(),
                dest_chain.clone()
            ));
            assert_ok!(mock::XAssets::transfer_native(
                Origin::signed(RELAYER_A),
                amount.clone(),
                recipient.clone(),
                dest_chain,
            ));

            expect_event(parami_chainbridge::Event::FungibleTransfer(
                dest_chain,
                1,
                resource_id,
                amount.into(),
                recipient,
            ));
        })
}

#[test]
fn execute_remark() {
    TestExternalitiesBuilder::default()
        .build()
        .execute_with(|| {
            let hash: H256 = "ABC".using_encoded(blake2_256).into();
            let proposal = make_remark_proposal(hash.clone());
            let prop_id = 1;
            let src_id = 1;
            let r_id = parami_chainbridge::derive_resource_id(src_id, b"hash");
            let resource = b"Example.remark".to_vec();

            assert_ok!(mock::ChainBridge::set_threshold(
                Origin::root(),
                TEST_THRESHOLD,
            ));
            assert_ok!(mock::ChainBridge::add_relayer(Origin::root(), RELAYER_A));
            assert_ok!(mock::ChainBridge::add_relayer(Origin::root(), RELAYER_B));
            assert_ok!(mock::ChainBridge::whitelist_chain(Origin::root(), src_id));
            assert_ok!(mock::ChainBridge::set_resource(
                Origin::root(),
                r_id,
                resource
            ));

            assert_ok!(mock::ChainBridge::acknowledge_proposal(
                Origin::signed(RELAYER_A),
                prop_id,
                src_id,
                r_id,
                Box::new(proposal.clone())
            ));
            assert_ok!(mock::ChainBridge::acknowledge_proposal(
                Origin::signed(RELAYER_B),
                prop_id,
                src_id,
                r_id,
                Box::new(proposal.clone())
            ));

            event_exists(mock::Event::XAssets(parami_xassets::Event::Remark(hash)));
        })
}

#[test]
fn execute_remark_bad_origin() {
    TestExternalitiesBuilder::default()
        .build()
        .execute_with(|| {
            let hash: H256 = "ABC".using_encoded(blake2_256).into();
            let resource_id = HashId::get();
            assert_ok!(mock::XAssets::remark(
                Origin::signed(mock::ChainBridge::account_id()),
                hash,
                resource_id
            ));
            // Don't allow any signed origin except from bridge addr
            assert_noop!(
                mock::XAssets::remark(Origin::signed(RELAYER_A), hash, resource_id),
                DispatchError::BadOrigin
            );
            // Don't allow root calls
            assert_noop!(
                mock::XAssets::remark(Origin::root(), hash, resource_id),
                DispatchError::BadOrigin
            );
        })
}

#[test]
fn transfer() {
    TestExternalitiesBuilder::default()
        .build()
        .execute_with(|| {
            // Check inital state
            let bridge_id: u64 = mock::ChainBridge::account_id();
            let resource_id = HashId::get();
            assert_eq!(Balances::free_balance(&bridge_id), ENDOWED_BALANCE);
            // Transfer and check result
            assert_ok!(mock::XAssets::transfer(
                Origin::signed(mock::ChainBridge::account_id()),
                RELAYER_A,
                10,
                resource_id,
            ));
            assert_eq!(Balances::free_balance(&bridge_id), ENDOWED_BALANCE - 10);
            assert_eq!(Balances::free_balance(RELAYER_A), ENDOWED_BALANCE + 10);

            assert_events(vec![mock::Event::Balances(
                pallet_balances::Event::Transfer {
                    from: mock::ChainBridge::account_id(),
                    to: RELAYER_A,
                    amount: 10,
                },
            )]);
        })
}

#[test]
fn create_sucessful_transfer_proposal() {
    TestExternalitiesBuilder::default()
        .build()
        .execute_with(|| {
            let prop_id = 1;
            let src_id = 1;
            let r_id = parami_chainbridge::derive_resource_id(src_id, b"transfer");
            let resource = b"Example.transfer".to_vec();
            let proposal = make_transfer_proposal(RELAYER_A, 10);

            assert_ok!(mock::ChainBridge::set_threshold(
                Origin::root(),
                TEST_THRESHOLD,
            ));
            assert_ok!(mock::ChainBridge::add_relayer(Origin::root(), RELAYER_A));
            assert_ok!(mock::ChainBridge::add_relayer(Origin::root(), RELAYER_B));
            assert_ok!(mock::ChainBridge::add_relayer(Origin::root(), RELAYER_C));
            assert_ok!(mock::ChainBridge::whitelist_chain(Origin::root(), src_id));
            assert_ok!(mock::ChainBridge::set_resource(
                Origin::root(),
                r_id,
                resource
            ));

            // Create proposal (& vote)
            assert_ok!(mock::ChainBridge::acknowledge_proposal(
                Origin::signed(RELAYER_A),
                prop_id,
                src_id,
                r_id,
                Box::new(proposal.clone())
            ));
            let prop =
                mock::ChainBridge::get_votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
            let expected = parami_chainbridge::ProposalVotes {
                votes_for: vec![RELAYER_A],
                votes_against: vec![],
                status: parami_chainbridge::ProposalStatus::Initiated,
                expiry: ProposalLifetime::get() + 1,
            };
            assert_eq!(prop, expected);

            // Second relayer votes against
            assert_ok!(mock::ChainBridge::reject_proposal(
                Origin::signed(RELAYER_B),
                prop_id,
                src_id,
                r_id,
                Box::new(proposal.clone())
            ));
            let prop =
                mock::ChainBridge::get_votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
            let expected = parami_chainbridge::ProposalVotes {
                votes_for: vec![RELAYER_A],
                votes_against: vec![RELAYER_B],
                status: parami_chainbridge::ProposalStatus::Initiated,
                expiry: ProposalLifetime::get() + 1,
            };
            assert_eq!(prop, expected);

            // Third relayer votes in favour
            assert_ok!(mock::ChainBridge::acknowledge_proposal(
                Origin::signed(RELAYER_C),
                prop_id,
                src_id,
                r_id,
                Box::new(proposal.clone())
            ));
            let prop =
                mock::ChainBridge::get_votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
            let expected = parami_chainbridge::ProposalVotes {
                votes_for: vec![RELAYER_A, RELAYER_C],
                votes_against: vec![RELAYER_B],
                status: parami_chainbridge::ProposalStatus::Approved,
                expiry: ProposalLifetime::get() + 1,
            };
            assert_eq!(prop, expected);

            assert_eq!(Balances::free_balance(RELAYER_A), ENDOWED_BALANCE + 10);
            assert_eq!(
                Balances::free_balance(mock::ChainBridge::account_id()),
                ENDOWED_BALANCE - 10
            );

            assert_events(vec![
                mock::Event::ChainBridge(parami_chainbridge::Event::VoteFor(
                    src_id, prop_id, RELAYER_A,
                )),
                mock::Event::ChainBridge(parami_chainbridge::Event::VoteAgainst(
                    src_id, prop_id, RELAYER_B,
                )),
                mock::Event::ChainBridge(parami_chainbridge::Event::VoteFor(
                    src_id, prop_id, RELAYER_C,
                )),
                mock::Event::ChainBridge(parami_chainbridge::Event::ProposalApproved(
                    src_id, prop_id,
                )),
                mock::Event::Balances(pallet_balances::Event::Transfer {
                    from: mock::ChainBridge::account_id(),
                    to: RELAYER_A,
                    amount: 10,
                }),
                mock::Event::ChainBridge(parami_chainbridge::Event::ProposalSucceeded(
                    src_id, prop_id,
                )),
            ]);
        })
}

#[test]
fn can_force_set_resource() {
    TestExternalitiesBuilder::default()
        .build()
        .execute_with(|| {
            let resource_id = H256::from_slice("00000000001111111111222222222233".as_bytes());
            let asset_id = 1;

            assert!(mock::XAssets::resource(asset_id).is_none());
            assert!(
                mock::XAssets::force_set_resource(Origin::root(), resource_id, asset_id).is_ok()
            );

            let actual_resource = mock::XAssets::resource(asset_id);
            assert!(actual_resource.is_some());
            assert_eq!(actual_resource.unwrap(), resource_id);
        });
}

#[test]
fn can_transfer_token() {
    TestExternalitiesBuilder::default()
        .build()
        .execute_with(|| {
            let resource_id = H256::from_slice("00000000001111111111222222222233".as_bytes());
            let chain_id = 100u8;
            let asset_id = 1;
            let recipient: Vec<u8> = "address".as_bytes().into();
            mock::ChainBridge::whitelist_chain(Origin::root(), chain_id).unwrap();
            mock::XAssets::force_set_resource(Origin::root(), resource_id, asset_id).unwrap();
            mock::Assets::force_create(Origin::root(), asset_id, chain_id as u64, true, 1).unwrap();
            mock::Assets::mint(
                Origin::signed(chain_id as u64),
                asset_id,
                chain_id as u64,
                101,
            )
            .unwrap();
            assert_ok!(mock::XAssets::transfer_token(
                Origin::signed(chain_id as u64),
                chain_id as u64,
                recipient.clone(),
                100,
                asset_id
            ));
            assert_events(vec![
                mock::Event::Assets(pallet_assets::Event::Transferred {
                    asset_id,
                    from: chain_id as u64,
                    to: <parami_chainbridge::Pallet<MockRuntime>>::account_id(),
                    amount: 100,
                }),
                mock::Event::ChainBridge(parami_chainbridge::Event::FungibleTransfer(
                    chain_id,
                    1,
                    resource_id,
                    100u32.into(),
                    recipient.clone(),
                )),
            ]);
        });
}

#[test]
fn fail_to_tranfer_token_if_chain_is_not_whitelisted() {
    TestExternalitiesBuilder::default()
        .build()
        .execute_with(|| {
            let chain_id = 100u8;
            let asset_id = 1;
            let recipient: Vec<u8> = "address".as_bytes().into();

            assert_noop!(
                mock::XAssets::transfer_token(
                    Origin::signed(chain_id as u64),
                    100,
                    recipient.clone(),
                    chain_id,
                    asset_id,
                ),
                parami_xassets::Error::<mock::MockRuntime>::InvalidTransfer
            );
        });
}

#[test]
fn fail_to_tranfer_token_if_not_exist_asset_2_resource_id_config() {
    TestExternalitiesBuilder::default()
        .build()
        .execute_with(|| {
            let chain_id = 100u8;
            let asset_id = 1;
            let recipient: Vec<u8> = "address".as_bytes().into();

            mock::ChainBridge::whitelist_chain(Origin::root(), chain_id).unwrap();

            assert_noop!(
                mock::XAssets::transfer_token(
                    Origin::signed(chain_id as u64),
                    100,
                    recipient.clone(),
                    chain_id,
                    asset_id,
                ),
                parami_xassets::Error::<mock::MockRuntime>::NotExists
            );
        });
}

#[test]
fn fail_to_transfer_if_no_asset() {
    TestExternalitiesBuilder::default()
        .build()
        .execute_with(|| {
            let resource_id = H256::from_slice("00000000001111111111222222222233".as_bytes());
            let chain_id = 100u8;
            let asset_id = 1;
            let recipient: Vec<u8> = "address".as_bytes().into();
            mock::ChainBridge::whitelist_chain(Origin::root(), chain_id).unwrap();
            mock::XAssets::force_set_resource(Origin::root(), resource_id, asset_id).unwrap();
            assert_noop!(
                mock::XAssets::transfer_token(
                    Origin::signed(chain_id as u64),
                    100,
                    recipient.clone(),
                    chain_id,
                    asset_id,
                ),
                pallet_assets::Error::<mock::MockRuntime>::Unknown
            );
        });
}

#[test]
fn fail_to_transfer_if_no_enough_balance() {
    TestExternalitiesBuilder::default()
        .build()
        .execute_with(|| {
            let resource_id = H256::from_slice("00000000001111111111222222222233".as_bytes());
            let chain_id = 100u8;
            let asset_id = 1;
            let recipient: Vec<u8> = "address".as_bytes().into();
            mock::ChainBridge::whitelist_chain(Origin::root(), chain_id).unwrap();
            mock::XAssets::force_set_resource(Origin::root(), resource_id, asset_id).unwrap();
            mock::Assets::force_create(Origin::root(), asset_id, chain_id as u64, true, 1).unwrap();
            mock::Assets::mint(
                Origin::signed(chain_id as u64),
                asset_id,
                chain_id as u64,
                10,
            )
            .unwrap();
            assert_noop!(
                mock::XAssets::transfer_token(
                    Origin::signed(chain_id as u64),
                    100,
                    recipient.clone(),
                    chain_id,
                    asset_id,
                ),
                pallet_assets::Error::<mock::MockRuntime>::BalanceLow
            );
        });
}
