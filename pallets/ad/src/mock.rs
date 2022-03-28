use crate as parami_ad;
use frame_support::{parameter_types, traits::Currency, traits::GenesisBuild, PalletId};
use frame_system::{self as system, EnsureRoot};
use sp_core::{sr25519, H160, H256};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, Keccak256},
};

type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
type Block = system::mocking::MockBlock<Test>;

pub const ALICE: sr25519::Public = sr25519::Public([1; 32]);
pub const BOB: sr25519::Public = sr25519::Public([2; 32]);
pub const CHARLIE: sr25519::Public = sr25519::Public([3; 32]);

pub const DID_ALICE: H160 = H160([0xff; 20]);
pub const DID_BOB: H160 = H160([0xee; 20]);
pub const DID_CHARLIE: H160 = H160([0xdd; 20]);

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: system::{Pallet, Call, Config, Storage, Event<T>},
        Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Uniques: pallet_uniques::{Pallet, Storage, Event<T>},

        Did: parami_did::{Pallet, Call, Storage, Config<T>, Event<T>},
        Nft: parami_nft::{Pallet, Call, Storage, Event<T>},
        Ocw: parami_ocw::{Pallet},
        Swap: parami_swap::{Pallet, Call, Storage, Event<T>},
        Tag: parami_tag::{Pallet, Call, Storage, Config<T>, Event<T>},
        Ad: parami_ad::{Pallet, Call, Storage, Event<T>},
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
    type Lookup = Did;
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
    pub const ClassDeposit: Balance = 0;
    pub const InstanceDeposit: Balance = 0;
    pub const AttributeDepositBase: Balance = 0;
}

impl pallet_uniques::Config for Test {
    type Event = Event;
    type ClassId = AssetId;
    type InstanceId = AssetId;
    type Currency = Balances;
    type ForceOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type ClassDeposit = ClassDeposit;
    type InstanceDeposit = InstanceDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type AttributeDepositBase = AttributeDepositBase;
    type DepositPerByte = MetadataDepositPerByte;
    type StringLimit = StringLimit;
    type KeyLimit = StringLimit;
    type ValueLimit = StringLimit;
    type WeightInfo = ();
}

parameter_types! {
    pub const DidPalletId: PalletId = PalletId(*b"prm/did ");
}

impl parami_did::Config for Test {
    type Event = Event;
    type Currency = Balances;
    type DecentralizedId = H160;
    type Hashing = Keccak256;
    type PalletId = DidPalletId;
    type WeightInfo = ();
}

parameter_types! {
    pub const InitialMintingDeposit: Balance = 1_000_000;
    pub const InitialMintingLockupPeriod: BlockNumber = 5;
    pub const InitialMintingValueBase: Balance = 1_000_000;
    pub const PendingLifetime: BlockNumber = 5;
}

impl parami_nft::Config for Test {
    type Event = Event;
    type AssetId = AssetId;
    type Assets = Assets;
    type InitialMintingDeposit = InitialMintingDeposit;
    type InitialMintingLockupPeriod = InitialMintingLockupPeriod;
    type InitialMintingValueBase = InitialMintingValueBase;
    type Nft = Uniques;
    type PendingLifetime = PendingLifetime;
    type StringLimit = StringLimit;
    type Swaps = Swap;
    type WeightInfo = ();
}

impl parami_ocw::Config for Test {}

parameter_types! {
    pub const SwapPalletId: PalletId = PalletId(*b"prm/swap");
}

impl parami_swap::Config for Test {
    type Event = Event;
    type AssetId = AssetId;
    type Assets = Assets;
    type Currency = Balances;
    type FarmingCurve = ();
    type PalletId = SwapPalletId;
    type WeightInfo = ();
}

parameter_types! {
    pub const SubmissionFee: Balance = 1;
}

impl parami_tag::Config for Test {
    type Event = Event;
    type Currency = Balances;
    type DecentralizedId = H160;
    type SubmissionFee = SubmissionFee;
    type CallOrigin = parami_did::EnsureDid<Self>;
    type ForceOrigin = EnsureRoot<Self::AccountId>;
    type WeightInfo = ();
}

parameter_types! {
    pub const AdPalletId: PalletId = PalletId(*b"prm/ad  ");
    pub const AdvertiserMinimumFee: Balance = 1;
    pub const PayoutBase: Balance = 1;
    pub const SlotLifetime: BlockNumber = 43200;
}

pub struct MockAccounts;
impl parami_traits::Accounts for MockAccounts {
    type AccountId = <Test as system::Config>::AccountId;
    type Balance = Balance;

    fn fee_account(account: &Self::AccountId) -> Self::AccountId {
        account.clone()
    }

    fn fee_account_balance(account: &Self::AccountId) -> Self::Balance {
        <Test as parami_did::Config>::Currency::free_balance(&Self::fee_account(account))
            - <Test as parami_did::Config>::Currency::minimum_balance()
    }
}

impl parami_ad::Config for Test {
    type Event = Event;
    type Accounts = MockAccounts;
    type Assets = Assets;
    type MinimumFeeBalance = AdvertiserMinimumFee;
    type PalletId = AdPalletId;
    type PayoutBase = PayoutBase;
    type SlotLifetime = SlotLifetime;
    type Tags = Tag;
    type CallOrigin = parami_did::EnsureDid<Self>;
    type ForceOrigin = EnsureRoot<Self::AccountId>;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(ALICE, 100), (BOB, 3_000_000), (CHARLIE, 3_000_000)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    parami_did::GenesisConfig::<Test> {
        ids: vec![(ALICE, DID_ALICE), (BOB, DID_BOB), (CHARLIE, DID_CHARLIE)],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    parami_tag::GenesisConfig::<Test> {
        tag: vec![
            vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8],
            vec![5u8, 4u8, 3u8, 2u8, 1u8, 0u8],
        ],
        personas: vec![(DID_CHARLIE, vec![0u8, 1u8, 2u8, 3u8, 4u8, 5u8], 5)],
        ..Default::default()
    }
    .assimilate_storage(&mut t)
    .unwrap();

    parami_nft::GenesisConfig::<Test> {
        deposit: Default::default(),
        deposits: Default::default(),
        next_instance_id: 1,
        nfts: vec![(0, DID_ALICE, false)],
        externals: Default::default(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}
