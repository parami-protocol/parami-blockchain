use frame_support::{
    assert_ok, parameter_types,
    traits::{ConstU32, SortedMembers},
    weights::Weight,
    PalletId,
};

use frame_system::EnsureSignedBy;

use sp_core::H256;

use sp_io::TestExternalities;

use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

use crate::{
    self as pallet_chainbridge,
    types::{ChainId, ResourceId},
    weights::WeightInfo,
};

type Balance = u64;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<MockRuntime>;
type Block = frame_system::mocking::MockBlock<MockRuntime>;

// Implement testing extrinsic weights for the pallet
pub struct MockWeightInfo;
impl WeightInfo for MockWeightInfo {
    fn set_threshold() -> Weight {
        0 as Weight
    }

    fn set_resource() -> Weight {
        0 as Weight
    }

    fn remove_resource() -> Weight {
        0 as Weight
    }

    fn whitelist_chain() -> Weight {
        0 as Weight
    }

    fn add_relayer() -> Weight {
        0 as Weight
    }

    fn remove_relayer() -> Weight {
        0 as Weight
    }

    fn acknowledge_proposal(_: Weight) -> Weight {
        0 as Weight
    }

    fn reject_proposal() -> Weight {
        0 as Weight
    }

    fn eval_vote_state(_: Weight) -> Weight {
        0 as Weight
    }
}

// Constants definition
pub(crate) const RELAYER_A: u64 = 0x2;
pub(crate) const RELAYER_B: u64 = 0x3;
pub(crate) const RELAYER_C: u64 = 0x4;
pub(crate) const ENDOWED_BALANCE: u64 = 100_000_000;
pub(crate) const TEST_THRESHOLD: u32 = 2;

// Build mock runtime
frame_support::construct_runtime!(

    pub enum MockRuntime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Config<T>, Storage, Event<T>},
        ChainBridge: pallet_chainbridge::{Pallet, Call, Storage, Config, Event<T>},
    }
);

// Parameterize default test user identifier (with id 1)
parameter_types! {
    pub const TestUserId: u64 = 1;
}

impl SortedMembers<u64> for TestUserId {
    fn sorted_members() -> Vec<u64> {
        vec![1]
    }
}

// Parameterize FRAME system pallet
parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
    pub const MaxLocks: u32 = 100;
}

// Implement FRAME system pallet configuration trait for the mock runtime
impl frame_system::Config for MockRuntime {
    type BaseCallFilter = frame_support::traits::Everything;
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type PalletInfo = PalletInfo;
    type BlockWeights = ();
    type BlockLength = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

// Parameterize FRAME balances pallet
parameter_types! {
    pub const ExistentialDeposit: u64 = 1;

    pub const MaxReserves: u32 = 50;
}

// Implement FRAME balances pallet configuration trait for the mock runtime
impl pallet_balances::Config for MockRuntime {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
}

// Parameterize chainbridge pallet
parameter_types! {
    pub const MockChainId: ChainId = 5;
    pub const ChainBridgePalletId: PalletId = PalletId(*b"chnbrdge");
    pub const ProposalLifetime: u64 = 10;
}

// Implement chainbridge pallet configuration trait for the mock runtime
impl pallet_chainbridge::Config for MockRuntime {
    type Event = Event;
    type Proposal = Call;
    type ChainId = MockChainId;
    type PalletId = ChainBridgePalletId;
    type AdminOrigin = EnsureSignedBy<TestUserId, u64>;
    type ProposalLifetime = ProposalLifetime;
    type WeightInfo = MockWeightInfo;
}

// Test externalities builder type declaraction.
//
// This type is mainly used for mocking storage in tests. It is the type alias
// for an in-memory, hashmap-based externalities implementation.
pub struct TestExternalitiesBuilder {}

// Default trait implementation for test externalities builder
impl Default for TestExternalitiesBuilder {
    fn default() -> Self {
        Self {}
    }
}

impl TestExternalitiesBuilder {
    // Build a genesis storage key/value store
    pub(crate) fn build(self) -> TestExternalities {
        let bridge_id = ChainBridge::account_id();

        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<MockRuntime>()
            .unwrap();

        // pre-fill balances
        pallet_balances::GenesisConfig::<MockRuntime> {
            balances: vec![(bridge_id, ENDOWED_BALANCE)],
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        let mut externalities = TestExternalities::new(storage);
        externalities.execute_with(|| System::set_block_number(1));
        externalities
    }

    pub fn build_with(
        self,
        src_id: ChainId,
        r_id: ResourceId,
        resource: Vec<u8>,
    ) -> TestExternalities {
        let mut externalities = Self::build(self);

        externalities.execute_with(|| {
            // Set and check threshold
            assert_ok!(ChainBridge::set_threshold(Origin::root(), TEST_THRESHOLD));
            assert_eq!(ChainBridge::get_relayer_threshold(), TEST_THRESHOLD);
            // Add relayers
            assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_A));
            assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_B));
            assert_ok!(ChainBridge::add_relayer(Origin::root(), RELAYER_C));
            // Whitelist chain
            assert_ok!(ChainBridge::whitelist_chain(Origin::root(), src_id));
            // Set and check resource ID mapped to some junk data
            assert_ok!(ChainBridge::set_resource(Origin::root(), r_id, resource));
            assert_eq!(ChainBridge::resource_exists(r_id), true);
        });
        externalities
    }
}

// Checks events against the latest. A contiguous set of events must be provided. They must
// include the most recent event, but do not have to include every past event.
pub fn assert_events(mut expected: Vec<Event>) {
    let mut actual: Vec<Event> = frame_system::Pallet::<MockRuntime>::events()
        .iter()
        .map(|e| e.event.clone())
        .collect();

    expected.reverse();

    for evt in expected {
        let next = actual.pop().expect("event expected");
        assert_eq!(next, evt.into(), "Events don't match (actual,expected)");
    }
}
