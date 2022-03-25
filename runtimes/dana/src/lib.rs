#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod filter;

use codec::{Decode, Encode};
use sp_api::impl_runtime_apis;
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{
    crypto::KeyTypeId,
    u32_trait::{_1, _2, _3, _4, _5},
    OpaqueMetadata,
};
use sp_io::hashing::blake2_128;
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys,
    traits::{
        BlakeTwo256, Block as BlockT, ConvertInto, Extrinsic, Keccak256, NumberFor, StaticLookup,
        Verify,
    },
    transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, DispatchError, FixedPointNumber, Perbill, Percent, Permill, Perquintill,
    SaturatedConversion,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
use frame_support::{
    construct_runtime, parameter_types,
    traits::{
        EqualPrivilegeOnly, Everything, KeyOwnerProofSystem, LockIdentifier, Nothing,
        U128CurrencyToVote,
    },
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        DispatchClass, IdentityFee, Weight,
    },
    PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureOneOf, EnsureRoot,
};
use pallet_contracts::weights::WeightInfo;
use pallet_grandpa::{
    fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
pub use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
#[cfg(any(feature = "std", test))]
pub use pallet_staking::StakerStatus;
use pallet_transaction_payment::{Multiplier, TargetedFeeAdjustment};
pub use parami_primitives::{
    constants::{
        AVERAGE_ON_INITIALIZE_RATIO, CENTS, DAYS, DOLLARS, EPOCH_DURATION_IN_BLOCKS,
        EXISTENTIAL_DEPOSIT, HOURS, MILLICENTS, MILLISECS_PER_BLOCK, MINUTES,
        NORMAL_DISPATCH_RATIO, SLOT_DURATION,
    },
    deposit, names, AccountId, Address, AssetId, Balance, BalanceWrapper, BlockNumber,
    DecentralizedId, Hash, Header, Index, Moment, Signature,
};
use parami_swap::LinearFarmingCurve;
use parami_traits::Swaps;

/// We allow for 0.5 of a second of compute with a 12 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = WEIGHT_PER_SECOND / 2;

/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    filter::ExtrinsicFilter<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;

/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;

/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

/// Era type as expected by this runtime.
pub type Era = generic::Era;

/// MMR helper types.
mod mmr {
    use super::Runtime;
    pub use pallet_mmr::primitives::*;

    pub type Leaf = <<Runtime as pallet_mmr::Config>::LeafData as LeafDataProvider>::LeafData;
    pub type Hash = <Runtime as pallet_mmr::Config>::Hash;
    pub type Hashing = <Runtime as pallet_mmr::Config>::Hashing;
}

/// more than 1/2
type HalfCouncil =
    pallet_collective::EnsureProportionMoreThan<_1, _2, AccountId, CouncilCollective>;
type EnsureRootOrHalfCouncil = EnsureOneOf<AccountId, EnsureRoot<AccountId>, HalfCouncil>;

/// at least 3/5
type PluralityCouncil =
    pallet_collective::EnsureProportionAtLeast<_3, _5, AccountId, CouncilCollective>;
type EnsureRootOrPluralityCouncil = EnsureOneOf<AccountId, EnsureRoot<AccountId>, PluralityCouncil>;

/// at least 3/4
type MajoritarianCouncil =
    pallet_collective::EnsureProportionAtLeast<_3, _4, AccountId, CouncilCollective>;
type EnsureRootOrMajoritarianCouncil =
    EnsureOneOf<AccountId, EnsureRoot<AccountId>, MajoritarianCouncil>;

/// whole
type OverallCouncil =
    pallet_collective::EnsureProportionAtLeast<_1, _1, AccountId, CouncilCollective>;
type EnsureRootOrOverallCouncil = EnsureOneOf<AccountId, EnsureRoot<AccountId>, OverallCouncil>;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
    use super::*;

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;

    /// Opaque block header type.
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    /// Opaque block identifier type.
    pub type BlockId = generic::BlockId<Block>;
}

impl_opaque_keys! {
    pub struct SessionKeys {
        pub authority_discovery: AuthorityDiscovery,
        pub aura: Aura,
        pub grandpa: Grandpa,
        pub im_online: ImOnline,
    }
}

// To learn more about runtime versioning and what each of the following value means:
//   https://docs.substrate.io/v3/runtime/upgrades#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("parami"),
    impl_name: create_runtime_str!("parami-node"),
    authoring_version: 20,
    spec_version: 323,
    impl_version: 0,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 2,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

parameter_types! {
    pub const Version: RuntimeVersion = VERSION;
    pub const BlockHashCount: BlockNumber = 2400;

    // This part is copied from Substrate's `bin/node/runtime/src/lib.rs`.
    //  The `RuntimeBlockLength` and `RuntimeBlockWeights` exist here because the
    // `DeletionWeightLimit` and `DeletionQueueDepth` depend on those to parameterize
    // the lazy contract deletion.
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub const SS58Prefix: u8 = 42;
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
    /// The basic call filter to use in dispatchable.
    type BaseCallFilter = Everything;
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = RuntimeBlockWeights;
    /// The maximum length of a block (in bytes).
    type BlockLength = RuntimeBlockLength;
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The aggregated dispatch type that is available for extrinsics.
    type Call = Call;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = Did;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = Index;
    /// The index type for blocks.
    type BlockNumber = BlockNumber;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The header type.
    type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// The ubiquitous event type.
    type Event = Event;
    /// The ubiquitous origin type.
    type Origin = Origin;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// Version of the runtime.
    type Version = Version;
    /// Converts a module to the index of the module in `construct_runtime!`.
    ///
    /// This type is being generated by `construct_runtime!`.
    type PalletInfo = PalletInfo;
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// What to do if an account is fully reaped from the system.
    type OnKilledAccount = ();
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = ();
    /// This is used as an identifier of the chain. 42 is the generic substrate prefix.
    type SS58Prefix = SS58Prefix;
    /// The set code logic, just the default since we're not a parachain.
    type OnSetCode = ();
}

impl frame_election_provider_support::onchain::Config for Runtime {
    type Accuracy = Perbill;
    type DataProvider = Staking;
}

parameter_types! {
    pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
    /// We prioritize im-online heartbeats over election solution submission.
    pub const StakingUnsignedPriority: TransactionPriority = TransactionPriority::max_value() / 2;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
    Call: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: Call,
        public: <Signature as Verify>::Signer,
        account: AccountId,
        nonce: Index,
    ) -> Option<(Call, <UncheckedExtrinsic as Extrinsic>::SignaturePayload)> {
        let tip = 0;
        // take the biggest period possible.
        let period = BlockHashCount::get()
            .checked_next_power_of_two()
            .map(|c| c / 2)
            .unwrap_or(2) as u64;
        let current_block = System::block_number()
            .saturated_into::<u64>()
            // The `System::block_number` is initialized with `n+1`,
            // so the actual block number is `n`.
            .saturating_sub(1);
        let era = Era::mortal(period, current_block);
        let extra = (
            frame_system::CheckSpecVersion::<Runtime>::new(),
            frame_system::CheckTxVersion::<Runtime>::new(),
            frame_system::CheckGenesis::<Runtime>::new(),
            frame_system::CheckEra::<Runtime>::from(era),
            frame_system::CheckNonce::<Runtime>::from(nonce),
            frame_system::CheckWeight::<Runtime>::new(),
            filter::ExtrinsicFilter::<Runtime>::new(),
            pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
        );
        let raw_payload = SignedPayload::new(call, extra).ok()?;
        let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
        let address = Self::Lookup::unlookup(account);
        let (call, extra, _) = raw_payload.deconstruct();
        Some((call, (address, signature.into(), extra)))
    }
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
    Call: From<C>,
{
    type Extrinsic = UncheckedExtrinsic;
    type OverarchingCall = Call;
}

impl frame_system::offchain::SigningTypes for Runtime {
    type Public = <Signature as Verify>::Signer;
    type Signature = Signature;
}

parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
        RuntimeBlockWeights::get().max_block;
    pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
    type Event = Event;
    type Origin = Origin;
    type PalletsOrigin = OriginCaller;
    type Call = Call;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRootOrHalfCouncil;
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = Moment;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type MaxLocks = MaxLocks;
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type Event = Event;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
}

parameter_types! {
    pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
    pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
    pub OperationalFeeMultiplier: u8 = 5;
    pub const TransactionByteFee: Balance = 10 * MILLICENTS;
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
}

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
    type TransactionByteFee = TransactionByteFee;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate =
        TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
}

parameter_types! {
    pub const AssetDeposit: Balance = 100 * DOLLARS;
    pub const ApprovalDeposit: Balance = 1 * DOLLARS;
    pub const MetadataDepositBase: Balance = 0;
    pub const MetadataDepositPerByte: Balance = 0;
}

#[cfg(not(feature = "runtime-benchmarks"))]
parameter_types! {
    pub const StringLimit: u32 = 50;
}
#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
    pub const StringLimit: u32 = 1000;
}

impl pallet_assets::Config for Runtime {
    type Event = Event;
    type Balance = Balance;
    type AssetId = AssetId;
    type Currency = Balances;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = ();
    type Extra = ();
    type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const ClassDeposit: Balance = 0;
    pub const InstanceDeposit: Balance = 0;
    pub const AttributeDepositBase: Balance = 0;
}

impl pallet_uniques::Config for Runtime {
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
    type WeightInfo = pallet_uniques::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const UncleGenerations: BlockNumber = 5;
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = (Staking, ImOnline);
}

parameter_types! {
    pub const Period: u32 = 6 * HOURS;
    pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
    type Event = Event;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    // we don't have stash and controller, thus we don't need the convert as well.
    type ValidatorIdOf = pallet_staking::StashOf<Self>;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
    // Essentially just Aura, but lets be pedantic.
    type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type WeightInfo = ();
}

parameter_types! {
    pub const MaxAuthorities: u32 = 100_000;
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = MaxAuthorities;
}

impl pallet_grandpa::Config for Runtime {
    type Event = Event;
    type Call = Call;

    type KeyOwnerProof =
        <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;

    type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        GrandpaId,
    )>>::IdentificationTuple;

    type KeyOwnerProofSystem = Historical;

    type HandleEquivocation = ();

    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
    pub const MaxKeys: u32 = 10_000;
    pub const MaxPeerInHeartbeats: u32 = 10_000;
    pub const MaxPeerDataEncodingSize: u32 = 1_000;
}

impl pallet_im_online::Config for Runtime {
    type AuthorityId = ImOnlineId;
    type MaxKeys = MaxKeys;
    type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
    type MaxPeerDataEncodingSize = MaxPeerDataEncodingSize;
    type Event = Event;
    type ValidatorSet = Historical;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type ReportUnresponsiveness = Offences;
    type UnsignedPriority = ImOnlineUnsignedPriority;
    type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
}

impl pallet_authority_discovery::Config for Runtime {
    type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
    pub const BondingDuration: pallet_staking::EraIndex = 24 * 28;
    pub const MaxNominatorRewardedPerValidator: u32 = 256;
    pub OffchainRepeat: BlockNumber = 5;
    pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
    pub const SessionsPerEra: sp_staking::SessionIndex = 6;
    pub const SlashDeferDuration: pallet_staking::EraIndex = 24 * 7; // 1/4 the bonding duration.
}

pub struct StackingEraPayout;
impl pallet_staking::EraPayout<Balance> for StackingEraPayout {
    fn era_payout(
        _total_staked: Balance,
        total_issuance: Balance,
        _era_duration_millis: u64,
    ) -> (Balance, Balance) {
        // We have 100 million tokens
        const MAX_SUPPLY: Balance = 100_000_000 * DOLLARS;
        const BASE_SUPPLY_THIS_YEAR: Balance = 70_000_000 * DOLLARS;

        // 1 era is 1 hour, so 1 year has 365.25 * 24 eras
        const YEAR: Balance = 8766;

        // We will pay out 30,000,000 to staked accounts
        const MAX_PAYOUT: Balance = 30_000_000 * DOLLARS;
        // The first year we will pay out 1/5
        // and we want to reduce the payout per year
        const CLIFF: Balance = MAX_PAYOUT / 5;
        const REST: Balance = CLIFF / YEAR;

        match total_issuance {
            _ if total_issuance >= BASE_SUPPLY_THIS_YEAR + CLIFF => (0, 0),
            _ if total_issuance >= MAX_SUPPLY => (0, 0),
            _ => (REST, REST),
        }
    }
}

impl pallet_staking::Config for Runtime {
    type Currency = Balances;
    type UnixTime = Timestamp;
    type CurrencyToVote = U128CurrencyToVote;
    type ElectionProvider = ElectionProviderMultiPhase;
    type GenesisElectionProvider =
        frame_election_provider_support::onchain::OnChainSequentialPhragmen<Runtime>;
    const MAX_NOMINATIONS: u32 = MAX_NOMINATIONS;
    type RewardRemainder = Treasury;
    type Event = Event;
    type Slash = Treasury; // send the slashed funds to the treasury.
    type Reward = (); // rewards are from the treasury.
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;
    /// A super-majority of the council can cancel the slash.
    type SlashCancelOrigin = EnsureRootOrMajoritarianCouncil;
    type SessionInterface = Self;
    type EraPayout = StackingEraPayout;
    type NextNewSession = Session;
    type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
    type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
    type SortedListProvider = pallet_staking::UseNominatorsMap<Runtime>;
    type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
}

impl pallet_offences::Config for Runtime {
    type Event = Event;
    type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
    type OnOffenceHandler = Staking;
}

impl pallet_session::historical::Config for Runtime {
    type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
    type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

parameter_types! {
    pub const CooloffPeriod: BlockNumber = 28 * DAYS;
    pub const EnactmentPeriod: BlockNumber = 30 * DAYS;
    pub const FastTrackVotingPeriod: BlockNumber = 3 * DAYS;
    pub const InstantAllowed: bool = true;
    pub const LaunchPeriod: BlockNumber = 28 * DAYS;
    pub const MaxProposals: u32 = 100;
    pub const MaxVotes: u32 = 100;
    pub const MinimumDeposit: Balance = 100 * DOLLARS;
    // One cent: $10,000 / MB
    pub const PreimageByteDeposit: Balance = 1 * CENTS;
    pub const VoteLockingPeriod: u32 = 42 * DAYS;
    pub const VotingPeriod: BlockNumber = 28 * DAYS;
}

impl pallet_democracy::Config for Runtime {
    type Proposal = Call;
    type Event = Event;
    type Currency = Balances;
    type EnactmentPeriod = EnactmentPeriod;
    type LaunchPeriod = LaunchPeriod;
    type VotingPeriod = VotingPeriod;
    type VoteLockingPeriod = VoteLockingPeriod;
    type MinimumDeposit = MinimumDeposit;
    /// A straight majority of the council can decide what their next motion is.
    type ExternalOrigin = HalfCouncil;
    /// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
    type ExternalMajorityOrigin = MajoritarianCouncil;
    /// A unanimous council can have the next scheduled referendum be a straight default-carries
    /// (NTB) vote.
    type ExternalDefaultOrigin = OverallCouncil;
    /// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
    /// be tabled immediately and with a shorter voting/enactment period.
    type FastTrackOrigin = MajoritarianCouncil;
    type InstantOrigin = OverallCouncil;
    type InstantAllowed = InstantAllowed;
    type FastTrackVotingPeriod = FastTrackVotingPeriod;
    // To cancel a proposal which has been passed, 3/4 of the council must agree to it.
    type CancellationOrigin = MajoritarianCouncil;
    type BlacklistOrigin = EnsureRoot<AccountId>;
    // To cancel a proposal before it has been passed, the technical committee must be unanimous or
    // Root must agree.
    type CancelProposalOrigin = EnsureRootOrOverallCouncil;
    // Any single technical committee member may veto a coming council proposal, however they can
    // only do it once and it lasts only for the cool-off period.
    type VetoOrigin = pallet_collective::EnsureMember<AccountId, TechnicalCollective>;
    type CooloffPeriod = CooloffPeriod;
    type PreimageByteDeposit = PreimageByteDeposit;
    type OperationalPreimageOrigin = pallet_collective::EnsureMember<AccountId, CouncilCollective>;
    type Slash = Treasury;
    type Scheduler = Scheduler;
    type PalletsOrigin = OriginCaller;
    type MaxVotes = MaxVotes;
    type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
    type MaxProposals = MaxProposals;
}

parameter_types! {
    pub const CouncilMaxMembers: u32 = 100;
    pub const CouncilMaxProposals: u32 = 100;
    pub const CouncilMotionDuration: BlockNumber = 5 * DAYS;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
    type Origin = Origin;
    type Proposal = Call;
    type Event = Event;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = CouncilMaxProposals;
    type MaxMembers = CouncilMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const TechnicalMaxMembers: u32 = 100;
    pub const TechnicalMaxProposals: u32 = 100;
    pub const TechnicalMotionDuration: BlockNumber = 5 * DAYS;
}

type TechnicalCollective = pallet_collective::Instance2;
impl pallet_collective::Config<TechnicalCollective> for Runtime {
    type Origin = Origin;
    type Proposal = Call;
    type Event = Event;
    type MotionDuration = TechnicalMotionDuration;
    type MaxProposals = TechnicalMaxProposals;
    type MaxMembers = TechnicalMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const CandidacyBond: Balance = 10 * DOLLARS;
    pub const DesiredMembers: u32 = 13;
    pub const DesiredRunnersUp: u32 = 7;
    pub const ElectionsPhragmenPalletId: LockIdentifier = *b"phrelect";
    pub const TermDuration: BlockNumber = 7 * DAYS;
    // 1 storage item created, key size is 32 bytes, value size is 16+16.
    pub const VotingBondBase: Balance = deposit(1, 64);
    // additional data per vote is 32 bytes (account id).
    pub const VotingBondFactor: Balance = deposit(0, 32);
}

// Make sure that there are no more than `MaxMembers` members elected via elections-phragmen.
// const_assert!(DesiredMembers::get() <= CouncilMaxMembers::get());

impl pallet_elections_phragmen::Config for Runtime {
    type Event = Event;
    type PalletId = ElectionsPhragmenPalletId;
    type Currency = Balances;
    type ChangeMembers = Council;
    // NOTE: this implies that council's genesis members cannot be set directly and must come from
    // this module.
    type InitializeMembers = Council;
    type CurrencyToVote = U128CurrencyToVote;
    type CandidacyBond = CandidacyBond;
    type VotingBondBase = VotingBondBase;
    type VotingBondFactor = VotingBondFactor;
    type LoserCandidate = ();
    type KickedMember = ();
    type DesiredMembers = DesiredMembers;
    type DesiredRunnersUp = DesiredRunnersUp;
    type TermDuration = TermDuration;
    type WeightInfo = pallet_elections_phragmen::weights::SubstrateWeight<Runtime>;
}

impl pallet_membership::Config<pallet_membership::Instance1> for Runtime {
    type Event = Event;
    type AddOrigin = EnsureRootOrHalfCouncil;
    type RemoveOrigin = EnsureRootOrHalfCouncil;
    type SwapOrigin = EnsureRootOrHalfCouncil;
    type ResetOrigin = EnsureRootOrHalfCouncil;
    type PrimeOrigin = EnsureRootOrHalfCouncil;
    type MembershipInitialized = TechnicalCommittee;
    type MembershipChanged = TechnicalCommittee;
    type MaxMembers = TechnicalMaxMembers;
    type WeightInfo = pallet_membership::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const Burn: Permill = Permill::from_percent(50);
    pub const MaxApprovals: u32 = 100;
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub const ProposalBondMinimum: Balance = 1 * DOLLARS;
    pub const SpendPeriod: BlockNumber = 1 * DAYS;
    pub const TreasuryPalletId: PalletId = PalletId(*names::TREASURY);
}

impl pallet_treasury::Config for Runtime {
    type Currency = Balances;
    type ApproveOrigin = EnsureRootOrPluralityCouncil;
    type RejectOrigin = EnsureRootOrHalfCouncil;
    type Event = Event;
    type OnSlash = ();
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type SpendPeriod = SpendPeriod;
    type Burn = Burn;
    type PalletId = TreasuryPalletId;
    type BurnDestination = ();
    type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
    type SpendFunds = Bounties;
    type MaxApprovals = MaxApprovals;
}

parameter_types! {
    pub const BountyCuratorDeposit: Permill = Permill::from_percent(50);
    pub const BountyDepositBase: Balance = 1 * DOLLARS;
    pub const BountyDepositPayoutDelay: BlockNumber = 1 * DAYS;
    pub const BountyUpdatePeriod: BlockNumber = 14 * DAYS;
    pub const BountyValueMinimum: Balance = 5 * DOLLARS;
    pub const MaximumReasonLength: u32 = 16384;
}

impl pallet_bounties::Config for Runtime {
    type BountyDepositBase = BountyDepositBase;
    type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
    type BountyUpdatePeriod = BountyUpdatePeriod;
    type BountyCuratorDeposit = BountyCuratorDeposit;
    type BountyValueMinimum = BountyValueMinimum;
    type DataDepositPerByte = DataDepositPerByte;
    type Event = Event;
    type MaximumReasonLength = MaximumReasonLength;
    type WeightInfo = pallet_bounties::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub ContractDeposit: Balance = deposit(
        1,
        <pallet_contracts::Pallet<Runtime>>::contract_info_size(),
    );
    // The lazy deletion runs inside on_initialize.
    pub DeletionWeightLimit: Weight = AVERAGE_ON_INITIALIZE_RATIO *
        RuntimeBlockWeights::get().max_block;
    // The weight needed for decoding the queue should be less or equal than a fifth
    // of the overall weight dedicated to the lazy deletion.
    pub DeletionQueueDepth: u32 = ((DeletionWeightLimit::get() / (
            <Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(1) -
            <Runtime as pallet_contracts::Config>::WeightInfo::on_initialize_per_queue_item(0)
        )) / 5) as u32;
    pub Schedule: pallet_contracts::Schedule<Runtime> = Default::default();
}

impl pallet_contracts::Config for Runtime {
    type Time = Timestamp;
    type Randomness = RandomnessCollectiveFlip;
    type Currency = Balances;
    type Event = Event;
    type Call = Call;
    /// The safest default is to allow no calls at all.
    ///
    /// Runtimes should whitelist dispatchables that are allowed to be called from contracts
    /// and make sure they are stable. Dispatchables exposed to contracts are not allowed to
    /// change because that would break already deployed contracts. The `Call` structure itself
    /// is not allowed to change the indices of existing pallets, too.
    type CallFilter = Nothing;
    type WeightPrice = pallet_transaction_payment::Pallet<Self>;
    type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
    type ChainExtension = ();
    type Schedule = Schedule;
    type ContractDeposit = ContractDeposit;
    type CallStack = [pallet_contracts::Frame<Self>; 31];
    type DeletionQueueDepth = DeletionQueueDepth;
    type DeletionWeightLimit = DeletionWeightLimit;
}

parameter_types! {
    // phase durations. 1/4 of the last session for each.
    pub const SignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;
    pub const UnsignedPhase: u32 = EPOCH_DURATION_IN_BLOCKS / 4;

    // signed config
    pub const SignedMaxSubmissions: u32 = 10;
    pub const SignedRewardBase: Balance = 1 * DOLLARS;
    pub const SignedDepositBase: Balance = 1 * DOLLARS;
    pub const SignedDepositByte: Balance = 1 * CENTS;

    pub const VoterSnapshotPerBlock: u32 = u32::max_value();

    pub SolutionImprovementThreshold: Perbill = Perbill::from_rational(1u32, 10_000);

    // miner configs
    pub const MultiPhaseUnsignedPriority: TransactionPriority = StakingUnsignedPriority::get() - 1u64;
    pub const MinerMaxIterations: u32 = 10;
    pub MinerMaxWeight: Weight = RuntimeBlockWeights::get()
        .get(DispatchClass::Normal)
        .max_extrinsic.expect("Normal extrinsics have a weight limit configured; qed")
        .saturating_sub(BlockExecutionWeight::get());
    // Solution can occupy 90% of normal block size
    pub MinerMaxLength: u32 = Perbill::from_rational(9u32, 10) *
        *RuntimeBlockLength::get()
        .max
        .get(DispatchClass::Normal);
}

sp_npos_elections::generate_solution_type!(
    #[compact]
    pub struct NposSolution16::<
        VoterIndex = u32,
        TargetIndex = u16,
        Accuracy = sp_runtime::PerU16,
    >(16)
);

pub const MAX_NOMINATIONS: u32 = <NposSolution16 as sp_npos_elections::NposSolution>::LIMIT as u32;

/// The numbers configured here should always be more than the the maximum limits of staking pallet
/// to ensure election snapshot will not run out of memory.
pub struct BenchmarkConfig;
impl pallet_election_provider_multi_phase::BenchmarkingConfig for BenchmarkConfig {
    const VOTERS: [u32; 2] = [5_000, 10_000];
    const TARGETS: [u32; 2] = [1_000, 2_000];
    const ACTIVE_VOTERS: [u32; 2] = [1000, 4_000];
    const DESIRED_TARGETS: [u32; 2] = [400, 800];
    const SNAPSHOT_MAXIMUM_VOTERS: u32 = 25_000;
    const MINER_MAXIMUM_VOTERS: u32 = 15_000;
    const MAXIMUM_TARGETS: u32 = 2000;
}

pub struct OnChainFallback;
impl frame_election_provider_support::ElectionProvider<AccountId, BlockNumber> for OnChainFallback {
    type Error = &'static str;
    type DataProvider = Staking;

    fn elect() -> Result<frame_election_provider_support::Supports<AccountId>, Self::Error> {
        frame_election_provider_support::onchain::OnChainSequentialPhragmen::<Runtime>::elect()
            .map_err(|_| "OnChainSequentialPhragmen failed")
    }
}

pub const MINER_MAX_ITERATIONS: u32 = 10;

pub struct OffchainRandomBalancing;
impl frame_support::pallet_prelude::Get<Option<(usize, sp_npos_elections::ExtendedBalance)>>
    for OffchainRandomBalancing
{
    fn get() -> Option<(usize, sp_npos_elections::ExtendedBalance)> {
        use sp_runtime::traits::TrailingZeroInput;
        let iters = match MINER_MAX_ITERATIONS {
            0 => 0,
            max @ _ => {
                let seed = sp_io::offchain::random_seed();
                let random = <u32>::decode(&mut TrailingZeroInput::new(&seed))
                    .expect("input is padded with zeroes; qed")
                    % max.saturating_add(1);
                random as usize
            }
        };

        Some((iters, 0))
    }
}

impl pallet_election_provider_multi_phase::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type EstimateCallFee = TransactionPayment;
    type UnsignedPhase = UnsignedPhase;
    type SignedPhase = SignedPhase;
    type SolutionImprovementThreshold = SolutionImprovementThreshold;
    type OffchainRepeat = OffchainRepeat;
    // type MinerMaxIterations = MinerMaxIterations;
    type MinerTxPriority = MultiPhaseUnsignedPriority;
    type MinerMaxWeight = MinerMaxWeight;
    type SignedMaxSubmissions = SignedMaxSubmissions;
    type SignedMaxWeight = MinerMaxWeight;
    type SignedRewardBase = SignedRewardBase;
    type SignedDepositBase = SignedDepositBase;
    type SignedDepositByte = SignedDepositByte;
    type SignedDepositWeight = ();
    type VoterSnapshotPerBlock = VoterSnapshotPerBlock;
    type SlashHandler = (); // burn slashes
    type RewardHandler = (); // nothing to do upon rewards
    type MinerMaxLength = MinerMaxLength;
    type DataProvider = Staking;
    type Solution = NposSolution16;
    // type OnChainAccuracy = Perbill;
    type Fallback = OnChainFallback;
    type Solver = frame_election_provider_support::SequentialPhragmen<
        AccountId,
        pallet_election_provider_multi_phase::SolutionAccuracyOf<Self>,
        OffchainRandomBalancing,
    >;
    type ForceOrigin = EnsureRootOrHalfCouncil;
    type BenchmarkingConfig = BenchmarkConfig;
    type WeightInfo = pallet_election_provider_multi_phase::weights::SubstrateWeight<Runtime>;
}

impl pallet_mmr::Config for Runtime {
    const INDEXING_PREFIX: &'static [u8] = b"mmr";
    type Hashing = <Runtime as frame_system::Config>::Hashing;
    type Hash = <Runtime as frame_system::Config>::Hash;
    type LeafData = frame_system::Pallet<Self>;
    type OnNewRoot = ();
    type WeightInfo = ();
}

parameter_types! {
    // One storage item; key size is 32; value is size 4+4+16+32 bytes = 56 bytes.
    pub const DepositBase: Balance = deposit(1, 88);
    // Additional storage item size of 32 bytes.
    pub const DepositFactor: Balance = deposit(0, 32);
    pub const MaxSignatories: u16 = 100;
}

impl pallet_multisig::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = MaxSignatories;
    type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

impl pallet_randomness_collective_flip::Config for Runtime {}

parameter_types! {
    pub const ConfigDepositBase: Balance = 5 * DOLLARS;
    pub const FriendDepositFactor: Balance = 50 * CENTS;
    pub const MaxFriends: u16 = 9;
    pub const RecoveryDeposit: Balance = 5 * DOLLARS;
}

impl pallet_recovery::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type Currency = Balances;
    type ConfigDepositBase = ConfigDepositBase;
    type FriendDepositFactor = FriendDepositFactor;
    type MaxFriends = MaxFriends;
    type RecoveryDeposit = RecoveryDeposit;
}

parameter_types! {
    pub const CandidateDeposit: Balance = 10 * DOLLARS;
    pub const ChallengePeriod: BlockNumber = 7 * DAYS;
    pub const MaxCandidateIntake: u32 = 10;
    pub const MaxLockDuration: BlockNumber = 36 * 30 * DAYS;
    pub const MaxStrikes: u32 = 10;
    pub const PeriodSpend: Balance = 500 * DOLLARS;
    pub const RotationPeriod: BlockNumber = 80 * HOURS;
    pub const SocietyPalletId: PalletId = PalletId(*names::SOCIETY);
    pub const WrongSideDeduction: Balance = 2 * DOLLARS;
}

impl pallet_society::Config for Runtime {
    type Event = Event;
    type PalletId = SocietyPalletId;
    type Currency = Balances;
    type Randomness = RandomnessCollectiveFlip;
    type CandidateDeposit = CandidateDeposit;
    type WrongSideDeduction = WrongSideDeduction;
    type MaxStrikes = MaxStrikes;
    type PeriodSpend = PeriodSpend;
    type MembershipChanged = ();
    type RotationPeriod = RotationPeriod;
    type MaxLockDuration = MaxLockDuration;
    type FounderSetOrigin = HalfCouncil;
    type SuspensionJudgementOrigin = pallet_society::EnsureFounder<Runtime>;
    type ChallengePeriod = ChallengePeriod;
    type MaxCandidateIntake = MaxCandidateIntake;
}

impl pallet_sudo::Config for Runtime {
    type Event = Event;
    type Call = Call;
}

parameter_types! {
    pub const DataDepositPerByte: Balance = 1 * CENTS;
    pub const TipCountdown: BlockNumber = 1 * DAYS;
    pub const TipFindersFee: Percent = Percent::from_percent(20);
    pub const TipReportDepositBase: Balance = 1 * DOLLARS;
}

impl pallet_tips::Config for Runtime {
    type Event = Event;
    type DataDepositPerByte = DataDepositPerByte;
    type MaximumReasonLength = MaximumReasonLength;
    type TipCountdown = TipCountdown;
    type TipFindersFee = TipFindersFee;
    type TipReportDepositBase = TipReportDepositBase;
    type Tippers = PhragmenElection;
    type WeightInfo = pallet_tips::weights::SubstrateWeight<Runtime>;
}

impl pallet_utility::Config for Runtime {
    type Event = Event;
    type Call = Call;
    type PalletsOrigin = OriginCaller;
    type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const MinVestedTransfer: Balance = 100 * DOLLARS;
}

impl pallet_vesting::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type BlockNumberToBalance = ConvertInto;
    type MinVestedTransfer = MinVestedTransfer;
    type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
    const MAX_VESTING_SCHEDULES: u32 = 28;
}

// Configure parami pallets.

parameter_types! {
    pub const AdPalletId: PalletId = PalletId(*names::AD);
    pub const AdvertiserMinimumFee: Balance = 50 * MILLICENTS;
    pub const PayoutBase: Balance = 1 * CENTS;
    pub const SlotLifetime: BlockNumber = 3 * DAYS;
}

impl parami_ad::Config for Runtime {
    type Event = Event;
    type Accounts = Magic;
    type Assets = Assets;
    type MinimumFeeBalance = AdvertiserMinimumFee;
    type PalletId = AdPalletId;
    type PayoutBase = PayoutBase;
    type SlotLifetime = SlotLifetime;
    type Tags = Tag;
    type CallOrigin = parami_advertiser::EnsureAdvertiser<Self>;
    type ForceOrigin = EnsureRootOrHalfCouncil;
    type WeightInfo = ();
}

parameter_types! {
    pub const AdvertiserMinimumDeposit: Balance = 1000 * DOLLARS;
    pub const AdvertiserPalletId: PalletId = PalletId(*names::ADVERTISER);
}

impl parami_advertiser::Config for Runtime {
    type Event = Event;
    type MinimumDeposit = AdvertiserMinimumDeposit;
    type PalletId = AdvertiserPalletId;
    type Slash = Treasury;
    type ForceOrigin = EnsureRootOrHalfCouncil;
    type WeightInfo = parami_advertiser::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const ChainBridgePalletId: PalletId = PalletId(*names::CHAIN_BRIDGE);
    pub const ParamiChainId: parami_chainbridge::ChainId = 233;
    pub const ProposalLifetime: BlockNumber = 50;
}

impl parami_chainbridge::Config for Runtime {
    type Event = Event;
    type AdminOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type Proposal = Call;
    type ChainId = ParamiChainId;
    type PalletId = ChainBridgePalletId;
    type ProposalLifetime = ProposalLifetime;
    type WeightInfo = parami_chainbridge::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    // &blake2_128(b"hash")
    // 0x000000000000000000000000000000f44be64d2de895454c3467021928e55ee9
    pub HashId: parami_chainbridge::ResourceId = parami_chainbridge::derive_resource_id(233, &blake2_128(b"hash"));

    // &blake2_128(b"AD3")
    // Note: Chain ID is 0 indicating this is native to another chain
    // 0x000000000000000000000000000000a56889c89dddcbb363cbd6a8d11de9e100
    pub NativeTokenId: parami_chainbridge::ResourceId = parami_chainbridge::derive_resource_id(0, &blake2_128(b"AD3"));
}

impl parami_xassets::Config for Runtime {
    type Event = Event;
    type BridgeOrigin = parami_chainbridge::EnsureBridge<Runtime>;
    type Currency = Balances;
    type HashId = HashId;
    type NativeTokenId = NativeTokenId;
    type WeightInfo = parami_xassets::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const DidPalletId: PalletId = PalletId(*names::DID);
}

impl parami_did::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type DecentralizedId = DecentralizedId;
    type Hashing = Keccak256;
    type PalletId = DidPalletId;
    type WeightInfo = parami_did::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const LinkerPalletId: PalletId = PalletId(*names::LINKER);
    pub const PendingLifetime: BlockNumber = 5;
    pub const RegistrarMinimumDeposit: Balance = 1_000_000 * DOLLARS;
    pub const UnsignedPriority: TransactionPriority = 3;
}

impl parami_linker::Config for Runtime {
    type Event = Event;
    type ForceOrigin = EnsureRootOrHalfCouncil;
    type MinimumDeposit = RegistrarMinimumDeposit;
    type PalletId = LinkerPalletId;
    type PendingLifetime = PendingLifetime;
    type Slash = Treasury;
    type Tags = Tag;
    type UnsignedPriority = UnsignedPriority;
    type WeightInfo = parami_linker::weights::SubstrateWeight<Runtime>;
}

impl parami_zkp::Config for Runtime {
    type Event = Event;
    type UnsignedPriority = UnsignedPriority;
    type WeightInfo = parami_zkp::weights::SubstrateWeight<Runtime>;
    type PalletId = LinkerPalletId;
    type PendingLifetime = PendingLifetime;
}

parameter_types! {
    pub const AutomaticDeposit: Balance = 2 * CENTS;
    pub const MagicPalletId: PalletId = PalletId(*names::MAGIC);
}

impl parami_magic::Config for Runtime {
    type Event = Event;
    type AutomaticDeposit = AutomaticDeposit;
    type Call = Call;
    type PalletId = MagicPalletId;
    type WeightInfo = parami_magic::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const InitialMintingDeposit: Balance = 1_000 * DOLLARS;
    pub const InitialMintingLockupPeriod: BlockNumber = 6 * 30 * DAYS;
    pub const InitialMintingValueBase: Balance = 1_000_000 * DOLLARS;
}

impl parami_nft::Config for Runtime {
    type Event = Event;
    type AssetId = AssetId;
    type Assets = Assets;
    type InitialMintingDeposit = InitialMintingDeposit;
    type InitialMintingLockupPeriod = InitialMintingLockupPeriod;
    type InitialMintingValueBase = InitialMintingValueBase;
    type Nft = Uniques;
    type StringLimit = StringLimit;
    type Swaps = Swap;
    type WeightInfo = parami_nft::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const InitialFarmingReward: Balance = 100 * DOLLARS;
    pub const SwapPalletId: PalletId = PalletId(*names::SWAP);
}

impl parami_swap::Config for Runtime {
    type Event = Event;
    type AssetId = AssetId;
    type Assets = Assets;
    type Currency = Balances;
    type FarmingCurve = LinearFarmingCurve<Runtime, InitialFarmingReward, InitialMintingValueBase>;
    type PalletId = SwapPalletId;
    type WeightInfo = parami_swap::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const SubmissionFee: Balance = 1 * DOLLARS;
}

impl parami_tag::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type DecentralizedId = <Self as parami_did::Config>::DecentralizedId;
    type SubmissionFee = SubmissionFee;
    type CallOrigin = parami_advertiser::EnsureAdvertiser<Self>;
    type ForceOrigin = EnsureRootOrHalfCouncil;
    type WeightInfo = parami_tag::weights::SubstrateWeight<Runtime>;
}

// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic
    {
        // System support stuff.
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>} = 0,
        Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>} = 2,
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 3,

        // Monetary stuff.
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 10,
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage} = 11,
        Assets: pallet_assets::{Pallet, Call, Storage, Config<T>, Event<T>} = 12,
        Uniques: pallet_uniques::{Pallet, Storage, Event<T>} = 13,

        // Collator support. The order of these 4 are important and shall not change.
        Authorship: pallet_authorship::{Pallet, Call, Storage} = 20,
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 22,
        Aura: pallet_aura::{Pallet, Storage, Config<T>} = 23,
        Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event, ValidateUnsigned} = 25,

        ImOnline: pallet_im_online::{Pallet, Call, Storage, Event<T>, Config<T>} = 30,
        AuthorityDiscovery: pallet_authority_discovery::{Pallet, Config} = 31,
        Staking: pallet_staking::{Pallet, Call, Config<T>, Storage, Event<T>} = 32,
        Offences: pallet_offences::{Pallet, Storage, Event} = 33,
        Historical: pallet_session::historical::{Pallet} = 34,

        // Governance stuff; uncallable initially.
        Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>} = 50,
        Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 51,
        TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>} = 52,
        PhragmenElection: pallet_elections_phragmen::{Pallet, Call, Storage, Event<T>, Config<T>} = 53,
        TechnicalMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>} = 54,
        Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>} = 55,

        // Miscellaneous.
        Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>} = 60,
        Contracts: pallet_contracts::{Pallet, Call, Storage, Event<T>} = 61,
        ElectionProviderMultiPhase: pallet_election_provider_multi_phase::{Pallet, Call, Storage, Event<T>, ValidateUnsigned} = 62,
        Mmr: pallet_mmr::{Pallet, Storage} = 63,
        Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>} = 64,
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage} = 65,
        Recovery: pallet_recovery::{Pallet, Call, Storage, Event<T>} = 66,
        Society: pallet_society::{Pallet, Call, Storage, Event<T>, Config<T>} = 67,
        Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>} = 68,
        Tips: pallet_tips::{Pallet, Call, Storage, Event<T>} = 69,
        Utility: pallet_utility::{Pallet, Call, Event} = 70,
        Vesting: pallet_vesting::{Pallet, Call, Storage, Event<T>, Config<T>} = 71,

        // Parami pallets.
        Ad: parami_ad::{Pallet, Call, Storage, Config, Event<T>} = 100,
        Advertiser: parami_advertiser::{Pallet, Call, Storage, Config<T>, Event<T>} = 101,
        ChainBridge: parami_chainbridge::{Pallet, Call, Storage, Event<T>} = 102,
        XAssets: parami_xassets::{Pallet, Call, Event<T>} = 103,
        Did: parami_did::{Pallet, Call, Storage, Config<T>, Event<T>} = 104,
        Linker: parami_linker::{Pallet, Call, Storage, Config<T>, Event<T>, ValidateUnsigned} = 105,
        Magic: parami_magic::{Pallet, Call, Storage, Config<T>, Event<T>} = 106,
        Nft: parami_nft::{Pallet, Call, Storage, Config<T>, Event<T>} = 107,
        Swap: parami_swap::{Pallet, Call, Storage, Config<T>, Event<T>} = 108,
        Tag: parami_tag::{Pallet, Call, Storage, Config<T>, Event<T>} = 109,
        Zkp: parami_zkp = 110,
    }
);

impl_runtime_apis! {
    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(Aura::slot_duration())
        }

        fn authorities() -> Vec<AuraId> {
            Aura::authorities().into_inner()
        }
    }

    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block);
        }

        fn initialize_block(header: &<Block as BlockT>::Header) {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl fg_primitives::GrandpaApi<Block> for Runtime {
        fn grandpa_authorities() -> GrandpaAuthorityList {
            Grandpa::grandpa_authorities()
        }

        fn current_set_id() -> fg_primitives::SetId {
            Grandpa::current_set_id()
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            equivocation_proof: fg_primitives::EquivocationProof<
                <Block as BlockT>::Hash,
                NumberFor<Block>,
            >,
            key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;
            Grandpa::submit_unsigned_equivocation_report(equivocation_proof, key_owner_proof)
        }

        fn generate_key_ownership_proof(
            _set_id: fg_primitives::SetId,
            authority_id: GrandpaId,
        ) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
            Historical::prove((fg_primitives::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(fg_primitives::OpaqueKeyOwnershipProof::new)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
        fn account_nonce(account: AccountId) -> Index {
            System::account_nonce(account)
        }
    }

    impl pallet_contracts_rpc_runtime_api::ContractsApi<Block, AccountId, Balance, BlockNumber, Hash>
        for Runtime
    {
        fn call(
            origin: AccountId,
            dest: AccountId,
            value: Balance,
            gas_limit: u64,
            input_data: Vec<u8>,
        ) -> pallet_contracts_primitives::ContractExecResult {
            Contracts::bare_call(origin, dest, value, gas_limit, input_data, true)
        }

        fn instantiate(
            origin: AccountId,
            endowment: Balance,
            gas_limit: u64,
            code: pallet_contracts_primitives::Code<Hash>,
            data: Vec<u8>,
            salt: Vec<u8>,
        ) -> pallet_contracts_primitives::ContractInstantiateResult<AccountId> {
            Contracts::bare_instantiate(origin, endowment, gas_limit, code, data, salt, true)
        }

        fn get_storage(
            address: AccountId,
            key: [u8; 32],
        ) -> pallet_contracts_primitives::GetStorageResult {
            Contracts::get_storage(address, key)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }

        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
    }

    impl pallet_mmr::primitives::MmrApi<Block, mmr::Hash> for Runtime {
        fn generate_proof(
            leaf_index: u64,
        ) -> Result<(mmr::EncodableOpaqueLeaf, mmr::Proof<mmr::Hash>), mmr::Error> {
            Mmr::generate_proof(leaf_index)
                .map(|(leaf, proof)| (mmr::EncodableOpaqueLeaf::from_leaf(&leaf), proof))
        }

        fn verify_proof(
            leaf: mmr::EncodableOpaqueLeaf,
            proof: mmr::Proof<mmr::Hash>,
        ) -> Result<(), mmr::Error> {
            let leaf: mmr::Leaf = leaf
                .into_opaque_leaf()
                .try_decode()
                .ok_or(mmr::Error::Verify)?;
            Mmr::verify_leaf(leaf, proof)
        }

        fn verify_proof_stateless(
            root: mmr::Hash,
            leaf: mmr::EncodableOpaqueLeaf,
            proof: mmr::Proof<mmr::Hash>,
        ) -> Result<(), mmr::Error> {
            let node = mmr::DataOrHash::Data(leaf.into_opaque_leaf());
            pallet_mmr::verify_leaf_proof::<mmr::Hashing, _>(root, node, proof)
        }
    }

    impl parami_swap_rpc_runtime_api::SwapRuntimeApi<Block, AssetId, Balance> for Runtime {
        fn dryly_add_liquidity(
            token_id: AssetId,
            currency: BalanceWrapper<Balance>,
            max_tokens: BalanceWrapper<Balance>,
        ) -> Result<(
            BalanceWrapper<Balance>,
            BalanceWrapper<Balance>,
        ), DispatchError> {
            Swap::mint_dry(token_id, currency.into(), max_tokens.into())
                .map(|(tokens, liquidity)| (tokens.into(), liquidity.into()))
        }

        fn dryly_remove_liquidity(lp_token_id: AssetId) -> Result<(
            AssetId,
            BalanceWrapper<Balance>,
            BalanceWrapper<Balance>,
            BalanceWrapper<Balance>,
        ), DispatchError> {
            Swap::burn_dry(lp_token_id).map(|(token_id, liquidity, tokens, currency)| {
                (token_id, liquidity.into(), tokens.into(), currency.into())
            })
        }

        fn dryly_buy_tokens(
            token_id: AssetId,
            tokens: BalanceWrapper<Balance>,
        ) -> Result<BalanceWrapper<Balance>, DispatchError> {
            Swap::token_out_dry(token_id, tokens.into())
                .map(|currency| currency.into())
        }

        fn dryly_sell_tokens(
            token_id: AssetId,
            tokens: BalanceWrapper<Balance>,
        ) -> Result<BalanceWrapper<Balance>, DispatchError> {
            Swap::token_in_dry(token_id, tokens.into())
                .map(|currency| currency.into())
        }

        fn dryly_sell_currency(
            token_id: AssetId,
            currency: BalanceWrapper<Balance>,
        ) -> Result<BalanceWrapper<Balance>, DispatchError> {
            Swap::quote_in_dry(token_id, currency.into())
                .map(|tokens| tokens.into())
        }

        fn dryly_buy_currency(
            token_id: AssetId,
            currency: BalanceWrapper<Balance>,
        ) -> Result<BalanceWrapper<Balance>, DispatchError> {
            Swap::quote_out_dry(token_id, currency.into())
                .map(|tokens| tokens.into())
        }

        fn calculate_reward(
            lp_token_id: AssetId,
        ) -> Result<BalanceWrapper<Balance>, DispatchError> {
            Swap::calculate_reward(lp_token_id)
                .map(|(_, reward)| reward.into())
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::{list_benchmark, baseline, Benchmarking, BenchmarkList};
            use frame_support::traits::StorageInfoTrait;
            use frame_system_benchmarking::Pallet as SystemBench;
            use baseline::Pallet as BaselineBench;

            let mut list = Vec::<BenchmarkList>::new();

            list_benchmark!(list, extra, frame_benchmarking, BaselineBench::<Runtime>);
            list_benchmark!(list, extra, frame_system, SystemBench::<Runtime>);
            list_benchmark!(list, extra, pallet_balances, Balances);
            list_benchmark!(list, extra, pallet_timestamp, Timestamp);

            list_benchmark!(list, extra, parami_ad, Ad);
            list_benchmark!(list, extra, parami_advertiser, Advertiser);
            list_benchmark!(list, extra, parami_did, Did);
            list_benchmark!(list, extra, parami_linker, Linker);
            list_benchmark!(list, extra, parami_magic, Magic);
            list_benchmark!(list, extra, parami_nft, Nft);
            list_benchmark!(list, extra, parami_swap, Swap);
            list_benchmark!(list, extra, parami_tag, Tag);
            list_benchmark!(list, extra, parami_zkp, Zkp);
            let storage_info = AllPalletsWithSystem::storage_info();

            return (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{baseline, Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

            use frame_system_benchmarking::Pallet as SystemBench;
            use baseline::Pallet as BaselineBench;

            impl frame_system_benchmarking::Config for Runtime {}
            impl baseline::Config for Runtime {}

            let whitelist: Vec<TrackedStorageKey> = vec![
                // Block Number
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
                // Total Issuance
                hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
                // Execution Phase
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
                // Event Count
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
                // System Events
                hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
            ];

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);

            add_benchmark!(params, batches, frame_benchmarking, BaselineBench::<Runtime>);
            add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
            add_benchmark!(params, batches, pallet_balances, Balances);
            add_benchmark!(params, batches, pallet_timestamp, Timestamp);

            add_benchmark!(params, batches, parami_ad, Ad);
            add_benchmark!(params, batches, parami_advertiser, Advertiser);
            add_benchmark!(params, batches, parami_did, Did);
            add_benchmark!(params, batches, parami_linker, Linker);
            add_benchmark!(params, batches, parami_magic, Magic);
            add_benchmark!(params, batches, parami_nft, Nft);
            add_benchmark!(params, batches, parami_swap, Swap);
            add_benchmark!(params, batches, parami_tag, Tag);
            add_benchmark!(params, batches, parami_zkp, Zkp);

            Ok(batches)
        }
    }
}
