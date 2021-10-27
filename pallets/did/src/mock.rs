use crate as parami_did;
use frame_support::{parameter_types, traits::GenesisBuild};
use frame_system as system;
use sp_core::{sr25519, H256};
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, Keccak256},
};

type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
type Block = system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: system::{Pallet, Call, Config, Storage, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},

        Did: parami_did::{Pallet, Call, Storage, Event<T>},
    }
);

pub type DID = <Test as parami_did::Config>::DecentralizedId;
type Moment = u64;

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
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = sr25519::Public;
    type Lookup = Did;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
}

parameter_types! {
    pub const MinimumPeriod: Moment = 1;
}

impl pallet_timestamp::Config for Test {
    type Moment = Moment;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

impl parami_did::Config for Test {
    type Event = Event;
    type DecentralizedId = sp_core::H160;
    type Hashing = Keccak256;
    type Time = Timestamp;
    type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let alice = sr25519::Public([1; 32]);

    let mut t = system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    parami_did::GenesisConfig::<Test> {
        ids: vec![(alice, DID::from_slice(&[0xff; 20]))],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}
