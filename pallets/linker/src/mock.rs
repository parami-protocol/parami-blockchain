use crate as parami_linker;
use frame_support::{parameter_types, traits::EnsureOrigin};
use frame_system::{self as system};
use sp_core::{
    sr25519::{self, Signature},
    H160, H256,
};
use sp_runtime::{
    testing::{Header, TestXt},
    traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentifyAccount, IdentityLookup, Verify},
};
use sp_std::{marker::PhantomData, num::ParseIntError};

type UncheckedExtrinsic = system::mocking::MockUncheckedExtrinsic<Test>;
type Block = system::mocking::MockBlock<Test>;

type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Extrinsic = TestXt<Call, ()>;

pub const ALICE: sr25519::Public = sr25519::Public([1; 32]);
pub const DID: H160 = H160([
    0x32, 0xac, 0x79, 0x9d, //
    0x35, 0xde, 0x72, 0xa2, //
    0xae, 0x57, 0xa4, 0x6c, //
    0xa9, 0x75, 0x31, 0x9f, //
    0xbb, 0xb1, 0x25, 0xa9,
]);

pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: system::{Pallet, Call, Config, Storage, Event<T>},

        Linker: parami_linker::{Pallet, Call, Storage, Config<T>, Event<T>},
    }
);

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
    type Lookup = IdentityLookup<Self::AccountId>;
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

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
    Call: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: Call,
        _public: <Signature as Verify>::Signer,
        _account: AccountId,
        nonce: u64,
    ) -> Option<(Call, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
        Some((call, (nonce, ())))
    }
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
    Call: From<LocalCall>,
{
    type OverarchingCall = Call;
    type Extrinsic = Extrinsic;
}

impl frame_system::offchain::SigningTypes for Test {
    type Public = <Signature as Verify>::Signer;
    type Signature = Signature;
}

pub struct EnsureDid<T>(PhantomData<T>);
impl<T: parami_linker::Config> EnsureOrigin<T::Origin> for EnsureDid<T> {
    type Success = (H160, T::AccountId);

    fn try_origin(o: T::Origin) -> Result<Self::Success, T::Origin> {
        use frame_system::RawOrigin;

        o.into().and_then(|o| match o {
            RawOrigin::Signed(who) => Ok((DID, who)),
            r => Err(T::Origin::from(r)),
        })
    }
}

parameter_types! {
    pub const PendingLifetime: u64 = 5;
    pub const UnsignedPriority: u64 = 3;
}

impl parami_linker::Config for Test {
    type Event = Event;
    type DecentralizedId = H160;
    type PendingLifetime = PendingLifetime;
    type UnsignedPriority = UnsignedPriority;
    type CallOrigin = EnsureDid<Self>;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap()
        .into()
}
