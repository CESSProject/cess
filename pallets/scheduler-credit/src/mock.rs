//! Test utilities

use super::*;
use crate as scheduler_credit;
use frame_support::{parameter_types, traits::ConstU32};
use sp_core::H256;
use sp_runtime::{
	BuildStorage,
	traits::{BlakeTwo256, IdentityLookup},	
};
use std::marker::PhantomData;

pub(crate) type AccountId = u32;
type Block = frame_system::mocking::MockBlock<Test>;
type BlockNumber = u64;
parameter_types! {
	pub const BlockHashCount: u64 = 100;
}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Block = Block;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
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

pub struct MockStashAccountFinder<AccountId>(PhantomData<AccountId>);

impl<AccountId: Clone> SchedulerStashAccountFinder<AccountId>
	for MockStashAccountFinder<AccountId>
{
	fn find_stash_account_id(ctrl_account_id: &AccountId) -> Option<AccountId> {
		Some(ctrl_account_id.clone())
	}
}

parameter_types! {
	pub const PeriodDuration: BlockNumber = 3600;
}

impl Config for Test {
	type StashAccountFinder = MockStashAccountFinder<Self::AccountId>;

	type PeriodDuration = PeriodDuration;
}

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>},
		SchedulerCredit: scheduler_credit::{Pallet, Storage},
	}
);

pub struct ExtBuilder;

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {}
	}
}

impl ExtBuilder {
	fn build(self) -> sp_io::TestExternalities {
		let storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
		sp_io::TestExternalities::from(storage)
	}

	pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
		self.build().execute_with(test);
	}
}
