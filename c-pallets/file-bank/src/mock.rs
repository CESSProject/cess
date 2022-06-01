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
use crate as file_bank;
use frame_support::{
    parameter_types,
    weights::Weight,
    traits::{ConstU32, EqualPrivilegeOnly},
};
use frame_system::{EnsureRoot};
use sp_core::{H256, sr25519::Signature};
use sp_runtime::{
    testing::{Header, TestXt},
    traits::{BlakeTwo256, Extrinsic as ExtrinsicT, IdentityLookup, IdentifyAccount, Verify},
    Perbill,
};
use frame_support_test::TestRandomness;
use frame_benchmarking::account;

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
		FileBank: file_bank::{Pallet, Call, Storage, Event<T>},
		Sminer: pallet_sminer::{Pallet, Call, Storage, Event<T>},
		Scheduler: pallet_scheduler::{Pallet, Call, Storage, Event<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
	}
);

parameter_types! {
	#[derive(Clone, PartialEq, Eq)]
	pub const StringLimit: u32 = 100;
	pub const OneHours: u32 = 60 * 20;
	pub const OneDay: u32 = 60 * 20 * 24;
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
	pub const RewardPalletId: PalletId = PalletId(*b"sminerpt");
    pub const ItemLimit: u32 = 1024;
}

impl pallet_sminer::Config for Test {
    type Event = Event;
    // type Call = Call;
    type Currency = Balances;
    type PalletId = RewardPalletId;
    type ItemLimit = ItemLimit;
    type SScheduler = Scheduler;
    type SPalletsOrigin = OriginCaller;
    type SProposal = Call;
    type WeightInfo = ();
}

type Extrinsic = TestXt<Call, ()>;
pub(crate) type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

impl frame_system::offchain::SigningTypes for Test {
    type Public = <Signature as Verify>::Signer;
    type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
    where
        Call: From<LocalCall>,
{
    type Extrinsic = Extrinsic;
    type OverarchingCall = Call;
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
	pub const FilbakPalletId: PalletId = PalletId(*b"filebank");
}

pub struct MockingMinerControl;

impl pallet_sminer::MinerControl for MockingMinerControl {
    fn add_power(_peer_id: u64, _power: u128) -> DispatchResult {
        todo!()
    }

    fn sub_power(_peer_id: u64, _power: u128) -> DispatchResult {
        todo!()
    }

    fn add_space(_peer_id: u64, _power: u128) -> DispatchResult {
        todo!()
    }

    fn sub_space(_peer_id: u64, _power: u128) -> DispatchResult {
        todo!()
    }

    fn get_power_and_space(_peer_id: u64) -> Result<(u128, u128), DispatchError> {
        todo!()
    }

    fn punish_miner(_peer_id: u64, _file_size: u64) -> DispatchResult {
        todo!()
    }

    fn miner_is_exist(_peer_id: u64) -> bool {
        todo!()
    }
}

pub struct MockingScheduleFind;

impl<AccountId> pallet_file_map::ScheduleFind<AccountId> for MockingScheduleFind {
    fn contains_scheduler(_acc: AccountId) -> bool {
        false
    }

    fn get_controller_acc(acc: AccountId) -> AccountId {
        acc
    }
}

impl Config for Test {
    type Event = Event;
    type Currency = Balances;
    type WeightInfo = ();
    type Call = Call;
    type FindAuthor = ();
    type AuthorityId = file_bank::crypto::TestAuthId;
    type Scheduler = MockingScheduleFind;
    type MinerControl = MockingMinerControl;
    type MyRandomness = TestRandomness<Self>;
    type FilbakPalletId = FilbakPalletId;
    type StringLimit = StringLimit;
    type OneDay = OneDay;
}

pub fn account1() -> AccountId {
    account("account1", 0, 0)
}

pub fn account2() -> AccountId {
    account("account2", 0, 0)
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(account1(), 1000000000000000000000), (account2(), 1000000000000)],
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

