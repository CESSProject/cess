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

// //! Test utilities

// use super::*;
// use crate as file_bank;
// use frame_support::{
// 	parameter_types,
// 	weights::Weight,
// };
// use frame_system::{EnsureRoot};
// use sp_core::H256;
// use sp_runtime::{
// 	testing::Header,
// 	traits::{BlakeTwo256, IdentityLookup},
// 	Perbill,
// };
// use frame_support_test::TestRandomness;

// type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
// type Block = frame_system::mocking::MockBlock<Test>;

// frame_support::construct_runtime!(
// 	pub enum Test where
// 		Block = Block,
// 		NodeBlock = Block,
// 		UncheckedExtrinsic = UncheckedExtrinsic,
// 	{
// 		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
// 		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
// 		FileBank: file_bank::{Pallet, Call, Storage, Event<T>},
// 		Sminer: pallet_sminer::{Pallet, Call, Storage, Event<T>},
// 		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Config, Event<T>},
// 		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
// 		SegmentBook: pallet_segment_book::{Pallet, Call, Storage, Event<T>},
// 	}
// );

// parameter_types! {
// 	pub const SegmentBookPalletId: PalletId = PalletId(*b"SegmentB");
// }

// impl pallet_segment_book::Config for Test {
// 	type Event = Event;
// 	// type Call = Call;
// 	type Currency = Balances;
// 	type MyPalletId = SegmentBookPalletId;
// 	type WeightInfo = ();
// 	type MyRandomness = TestRandomness<Self>;
// }

// parameter_types! {
// 	pub const MinimumPeriod: u64 = 1;
// }

// impl pallet_timestamp::Config for Test {
// 	type Moment = u64;
// 	type OnTimestampSet = ();
// 	type MinimumPeriod = MinimumPeriod;
// 	type WeightInfo = ();
// }

// parameter_types! {
// 	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * BlockWeights::get().max_block;
// }

// impl pallet_scheduler::Config for Test {
// 	type Event = Event;
// 	type Origin = Origin;
// 	type PalletsOrigin = OriginCaller;
// 	type Call = Call;
// 	type MaximumWeight = MaximumSchedulerWeight;
// 	type ScheduleOrigin = EnsureRoot<u64>;
// 	type MaxScheduledPerBlock = ();
// 	type WeightInfo = ();
// }

// parameter_types! {
// 	pub const RewardPalletId: PalletId = PalletId(*b"sminerpt");
// }

// impl pallet_sminer::Config for Test {
// 	type Event = Event;
// 	// type Call = Call;
// 	type Currency = Balances;
// 	type PalletId = RewardPalletId;
// 	type SScheduler = Scheduler;
// 	type SPalletsOrigin = OriginCaller;
// 	type SProposal = Call;
// 	type WeightInfo = ();
// }

// parameter_types! {
// 	pub const BlockHashCount: u64 = 250;
// 	pub BlockWeights: frame_system::limits::BlockWeights =
// 		frame_system::limits::BlockWeights::simple_max(1024);
// }

// impl frame_system::Config for Test {
// 	type BaseCallFilter = frame_support::traits::Everything;
// 	type BlockWeights = ();
// 	type BlockLength = ();
// 	type DbWeight = ();
// 	type Origin = Origin;
// 	type Call = Call;
// 	type Index = u64;
// 	type BlockNumber = u64;
// 	type Hash = H256;
// 	type Hashing = BlakeTwo256;
// 	type AccountId = u64;
// 	type Lookup = IdentityLookup<Self::AccountId>;
// 	type Header = Header;
// 	type Event = Event;
// 	type BlockHashCount = BlockHashCount;
// 	type Version = ();
// 	type PalletInfo = PalletInfo;
// 	type AccountData = pallet_balances::AccountData<u128>;
// 	type OnNewAccount = ();
// 	type OnKilledAccount = ();
// 	type SystemWeightInfo = ();
// 	type SS58Prefix = ();
// 	type OnSetCode = ();
// }

// parameter_types! {
// 	pub const ExistentialDeposit: u64 = 1;
// }

// impl pallet_balances::Config for Test {
// 	type MaxLocks = ();
// 	type MaxReserves = ();
// 	type ReserveIdentifier = [u8; 8];
// 	type Balance = u128;
// 	type DustRemoval = ();
// 	type Event = Event;
// 	type ExistentialDeposit = ExistentialDeposit;
// 	type AccountStore = System;
// 	type WeightInfo = ();
// }

// parameter_types! {
// 	pub const FilbakPalletId: PalletId = PalletId(*b"filebank");
// }

// impl Config for Test {
// 	type Event = Event;
// 	type Currency = Balances;
// 	type FilbakPalletId = FilbakPalletId;
// 	type WeightInfo = ();
// }

// pub fn new_test_ext() -> sp_io::TestExternalities {
// 	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
// 	pallet_balances::GenesisConfig::<Test> {
// 		balances: vec![(1, 1000000000000000000000), (2, 100), (3, 100), (4, 100), (5, 100)],
// 	}
// 	.assimilate_storage(&mut t)
// 	.unwrap();
// 	t.into()
// }

