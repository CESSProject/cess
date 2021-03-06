// This file is part of Substrate.

// Copyright (C) 2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Autogenerated weights for pallet_sminer
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2021-12-10, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("cess-hacknet"), DB CACHE: 128

// Executed Command:

// D:\workspace\substrate\internal-cess-node\target\release\cess-node.exe

// benchmark

// --chain

// cess-hacknet

// --execution=wasm

// --wasm-execution=compiled

// --pallet

// pallet_sminer

// --extrinsic

// *

// --steps

// 50

// --repeat

// 20

// --template=./.maintain/frame-weight-template.hbs

// --output=./c-pallets/sminer/src/weights.rs


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for pallet_sminer.
pub trait WeightInfo {
	
	fn regnstk() -> Weight;
	
	fn redeem() -> Weight;
	
	fn claim() -> Weight;
	
	fn initi() -> Weight;
	
	fn setaddress() -> Weight;
	
	fn updateaddress() -> Weight;
	
	fn setetcd() -> Weight;
	
	fn setetcdtoken() -> Weight;
	
	fn setserviceport() -> Weight;
	
	fn add_available_storage() -> Weight;
	
	fn add_used_storage() -> Weight;
	
	fn timing_storage_space() -> Weight;
	
	fn timing_task_storage_space() -> Weight;
	
	fn timing_storage_space_thirty_days() -> Weight;
	
	fn timed_increase_rewards() -> Weight;
	
	fn timing_task_increase_power_rewards() -> Weight;
	
	fn del_reward_order() -> Weight;
	
	fn timed_user_receive_award1() -> Weight;
	
	fn timing_user_receive_award() -> Weight;
	
	fn timed_task_award_table() -> Weight;
	
	fn timing_task_award_table() -> Weight;
	
	fn punishment() -> Weight;
	
	fn faucet_top_up() -> Weight;
	
	fn faucet() -> Weight;
	
}

/// Weights for pallet_sminer using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {

	
	
	// Storage: Sminer MinerItems (r:1 w:1)
	
	// Storage: Sminer PeerIndex (r:1 w:1)
	
	// Storage: Sminer AllMiner (r:1 w:1)
	
	// Storage: Sminer MinerStatValue (r:1 w:1)
	
	// Storage: Sminer SegInfo (r:0 w:1)
	
	// Storage: Sminer MinerTable (r:0 w:1)
	
	// Storage: Sminer WalletMiners (r:0 w:1)
	
	// Storage: Sminer MinerDetails (r:0 w:1)
	
	fn regnstk() -> Weight {
		(60_600_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(8 as Weight))
			
			
	}
	
	
	// Storage: Sminer MinerItems (r:1 w:1)
	
	// Storage: Sminer AllMiner (r:1 w:1)
	
	// Storage: Sminer WalletMiners (r:1 w:1)
	
	// Storage: Sminer SegInfo (r:0 w:1)
	
	fn redeem() -> Weight {
		(52_400_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
			
			
	}
	
	
	// Storage: Sminer MinerItems (r:1 w:0)
	
	// Storage: Sminer WalletMiners (r:1 w:0)
	
	// Storage: System Account (r:1 w:1)
	
	fn claim() -> Weight {
		(80_200_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer MinerStatValue (r:0 w:1)
	
	fn initi() -> Weight {
		(4_300_000 as Weight)
			
			
			
			
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer Control (r:1 w:1)
	
	// Storage: Sminer EtcdRegisterOwner (r:1 w:1)
	
	// Storage: Sminer EtcdOwner (r:0 w:1)
	
	fn setaddress() -> Weight {
		(22_500_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
			
			
	}
	
	
	// Storage: Sminer EtcdRegisterOwner (r:1 w:1)
	
	fn updateaddress() -> Weight {
		(35_000_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer EtcdRegisterOwner (r:1 w:1)
	
	// Storage: Sminer EtcdRegister (r:0 w:1)
	
	fn setetcd() -> Weight {
		(29_500_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
			
			
	}
	
	
	// Storage: Sminer EtcdOwner (r:1 w:0)
	
	// Storage: Sminer EtcdToken (r:0 w:1)
	
	fn setetcdtoken() -> Weight {
		(27_500_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer EtcdOwner (r:1 w:0)
	
	// Storage: Sminer ServicePort (r:0 w:1)
	
	fn setserviceport() -> Weight {
		(26_900_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer StorageInfoValue (r:1 w:1)
	
	fn add_available_storage() -> Weight {
		(7_900_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer StorageInfoValue (r:1 w:1)
	
	fn add_used_storage() -> Weight {
		(8_100_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Timestamp Now (r:1 w:0)
	
	// Storage: Sminer StorageInfoValue (r:1 w:0)
	
	// Storage: Sminer StorageInfoVec (r:1 w:1)
	
	fn timing_storage_space() -> Weight {
		(30_700_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Scheduler Lookup (r:1 w:1)
	
	// Storage: Scheduler Agenda (r:1 w:1)
	
	fn timing_task_storage_space() -> Weight {
		(48_200_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
			
			
	}
	
	
	// Storage: Timestamp Now (r:1 w:0)
	
	// Storage: Sminer StorageInfoVec (r:1 w:1)
	
	fn timing_storage_space_thirty_days() -> Weight {
		(29_300_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer TotalPower (r:1 w:0)
	
	// Storage: Sminer MinerDetails (r:1 w:0)
	
	// Storage: Sminer MinerStatValue (r:1 w:1)
	
	fn timed_increase_rewards() -> Weight {
		(37_800_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Scheduler Lookup (r:1 w:1)
	
	// Storage: Scheduler Agenda (r:1 w:1)
	
	fn timing_task_increase_power_rewards() -> Weight {
		(49_600_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
			
			
	}
	
	
	// Storage: Sminer CalculateRewardOrderMap (r:1 w:1)
	
	fn del_reward_order() -> Weight {
		(32_100_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer MinerDetails (r:2 w:1)
	
	// Storage: Sminer RewardClaimMap (r:1 w:1)
	
	// Storage: System Account (r:1 w:1)
	
	fn timed_user_receive_award1() -> Weight {
		(98_200_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
			
			
	}
	
	
	// Storage: Scheduler Lookup (r:1 w:1)
	
	// Storage: Scheduler Agenda (r:1 w:1)
	
	fn timing_user_receive_award() -> Weight {
		(51_400_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
			
			
	}
	
	
	// Storage: Sminer CalculateRewardOrderMap (r:2 w:0)
	
	// Storage: Sminer MinerItems (r:1 w:0)
	
	// Storage: Sminer MinerTable (r:1 w:1)
	
	// Storage: Sminer RewardClaimMap (r:1 w:1)
	
	// Storage: Sminer MinerDetails (r:1 w:1)
	
	fn timed_task_award_table() -> Weight {
		(81_600_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
			
			
	}
	
	
	// Storage: Scheduler Lookup (r:1 w:1)
	
	// Storage: Scheduler Agenda (r:1 w:1)
	
	fn timing_task_award_table() -> Weight {
		(50_100_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
			
			
	}
	
	
	// Storage: System Account (r:1 w:1)
	
	fn punishment() -> Weight {
		(75_400_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: System Account (r:1 w:1)
	
	fn faucet_top_up() -> Weight {
		(88_400_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer FaucetRecordMap (r:1 w:1)
	
	// Storage: System Account (r:1 w:1)
	
	fn faucet() -> Weight {
		(79_500_000 as Weight)
			
			
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			
			
			
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
			
			
	}
	
}

// For backwards compatibility and tests
impl WeightInfo for () {
	
	
	// Storage: Sminer MinerItems (r:1 w:1)
	
	// Storage: Sminer PeerIndex (r:1 w:1)
	
	// Storage: Sminer AllMiner (r:1 w:1)
	
	// Storage: Sminer MinerStatValue (r:1 w:1)
	
	// Storage: Sminer SegInfo (r:0 w:1)
	
	// Storage: Sminer MinerTable (r:0 w:1)
	
	// Storage: Sminer WalletMiners (r:0 w:1)
	
	// Storage: Sminer MinerDetails (r:0 w:1)
	
	fn regnstk() -> Weight {
		(60_600_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(4 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(8 as Weight))
			
			
	}
	
	
	// Storage: Sminer MinerItems (r:1 w:1)
	
	// Storage: Sminer AllMiner (r:1 w:1)
	
	// Storage: Sminer WalletMiners (r:1 w:1)
	
	// Storage: Sminer SegInfo (r:0 w:1)
	
	fn redeem() -> Weight {
		(52_400_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(4 as Weight))
			
			
	}
	
	
	// Storage: Sminer MinerItems (r:1 w:0)
	
	// Storage: Sminer WalletMiners (r:1 w:0)
	
	// Storage: System Account (r:1 w:1)
	
	fn claim() -> Weight {
		(80_200_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer MinerStatValue (r:0 w:1)
	
	fn initi() -> Weight {
		(4_300_000 as Weight)
			
			
			
			
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer Control (r:1 w:1)
	
	// Storage: Sminer EtcdRegisterOwner (r:1 w:1)
	
	// Storage: Sminer EtcdOwner (r:0 w:1)
	
	fn setaddress() -> Weight {
		(22_500_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
			
			
	}
	
	
	// Storage: Sminer EtcdRegisterOwner (r:1 w:1)
	
	fn updateaddress() -> Weight {
		(35_000_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer EtcdRegisterOwner (r:1 w:1)
	
	// Storage: Sminer EtcdRegister (r:0 w:1)
	
	fn setetcd() -> Weight {
		(29_500_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
			
			
	}
	
	
	// Storage: Sminer EtcdOwner (r:1 w:0)
	
	// Storage: Sminer EtcdToken (r:0 w:1)
	
	fn setetcdtoken() -> Weight {
		(27_500_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer EtcdOwner (r:1 w:0)
	
	// Storage: Sminer ServicePort (r:0 w:1)
	
	fn setserviceport() -> Weight {
		(26_900_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer StorageInfoValue (r:1 w:1)
	
	fn add_available_storage() -> Weight {
		(7_900_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer StorageInfoValue (r:1 w:1)
	
	fn add_used_storage() -> Weight {
		(8_100_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Timestamp Now (r:1 w:0)
	
	// Storage: Sminer StorageInfoValue (r:1 w:0)
	
	// Storage: Sminer StorageInfoVec (r:1 w:1)
	
	fn timing_storage_space() -> Weight {
		(30_700_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Scheduler Lookup (r:1 w:1)
	
	// Storage: Scheduler Agenda (r:1 w:1)
	
	fn timing_task_storage_space() -> Weight {
		(48_200_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
			
			
	}
	
	
	// Storage: Timestamp Now (r:1 w:0)
	
	// Storage: Sminer StorageInfoVec (r:1 w:1)
	
	fn timing_storage_space_thirty_days() -> Weight {
		(29_300_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer TotalPower (r:1 w:0)
	
	// Storage: Sminer MinerDetails (r:1 w:0)
	
	// Storage: Sminer MinerStatValue (r:1 w:1)
	
	fn timed_increase_rewards() -> Weight {
		(37_800_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(3 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Scheduler Lookup (r:1 w:1)
	
	// Storage: Scheduler Agenda (r:1 w:1)
	
	fn timing_task_increase_power_rewards() -> Weight {
		(49_600_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
			
			
	}
	
	
	// Storage: Sminer CalculateRewardOrderMap (r:1 w:1)
	
	fn del_reward_order() -> Weight {
		(32_100_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer MinerDetails (r:2 w:1)
	
	// Storage: Sminer RewardClaimMap (r:1 w:1)
	
	// Storage: System Account (r:1 w:1)
	
	fn timed_user_receive_award1() -> Weight {
		(98_200_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(4 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
			
			
	}
	
	
	// Storage: Scheduler Lookup (r:1 w:1)
	
	// Storage: Scheduler Agenda (r:1 w:1)
	
	fn timing_user_receive_award() -> Weight {
		(51_400_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
			
			
	}
	
	
	// Storage: Sminer CalculateRewardOrderMap (r:2 w:0)
	
	// Storage: Sminer MinerItems (r:1 w:0)
	
	// Storage: Sminer MinerTable (r:1 w:1)
	
	// Storage: Sminer RewardClaimMap (r:1 w:1)
	
	// Storage: Sminer MinerDetails (r:1 w:1)
	
	fn timed_task_award_table() -> Weight {
		(81_600_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(6 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(3 as Weight))
			
			
	}
	
	
	// Storage: Scheduler Lookup (r:1 w:1)
	
	// Storage: Scheduler Agenda (r:1 w:1)
	
	fn timing_task_award_table() -> Weight {
		(50_100_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
			
			
	}
	
	
	// Storage: System Account (r:1 w:1)
	
	fn punishment() -> Weight {
		(75_400_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: System Account (r:1 w:1)
	
	fn faucet_top_up() -> Weight {
		(88_400_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(1 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(1 as Weight))
			
			
	}
	
	
	// Storage: Sminer FaucetRecordMap (r:1 w:1)
	
	// Storage: System Account (r:1 w:1)
	
	fn faucet() -> Weight {
		(79_500_000 as Weight)
			
			
			.saturating_add(RocksDbWeight::get().reads(2 as Weight))
			
			
			
			.saturating_add(RocksDbWeight::get().writes(2 as Weight))
			
			
	}
	
}
