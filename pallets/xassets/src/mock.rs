use frame_support::{
    parameter_types,
    traits::{Everything, SortedMembers},
    weights::Weight,
    PalletId,
};

use frame_support::traits::{ConstU32, ConstU64};
use frame_system::EnsureRoot;
use sp_core::{hashing::blake2_128, H256};

use sp_io::TestExternalities;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

use pallet_assets;
use parami_chainbridge::ChainId;

use crate::{self as parami_xassets, weights::WeightInfo};

type Balance = u64;
type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<MockRuntime>;
type Block = frame_system::mocking::MockBlock<MockRuntime>;

pub struct MockWeightInfo;
impl WeightInfo for MockWeightInfo {
    fn transfer_hash() -> Weight {
        0 as Weight
    }

    fn transfer_native() -> Weight {
        0 as Weight
    }

    fn transfer() -> Weight {
        0 as Weight
    }

    fn remark() -> Weight {
        0 as Weight
    }

    fn transfer_token() -> Weight {
        0 as Weight
    }

    fn force_set_resource() -> Weight {
        0 as Weight
    }
}

pub(crate) const RELAYER_A: u64 = 0x2;
pub(crate) const RELAYER_B: u64 = 0x3;
pub(crate) const RELAYER_C: u64 = 0x4;
pub(crate) const ENDOWED_BALANCE: u64 = 100_000_000;
pub(crate) const TEST_THRESHOLD: u32 = 2;

frame_support::construct_runtime!(

    pub enum MockRuntime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Config<T>, Storage, Event<T>},
        ChainBridge: parami_chainbridge::{Pallet, Call, Storage, Config, Event<T>},
        Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
        XAssets: parami_xassets::{Pallet, Call, Event<T>}
    }
);

parameter_types! {
    pub const TestUserId: u64 = 1;
    pub NativeTokenId: parami_chainbridge::ResourceId = parami_chainbridge::derive_resource_id(0, &blake2_128(b"AD3"));
}

impl SortedMembers<u64> for TestUserId {
    fn sorted_members() -> Vec<u64> {
        vec![1]
    }
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
    pub const MaxLocks: u32 = 100;
}

impl frame_system::Config for MockRuntime {
    type MaxConsumers = ConstU32<16>;
    type BaseCallFilter = Everything;
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
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for MockRuntime {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = ();
}

impl pallet_assets::Config for MockRuntime {
    type Event = Event;
    type Balance = Balance;
    type AssetId = u32;
    type Currency = Balances;
    type ForceOrigin = EnsureRoot<Self::AccountId>;
    type AssetDeposit = ();
    type AssetAccountDeposit = ConstU64<10000>;
    type MetadataDepositBase = ();
    type MetadataDepositPerByte = ();
    type ApprovalDeposit = ();
    type StringLimit = ();
    type Freezer = ();
    type Extra = ();
    type WeightInfo = pallet_assets::weights::SubstrateWeight<MockRuntime>;
}

parameter_types! {
    pub const MockChainId: ChainId = 5;
    pub const ChainBridgePalletId: PalletId = PalletId(*b"chnbrdge");
    pub const ProposalLifetime: u64 = 10;
    pub HashId: parami_chainbridge::ResourceId = parami_chainbridge::derive_resource_id(233, &blake2_128(b"hash"));
}

impl parami_chainbridge::Config for MockRuntime {
    type Event = Event;
    type Proposal = Call;
    type ChainId = MockChainId;
    type PalletId = ChainBridgePalletId;
    type AdminOrigin = EnsureRoot<Self::AccountId>;
    type ProposalLifetime = ProposalLifetime;
    type WeightInfo = parami_chainbridge::weights::SubstrateWeight<MockRuntime>;
}

impl parami_xassets::Config for MockRuntime {
    type Event = Event;
    type BridgeOrigin = parami_chainbridge::EnsureBridge<MockRuntime>;
    type Currency = Balances;
    type HashId = HashId;
    type NativeTokenId = NativeTokenId;
    type WeightInfo = MockWeightInfo;
    type Assets = Assets;
    type AssetId = u32;
    type ForceOrigin = EnsureRoot<Self::AccountId>;
}

pub struct TestExternalitiesBuilder {}

impl Default for TestExternalitiesBuilder {
    fn default() -> Self {
        Self {}
    }
}

impl TestExternalitiesBuilder {
    pub(crate) fn build(self) -> TestExternalities {
        let bridge_id = ChainBridge::account_id();

        let mut storage = frame_system::GenesisConfig::default()
            .build_storage::<MockRuntime>()
            .unwrap();

        pallet_balances::GenesisConfig::<MockRuntime> {
            balances: vec![(bridge_id, ENDOWED_BALANCE), (RELAYER_A, ENDOWED_BALANCE)],
        }
        .assimilate_storage(&mut storage)
        .unwrap();

        let mut externalities = TestExternalities::new(storage);
        externalities.execute_with(|| System::set_block_number(1));
        externalities
    }
}

fn last_event() -> Event {
    frame_system::Pallet::<MockRuntime>::events()
        .pop()
        .map(|e| e.event)
        .expect("Event expected")
}

pub fn expect_event<E: Into<Event>>(e: E) {
    assert_eq!(last_event(), e.into());
}

pub fn event_exists<E: Into<Event>>(e: E) {
    let actual: Vec<Event> = frame_system::Pallet::<MockRuntime>::events()
        .iter()
        .map(|e| e.event.clone())
        .collect();
    let e: Event = e.into();
    let mut exists = false;
    for evt in actual {
        if evt == e {
            exists = true;
            break;
        }
    }
    assert!(exists);
}

pub fn assert_events(mut expected: Vec<Event>) {
    let mut actual: Vec<Event> = frame_system::Pallet::<MockRuntime>::events()
        .iter()
        .map(|e| e.event.clone())
        .collect();

    expected.reverse();

    for evt in expected {
        let next = actual.pop().expect("event expected");
        assert_eq!(next, evt.into(), "Events don't match");
    }
}
