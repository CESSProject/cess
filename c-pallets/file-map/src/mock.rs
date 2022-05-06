#![cfg(test)]

use std::cell::RefCell;
use crate as pallet_file_map;
use frame_support::parameter_types;
use frame_support::PalletId;
use frame_support::traits::{ConstU32, ConstU64, Currency, OneSessionHandler};
use sp_core::H256;
use sp_runtime::{
    testing::{Header, TestXt, UintAuthorityId},
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};
use sp_staking::{
    EraIndex, SessionIndex,
};
use frame_election_provider_support::{
    onchain, SequentialPhragmen, VoteWeight,
};

use pallet_cess_staking::{StashOf, Exposure, ExposureOf};

/// The AccountId alias in this test module.
type AccountId = u64;
type BlockNumber = u64;
type Balance = u64;

pub type BalanceOf<T> = <<T as crate::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// Another session handler struct to test on_disabled.
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

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
        Staking: pallet_cess_staking::{Pallet, Call, Config<T>, Storage, Event<T>},
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		Historical: pallet_session::historical::{Pallet, Storage},
		BagsList: pallet_bags_list::{Pallet, Call, Storage, Event<T>},
        FileMap: pallet_file_map::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type Origin = Origin;
    type Call = Call;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type DbWeight = ();
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
    type Balance = u64;
    type DustRemoval = ();
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = ConstU64<5>;
    type WeightInfo = ();
}

parameter_types! {
    pub const FileMapPalletId: PalletId = PalletId(*b"filmpdpt");
    #[derive(Clone, PartialEq, Eq)]
	pub const StringLimit: u32 = 1024;
}

impl pallet_file_map::Config for Test {
    type Event = Event;
    type Currency = Balances;
    type FileMapPalletId = FileMapPalletId;
    type StringLimit = StringLimit;
}

const THRESHOLDS: [sp_npos_elections::VoteWeight; 9] =
    [10, 20, 30, 40, 50, 60, 1_000, 2_000, 10_000];

parameter_types! {
	pub static BagThresholds: &'static [sp_npos_elections::VoteWeight] = &THRESHOLDS;
	pub static MaxNominations: u32 = 16;
}

impl pallet_bags_list::Config for Test {
    type Event = Event;
    type WeightInfo = ();
    type ScoreProvider = Staking;
    type BagThresholds = BagThresholds;
    type Score = VoteWeight;
}

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(
			frame_support::weights::constants::WEIGHT_PER_SECOND * 2
		);
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
    type Event = Event;
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

impl onchain::ExecutionConfig for OnChainSeqPhragmen {
    type System = Test;
    type Solver = SequentialPhragmen<AccountId, Perbill>;
    type DataProvider = Staking;
}

impl pallet_cess_staking::Config for Test {
    const ERAS_PER_YEAR: u64 = 8766;
    const FIRST_YEAR_VALIDATOR_REWARDS: BalanceOf<Test> = 618_000_000;
    const FIRST_YEAR_SMINER_REWARDS: BalanceOf<Test> = 309_000_000;
    const REWARD_DECREASE_RATIO: Perbill = Perbill::from_perthousand(794);
    type SminerRewardPool = ();
    type Currency = Balances;
    type UnixTime = Timestamp;
    type CurrencyToVote = frame_support::traits::SaturatingCurrencyToVote;
    type ElectionProvider = onchain::UnboundedExecution<OnChainSeqPhragmen>;
    type GenesisElectionProvider = Self::ElectionProvider;
    type MaxNominations = MaxNominations;
    type RewardRemainder = ();
    type Event = Event;
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
    type MaxUnlockingChunks = ConstU32<32>;
    type BenchmarkingConfig = pallet_cess_staking::TestBenchmarkingConfig;
    type WeightInfo = ();
}

pub type Extrinsic = TestXt<Call, ()>;

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
    where
        Call: From<LocalCall>,
{
    type Extrinsic = Extrinsic;
    type OverarchingCall = Call;
}

pub struct ExtBuilder;

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {}
    }
}

impl ExtBuilder {
    fn build(self) -> sp_io::TestExternalities {
        let storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
        let ext = sp_io::TestExternalities::from(storage);
        ext
    }

    pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
        self.build().execute_with(test);
    }
}


