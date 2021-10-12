#![cfg(test)]

use super::*;
use crate as parami_nft;
use frame_support::{construct_runtime, parameter_types, traits::Filter};
use orml_traits::AssetHandler;
use orml_traits::{Auction, AuctionInfo};
use primitives::{
    AccountId, AssetId, AuctionId, AuctionItem, AuctionType, Balance, BlockNumber, ItemId, Moment,
};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

pub struct NftAssetHandler;

impl AssetHandler<u32> for NftAssetHandler {
    fn check_item_in_auction(_asset_id: u32) -> bool {
        return false;
    }
}

parameter_types! {
    pub const BlockHashCount: u64 = 250;
}

impl frame_system::Config for Runtime {
    type BaseCallFilter = BaseFilter;
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Call = Call;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type BlockWeights = ();
    type BlockLength = ();
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
    pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Runtime {
    type Balance = u128;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Runtime>;
    type MaxLocks = ();
    type WeightInfo = ();
    type MaxReserves = ();
    type ReserveIdentifier = ();
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
}

impl pallet_timestamp::Config for Runtime {
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

impl pallet_assets::Config for Runtime {
    type Event = Event;
    type Balance = u128;
    type AssetId = u32;
    type Currency = Balances;
    type ForceOrigin = frame_system::EnsureRoot<u64>;
    type AssetDeposit = AssetDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = ();
    type Extra = ();
    type WeightInfo = ();
    type UnixTime = Timestamp;
}

impl orml_nft::Config for Runtime {
    type ClassId = u32;
    type TokenId = u32;
    type ClassData = parami_nft::ClassData<Balance>;
    type TokenData = parami_nft::AssetData<Balance>;
}

parameter_types! {
    pub CreateClassDeposit: Balance = 2;
    pub CreateAssetDeposit: Balance = 1;
    pub const NftPalletId: PalletId = PalletId(*b"par/pnft");
}

impl Config for Runtime {
    type Event = Event;
    type CreateClassDeposit = CreateClassDeposit;
    type CreateAssetDeposit = CreateAssetDeposit;
    type Currency = Balances;
    type PalletId = NftPalletId;
    type AssetsHandler = NftAssetHandler;
    type WeightInfo = ();
}

use frame_system::Call as SystemCall;

pub const CLASS_ID: <Runtime as orml_nft::Config>::ClassId = 0;
pub const CLASS_ID_BOUND: <Runtime as orml_nft::Config>::ClassId = 1;
pub const CLASS_ID_NOT_EXIST: <Runtime as orml_nft::Config>::ClassId = 2;
pub const NFT_TOKEN_ID: <Runtime as orml_nft::Config>::TokenId = 0;
pub const NFT_TOKEN_ID_NOT_EXIST: <Runtime as orml_nft::Config>::TokenId = 1;

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, Call, u32, ()>;

pub struct BaseFilter;
impl Filter<Call> for BaseFilter {
    fn filter(c: &Call) -> bool {
        match *c {
            // Remark is used as a no-op call in the benchmarking
            Call::System(SystemCall::remark(_)) => true,
            Call::System(_) => false,
            _ => true,
        }
    }
}

construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
        OrmlNft: orml_nft::{Pallet, Storage},
        Nft: parami_nft::{Pallet, Call, Event<T>},
    }
);

pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const DAVE: u64 = 3;

pub struct ExtBuilder;
impl Default for ExtBuilder {
    fn default() -> Self {
        ExtBuilder
    }
}

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut t = frame_system::GenesisConfig::default()
            .build_storage::<Runtime>()
            .unwrap();

        pallet_balances::GenesisConfig::<Runtime> {
            balances: vec![(ALICE, 100000), (BOB, 100000), (DAVE, 1)],
        }
        .assimilate_storage(&mut t)
        .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| {
            System::set_block_number(1);
        });
        ext
    }
}

pub fn last_event() -> Event {
    frame_system::Pallet::<Runtime>::events()
        .pop()
        .expect("Event expected")
        .event
}
