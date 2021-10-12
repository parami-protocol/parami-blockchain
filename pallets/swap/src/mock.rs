use crate as parami_swap;
use frame_support::parameter_types;
use frame_system as system;
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
type Block = system::mocking::MockBlock<Test>;

type Balance = u128;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: system::{Pallet, Call, Config, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
        Swap: parami_swap::{Pallet, Call, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: system::limits::BlockWeights = system::limits::BlockWeights::simple_max(1024);
}

impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
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
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
}

parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
}

parameter_types! {
    pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const AssetDeposit: Balance = 100;
    pub const ApprovalDeposit: Balance = 1;
    pub const StringLimit: u32 = 50;
    pub const MetadataDepositBase: Balance = 10;
    pub const MetadataDepositPerByte: Balance = 1;
}

impl pallet_assets::Config for Test {
    type Event = Event;
    // must be u128
    type Balance = Balance;
    type AssetId = u32;
    type Currency = Balances;
    type ForceOrigin = system::EnsureRoot<u64>;
    type AssetDeposit = AssetDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = ();
    type Extra = ();
    type WeightInfo = ();
    // type UnixTime = Timestamp;
}

impl parami_swap::Config for Test {
    type Event = Event;
    type Currency = Balances;
    type NativeBalance = Balance;
    // avoid name confliction with AssetBalance struct
    type SwapAssetBalance = Balance;
}

pub const A: u64 = 1;
pub const B: u64 = 2;
pub const C: u64 = 3;

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(A, 10000000), (B, 100000000), (C, 1)],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    t.into()
}
