use crate as pallet_oss;
use frame_benchmarking::account;
use frame_support::parameter_types;
use sp_runtime::{
	testing::{Header},
	traits::{BlakeTwo256, IdentityLookup},
};
use sp_core::H256;
use frame_support::traits::{ConstU32};

pub(crate) type AccountId = u32;
type BlockNumber = u64;
type Balance = u64;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Oss: pallet_oss::{Pallet, Call, Storage, Event<T>},
		// Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
	}
);

// parameter_types! {
// 	pub const ExistentialDeposit: u64 = 1;
// }
//
// impl pallet_balances::Config for Test {
// 	type Balance = u64;
// 	type DustRemoval = ();
// 	type Event = Event;
// 	type ExistentialDeposit = ExistentialDeposit;
// 	type AccountStore = System;
// 	type WeightInfo = ();
// 	type MaxLocks = ();
// 	type MaxReserves = ();
// 	type ReserveIdentifier = [u8; 8];
// }

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
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
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

impl pallet_oss::Config for Test {
	type Event = Event;

	type WeightInfo = ();
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

// pub fn new_test_ext() -> sp_io::TestExternalities {
// 	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
// 	pallet_balances::GenesisConfig::<Test> {
// 		balances: vec![
// 			(account1(), 18_000_000_000_000_000_000),
// 			(account2(), 1_000_000_000_000),
// 			(miner1(), 1_000_000_000_000),
// 			(stash1(), 1_000_000_000_000),
// 			(controller1(), 1_000_000_000_000),
// 		],
// 	}
// 		.assimilate_storage(&mut t)
// 		.unwrap();
// 	let mut ext = sp_io::TestExternalities::new(t);
// 	ext.execute_with(|| {
// 		System::set_block_number(1); //must set block_number, otherwise the deposit_event() don't work
// 	});
// 	ext
// }
