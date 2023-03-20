//! Test utilities

use super::*;
use crate as audit;
use frame_support::{
    parameter_types,
    weights::Weight,
    traits::{ConstU32, EqualPrivilegeOnly, OneSessionHandler},
};
use frame_system::{EnsureRoot};
use sp_core::{H256, sr25519::Signature};
use sp_runtime::{
    testing::{Header, TestXt, UintAuthorityId},
    traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentityLookup, IdentifyAccount, Verify},
    Perbill
};
use frame_benchmarking::account;
use frame_support_test::TestRandomness;
use pallet_cess_staking::{StashOf, Exposure, ExposureOf, BalanceOf};
use frame_election_provider_support::{
    onchain, SequentialPhragmen, VoteWeight,
};
use sp_staking::{
    EraIndex, SessionIndex,
};
use std::marker::PhantomData;
use std::cell::RefCell;
use cp_scheduler_credit::SchedulerStashAccountFinder;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type BlockNumber = u64;
type Balance = u64;


frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Balances: pallet_balances,
		Audit: audit,
		Scheduler: pallet_scheduler,
		Sminer: pallet_sminer,
		FileBank: pallet_file_bank,
		TeeWorker: pallet_tee_worker,
		Staking: pallet_cess_staking,
		Session: pallet_session,
		Historical: pallet_session::historical,
		BagsList: pallet_bags_list,
		Timestamp: pallet_timestamp,
		SchedulerCredit: pallet_scheduler_credit,
		Oss: pallet_oss,
        PreImage: pallet_preimage,
	}
);

impl pallet_oss::Config for Test {
	type RuntimeEvent = RuntimeEvent;

	type WeightInfo = ();
}

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights::simple_max(frame_support::weights::Weight::from_ref_time(1024));
}

pub(crate) type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Extrinsic = TestXt<RuntimeCall, ()>;

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
}

parameter_types! {
    pub const TeeWorkerPalletId: PalletId = PalletId(*b"filmpdpt");
		#[derive(Clone, PartialEq, Eq)]
		pub const SchedulerMaximum: u32 = 10000;
		#[derive(Clone, PartialEq, Eq)]
		pub const ParamsLimit: u32 = 359;
}

impl pallet_tee_worker::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type TeeWorkerPalletId = TeeWorkerPalletId;
    type StringLimit = StringLimit;
    type WeightInfo = ();
		type CreditCounter = SchedulerCredit;
		type SchedulerMaximum = SchedulerMaximum;
		type ParamsLimit = ParamsLimit;
}

parameter_types! {
	pub const FilbakPalletId: PalletId = PalletId(*b"filebank");
	#[derive(Clone, Eq, PartialEq)]
	pub const UploadFillerLimit: u8 = 10;
	#[derive(Clone, Eq, PartialEq)]
	pub const InvalidLimit: u32 = 100000;
	#[derive(Clone, Eq, PartialEq)]
	pub const RecoverLimit: u32 = 8000;
	#[derive(Clone, Eq, PartialEq)]
	pub const BucketLimit: u32 = 1000;
	#[derive(Clone, Eq, PartialEq)]
	pub const NameStrLimit: u32 = 40;
	#[derive(Clone, Eq, PartialEq)]
	pub const MinLength: u32 = 3;
	#[derive(Clone, Eq, PartialEq)]
	pub const FileListLimit: u32 = 500000;
	#[derive(Clone, Eq, PartialEq)]
	pub const FrozenDays: BlockNumber = 60 * 10 * 24 * 7;
}

impl pallet_file_bank::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type WeightInfo = ();
	type RuntimeCall = RuntimeCall;
	type FindAuthor = ();
	type CreditCounter = SchedulerCredit;
	type Scheduler = pallet_tee_worker::Pallet::<Test>;
	type MinerControl = pallet_sminer::Pallet::<Test>;
	type MyRandomness = TestRandomness<Self>;
	type FilbakPalletId = FilbakPalletId;
	type StringLimit = StringLimit;
	type OneDay = OneDay;
	type FileListLimit = FileListLimit;
	type NameStrLimit = NameStrLimit;
	type BucketLimit = BucketLimit;
	type OssFindAuthor = Oss;
	type FrozenDays = FrozenDays;
	type RecoverLimit = RecoverLimit;
	type InvalidLimit = InvalidLimit;
	type UploadFillerLimit = UploadFillerLimit;
	type MinLength = MinLength;
}

pub struct OtherSessionHandler;

impl OneSessionHandler<AccountId> for OtherSessionHandler {
    type Key = UintAuthorityId;

    fn on_genesis_session<'a, I: 'a>(_: I)
        where
            I: Iterator<Item=(&'a AccountId, Self::Key)>,
            AccountId: 'a,
    {}

    fn on_new_session<'a, I: 'a>(_: bool, _: I, _: I)
        where
            I: Iterator<Item=(&'a AccountId, Self::Key)>,
            AccountId: 'a,
    {}

    fn on_disabled(_validator_index: u32) {}
}

impl sp_runtime::BoundToRuntimeAppPublic for OtherSessionHandler {
    type Public = UintAuthorityId;
}

const THRESHOLDS: [sp_npos_elections::VoteWeight; 9] =
    [10, 20, 30, 40, 50, 60, 1_000, 2_000, 10_000];

parameter_types! {
	pub static BagThresholds: &'static [sp_npos_elections::VoteWeight] = &THRESHOLDS;
	pub static MaxNominations: u32 = 16;
}

impl pallet_bags_list::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type ScoreProvider = Staking;
    type BagThresholds = BagThresholds;
    type Score = VoteWeight;
}

pub struct MockStashAccountFinder<AccountId>(PhantomData<AccountId>);

impl<AccountId: Clone> SchedulerStashAccountFinder<AccountId>
for MockStashAccountFinder<AccountId>
{
	fn find_stash_account_id(ctrl_account_id: &AccountId) -> Option<AccountId> {
		Some(ctrl_account_id.clone())
	}
}

parameter_types! {
	pub const PeriodDuration: BlockNumber = 64_000;
}

impl pallet_scheduler_credit::Config for Test {
	type StashAccountFinder = MockStashAccountFinder<Self::AccountId>;

	type PeriodDuration = PeriodDuration;
}

parameter_types! {
	pub static SessionsPerEra: SessionIndex = 3;
	pub static SlashDeferDuration: EraIndex = 0;
	pub static Period: BlockNumber = 5;
	pub static Offset: BlockNumber = 0;
}

sp_runtime::impl_opaque_keys! {
	pub struct SessionKeys {
		pub other: OtherSessionHandler,
	}
}
impl pallet_session::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = AccountId;
    type ValidatorIdOf = StashOf<Test>;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = pallet_session::historical::NoteHistoricalRoot<Test, Staking>;
    type SessionHandler = (OtherSessionHandler, );
    type Keys = SessionKeys;
    type WeightInfo = ();
}

impl pallet_session::historical::Config for Test {
    type FullIdentification = Exposure<AccountId, Balance>;
    type FullIdentificationOf = ExposureOf<Test>;
}

thread_local! {
	pub static REWARD_REMAINDER_UNBALANCED: RefCell<u128> = RefCell::new(0);
}

pub struct OnChainSeqPhragmen;

impl onchain::Config for OnChainSeqPhragmen {
    type System = Test;
    type Solver = SequentialPhragmen<AccountId, Perbill>;
    type DataProvider = Staking;
    type WeightInfo = ();
    type MaxWinners = ConstU32<100>;
	type VotersBound = ConstU32<{ u32::MAX }>;
	type TargetsBound = ConstU32<{ u32::MAX }>;
    
}

impl pallet_cess_staking::Config for Test {
    const ERAS_PER_YEAR: u64 = 8766;
    const FIRST_YEAR_VALIDATOR_REWARDS: BalanceOf<Test> = 618_000_000;
    const FIRST_YEAR_SMINER_REWARDS: BalanceOf<Test> = 309_000_000;
    const REWARD_DECREASE_RATIO: Perbill = Perbill::from_perthousand(794);
    type SminerRewardPool = ();
    type Currency = Balances;
    type CurrencyBalance = <Self as pallet_balances::Config>::Balance;
    type UnixTime = Timestamp;
    type CurrencyToVote = frame_support::traits::SaturatingCurrencyToVote;
    type ElectionProvider = onchain::OnChainExecution<OnChainSeqPhragmen>;
    type GenesisElectionProvider = Self::ElectionProvider;
    type MaxNominations = MaxNominations;
    type RewardRemainder = ();
    type RuntimeEvent = RuntimeEvent;
    type Slash = ();
    type Reward = ();
    type SessionsPerEra = ();
    type BondingDuration = ();
    type SlashDeferDuration = ();
    type SlashCancelOrigin = frame_system::EnsureRoot<Self::AccountId>;
    type SessionInterface = Self;
    type EraPayout = ();
    type NextNewSession = ();
    type MaxNominatorRewardedPerValidator = ConstU32<64>;
    type OffendingValidatorsThreshold = ();
    type VoterList = BagsList;
    type TargetList = pallet_cess_staking::UseValidatorsMap<Self>;
    type MaxUnlockingChunks = ConstU32<32>;
    type HistoryDepth = ConstU32<84>;
    type OnStakerSlash = ();
    type BenchmarkingConfig = pallet_cess_staking::TestBenchmarkingConfig;
    type WeightInfo = ();
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type Balance = u64;
    type DustRemoval = ();
    type RuntimeEvent = RuntimeEvent;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
}

impl pallet_preimage::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = ();
	type ManagerOrigin = EnsureRoot<AccountId>;
	type BaseDeposit = ();
	type ByteDeposit = ();
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
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
}

impl pallet_scheduler::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRoot<AccountId>;
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type MaxScheduledPerBlock = ();
    type WeightInfo = ();
    type Preimages = PreImage;
}

parameter_types! {
    pub const RewardPalletId: PalletId = PalletId(*b"sminerpt");
    pub const MultipleFines: u8 = 7;
    pub const DepositBufferPeriod: u32 = 3;
    pub const ItemLimit: u32 = 1024;
		pub const MaxAward: u128 = 1_306_849_000_000_000_000;
		pub const LockInPeriod: u8 = 2;
}

impl pallet_sminer::Config for Test {
    type Currency = Balances;
      // The ubiquitous event type.
      type RuntimeEvent = RuntimeEvent;
      type PalletId = RewardPalletId;
      type SScheduler = Scheduler;
      type AScheduler = Scheduler;
      type SPalletsOrigin = OriginCaller;
      type SProposal = RuntimeCall;
      type WeightInfo = ();
      type ItemLimit = ItemLimit;
      type MultipleFines = MultipleFines;
      type DepositBufferPeriod = DepositBufferPeriod;
      type OneDayBlock = OneDay;
			type MaxAward = MaxAward;
			type LockInPeriod = LockInPeriod;
}

parameter_types! {
	pub const SegmentBookPalletId: PalletId = PalletId(*b"SegmentB");
	#[derive(Clone, PartialEq, Eq)]
	pub const StringLimit: u32 = 100;
	pub const OneHours: u32 = 60 * 20;
	pub const OneDay: u32 = 60 * 20 * 24;
	pub const LockTime: u32 = 1;
	pub const SegUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
	#[derive(Clone, PartialEq, Eq)]
	pub const SubmitProofLimit: u32 = 100;
	#[derive(Clone, PartialEq, Eq)]
	pub const SubmitValidationLimit: u32 = 50;
	#[derive(Clone, PartialEq, Eq)]
	pub const ChallengeMaximum: u32 = 8000;
}

impl Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type MyPalletId = SegmentBookPalletId;
    type WeightInfo = ();
    type MyRandomness = TestRandomness<Self>;
    type StringLimit = StringLimit;
    type RandomLimit = StringLimit;
    type OneDay = OneDay;
    type OneHours = OneHours;
    type FindAuthor = ();
    type File = pallet_file_bank::Pallet::<Test>;
    type Scheduler = pallet_tee_worker::Pallet::<Test>;
    type MinerControl = Sminer;
    type AuthorityId = audit::sr25519::AuthorityId;
		type ValidatorSet = Historical;
		type NextSessionRotation = ();
		type UnsignedPriority = SegUnsignedPriority;
		type LockTime = LockTime;
		type SubmitValidationLimit = SubmitValidationLimit;
		type SubmitProofLimit = SubmitProofLimit;
		type ChallengeMaximum = ChallengeMaximum;
}

pub fn account1() -> AccountId {
    account("account1", 0, 0)
}

pub fn account2() -> AccountId {
    account("account2", 0, 0)
}

pub fn miner1() -> AccountId {
    account("miner1", 0, 0)
}

pub fn stash1() -> AccountId {
    account("stash1", 0, 0)
}

pub fn controller1() -> AccountId {
    account("controller1", 0, 0)
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (account1(), 1_000_000_000_000_000_000),
            (account2(), 1_000_000_000_000_000_000),
            (miner1(), 1_000_000_000_000_000_000),
            (stash1(), 1_000_000_000_000_000_000),
            (controller1(), 1_000_000_000_000_000_000),
        ],
    }
        .assimilate_storage(&mut t)
        .unwrap();
		pallet_file_bank::GenesisConfig::<Test> {
			price: 30
		}
			.assimilate_storage(&mut t)
			.unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        System::set_block_number(1); //must set block_number, otherwise the deposit_event() don't work
    });
    ext
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
    where
        RuntimeCall: From<LocalCall>,
{
    type Extrinsic = Extrinsic;
    type OverarchingCall = RuntimeCall;
}

impl frame_system::offchain::SigningTypes for Test {
    type Public = <Signature as Verify>::Signer;
    type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
    where
        RuntimeCall: From<LocalCall>,
{
    fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
        call: RuntimeCall,
        _public: <Signature as Verify>::Signer,
        _account: AccountId,
        nonce: u64,
    ) -> Option<(RuntimeCall, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
        Some((call, (nonce, ())))
    }
}
