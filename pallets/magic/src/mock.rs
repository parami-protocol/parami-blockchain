#![cfg(test)]
#![allow(unused_imports)]

use super::*;
use crate as parami_ad;
use codec::{Decode, Encode};
use frame_support::{construct_runtime, parameter_types, traits::InstanceFilter, RuntimeDebug};
pub use parami_primitives::{AccountId, Balance};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
};

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
parameter_types! {
    pub const ExistentialDeposit: Balance = 1;
}
impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Runtime>;
    type MaxLocks = ();
    type WeightInfo = ();
    type MaxReserves = ();
    type ReserveIdentifier = ();
}
impl pallet_utility::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type WeightInfo = ();
}
parameter_types! {
    pub const ProxyDepositBase: u64 = 1;
    pub const ProxyDepositFactor: u64 = 1;
    pub const MaxProxies: u16 = 4;
    pub const MaxPending: u32 = 2;
    pub const AnnouncementDepositBase: u64 = 1;
    pub const AnnouncementDepositFactor: u64 = 1;
}
#[derive(Clone, Copy, Decode, Encode, Eq, Ord, PartialEq, PartialOrd, RuntimeDebug, TypeInfo)]
pub enum ProxyType {
    Any,
    JustTransfer,
    JustUtility,
}
impl Default for ProxyType {
    fn default() -> Self {
        Self::Any
    }
}
impl InstanceFilter<Call> for ProxyType {
    fn filter(&self, c: &Call) -> bool {
        match self {
            ProxyType::Any => true,
            ProxyType::JustTransfer => {
                matches!(c, Call::Balances(pallet_balances::Call::transfer { .. }))
            }
            ProxyType::JustUtility => matches!(c, Call::Utility(..)),
        }
    }
    fn is_superset(&self, o: &Self) -> bool {
        self == &ProxyType::Any || self == o
    }
}
pub struct BaseFilter;
impl Filter<Call> for BaseFilter {
    fn filter(c: &Call) -> bool {
        match *c {
            // Remark is used as a no-op call in the benchmarking
            Call::System(SystemCall::remark { .. }) => true,
            Call::System(_) => false,
            _ => true,
        }
    }
}

impl pallet_proxy::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type Currency = Balances;
    type ProxyType = ProxyType;
    type ProxyDepositBase = ProxyDepositBase;
    type ProxyDepositFactor = ProxyDepositFactor;
    type MaxProxies = MaxProxies;
    type WeightInfo = ();
    type CallHasher = BlakeTwo256;
    type MaxPending = MaxPending;
    type AnnouncementDepositBase = AnnouncementDepositBase;
    type AnnouncementDepositFactor = AnnouncementDepositFactor;
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

impl parami_ad::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type ConfigOrigin = frame_system::EnsureRoot<AccountId>;
    type Call = Call;
}

//----------------------------------------------
use frame_election_provider_support::onchain;
use pallet_session::historical as pallet_session_historical;
use sp_runtime::Perbill;

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

pub type Extrinsic = sp_runtime::testing::TestXt<Call, ()>;

impl onchain::Config for Runtime {
    type Accuracy = Perbill;
    type DataProvider = Staking;
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
//----------------------------------------------

use frame_system::Call as SystemCall;

pub type Block = sp_runtime::generic::Block<Header, UncheckedExtrinsic>;
pub type UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic<u32, Call, u32, ()>;

construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Staking: pallet_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
        Historical: pallet_session_historical::{Pallet},
        Proxy: pallet_proxy::{Pallet, Call, Storage, Event<T>},
        Utility: pallet_utility::{Pallet, Call, Event},
        Ad: parami_ad::{Pallet, Call, Event<T>},
    }
);

pub const ALICE: AccountId = AccountId::new([1u8; 32]);
pub const ALICE_INIT: BalanceOf<Runtime> = 60000 * UNIT;
pub const BOB: AccountId = AccountId::new([2u8; 32]);
pub const BOB_INIT: BalanceOf<Runtime> = 60000 * UNIT;
pub const CHARLIE: AccountId = AccountId::new([3u8; 32]);
pub const CHARLIE_INIT: BalanceOf<Runtime> = 60000 * UNIT;
pub const DAVE: AccountId = AccountId::new([4u8; 32]);
pub const DAVE_INIT: BalanceOf<Runtime> = 60000 * UNIT;

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
            balances: vec![
                (ALICE, ALICE_INIT),
                (BOB, BOB_INIT),
                (CHARLIE, CHARLIE_INIT),
                (DAVE, DAVE_INIT),
            ],
        }
        .assimilate_storage(&mut t)
        .unwrap();

        crate::GenesisConfig::<Runtime>::default()
            .assimilate_storage(&mut t)
            .unwrap();

        let mut ext = sp_io::TestExternalities::new(t);
        ext.execute_with(|| {
            System::set_block_number(1);
        });
        ext
    }
}
