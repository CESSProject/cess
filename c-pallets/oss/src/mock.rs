use crate as pallet_oss;
use frame_benchmarking::account;
use frame_support:: {
	parameter_types,
	pallet_prelude::*,
	traits::ConstU32,
};
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};
use sp_core::H256;

pub(crate) type AccountId = u32;
// type BlockNumber = u64;
// type Balance = u64;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Oss: pallet_oss,
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
		frame_system::limits::BlockWeights::simple_max(Weight::from_ref_time(1024));
	}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
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
	type RuntimeEvent = RuntimeEvent;

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
