// This file is part of Substrate.

// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Test utilities

use super::*;
use crate as segment_book;
use frame_support::{
    parameter_types,
    weights::Weight,
    traits::{ConstU32, EqualPrivilegeOnly},
};
use frame_system::{EnsureRoot};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::{BlakeTwo256, IdentityLookup}, Perbill, ConsensusEngineId};
use frame_support_test::TestRandomness;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		SegmentBook: segment_book::{Pallet, Call, Storage, Event<T>},
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
		Sminer: pallet_sminer::{Pallet, Call, Storage, Event<T>},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights::simple_max(1024);
}

pub type AccountId = u64;

impl frame_system::Config for Test {
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
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
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
    type Event = Event;
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
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
    type Event = Event;
    type Origin = Origin;
    type PalletsOrigin = OriginCaller;
    type Call = Call;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = EnsureRoot<AccountId>;
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
    type MaxScheduledPerBlock = ();
    type WeightInfo = ();
    type PreimageProvider = ();
    type NoPreimagePostponement = ();
}

parameter_types! {
	pub const RewardPalletId: PalletId = PalletId(*b"rewardpt");
}

impl pallet_sminer::Config for Test {
    type Currency = Balances;
    type Event = Event;
    type PalletId = RewardPalletId;
    type SScheduler = Scheduler;
    type SPalletsOrigin = OriginCaller;
    type SProposal = Call;
    type WeightInfo = ();
    type ItemLimit = StringLimit;
}

parameter_types! {
	pub const SegmentBookPalletId: PalletId = PalletId(*b"SegmentB");
	#[derive(Clone, PartialEq, Eq)]
	pub const StringLimit: u32 = 100;
	pub const OneHours: u32 = 60 * 20;
	pub const OneDay: u32 = 60 * 20 * 24;
}

pub struct MockingRandomFileList;

impl RandomFileList for MockingRandomFileList {
    fn get_random_challenge_data() -> Result<Vec<(u64, Vec<u8>, Vec<u32>, u64, u8, u32)>, DispatchError> {
        todo!()
    }

    fn delete_filler(_miner: u64, _filler_id: Vec<u8>) -> DispatchResult {
        todo!()
    }

    fn delete_file_dupl(_dupl_id: Vec<u8>) -> DispatchResult {
        todo!()
    }

    fn add_recovery_file(_file_id: Vec<u8>) -> DispatchResult {
        todo!()
    }

    fn add_invalid_file(_miner_id: u64, _file_id: Vec<u8>) -> DispatchResult {
        todo!()
    }
}

pub struct MockingScheduleFind;

impl ScheduleFind<AccountId> for MockingScheduleFind {
    fn contains_scheduler(_acc: AccountId) -> bool {
        true
    }
}

pub struct MockingFindAuthor;

impl FindAuthor<AccountId> for MockingFindAuthor {
    fn find_author<'a, I>(_digests: I) -> Option<AccountId> where I: 'a + IntoIterator<Item=(ConsensusEngineId, &'a [u8])> {
        Some(ACCOUNT1.0)
    }
}

impl Config for Test {
    type Event = Event;
    type Currency = Balances;
    type MyPalletId = SegmentBookPalletId;
    type WeightInfo = ();
    type MyRandomness = TestRandomness<Self>;
    type StringLimit = StringLimit;
    type RandomLimit = StringLimit;
    type OneDay = OneDay;
    type OneHours = OneHours;
    type FindAuthor = MockingFindAuthor;
    type File = MockingRandomFileList;
    type Scheduler = MockingScheduleFind;
    type MinerControl = Sminer;
}

pub const ACCOUNT1: (u64, u128) = (1, 100000000000000000);
pub const ACCOUNT2: (u64, u128) = (2, 100000000000000000);
pub const ACCOUNT3: (u64, u128) = (3, 100000000000000000);
pub const ACCOUNT4: (u64, u128) = (4, 100000000000000000);
pub const ACCOUNT5: (u64, u128) = (5, 100000000000000000);

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![ACCOUNT1, ACCOUNT2, ACCOUNT3, ACCOUNT4, ACCOUNT5],
    }
        .assimilate_storage(&mut t)
        .unwrap();
    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| {
        let _ = Sminer::initi(Origin::root());
        System::set_block_number(1); //must set block_number, otherwise the deposit_event() don't work
    });
    ext
}
