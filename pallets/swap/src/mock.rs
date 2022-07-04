use crate as parami_swap;
use frame_support::traits::{ConstU128, ConstU32};
use frame_support::{parameter_types, traits::GenesisBuild, PalletId};
use frame_system::{self as system, EnsureRoot};
use sp_core::{sr25519, H256};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
type Block = system::mocking::MockBlock<Test>;

pub const ALICE: sr25519::Public = sr25519::Public([1; 32]);

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: system::{Pallet, Call, Config, Storage, Event<T>},
        Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},

        Swap: parami_swap::{Pallet, Call, Storage, Event<T>},
    }
);

type AssetId = u64;
type Balance = u128;
type BlockNumber = u64;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
    type MaxConsumers = ConstU32<16>;
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = sr25519::Public;
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
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}

parameter_types! {
    pub const AssetDeposit: Balance = 100;
    pub const ApprovalDeposit: Balance = 1;
    pub const StringLimit: u32 = 50;
    pub const MetadataDepositBase: Balance = 0;
    pub const MetadataDepositPerByte: Balance = 0;
}

impl pallet_assets::Config for Test {
    type Event = Event;
    type Balance = Balance;
    type AssetId = AssetId;
    type Currency = Balances;
    type ForceOrigin = EnsureRoot<Self::AccountId>;
    type AssetAccountDeposit = ConstU128<10000u128>;
    type AssetDeposit = AssetDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = ();
    type Extra = ();
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 1;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
}

parameter_types! {
    pub const SwapPalletId: PalletId = PalletId(*b"prm/swap");
}

pub struct FarmingCurve;
impl parami_swap::FarmingCurve<Test> for FarmingCurve {
    fn calculate_farming_reward(
        _created_height: BlockNumber,
        _staked_height: BlockNumber,
        _current_height: BlockNumber,
        _total_supply: Balance,
    ) -> Balance {
        100
    }
}

impl parami_swap::Config for Test {
    type Event = Event;
    type AssetId = AssetId;
    type Assets = Assets;
    type Currency = Balances;
    type FarmingCurve = FarmingCurve;
    type PalletId = SwapPalletId;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(ALICE, 10000)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    pallet_assets::GenesisConfig::<Test> {
        assets: vec![(1, ALICE, false, 1)],
        metadata: vec![(1, b"Test Token".to_vec(), b"XTT".to_vec(), 18)],
        accounts: vec![(1, ALICE, 44)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}
