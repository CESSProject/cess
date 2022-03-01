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

//! Tests for the module.

use super::*;
use frame_support::{assert_ok};
use mock::{
	new_test_ext, Balances, Origin, Sminer, Test,
};
use frame_benchmarking::account;

#[test]
fn timing_storage_space_thirty_days_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::timing_storage_space_thirty_days(Origin::root()));	
	});
}

#[test]
fn timed_increase_rewards_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::initi(Origin::root()));
		assert_ok!(Sminer::regnstk(Origin::signed(1),account("source", 0, 0),0,0,0,0));
		assert_ok!(Sminer::add_power_test(Origin::root(),1,1000));	
		assert_ok!(Sminer::timed_increase_rewards(Origin::root()));	
	});
}

#[test]
fn timed_task_award_table_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::timed_task_award_table(Origin::root()));	
	});
}

#[test]
fn timed_user_receive_award1_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::timed_user_receive_award1(Origin::root()));	
	});
}

#[test]
fn faucet_works() {
	new_test_ext().execute_with(|| {
		pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 100000000000000000),
			],
		};
		assert_ok!(Sminer::faucet_top_up(Origin::signed(1), 10000000000000000));
		assert_ok!(Sminer::faucet(Origin::signed(1),5u64));
		assert_eq!(Balances::free_balance(5), 10000000000000100);
	});
}

#[test]
fn etcd_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::setaddress(Origin::signed(1),1,2,3,4));
		assert_ok!(Sminer::updateaddress(Origin::signed(1),account("source", 0, 0)));
		assert_ok!(Sminer::setetcd(Origin::signed(2),vec![127]));
		assert_ok!(Sminer::setetcdtoken(Origin::signed(4),vec![127]));
		assert_ok!(Sminer::setserviceport(Origin::signed(4),vec![127]));
	});
}

#[test]
fn redeem_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::initi(Origin::root()));
		assert_ok!(Sminer::regnstk(Origin::signed(1),account("source", 0, 0),0,0,0,0));
		assert_ok!(Sminer::redeem(Origin::signed(1)));
	});
}
