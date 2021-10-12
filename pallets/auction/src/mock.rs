#![cfg(test)]

use super::*;
use crate as auction;
use frame_support::{construct_runtime, parameter_types, traits::Filter};
use orml_traits::AssetHandler;
pub use orml_traits::{Auction, AuctionHandler};
pub use primitives::{
    AssetId, AuctionId, AuctionItem, AuctionType, Balance, BlockNumber, ItemId, Moment,
};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

use frame_election_provider_support::onchain;
use pallet_session::historical as pallet_session_historical;
use sp_core::crypto::AccountId32;

pub struct Handler;
pub type AccountId = AccountId32;

impl AuctionHandler<AccountId, u128, u64, u64> for Handler {
    fn on_new_bid(
        _now: u64,
        _id: u64,
        new_bid: (AccountId, u128),
        _last_bid: Option<(AccountId, u128)>,
    ) -> OnNewBidResult<u64> {
        if new_bid.0 == ALICE {
            OnNewBidResult {
                accept_bid: true,
                auction_end_change: Change::NoChange,
            }
        } else {
            OnNewBidResult {
                accept_bid: false,
                auction_end_change: Change::NoChange,
            }
        }
    }

    fn on_auction_ended(_id: u64, _winner: Option<(AccountId, u128)>) {}
}

pub struct NftAssetHandler;
pub type Extrinsic = sp_runtime::testing::TestXt<Call, ()>;

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
    type AccountId = AccountId;
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

sp_runtime::impl_opaque_keys! {
    pub struct SessionKeys {
        pub foo: sp_runtime::testing::UintAuthorityId,
    }
}

pub struct TestSessionHandler;
impl pallet_session::SessionHandler<AccountId> for TestSessionHandler {
    const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[];

    fn on_genesis_session<Ks: sp_runtime::traits::OpaqueKeys>(_validators: &[(AccountId, Ks)]) {}

    fn on_new_session<Ks: sp_runtime::traits::OpaqueKeys>(
        _: bool,
        _: &[(AccountId, Ks)],
        _: &[(AccountId, Ks)],
    ) {
    }

    fn on_disabled(_: usize) {}
}

parameter_types! {
    pub const Period: u64 = 1;
    pub const Offset: u64 = 0;
}

impl pallet_session::Config for Runtime {
    type SessionManager = pallet_session::historical::NoteHistoricalRoot<Runtime, Staking>;
    type Keys = SessionKeys;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionHandler = TestSessionHandler;
    type Event = Event;
    type ValidatorId = AccountId;
    type ValidatorIdOf = pallet_staking::StashOf<Runtime>;
    type DisabledValidatorsThreshold = ();
    type WeightInfo = ();
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

impl pallet_session::historical::Config for Runtime {
    type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
    type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

impl<T> frame_system::offchain::SendTransactionTypes<T> for Runtime
where
    Call: From<T>,
{
    type Extrinsic = Extrinsic;
    type OverarchingCall = Call;
}

pallet_staking_reward_curve::build! {
    const I_NPOS: sp_runtime::curve::PiecewiseLinear<'static> = curve!(
        min_inflation: 0_025_000,
        max_inflation: 0_100_000,
        ideal_stake: 0_500_000,
        falloff: 0_050_000,
        max_piece_count: 40,
        test_precision: 0_005_000,
    );
}

parameter_types! {
    pub const RewardCurve: &'static sp_runtime::curve::PiecewiseLinear<'static> = &I_NPOS;
    pub const MaxNominatorRewardedPerValidator: u32 = 64;
}

impl pallet_staking::Config for Runtime {
    const MAX_NOMINATIONS: u32 = 16;
    type Currency = Balances;
    type UnixTime = pallet_timestamp::Pallet<Self>;
    type CurrencyToVote = frame_support::traits::SaturatingCurrencyToVote;
    type RewardRemainder = ();
    type Event = Event;
    type Slash = ();
    type Reward = ();
    type SessionsPerEra = ();
    type SlashDeferDuration = ();
    type SlashCancelOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type BondingDuration = ();
    type SessionInterface = Self;
    type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
    type NextNewSession = Session;
    type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
    type ElectionProvider = onchain::OnChainSequentialPhragmen<Self>;
    type GenesisElectionProvider = Self::ElectionProvider;
    type WeightInfo = ();
}

impl onchain::Config for Runtime {
    type AccountId = AccountId;
    type BlockNumber = u64;
    type BlockWeights = ();
    type Accuracy = Perbill;
    type DataProvider = Staking;
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
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
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

impl parami_nft::Config for Runtime {
    type Event = Event;
    type CreateClassDeposit = CreateClassDeposit;
    type CreateAssetDeposit = CreateAssetDeposit;
    type Currency = Balances;
    type PalletId = NftPalletId;
    type AssetsHandler = NftAssetHandler;
    type WeightInfo = ();
}

impl orml_auction::Config for Runtime {
    type Event = Event;
    type Balance = u128;
    type AuctionId = u64;
    type Handler = AuctionPallet;
    type WeightInfo = ();
}

parameter_types! {
    pub const DidDeposit: Balance = 1;
}

impl parami_did::Config for Runtime {
    type Currency = Balances;
    type Deposit = DidDeposit;
    type Event = Event;
    type Public = sp_runtime::MultiSigner;
    type Signature = sp_runtime::MultiSignature;
    type Call = Call;
    type Time = Timestamp;
    type WeightInfo = ();
}

impl parami_ad::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type ConfigOrigin = frame_system::EnsureRoot<AccountId>;
}

parameter_types! {
    pub const MinimumAuctionDuration: BlockNumber = 100;
    pub const AuctionTimeToClose: u32 = 10;
    pub const AdsListDuration: u32 = 100;
}

impl Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type MinimumAuctionDuration = MinimumAuctionDuration;
    type Handler = Handler;
    type AuctionTimeToClose = AuctionTimeToClose;
    type AdsListDuration = AdsListDuration;
}

use frame_system::Call as SystemCall;

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
        Staking: pallet_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
        Historical: pallet_session_historical::{Pallet},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
        NftPallet: parami_nft::{Pallet, Call, Storage, Event<T>},
        OrmlNft: orml_nft::{Pallet, Storage},
        OrmlAuction: orml_auction::{Pallet, Storage, Event<T>},
        AuctionPallet: auction::{Pallet, Call, Storage, Event<T>},
        Did: parami_did::{Pallet, Call, Storage, Event<T>},
        Ads: parami_ad::{Pallet, Call, Config<T>, Storage, Event<T>},
    }
);

// pub const ALICE: u64 = 1;
// pub const BOB: u64 = 2;
// pub const DAVE: u64 = 3;
pub const CLASS_ID: u32 = 0;
pub const TOKEN_ID: u32 = 0;

pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const DAVE: AccountId = AccountId::new([3u8; 32]);

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
            balances: vec![(ALICE, 10000000), (BOB, 100000000), (DAVE, 100000000)],
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

pub fn run_to_block(n: u64) {
    while System::block_number() < n {
        OrmlAuction::on_finalize(System::block_number());
        System::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
        System::on_initialize(System::block_number());
        OrmlAuction::on_initialize(System::block_number());
    }
}

pub fn signer<T: Config>(who: T::AccountId) -> sp_runtime::MultiSigner {
    sp_runtime::MultiSigner::from(sp_core::sr25519::Public(
        std::convert::TryInto::<[u8; 32]>::try_into(who.as_ref()).unwrap(),
    ))
}
