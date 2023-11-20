// // This file is part of Substrate.

// // Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
// // SPDX-License-Identifier: Apache-2.0

// // Licensed under the Apache License, Version 2.0 (the "License");
// // you may not use this file except in compliance with the License.
// // You may obtain a copy of the License at
// //
// // 	http://www.apache.org/licenses/LICENSE-2.0
// //
// // Unless required by applicable law or agreed to in writing, software
// // distributed under the License is distributed on an "AS IS" BASIS,
// // WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// // See the License for the specific language governing permissions and
// // limitations under the License.

//! Test utilities

use super::*;
use crate as sminer;
use frame_support::{
	parameter_types,
	traits::{ConstU32, ConstU64, EqualPrivilegeOnly, OnFinalize, OnInitialize},
	weights::Weight,
};
use frame_system::EnsureRoot;
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	Perbill,
};

use std::convert::{ TryFrom };

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
pub type AccountId = u64;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Balances: pallet_balances,
		Sminer: sminer,
		Timestamp: pallet_timestamp,
		Scheduler: pallet_scheduler,
		Preimage: pallet_preimage,
	}
);

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(Weight::from_parts(1024, 0));
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
	type BlockHashCount = ConstU64<250>;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}

impl pallet_balances::Config for Test {
	type Balance = u128;
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
	type ManagerOrigin = EnsureRoot<u64>;
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
	type ScheduleOrigin = EnsureRoot<u64>;
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type MaxScheduledPerBlock = ();
	type WeightInfo = ();
	type Preimages = Preimage;
}

parameter_types! {
	pub const RewardPalletId: PalletId = PalletId(*b"sminerpt");
	pub const ItemLimit: u32 = 32;
	pub const MultipleFines: u8 = 7;
	pub const DepositBufferPeriod: u32 = 3;
	pub const OneDay: u32 = 14400;
	pub const MaxAward: u128 = 1_306_849_000_000_000_000;
	pub const LockInPeriod: u8 = 2;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type PalletId = RewardPalletId;
	type ItemLimit = ItemLimit;
	type MultipleFines = MultipleFines;
	type SScheduler = Scheduler;
	type SPalletsOrigin = OriginCaller;
	type SProposal = RuntimeCall;
	type WeightInfo = ();
	type DepositBufferPeriod = DepositBufferPeriod;
	type OneDayBlock = OneDay;
	type AScheduler = Scheduler;
	type LockInPeriod = LockInPeriod;
	type MaxAward = MaxAward;
}

pub mod consts {
	pub const ACCOUNT1: (u64, u128) = (1, 100000000000000000);
	pub const ACCOUNT2: (u64, u128) = (2, 100000000000000000);
	pub const ACCOUNT3: (u64, u128) = (3, 100000000000000000);
	pub const ACCOUNT4: (u64, u128) = (4, 100000000000000000);
	pub const ACCOUNT5: (u64, u128) = (5, 100000000000000000);
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	use consts::*;
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	let jackpot_account: u64 = RewardPalletId::get().into_account_truncating();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			ACCOUNT1,
			ACCOUNT2,
			ACCOUNT3,
			ACCOUNT4,
			ACCOUNT5,
			(jackpot_account, 900000000000000000),
		],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1); //must set block_number, otherwise the deposit_event() don't work
	});
	ext
}

pub fn run_to_block(n: u64) {
	while System::block_number() < n {
		Scheduler::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		Scheduler::on_initialize(System::block_number());
	}
}
