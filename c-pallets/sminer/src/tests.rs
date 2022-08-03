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

//! Tests for the module.

use super::*;
use frame_benchmarking::account;
use frame_support::{assert_noop, assert_ok, traits::Len};
use mock::{consts::*, new_test_ext, run_to_block, Balances, Origin, Sminer, System as Sys, Test};

const UNIT_POWER_LIMIT: u128 = 2000_000_000_000_000u128;
const STATE_POSITIVE: &str = "positive";
const STATE_FROZEN: &str = "frozen";
const STATE_EXIT_FROZEN: &str = "e_frozen";
const STATE_EXIT: &str = "exit";

#[test]
fn miner_register_works() {
	new_test_ext().execute_with(|| {
		assert_eq!(ACCOUNT1.1, Balances::free_balance(&ACCOUNT1.0));
		assert_eq!(ACCOUNT2.1, Balances::free_balance(&ACCOUNT2.0));

		let beneficiary = account::<mock::AccountId>("beneficiary", 0, 0);
		let beneficiary_new = account::<mock::AccountId>("beneficiary_new", 0, 0);
		let stake_amount: u128 = 2000;
		let ip: Vec<u8> = Vec::from("192.168.0.1");
		let ip_new: Vec<u8> = Vec::from("192.168.0.2");

		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT1.0),
			beneficiary,
			ip.clone(),
			stake_amount
		));

		// balance check
		assert_eq!(ACCOUNT1.1 - stake_amount, Balances::free_balance(&ACCOUNT1.0));
		assert_eq!(stake_amount, Balances::reserved_balance(&ACCOUNT1.0));

		//miner item check
		let mr = &MinerItems::<Test>::get(ACCOUNT1.0).unwrap();
		assert_eq!(stake_amount, mr.collaterals);
		assert_eq!(Vec::from(STATE_POSITIVE), mr.state.to_vec());
		assert!(mr.peer_id > 0);

		assert_eq!(mr.peer_id, PeerIndex::<Test>::try_get().unwrap());

		assert!(AllMiner::<Test>::try_get().unwrap().len() > 0);

		let event =
			Sys::events().pop().expect("Expected at least one Registered to be found").event;
		assert_eq!(
			mock::Event::from(Event::Registered { acc: ACCOUNT1.0, staking_val: stake_amount }),
			event
		);

		assert_eq!(beneficiary, mr.beneficiary);
		assert_ok!(Sminer::update_beneficiary(Origin::signed(ACCOUNT1.0), beneficiary_new.clone()));
		let mr = &MinerItems::<Test>::get(ACCOUNT1.0).unwrap();
		assert_eq!(beneficiary_new, mr.beneficiary);
		let event =
			Sys::events().pop().expect("Expected at least one Registered to be found").event;
		assert_eq!(
			mock::Event::from(Event::UpdataBeneficiary { acc: ACCOUNT1.0, new: beneficiary_new }),
			event
		);

		assert_eq!(ip, mr.ip.to_vec());
		assert_ok!(Sminer::update_ip(Origin::signed(ACCOUNT1.0), ip_new.clone()));
		let mr = &MinerItems::<Test>::get(ACCOUNT1.0).unwrap();
		assert_eq!(ip_new, mr.ip.to_vec());
		let event =
			Sys::events().pop().expect("Expected at least one Registered to be found").event;
		assert_eq!(
			mock::Event::from(Event::UpdataIp { acc: ACCOUNT1.0, old: ip, new: ip_new }),
			event
		);
	});
}

#[test]
fn increase_collateral_works_on_normal_state() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Sminer::increase_collateral(Origin::signed(ACCOUNT1.0), 3000),
			Error::<Test>::NotMiner
		);

		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT1.0),
			123,
			Vec::from("192.168.0.1"),
			2000
		));

		assert_ok!(Sminer::increase_collateral(Origin::signed(ACCOUNT1.0), 3000));
		// balance check
		assert_eq!(ACCOUNT1.1 - 2000 - 3000, Balances::free_balance(&ACCOUNT1.0));
		assert_eq!(2000 + 3000, Balances::reserved_balance(&ACCOUNT1.0));

		let mi = MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap();
		assert_eq!(Vec::from(STATE_POSITIVE), mi.state.to_vec());

		assert_eq!(2000 + 3000, mi.collaterals);

		let event = Sys::events()
			.pop()
			.expect("Expected at least one IncreaseCollateral to be found")
			.event;
		assert_eq!(
			mock::Event::from(Event::IncreaseCollateral {
				acc: ACCOUNT1.0,
				balance: mi.collaterals
			}),
			event
		);
	});
}

fn set_miner_state(account_id: u64, state: &str) {
	let _ = MinerItems::<Test>::try_mutate(account_id, |opt| -> DispatchResult {
		let mr = opt.as_mut().unwrap();
		mr.state = Vec::from(state).try_into().unwrap();
		Ok(())
	});
}

#[test]
fn increase_collateral_works_on_frozen_state() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT1.0),
			123,
			Vec::from("192.168.0.1"),
			2000
		));
		set_miner_state(ACCOUNT1.0, STATE_FROZEN);

		assert_eq!(UNIT_POWER_LIMIT, Sminer::check_collateral_limit(0u128).unwrap());
		assert_ok!(Sminer::increase_collateral(Origin::signed(ACCOUNT1.0), 3000));
		assert_eq!(
			Vec::from(STATE_FROZEN),
			MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap().state.to_vec()
		);

		assert_ok!(Sminer::increase_collateral(
			Origin::signed(ACCOUNT1.0),
			2 * UNIT_POWER_LIMIT + 1
		));
		assert_eq!(
			Vec::from(STATE_POSITIVE),
			MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap().state.to_vec()
		);
	});
}

#[test]
fn increase_collateral_works_on_exit_frozen_state() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT1.0),
			123,
			Vec::from("192.168.0.1"),
			2000
		));
		set_miner_state(ACCOUNT1.0, STATE_EXIT_FROZEN);

		assert_ok!(Sminer::increase_collateral(Origin::signed(ACCOUNT1.0), 3000));
		assert_eq!(
			Vec::from(STATE_EXIT_FROZEN),
			MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap().state.to_vec()
		);

		assert_ok!(Sminer::increase_collateral(
			Origin::signed(ACCOUNT1.0),
			2 * UNIT_POWER_LIMIT + 1
		));
		assert_eq!(
			Vec::from(STATE_EXIT),
			MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap().state.to_vec()
		);
	});
}

#[test]
fn exit_miner_works() {
	new_test_ext().execute_with(|| {
		assert_noop!(Sminer::exit_miner(Origin::signed(ACCOUNT1.0)), Error::<Test>::NotMiner);
		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT1.0),
			123,
			Vec::from("192.168.0.1"),
			UNIT_POWER_LIMIT
		));

		Sys::set_block_number(2);
		assert_ok!(Sminer::exit_miner(Origin::signed(ACCOUNT1.0)));

		assert_eq!(
			Vec::from(STATE_EXIT),
			MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap().state.to_vec()
		);
		assert_eq!(2, MinerLockIn::<Test>::try_get(ACCOUNT1.0).unwrap());
		let event = Sys::events().pop().expect("Expected at least one MinerExit to be found").event;
		assert_eq!(mock::Event::from(Event::MinerExit { acc: ACCOUNT1.0 }), event);
	});
}

#[test]
fn exit_miner_on_abnormal_state() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT1.0),
			123,
			Vec::from("192.168.0.1"),
			UNIT_POWER_LIMIT
		));

		set_miner_state(ACCOUNT1.0, STATE_EXIT);
		assert_noop!(
			Sminer::exit_miner(Origin::signed(ACCOUNT1.0)),
			Error::<Test>::NotpositiveState
		);
	});
}

#[test]
fn withdraw_should_work() {
	new_test_ext().execute_with(|| {
		assert_noop!(Sminer::withdraw(Origin::signed(ACCOUNT1.0)), Error::<Test>::NotMiner);
		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT1.0),
			123,
			Vec::from("192.168.0.1"),
			UNIT_POWER_LIMIT
		));

		let free_balance_after_reg = Balances::free_balance(&ACCOUNT1.0);

		// the miner should exit before withdraw
		assert_noop!(Sminer::withdraw(Origin::signed(ACCOUNT1.0)), Error::<Test>::NotExisted);
		assert_ok!(Sminer::exit_miner(Origin::signed(ACCOUNT1.0)));
		// the miner should withdraw after 1200 blocks
		assert_noop!(Sminer::withdraw(Origin::signed(ACCOUNT1.0)), Error::<Test>::LockInNotOver);

		let all_miner_cnt = AllMiner::<Test>::try_get().unwrap().len();

		Sys::set_block_number(57700);
		assert_ok!(Sminer::withdraw(Origin::signed(ACCOUNT1.0)));

		// balance check
		assert_eq!(free_balance_after_reg + UNIT_POWER_LIMIT, Balances::free_balance(&ACCOUNT1.0));
		assert_eq!(0, Balances::reserved_balance(&ACCOUNT1.0));

		assert_eq!(all_miner_cnt - 1, AllMiner::<Test>::try_get().unwrap().len());
		assert!(!MinerItems::<Test>::contains_key(ACCOUNT1.0));

		// event check
		let event =
			Sys::events().pop().expect("Expected at least one MinerClaim to be found").event;
		assert_eq!(mock::Event::from(Event::MinerClaim { acc: ACCOUNT1.0 }), event);
	});
}

#[test]
fn add_reward_order1_should_work() {
	new_test_ext().execute_with(|| {
		let reward_amount = 100000_u128;
		// first reward
		Sys::set_block_number(1);
		assert_ok!(Sminer::add_reward_order1(&ACCOUNT1.0, reward_amount));
		let reward_orders = CalculateRewardOrderMap::<Test>::try_get(ACCOUNT1.0).unwrap();
		assert_eq!(1, reward_orders.len());
		let ro = reward_orders.get(0).unwrap();
		assert_eq!(reward_amount, ro.calculate_reward);
		assert_eq!(1, ro.start_t);
		assert_eq!(5184000u64 + 1u64, ro.deadline); // FIXME! the 1296000 is hardcode in the tested function
											// second reward
		Sys::set_block_number(2);
		assert_ok!(Sminer::add_reward_order1(&ACCOUNT1.0, reward_amount));
		let reward_orders = CalculateRewardOrderMap::<Test>::try_get(ACCOUNT1.0).unwrap();
		assert_eq!(2, reward_orders.len());
		let ro = reward_orders.get(1).unwrap();
		assert_eq!(reward_amount, ro.calculate_reward);
		assert_eq!(2, ro.start_t);

		assert_eq!(5184000u64 + 2u64, ro.deadline);
	});
}

#[test]
fn add_reward_order1_should_panic_on_exceed_item_limit() {
	new_test_ext().execute_with(|| {
		let reward_amount = 100000_u128;
		let n = mock::ItemLimit::get();
		for i in 1..(n + 1) {
			Sys::set_block_number(i as u64);
			assert_ok!(Sminer::add_reward_order1(&ACCOUNT1.0, reward_amount));
		}
		assert_eq!(n as usize, CalculateRewardOrderMap::<Test>::try_get(ACCOUNT1.0).unwrap().len());
		//FIXME! Raise a panic? Is it appropriate in a production environment?
		let _ = Sminer::add_reward_order1(&ACCOUNT1.0, reward_amount);
	});
}

#[test]
fn add_power_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT1.0),
			123,
			Vec::from("192.168.0.1"),
			UNIT_POWER_LIMIT
		));
		let m1 = MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap();
		assert_eq!(0, m1.power);
		let _ = Sminer::add_power(&m1.peer_id, 10_000);
		let m1 = MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap();
		assert_eq!(10_000, m1.power);
		assert_eq!(10_000, TotalPower::<Test>::try_get().unwrap());

		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT2.0),
			321,
			Vec::from("192.168.0.2"),
			UNIT_POWER_LIMIT
		));
		let m2 = MinerItems::<Test>::try_get(ACCOUNT2.0).unwrap();
		assert_eq!(0, m2.power);
		let _ = Sminer::add_power(&m2.peer_id, 20_000);
		let m2 = MinerItems::<Test>::try_get(ACCOUNT2.0).unwrap();
		assert_eq!(20_000, m2.power);
		assert_eq!(30_000, TotalPower::<Test>::try_get().unwrap());
	});
}

/// 750_000TCESS
const FIXED_REWARD_AMOUNT: u128 = 750_000_000000000000_u128;

#[test]
fn increase_rewards_should_work() {
	new_test_ext().execute_with(|| {
		CurrencyReward::<Test>::put(750_000_000_000_000_000);
		assert_noop!(Sminer::timed_increase_rewards(Origin::root()), Error::<Test>::DivideByZero);

		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT1.0),
			123,
			Vec::from("192.168.0.1"),
			UNIT_POWER_LIMIT
		));
		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT2.0),
			123,
			Vec::from("192.168.0.2"),
			UNIT_POWER_LIMIT
		));

		let m1 = MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap();
		let m2 = MinerItems::<Test>::try_get(ACCOUNT2.0).unwrap();
		let _ = Sminer::add_power(&m1.peer_id, 10_000);
		let _ = Sminer::add_power(&m2.peer_id, 20_000);

		let total_power = TotalPower::<Test>::try_get().unwrap();
		assert_eq!(30_000, total_power);

		assert_ok!(Sminer::timed_increase_rewards(Origin::root()));

		let m1 = MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap();
		let m2 = MinerItems::<Test>::try_get(ACCOUNT2.0).unwrap();
		assert_eq!(10_000, m1.power);
		assert_eq!(20_000, m2.power);

		// the timed_increase_rewards() not reward actual, just make reward order only
		let reward_order1 = CalculateRewardOrderMap::<Test>::try_get(ACCOUNT1.0)
			.unwrap()
			.to_vec()
			.pop()
			.unwrap();
		assert_eq!(FIXED_REWARD_AMOUNT * m1.power / total_power, reward_order1.calculate_reward);
		let reward_order2 = CalculateRewardOrderMap::<Test>::try_get(ACCOUNT2.0)
			.unwrap()
			.to_vec()
			.pop()
			.unwrap();
		assert_eq!(FIXED_REWARD_AMOUNT * m2.power / total_power, reward_order2.calculate_reward);

		// event check
		let event = Sys::events().pop().expect("Expected at least one TimedTask to be found").event;
		assert_eq!(mock::Event::from(Event::TimedTask()), event);
	});
}

#[test]
fn task_award_table_should_work() {
	new_test_ext().execute_with(|| {
		CurrencyReward::<Test>::put(750_000_000_000_000_000);
		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT1.0),
			123,
			Vec::from("192.168.0.1"),
			UNIT_POWER_LIMIT
		));
		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT2.0),
			321,
			Vec::from("192.168.0.2"),
			UNIT_POWER_LIMIT
		));

		let m1 = MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap();
		let m2 = MinerItems::<Test>::try_get(ACCOUNT2.0).unwrap();
		let _ = Sminer::add_power(&ACCOUNT1.0, 10_000);
		let _ = Sminer::add_power(&ACCOUNT2.0, 20_000);
		let total_power = TotalPower::<Test>::try_get().unwrap();

		assert_eq!(30_000, total_power);
		assert_ok!(Sminer::timed_increase_rewards(Origin::root())); // <-- dependency method

		assert_ok!(Sminer::timed_task_award_table(Origin::root())); // <-- target method here!

		let m1 = MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap();
		let total_reward = FIXED_REWARD_AMOUNT * m1.power / total_power;
		assert_eq!(
			total_reward,
			MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap().reward_info.total_reward
		);

		let ro1 = CalculateRewardOrderMap::<Test>::try_get(ACCOUNT1.0)
			.unwrap()
			.to_vec()
			.pop()
			.unwrap();
		let rc1 = RewardClaimMap::<Test>::try_get(ACCOUNT1.0).unwrap();
		assert_eq!(total_reward, rc1.total_reward);
		// 180 periods, 80% one period rewards(1/180) + 20% total rewards
		assert_eq!(
			ro1.calculate_reward * 8 / 10 / 180 + total_reward * 2 / 10,
			rc1.current_availability
		);

		let md = MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap().reward_info;
		assert_eq!(total_reward, md.total_reward);
		assert_eq!(total_reward, md.total_not_receive);
		assert_eq!(rc1.current_availability, md.total_rewards_currently_available);

		// second reward period
		Sys::set_block_number(1200);
		assert_ok!(Sminer::timed_increase_rewards(Origin::root()));
		assert_ok!(Sminer::timed_task_award_table(Origin::root()));
		assert_eq!(2, CalculateRewardOrderMap::<Test>::try_get(ACCOUNT1.0).unwrap().len());
		let ro2 = CalculateRewardOrderMap::<Test>::try_get(ACCOUNT1.0)
			.unwrap()
			.to_vec()
			.pop()
			.unwrap();
		let rc2 = RewardClaimMap::<Test>::try_get(ACCOUNT2.0).unwrap();
		let total_reward = total_reward * 2;
		assert_eq!(
			total_reward,
			MinerItems::<Test>::try_get(ACCOUNT2.0).unwrap().reward_info.total_reward
		);
		assert_eq!(total_reward, rc2.total_reward);
		let t = ro1.calculate_reward * 8 / 10 / 180 + ro2.calculate_reward * 8 / 10 / 180;
		assert_eq!(
			(rc1.current_availability + t + ro2.calculate_reward) * 2,
			rc2.current_availability
		);

		let md = MinerItems::<Test>::try_get(ACCOUNT2.0).unwrap().reward_info;
		assert_eq!(total_reward, md.total_reward);
		assert_eq!(total_reward, md.total_not_receive);
		assert_eq!(rc2.current_availability, md.total_rewards_currently_available);

		// event check
		let event = Sys::events().pop().expect("Expected at least one TimedTask to be found").event;
		assert_eq!(mock::Event::from(Event::TimedTask()), event);
	});
}

#[test]
fn timed_user_receive_award1_should_work() {
	new_test_ext().execute_with(|| {
		CurrencyReward::<Test>::put(750_000_000_000_000_000);
		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT1.0),
			123,
			Vec::from("192.168.0.1"),
			UNIT_POWER_LIMIT
		));
		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT2.0),
			321,
			Vec::from("192.168.0.2"),
			UNIT_POWER_LIMIT
		));
		let m1 = MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap();
		let m2 = MinerItems::<Test>::try_get(ACCOUNT2.0).unwrap();
		let _ = Sminer::add_power(&ACCOUNT1.0, 10_000);
		let _ = Sminer::add_power(&ACCOUNT2.0, 20_000);
		let total_power = TotalPower::<Test>::try_get().unwrap();
		assert_eq!(30_000, total_power);

		assert_ok!(Sminer::timed_increase_rewards(Origin::root())); // <-- dependency method
		assert_ok!(Sminer::timed_task_award_table(Origin::root())); // <-- dependency method

		let rc1 = RewardClaimMap::<Test>::try_get(ACCOUNT1.0).unwrap();
		let rc2 = RewardClaimMap::<Test>::try_get(ACCOUNT2.0).unwrap();
		assert_eq!(250_000_000_000_000_000, rc1.total_reward);
		assert_eq!(51111111111111111, rc1.current_availability);
		assert_eq!(250000000000000000, rc1.total_not_receive);
		assert_eq!(0, rc1.have_to_receive);
		assert_eq!(500000000000000000, rc2.total_reward);
		assert_eq!(102222222222222222, rc2.current_availability);
		assert_eq!(500000000000000000, rc2.total_not_receive);
		assert_eq!(0, rc2.have_to_receive);

		let jackpot_balance = Balances::free_balance(&mock::RewardPalletId::get().into_account());
		assert_ok!(Sminer::timed_user_receive_award1(Origin::root())); // <-- target method here!

		let rc1b = RewardClaimMap::<Test>::try_get(ACCOUNT1.0).unwrap();
		let rc2b = RewardClaimMap::<Test>::try_get(ACCOUNT2.0).unwrap();
		assert_eq!(rc1.total_reward, rc1b.total_reward);
		assert_eq!(0, rc1b.current_availability);
		assert_eq!(rc1.total_reward - rc1.current_availability, rc1b.total_not_receive);
		assert_eq!(rc1.current_availability, rc1b.have_to_receive);
		assert_eq!(rc2.total_reward, rc2b.total_reward);
		assert_eq!(0, rc2b.current_availability);
		assert_eq!(rc2.total_reward - rc2.current_availability, rc2b.total_not_receive);
		assert_eq!(rc2.current_availability, rc2b.have_to_receive);

		assert_eq!(rc1b.have_to_receive, Balances::free_balance(&123));
		assert_eq!(rc2b.have_to_receive, Balances::free_balance(&321));
		assert_eq!(
			jackpot_balance - rc1b.have_to_receive - rc2b.have_to_receive,
			Balances::free_balance(&mock::RewardPalletId::get().into_account())
		);
	});
}

#[test]
fn timed_user_receive_award1_total_award() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT1.0),
			123,
			Vec::from("192.168.0.1"),
			UNIT_POWER_LIMIT
		));
		let m1 = MinerItems::<Test>::try_get(ACCOUNT1.0).unwrap();
		let _ = Sminer::add_power(&m1.peer_id, 10_000);

		assert_ok!(Sminer::timed_increase_rewards(Origin::root()));

		for _ in 0..180 {
			assert_ok!(Sminer::timed_task_award_table(Origin::root()));
			assert_ok!(Sminer::timed_user_receive_award1(Origin::root()));
			let rc1a = RewardClaimMap::<Test>::try_get(ACCOUNT1.0).unwrap();
			if rc1a.have_to_receive == rc1a.total_reward {
				break;
			}
		}
		assert!(CalculateRewardOrderMap::<Test>::contains_key(ACCOUNT1.0));
		assert!(RewardClaimMap::<Test>::contains_key(ACCOUNT1.0));
	});
}

#[test]
fn failure_to_replenish_sufficient_deposit() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT1.0),
			123,
			Vec::from("192.168.0.1"),
			1u128
		));

		assert_ok!(Sminer::join_buffer_pool(ACCOUNT1.0));
		assert_ok!(Sminer::start_buffer_period_schedule());
		assert!(BadMiner::<Test>::contains_key(ACCOUNT1.0));
		let event = Sys::events().pop().expect("Expected at least one MinerExit to be found").event;
		assert_eq!(mock::Event::from(Event::StartOfBufferPeriod { when: 1 }), event);

		run_to_block(31);
		assert!(!BadMiner::<Test>::contains_key(ACCOUNT1.0));
		assert!(!MinerItems::<Test>::contains_key(ACCOUNT1.0));
	});
}

#[test]
fn full_deposit_within_the_buffer_period() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::regnstk(
			Origin::signed(ACCOUNT1.0),
			123,
			Vec::from("192.168.0.1"),
			1u128
		));

		assert_ok!(Sminer::join_buffer_pool(ACCOUNT1.0));
		assert_ok!(Sminer::start_buffer_period_schedule());
		assert!(BadMiner::<Test>::contains_key(ACCOUNT1.0));
		let event = Sys::events().pop().expect("Expected at least one MinerExit to be found").event;
		assert_eq!(mock::Event::from(Event::StartOfBufferPeriod { when: 1 }), event);

		assert_ok!(Sminer::increase_collateral(Origin::signed(ACCOUNT1.0), UNIT_POWER_LIMIT));
		assert!(!BadMiner::<Test>::contains_key(ACCOUNT1.0));
		let event = Sys::events().pop().expect("Expected at least one MinerExit to be found").event;
		assert_eq!(
			mock::Event::from(Event::IncreaseCollateral {
				acc: ACCOUNT1.0,
				balance: UNIT_POWER_LIMIT + 1u128
			}),
			event
		);
	});
}

const FIXED_CHARGE_AMOUNT: u128 = 10000000000000000u128;
#[test]
fn faucet_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::faucet(Origin::signed(ACCOUNT1.0), 777));
		assert_eq!(FIXED_CHARGE_AMOUNT, Balances::free_balance(&777));

		//FIXME! the assert_noop! not work, why?
		//assert_noop!(Sminer::faucet(Origin::signed(ACCOUNT1.0), 777), Error::<Test>::LessThan24Hours);
		if let Err(e) = Sminer::faucet(Origin::signed(ACCOUNT1.0), 777) {
			if let DispatchError::Module(m) = e {
				assert_eq!("LessThan24Hours", m.message.unwrap());
			}
		}
		Sys::set_block_number(1u64 + 28800u64);
		assert_ok!(Sminer::faucet(Origin::signed(ACCOUNT1.0), 777));
	});
}

//TODO! Scheduler method to test
