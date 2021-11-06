#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use codec::{Decode, Encode};
use parami_swap_rpc_runtime_api::BalanceWrapper;
use parami_traits::Swaps;
use sp_api::impl_runtime_apis;
use sp_core::{
    crypto::KeyTypeId,
    u32_trait::{_1, _2, _3, _4, _5},
    OpaqueMetadata,
};
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_io::hashing::blake2_128;
#[cfg(any(feature = "std", test))]
use sp_runtime::BuildStorage;
use sp_runtime::{
    create_runtime_str,
    curve::PiecewiseLinear,
    generic, impl_opaque_keys,
    traits::{
        BlakeTwo256, Block as BlockT, ConvertInto, Extrinsic, Keccak256, NumberFor, OpaqueKeys,
        SaturatedConversion, StaticLookup, Verify,
    },
    transaction_validity::{TransactionPriority, TransactionSource, TransactionValidity},
    ApplyExtrinsicResult, FixedPointNumber, Perbill, Percent, Permill, Perquintill,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use frame_election_provider_support::{onchain, ElectionProvider, Supports};
use frame_support::{
    construct_runtime, parameter_types,
    traits::{
        Contains, Currency, Imbalance, KeyOwnerProofSystem, LockIdentifier, OnUnbalanced,
        U128CurrencyToVote,
    },
    weights::{
        constants::{BlockExecutionWeight, RocksDbWeight, WEIGHT_PER_SECOND},
        DispatchClass, IdentityFee, Weight,
    },
    Blake2_256, PalletId,
};
use frame_system::{EnsureOneOf, EnsureRoot};
use pallet_contracts::weights::WeightInfo;
use pallet_grandpa::{
    fg_primitives, AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
#[cfg(any(feature = "std", test))]
pub use pallet_staking::StakerStatus;
use pallet_transaction_payment::{CurrencyAdapter, Multiplier, TargetedFeeAdjustment};
pub use parami_primitives::{
    constants::{
        CENTS, DAYS, DOLLARS, EPOCH_DURATION_IN_BLOCKS, EPOCH_DURATION_IN_SLOTS, HOURS, MILLICENTS,
        MILLISECS_PER_BLOCK, MINUTES, PRIMARY_PROBABILITY, SLOT_DURATION,
    },
    deposit, AccountId, AssetId, Balance, BlockNumber, Hash, Index, Moment, Signature,
};

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

    impl_opaque_keys! {
        pub struct SessionKeys {
            pub authority_discovery: AuthorityDiscovery,
            pub babe: Babe,
            pub grandpa: Grandpa,
            pub im_online: ImOnline,
        }
    }
}

/// The BABE epoch configuration at genesis.
pub const BABE_GENESIS_EPOCH_CONFIG: sp_consensus_babe::BabeEpochConfiguration =
    sp_consensus_babe::BabeEpochConfiguration {
        c: PRIMARY_PROBABILITY,
        allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
    };

// To learn more about runtime versioning and what each of the following value means:
//   https://substrate.dev/docs/en/knowledgebase/runtime/upgrades#runtime-versioning
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("parami"),
    impl_name: create_runtime_str!("parami-node"),
    authoring_version: 20,
    // The version of the runtime specification. A full node will not attempt to use its native
    //   runtime in substitute for the on-chain Wasm runtime unless all of `spec_name`,
    //   `spec_version`, and `authoring_version` are the same between Wasm and native.
    spec_version: 300,
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

impl frame_election_provider_support::onchain::Config for Runtime {
    type Accuracy = Perbill;
    type DataProvider = Staking;
}

const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
    pub const BlockHashCount: BlockNumber = 2400;
    pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights::with_sensible_defaults(2 * WEIGHT_PER_SECOND, NORMAL_DISPATCH_RATIO);
    pub const SS58Prefix: u8 = 42;
    pub const Version: RuntimeVersion = VERSION;
}

pub struct ParamiBaseCallFilter;
impl Contains<Call> for ParamiBaseCallFilter {
    fn contains(c: &Call) -> bool {
        match c {
            Call::Assets(pallet_assets::Call::create { .. }) => false,
            _ => true,
        }
    }
}

impl frame_system::Config for Runtime {
    /// The basic call filter to use in dispatchable.
    type BaseCallFilter = ParamiBaseCallFilter;
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = BlockWeights;
    /// The maximum length of a block (in bytes).
    type BlockLength = BlockLength;
    /// The ubiquitous origin type.
    type Origin = Origin;
    /// The aggregated dispatch type that is available for extrinsics.
    type Call = Call;
    /// The index type for storing how many extrinsics an account has signed.
    type Index = Index;
    /// The index type for blocks.
    type BlockNumber = BlockNumber;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = Did;
    /// The header type.
    type Header = Header;
    /// The ubiquitous event type.
    type Event = Event;
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
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// What to do if an account is fully reaped from the system.
    type OnKilledAccount = ();
    /// Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = frame_system::weights::SubstrateWeight<Runtime>;
    /// This is used as an identifier of the chain. 42 is the generic substrate prefix.
    type SS58Prefix = SS58Prefix;
    /// The set code logic, just the default since we're not a parachain.
    type OnSetCode = ();
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
            pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
        );
        let raw_payload = SignedPayload::new(call, extra).ok()?;
        let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
        let address = Did::unlookup(account);
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
    pub const MaxAuthorities: u32 = 100;
}

impl pallet_authority_discovery::Config for Runtime {
    type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
    pub const UncleGenerations: BlockNumber = 5;
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
    type UncleGenerations = UncleGenerations;
    type FilterUncle = ();
    type EventHandler = (Staking, ImOnline);
}

parameter_types! {
    // NOTE: Currently it is not possible to change the epoch duration after the chain has started.
    //       Attempting to do so will brick block production.
    pub const EpochDuration: Moment = EPOCH_DURATION_IN_SLOTS;
    pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
    pub const ReportLongevity: Moment = BondingDuration::get() as Moment * SessionsPerEra::get()  as Moment * EpochDuration::get() as Moment;
}

impl pallet_babe::Config for Runtime {
    type EpochDuration = EpochDuration;
    type ExpectedBlockTime = ExpectedBlockTime;
    type EpochChangeTrigger = pallet_babe::ExternalTrigger;
    type DisabledValidators = Session;

    type KeyOwnerProofSystem = Historical;

    type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        pallet_babe::AuthorityId,
    )>>::Proof;

    type KeyOwnerIdentification = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(
        KeyTypeId,
        pallet_babe::AuthorityId,
    )>>::IdentificationTuple;

    type HandleEquivocation =
        pallet_babe::EquivocationHandler<Self::KeyOwnerIdentification, Offences, ReportLongevity>;

    type WeightInfo = ();
    type MaxAuthorities = MaxAuthorities;
}

parameter_types! {
    // NOTE: minimum balance is 1 cent, 0.01 dollar
    pub const ExistentialDeposit: Balance = CENTS;
    // For weight estimation, we assume that the most locks on an individual account will be 50.
    // This number may need to be adjusted in the future if this assumption no longer holds true.
    pub const MaxLocks: u32 = 50;
    pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
    type Balance = Balance;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = frame_system::Pallet<Runtime>;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type MaxLocks = MaxLocks;
    type MaxReserves = MaxReserves;
    type ReserveIdentifier = [u8; 8];
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

const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);

parameter_types! {
    pub TombstoneDeposit: Balance = deposit(
        1,
        <pallet_contracts::Pallet<Runtime>>::contract_info_size().into(),
    );
    pub DepositPerContract: Balance = TombstoneDeposit::get();
    pub const DepositPerStorageByte: Balance = deposit(0, 1);
    pub const DepositPerStorageItem: Balance = deposit(1, 0);
    pub RentFraction: Perbill = Perbill::from_rational(1u32, 30 * DAYS);
    pub const SurchargeReward: Balance = 150 * MILLICENTS;
    pub const SignedClaimHandicap: u32 = 2;
    pub const MaxValueSize: u32 = 16 * 1024;
    // The lazy deletion runs inside on_initialize.
    pub DeletionWeightLimit: Weight = AVERAGE_ON_INITIALIZE_RATIO * BlockWeights::get().max_block;
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
    type CallFilter = frame_support::traits::Nothing;
    type WeightPrice = pallet_transaction_payment::Pallet<Self>;
    type WeightInfo = pallet_contracts::weights::SubstrateWeight<Self>;
    type ChainExtension = ();
    type Schedule = Schedule;
    type ContractDeposit = DepositPerContract;
    type CallStack = [pallet_contracts::Frame<Self>; 31];
    type DeletionQueueDepth = DeletionQueueDepth;
    type DeletionWeightLimit = DeletionWeightLimit;
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
    pub MinerMaxWeight: Weight = BlockWeights::get()
        .get(DispatchClass::Normal)
        .max_extrinsic.expect("Normal extrinsics have a weight limit configured; qed")
        .saturating_sub(BlockExecutionWeight::get());
    // Solution can occupy 90% of normal block size
    pub MinerMaxLength: u32 = Perbill::from_rational(9u32, 10) *
        *BlockLength::get()
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
impl ElectionProvider<AccountId, BlockNumber> for OnChainFallback {
    type Error = &'static str;
    type DataProvider = Staking;

    fn elect() -> Result<Supports<AccountId>, Self::Error> {
        onchain::OnChainSequentialPhragmen::<Runtime>::elect()
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

    type HandleEquivocation = pallet_grandpa::EquivocationHandler<
        Self::KeyOwnerIdentification,
        Offences,
        ReportLongevity,
    >;

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
    type NextSessionRotation = Babe;
    type ReportUnresponsiveness = Offences;
    type UnsignedPriority = ImOnlineUnsignedPriority;
    type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
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

impl pallet_offences::Config for Runtime {
    type Event = Event;
    type IdentificationTuple = pallet_session::historical::IdentificationTuple<Self>;
    type OnOffenceHandler = Staking;
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
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
    pub const MaxScheduledPerBlock: u32 = 50;
}

impl pallet_scheduler::Config for Runtime {
    type Event = Event;
    type Origin = Origin;
    type PalletsOrigin = OriginCaller;
    type Call = Call;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRootOrHalfCouncil;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type WeightInfo = pallet_scheduler::weights::SubstrateWeight<Runtime>;
}

impl pallet_session::Config for Runtime {
    type Event = Event;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = pallet_staking::StashOf<Self>;
    type ShouldEndSession = Babe;
    type NextSessionRotation = Babe;
    type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
    type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
    type Keys = opaque::SessionKeys;
    type WeightInfo = pallet_session::weights::SubstrateWeight<Runtime>;
}

impl pallet_session::historical::Config for Runtime {
    type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
    type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

parameter_types! {
    pub const CandidateDeposit: Balance = 10 * DOLLARS;
    pub const ChallengePeriod: BlockNumber = 7 * DAYS;
    pub const MaxCandidateIntake: u32 = 10;
    pub const MaxLockDuration: BlockNumber = 36 * 30 * DAYS;
    pub const MaxStrikes: u32 = 10;
    pub const PeriodSpend: Balance = 500 * DOLLARS;
    pub const RotationPeriod: BlockNumber = 80 * HOURS;
    pub const SocietyPalletId: PalletId = PalletId(*b"py/socie");
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

pallet_staking_reward_curve::build! {
    const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
        min_inflation: 0_025_000,
        max_inflation: 0_100_000,
        ideal_stake: 0_500_000,
        falloff: 0_050_000,
        max_piece_count: 40,
        test_precision: 0_005_000,
    );
}

parameter_types! {
    pub const BondingDuration: pallet_staking::EraIndex = 24 * 28;
    pub const MaxNominatorRewardedPerValidator: u32 = 256;
    pub OffchainRepeat: BlockNumber = 5;
    pub const OffendingValidatorsThreshold: Perbill = Perbill::from_percent(17);
    pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
    pub const SessionsPerEra: sp_staking::SessionIndex = 6;
    pub const SlashDeferDuration: pallet_staking::EraIndex = 24 * 7; // 1/4 the bonding duration.
}

impl pallet_staking::Config for Runtime {
    type Currency = Balances;
    type UnixTime = Timestamp;
    type CurrencyToVote = U128CurrencyToVote;
    type ElectionProvider = ElectionProviderMultiPhase;
    type GenesisElectionProvider = onchain::OnChainSequentialPhragmen<Runtime>;
    const MAX_NOMINATIONS: u32 = MAX_NOMINATIONS;
    type RewardRemainder = Treasury;
    type Event = Event;
    type Slash = Treasury; // send the slashed funds to the treasury.
    type Reward = (); // rewards are minted from the void
    type SessionsPerEra = SessionsPerEra;
    type BondingDuration = BondingDuration;
    type SlashDeferDuration = SlashDeferDuration;
    /// A super-majority of the council can cancel the slash.
    type SlashCancelOrigin = EnsureRootOrMajoritarianCouncil;
    type SessionInterface = Self;
    type EraPayout = pallet_staking::ConvertCurve<RewardCurve>;
    type NextNewSession = Session;
    type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
    type OffendingValidatorsThreshold = OffendingValidatorsThreshold;
    type SortedListProvider = pallet_staking::UseNominatorsMap<Runtime>;
    type WeightInfo = pallet_staking::weights::SubstrateWeight<Runtime>;
}

impl pallet_sudo::Config for Runtime {
    type Event = Event;
    type Call = Call;
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
    type Moment = Moment;
    type OnTimestampSet = Babe;
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
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
    type Tippers = Elections;
    type WeightInfo = pallet_tips::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub AdjustmentVariable: Multiplier = Multiplier::saturating_from_rational(1, 100_000);
    pub MinimumMultiplier: Multiplier = Multiplier::saturating_from_rational(1, 1_000_000_000u128);
    pub OperationalFeeMultiplier: u8 = 5;
    pub const TransactionByteFee: Balance = 10 * MILLICENTS;
    pub const TargetBlockFullness: Perquintill = Perquintill::from_percent(25);
}

type NegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;

pub struct Author;
impl OnUnbalanced<NegativeImbalance> for Author {
    fn on_nonzero_unbalanced(amount: NegativeImbalance) {
        Balances::resolve_creating(&Authorship::author(), amount);
    }
}

pub struct DealWithFees;
impl OnUnbalanced<NegativeImbalance> for DealWithFees {
    fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = NegativeImbalance>) {
        if let Some(fees) = fees_then_tips.next() {
            // for fees, 80% to treasury, 20% to author
            let mut split = fees.ration(80, 20);
            if let Some(tips) = fees_then_tips.next() {
                // for tips, if any, 80% to treasury, 20% to author (though this can be anything)
                tips.ration_merge_into(80, 20, &mut split);
            }

            Treasury::on_unbalanced(split.0);
            Author::on_unbalanced(split.1);
        }
    }
}

impl pallet_transaction_payment::Config for Runtime {
    type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees>;
    type TransactionByteFee = TransactionByteFee;
    type OperationalFeeMultiplier = OperationalFeeMultiplier;
    type WeightToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate =
        TargetedFeeAdjustment<Self, TargetBlockFullness, AdjustmentVariable, MinimumMultiplier>;
}

parameter_types! {
    pub const Burn: Permill = Permill::from_percent(50);
    pub const MaxApprovals: u32 = 100;
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub const ProposalBondMinimum: Balance = 1 * DOLLARS;
    pub const SpendPeriod: BlockNumber = 1 * DAYS;
    pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
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

impl pallet_utility::Config for Runtime {
    type Event = Event;
    type Call = Call;
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

parameter_types! {
    pub const AdPalletId: PalletId = PalletId(*b"prm/ad  ");
}

impl parami_ad::Config for Runtime {
    type Event = Event;
    type Assets = Assets;
    type PalletId = AdPalletId;
    type Swaps = Swap;
    type TagsStore = parami_tag::Pallet<Self>;
    type CallOrigin = parami_advertiser::EnsureAdvertiser<Self>;
    type ForceOrigin = EnsureRootOrHalfCouncil;
    type WeightInfo = ();
}

parameter_types! {
    pub const MinimalDeposit: Balance = 10 * DOLLARS;
    pub const AdvertiserPalletId: PalletId = PalletId(*b"prm/ader");
}

impl parami_advertiser::Config for Runtime {
    type Event = Event;
    type MinimalDeposit = MinimalDeposit;
    type PalletId = AdvertiserPalletId;
    type Slash = Treasury;
    type ForceOrigin = EnsureRootOrHalfCouncil;
    type WeightInfo = parami_advertiser::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const ChainBridgePalletId: PalletId = PalletId(*b"chnbrdge");
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
    pub HashId: parami_chainbridge::ResourceId = parami_chainbridge::derive_resource_id(233, &blake2_128(b"hash"));

    // &blake2_128(b"AD3")
    // Note: Chain ID is 0 indicating this is native to another chain
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
    pub const CreationDeposit: Balance = 1 * DOLLARS;
    pub const DidPalletId: PalletId = PalletId(*b"prm/did ");
}

impl parami_did::Config for Runtime {
    type Event = Event;
    type AssetId = AssetId;
    type CreationDeposit = CreationDeposit;
    type Currency = Balances;
    type DecentralizedId = sp_core::H160;
    type Hashing = Keccak256;
    type PalletId = DidPalletId;
    type WeightInfo = parami_did::weights::SubstrateWeight<Runtime>;
}

impl parami_linker::Config for Runtime {
    type Event = Event;
    type WeightInfo = parami_linker::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const CreationFee: Balance = 50 * CENTS;
    pub const MagicPalletId: PalletId = PalletId(*b"prm/stab");
}

impl parami_magic::Config for Runtime {
    type Event = Event;
    type Currency = Balances;
    type Call = Call;
    type CreationFee = CreationFee;
    type PalletId = MagicPalletId;
    type WeightInfo = parami_magic::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const MaxClassMetadata: u32 = 256;
    pub const MaxTokenMetadata: u32 = 256;
}

impl orml_nft::Config for Runtime {
    type ClassId = AssetId;
    type TokenId = AssetId;
    type ClassData = ();
    type TokenData = ();
    type MaxClassMetadata = MaxClassMetadata;
    type MaxTokenMetadata = MaxTokenMetadata;
}

parameter_types! {
    pub const InitialMintingDeposit: Balance = 1_000 * DOLLARS;
    pub const InitialMintingValueBase: Balance = 1_000_000 * DOLLARS;
}

impl parami_nft::Config for Runtime {
    type Event = Event;
    type Assets = Assets;
    type InitialMintingValueBase = InitialMintingValueBase;
    type InitialMintingDeposit = InitialMintingDeposit;
    type StringLimit = StringLimit;
    type Swaps = Swap;
    type WeightInfo = parami_nft::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const SwapPalletId: PalletId = PalletId(*b"prm/swap");
}

impl parami_swap::Config for Runtime {
    type Event = Event;
    type AssetId = AssetId;
    type Assets = Assets;
    type Currency = Balances;
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
    type Hashing = Blake2_256;
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
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},

        Assets: pallet_assets::{Pallet, Call, Storage, Config<T>, Event<T>},
        AuthorityDiscovery: pallet_authority_discovery::{Pallet, Config},
        Authorship: pallet_authorship::{Pallet, Call, Storage, Inherent},
        Babe: pallet_babe::{Pallet, Call, Storage, Config, ValidateUnsigned},
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Bounties: pallet_bounties::{Pallet, Call, Storage, Event<T>},
        Contracts: pallet_contracts::{Pallet, Call, Storage, Event<T>},
        Council: pallet_collective::<Instance1>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
        Democracy: pallet_democracy::{Pallet, Call, Storage, Config<T>, Event<T>},
        ElectionProviderMultiPhase: pallet_election_provider_multi_phase::{Pallet, Call, Storage, Event<T>, ValidateUnsigned},
        Elections: pallet_elections_phragmen::{Pallet, Call, Storage, Event<T>, Config<T>},
        Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event, ValidateUnsigned},
        Historical: pallet_session::historical::{Pallet},
        ImOnline: pallet_im_online::{Pallet, Call, Storage, Event<T>, ValidateUnsigned, Config<T>},
        Mmr: pallet_mmr::{Pallet, Storage},
        Multisig: pallet_multisig::{Pallet, Call, Storage, Event<T>},
        Offences: pallet_offences::{Pallet, Storage, Event},
        RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Storage},
        Recovery: pallet_recovery::{Pallet, Call, Storage, Event<T>},
        Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
        Society: pallet_society::{Pallet, Call, Storage, Event<T>, Config<T>},
        Staking: pallet_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
        Sudo: pallet_sudo::{Pallet, Call, Config<T>, Storage, Event<T>},
        TechnicalCommittee: pallet_collective::<Instance2>::{Pallet, Call, Storage, Origin<T>, Event<T>, Config<T>},
        TechnicalMembership: pallet_membership::<Instance1>::{Pallet, Call, Storage, Event<T>, Config<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Tips: pallet_tips::{Pallet, Call, Storage, Event<T>},
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage},
        Treasury: pallet_treasury::{Pallet, Call, Storage, Config, Event<T>},
        Utility: pallet_utility::{Pallet, Call, Event},
        Vesting: pallet_vesting::{Pallet, Call, Storage, Event<T>, Config<T>},

        OrmlNft: orml_nft::{Pallet, Storage, Config<T>} = 100,

        Ad: parami_ad::{Pallet, Call, Storage, Event<T>},
        Advertiser: parami_advertiser::{Pallet, Call, Storage, Config<T>, Event<T>},
        ChainBridge: parami_chainbridge::{Pallet, Call, Storage, Event<T>},
        XAssets: parami_xassets::{Pallet, Call, Event<T>},
        Did: parami_did::{Pallet, Call, Storage, Config<T>, Event<T>},
        Linker: parami_linker::{Pallet, Call, Storage, Event<T>},
        Magic: parami_magic::{Pallet, Call, Storage, Config<T>, Event<T>},
        Nft: parami_nft::{Pallet, Call, Storage, Config<T>, Event<T>},
        Swap: parami_swap::{Pallet, Call, Storage, Config<T>, Event<T>},
        Tag: parami_tag::{Pallet, Call, Storage, Config<T>, Event<T>},
    }
);

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// Block Id type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
);
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// The payload being signed in transactions.
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPallets,
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

impl_runtime_apis! {
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

        fn inherent_extrinsics(data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(block: Block, data: InherentData) -> CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_consensus_babe::BabeApi<Block> for Runtime {
        fn configuration() -> sp_consensus_babe::BabeGenesisConfiguration {
            // The choice of `c` parameter (where `1 - c` represents the
            // probability of a slot being empty), is done in accordance to the
            // slot duration and expected target block time, for safely
            // resisting network delays of maximum two seconds.
            // <https://research.web3.foundation/en/latest/polkadot/BABE/Babe/#6-practical-results>
            sp_consensus_babe::BabeGenesisConfiguration {
                slot_duration: Babe::slot_duration(),
                epoch_length: EpochDuration::get(),
                c: BABE_GENESIS_EPOCH_CONFIG.c,
                genesis_authorities: Babe::authorities().to_vec(),
                randomness: Babe::randomness(),
                allowed_slots: BABE_GENESIS_EPOCH_CONFIG.allowed_slots,
            }
        }

        fn current_epoch_start() -> sp_consensus_babe::Slot {
            Babe::current_epoch_start()
        }

        fn current_epoch() -> sp_consensus_babe::Epoch {
            Babe::current_epoch()
        }

        fn next_epoch() -> sp_consensus_babe::Epoch {
            Babe::next_epoch()
        }

        fn generate_key_ownership_proof(
            _slot: sp_consensus_babe::Slot,
            authority_id: sp_consensus_babe::AuthorityId,
        ) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
            Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
                .map(|p| p.encode())
                .map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
        }

        fn submit_report_equivocation_unsigned_extrinsic(
            equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
            key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
        ) -> Option<()> {
            let key_owner_proof = key_owner_proof.decode()?;
            Babe::submit_unsigned_equivocation_report(equivocation_proof, key_owner_proof)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            opaque::SessionKeys::generate(seed)
        }

        fn decode_session_keys(encoded: Vec<u8>) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
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
            tokens: BalanceWrapper<Balance>,
        ) -> Option<(
            AssetId,
            BalanceWrapper<Balance>,
            AssetId,
            BalanceWrapper<Balance>,
        )> {
            Swap::mint_dry(token_id, currency.into(), tokens.into())
                .map(|(token_id, tokens, lp_token_id, liquidity)| {
                    (token_id, tokens.into(), lp_token_id, liquidity.into())
                })
                .ok()
        }

        fn dryly_remove_liquidity(
            token_id: AssetId,
            liquidity: BalanceWrapper<Balance>,
        ) -> Option<(
            AssetId,
            BalanceWrapper<Balance>,
            AssetId,
            BalanceWrapper<Balance>,
        )> {
            Swap::burn_dry(token_id, liquidity.into())
                .map(|(token_id, tokens, lp_token_id, currency)| {
                    (token_id, tokens.into(), lp_token_id, currency.into())
                })
                .ok()
        }

        fn dryly_buy_tokens(
            token_id: AssetId,
            tokens: BalanceWrapper<Balance>,
        ) -> Option<BalanceWrapper<Balance>> {
            Swap::token_out_dry(token_id, tokens.into())
                .map(|currency| currency.into())
                .ok()
        }

        fn dryly_sell_tokens(
            token_id: AssetId,
            tokens: BalanceWrapper<Balance>,
        ) -> Option<BalanceWrapper<Balance>> {
            Swap::token_in_dry(token_id, tokens.into())
                .map(|currency| currency.into())
                .ok()
        }

        fn dryly_sell_currency(
            token_id: AssetId,
            currency: BalanceWrapper<Balance>,
        ) -> Option<BalanceWrapper<Balance>> {
            Swap::quote_in_dry(token_id, currency.into())
                .map(|tokens| tokens.into())
                .ok()
        }

        fn dryly_buy_currency(
            token_id: AssetId,
            currency: BalanceWrapper<Balance>,
        ) -> Option<BalanceWrapper<Balance>> {
            Swap::quote_out_dry(token_id, currency.into())
                .map(|tokens| tokens.into())
                .ok()
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::{list_benchmark, BenchmarkList, Benchmarking};
            use frame_support::traits::StorageInfoTrait;
            use frame_system_benchmarking::Pallet as SystemBench;

            let mut list = Vec::<BenchmarkList>::new();

            list_benchmark!(list, extra, frame_system, SystemBench::<Runtime>);
            list_benchmark!(list, extra, pallet_assets, Assets);
            list_benchmark!(list, extra, pallet_babe, Babe);
            list_benchmark!(list, extra, pallet_balances, Balances);
            list_benchmark!(list, extra, pallet_bounties, Bounties);
            list_benchmark!(list, extra, pallet_collective, Council);
            list_benchmark!(list, extra, pallet_contracts, Contracts);
            list_benchmark!(list, extra, pallet_democracy, Democracy);
            list_benchmark!(list, extra, pallet_election_provider_multi_phase, ElectionProviderMultiPhase);
            list_benchmark!(list, extra, pallet_elections_phragmen, Elections);
            list_benchmark!(list, extra, pallet_grandpa, Grandpa);
            list_benchmark!(list, extra, pallet_im_online, ImOnline);
            list_benchmark!(list, extra, pallet_membership, TechnicalMembership);
            // list_benchmark!(list, extra, pallet_lottery, Lottery);
            list_benchmark!(list, extra, pallet_mmr, Mmr);
            list_benchmark!(list, extra, pallet_multisig, Multisig);
            // list_benchmark!(list, extra, pallet_offences, OffencesBench::<Runtime>);
            list_benchmark!(list, extra, pallet_scheduler, Scheduler);
            // list_benchmark!(list, extra, pallet_session, SessionBench::<Runtime>);
            // list_benchmark!(list, extra, pallet_society, Society);
            list_benchmark!(list, extra, pallet_staking, Staking);
            list_benchmark!(list, extra, pallet_timestamp, Timestamp);
            list_benchmark!(list, extra, pallet_tips, Tips);
            list_benchmark!(list, extra, pallet_treasury, Treasury);
            list_benchmark!(list, extra, pallet_utility, Utility);
            list_benchmark!(list, extra, pallet_vesting, Vesting);

            list_benchmark!(list, extra, parami_ad, Ad);
            list_benchmark!(list, extra, parami_advertiser, Advertiser);
            list_benchmark!(list, extra, parami_did, Did);
            list_benchmark!(list, extra, parami_magic, Magic);
            list_benchmark!(list, extra, parami_nft, Nft);
            list_benchmark!(list, extra, parami_swap, Swap);
            list_benchmark!(list, extra, parami_tag, Tag);

            let storage_info = AllPalletsWithSystem::storage_info();

            (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig,
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{add_benchmark, BenchmarkBatch, Benchmarking, TrackedStorageKey};
            use frame_system_benchmarking::Pallet as SystemBench;
            impl frame_system_benchmarking::Config for Runtime {}

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

            add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
            add_benchmark!(params, batches, pallet_assets, Assets);
            add_benchmark!(params, batches, pallet_babe, Babe);
            add_benchmark!(params, batches, pallet_balances, Balances);
            add_benchmark!(params, batches, pallet_bounties, Bounties);
            add_benchmark!(params, batches, pallet_collective, Council);
            add_benchmark!(params, batches, pallet_contracts, Contracts);
            add_benchmark!(params, batches, pallet_democracy, Democracy);
            add_benchmark!(params, batches, pallet_election_provider_multi_phase, ElectionProviderMultiPhase);
            add_benchmark!(params, batches, pallet_elections_phragmen, Elections);
            add_benchmark!(params, batches, pallet_grandpa, Grandpa);
            add_benchmark!(params, batches, pallet_im_online, ImOnline);
            // add_benchmark!(params, batches, pallet_lottery, Lottery);
            add_benchmark!(params, batches, pallet_membership, TechnicalMembership);
            add_benchmark!(params, batches, pallet_mmr, Mmr);
            add_benchmark!(params, batches, pallet_multisig, Multisig);
            // add_benchmark!(params, batches, pallet_offences, OffencesBench::<Runtime>);
            add_benchmark!(params, batches, pallet_scheduler, Scheduler);
            // add_benchmark!(params, batches, pallet_session, SessionBench::<Runtime>);
            // add_benchmark!(params, batches, pallet_society, Society);
            add_benchmark!(params, batches, pallet_staking, Staking);
            add_benchmark!(params, batches, pallet_timestamp, Timestamp);
            add_benchmark!(params, batches, pallet_tips, Tips);
            add_benchmark!(params, batches, pallet_treasury, Treasury);
            add_benchmark!(params, batches, pallet_utility, Utility);
            add_benchmark!(params, batches, pallet_vesting, Vesting);

            add_benchmark!(params, batches, parami_ad, Ad);
            add_benchmark!(params, batches, parami_advertiser, Advertiser);
            add_benchmark!(params, batches, parami_did, Did);
            add_benchmark!(params, batches, parami_magic, Magic);
            add_benchmark!(params, batches, parami_nft, Nft);
            add_benchmark!(params, batches, parami_swap, Swap);
            add_benchmark!(params, batches, parami_tag, Tag);

            if batches.is_empty() {
                return Err("Benchmark not found for this pallet.".into());
            }

            Ok(batches)
        }
    }
}
