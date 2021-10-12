use crate::{self as parami_xassets, mock::*, *};

use codec::Encode;

use frame_support::{assert_noop, assert_ok, dispatch::DispatchError};

use sp_core::{blake2_256, H256};

fn make_remark_proposal(hash: H256) -> mock::Call {
    let resource_id = HashId::get();
    mock::Call::Example(crate::Call::remark(hash, resource_id))
}

fn make_transfer_proposal(to: u64, amount: u64) -> mock::Call {
    let resource_id = HashId::get();
    mock::Call::Example(crate::Call::transfer(to, amount.into(), resource_id))
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

            assert_ok!(ChainBridge::set_threshold(Origin::root(), TEST_THRESHOLD,));

            assert_ok!(ChainBridge::whitelist_chain(
                Origin::root(),
                dest_chain.clone()
            ));
            assert_ok!(Example::transfer_hash(
                Origin::signed(1),
                hash.clone(),
                dest_chain,
            ));

            expect_event(chainbridge::Event::GenericTransfer(
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

            assert_ok!(ChainBridge::whitelist_chain(
                Origin::root(),
                dest_chain.clone()
            ));
            assert_ok!(Example::transfer_native(
                Origin::signed(RELAYER_A),
                amount.clone(),
                recipient.clone(),
                dest_chain,
            ));

            expect_event(chainbridge::Event::FungibleTransfer(
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

            assert_ok!(ChainBridge::set_threshold(Origin::root(), TEST_THRESHOLD,));
            assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_A));
            assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_B));
            assert_ok!(ChainBridge::whitelist_chain(Origin::root(), src_id));
            assert_ok!(ChainBridge::set_resource(Origin::root(), r_id, resource));

            assert_ok!(ChainBridge::acknowledge_proposal(
                Origin::signed(RELAYER_A),
                prop_id,
                src_id,
                r_id,
                Box::new(proposal.clone())
            ));
            assert_ok!(ChainBridge::acknowledge_proposal(
                Origin::signed(RELAYER_B),
                prop_id,
                src_id,
                r_id,
                Box::new(proposal.clone())
            ));

            event_exists(parami_xassets::Event::<MockRuntime>::Remark(hash));
        })
}

#[test]
fn execute_remark_bad_origin() {
    TestExternalitiesBuilder::default()
        .build()
        .execute_with(|| {
            let hash: H256 = "ABC".using_encoded(blake2_256).into();
            let resource_id = HashId::get();
            assert_ok!(Example::remark(
                Origin::signed(ChainBridge::account_id()),
                hash,
                resource_id
            ));
            // Don't allow any signed origin except from bridge addr
            assert_noop!(
                Example::remark(Origin::signed(RELAYER_A), hash, resource_id),
                DispatchError::BadOrigin
            );
            // Don't allow root calls
            assert_noop!(
                Example::remark(Origin::root(), hash, resource_id),
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
            let bridge_id: u64 = ChainBridge::account_id();
            let resource_id = HashId::get();
            assert_eq!(Balances::free_balance(&bridge_id), ENDOWED_BALANCE);
            // Transfer and check result
            assert_ok!(Example::transfer(
                Origin::signed(ChainBridge::account_id()),
                RELAYER_A,
                10,
                resource_id,
            ));
            assert_eq!(Balances::free_balance(&bridge_id), ENDOWED_BALANCE - 10);
            assert_eq!(Balances::free_balance(RELAYER_A), ENDOWED_BALANCE + 10);

            assert_events(vec![mock::Event::pallet_balances(
                pallet_balances::Event::Transfer(ChainBridge::account_id(), RELAYER_A, 10),
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

            assert_ok!(ChainBridge::set_threshold(Origin::root(), TEST_THRESHOLD,));
            assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_A));
            assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_B));
            assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_C));
            assert_ok!(ChainBridge::whitelist_chain(Origin::root(), src_id));
            assert_ok!(ChainBridge::set_resource(Origin::root(), r_id, resource));

            // Create proposal (& vote)
            assert_ok!(ChainBridge::acknowledge_proposal(
                Origin::signed(RELAYER_A),
                prop_id,
                src_id,
                r_id,
                Box::new(proposal.clone())
            ));
            let prop = ChainBridge::get_votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
            let expected = parami_chainbridge::ProposalVotes {
                votes_for: vec![RELAYER_A],
                votes_against: vec![],
                status: parami_chainbridge::ProposalStatus::Initiated,
                expiry: ProposalLifetime::get() + 1,
            };
            assert_eq!(prop, expected);

            // Second relayer votes against
            assert_ok!(ChainBridge::reject_proposal(
                Origin::signed(RELAYER_B),
                prop_id,
                src_id,
                r_id,
                Box::new(proposal.clone())
            ));
            let prop = ChainBridge::get_votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
            let expected = parami_chainbridge::ProposalVotes {
                votes_for: vec![RELAYER_A],
                votes_against: vec![RELAYER_B],
                status: parami_chainbridge::ProposalStatus::Initiated,
                expiry: ProposalLifetime::get() + 1,
            };
            assert_eq!(prop, expected);

            // Third relayer votes in favour
            assert_ok!(ChainBridge::acknowledge_proposal(
                Origin::signed(RELAYER_C),
                prop_id,
                src_id,
                r_id,
                Box::new(proposal.clone())
            ));
            let prop = ChainBridge::get_votes(src_id, (prop_id.clone(), proposal.clone())).unwrap();
            let expected = parami_chainbridge::ProposalVotes {
                votes_for: vec![RELAYER_A, RELAYER_C],
                votes_against: vec![RELAYER_B],
                status: parami_chainbridge::ProposalStatus::Approved,
                expiry: ProposalLifetime::get() + 1,
            };
            assert_eq!(prop, expected);

            assert_eq!(Balances::free_balance(RELAYER_A), ENDOWED_BALANCE + 10);
            assert_eq!(
                Balances::free_balance(ChainBridge::account_id()),
                ENDOWED_BALANCE - 10
            );

            assert_events(vec![
                mock::Event::chainbridge(chainbridge::Event::VoteFor(src_id, prop_id, RELAYER_A)),
                mock::Event::chainbridge(chainbridge::Event::VoteAgainst(
                    src_id, prop_id, RELAYER_B,
                )),
                mock::Event::chainbridge(chainbridge::Event::VoteFor(src_id, prop_id, RELAYER_C)),
                mock::Event::chainbridge(chainbridge::Event::ProposalApproved(src_id, prop_id)),
                mock::Event::pallet_balances(pallet_balances::Event::Transfer(
                    ChainBridge::account_id(),
                    RELAYER_A,
                    10,
                )),
                mock::Event::chainbridge(chainbridge::Event::ProposalSucceeded(src_id, prop_id)),
            ]);
        })
}
