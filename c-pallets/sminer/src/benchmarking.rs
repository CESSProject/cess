use super::*;
use crate::Pallet as Sminer;
use codec::{alloc::string::ToString, Decode};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_support::{
	dispatch::UnfilteredDispatchable,
	pallet_prelude::*,
	traits::{Currency, CurrencyToVote, Get, Imbalance},
};

use frame_system::RawOrigin;
use sp_runtime::{
	traits::{Bounded, CheckedMul, One, StaticLookup, TrailingZeroInput, Zero},
	Perbill, Percent,
};

const SEED: u32 = 2190502;
const MAX_SPANS: u32 = 100;

pub fn add_miner<T: Config>(name: &'static str) -> T::AccountId {
	let miner: T::AccountId = account(name.clone(), 100, SEED);
	let ip = IpAddress::IPV4([127, 0, 0, 1], 15001);
	T::Currency::make_free_balance_be(&miner, BalanceOf::<T>::max_value());
	whitelist_account!(miner);
	let _ = Sminer::<T>::regnstk(
		RawOrigin::Signed(miner.clone()).into(),
		miner.clone(),
		ip,
		2_000u32.into(),
	);
	miner.clone()
}

benchmarks! {
	regnstk {
		let caller = account("user1", 100, SEED);
		let ip = IpAddress::IPV4([127,0,0,1], 15001);
		T::Currency::make_free_balance_be(
			&caller,
			BalanceOf::<T>::max_value(),
		);
	}: _(RawOrigin::Signed(caller.clone()), caller.clone(), ip, 2_000u32.into())
	verify {
		assert!(<MinerItems<T>>::contains_key(&caller));
	}

	increase_collateral {
		let miner = add_miner::<T>("miner1");
	}: _(RawOrigin::Signed(miner.clone()), 2_000u32.into())
	verify {
		let miner_info = <MinerItems<T>>::get(&miner).unwrap();
		assert_eq!(4000u32, miner_info.collaterals.saturated_into::<u32>());
	}

	update_beneficiary {
		let miner = add_miner::<T>("miner1");
		let caller: AccountOf<T> = account("user1", 100, SEED);
	}: _(RawOrigin::Signed(miner.clone()), caller.clone())
	verify {
		let miner_info = <MinerItems<T>>::get(&miner).unwrap();
		assert_eq!(caller, miner_info.beneficiary);
	}

	update_ip {
		let miner = add_miner::<T>("miner1");
		let new_ip = IpAddress::IPV4([127,0,0,1], 15001);
	}: _(RawOrigin::Signed(miner.clone()), new_ip.clone())
	verify {
		let miner_info = <MinerItems<T>>::get(&miner).unwrap();
		assert_eq!(new_ip, miner_info.ip);
	}

	exit_miner {
		let miner = add_miner::<T>("miner1");
		let miner_info = <MinerItems<T>>::get(&miner).unwrap();
		let state = "positive".as_bytes().to_vec();
		assert_eq!(state, miner_info.state.to_vec());
	}: _(RawOrigin::Signed(miner.clone()))
	verify {
		let miner_info = <MinerItems<T>>::get(&miner).unwrap();
		let state = "exit".as_bytes().to_vec();
		assert_eq!(state, miner_info.state.to_vec());
	}

	withdraw {
		let miner = add_miner::<T>("miner1");
		Sminer::<T>::exit_miner(RawOrigin::Signed(miner.clone()).into())?;
		let colling = MinerLockIn::<T>::get(&miner).unwrap();
		let colling_number: u32 = colling.saturated_into();
		// log::info!("{}", colling_number);
		<frame_system::Pallet<T>>::set_block_number((colling_number + 60000).saturated_into());
	}: _(RawOrigin::Signed(miner.clone()))
	verify {
		assert!(!<MinerItems<T>>::contains_key(&miner));
	}

	timed_increase_rewards {
		let miner = add_miner::<T>("miner1");
		let reward: BalanceOf<T> = 8000u32.saturated_into();
		<CurrencyReward<T>>::put(reward);
		<TotalIdleSpace<T>>::put(1000);
		<MinerItems<T>>::try_mutate(&miner, |s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.power = 1000;
			Ok(())
		})?;
	}: _(RawOrigin::Root)
	verify {
		let reward_map = <CalculateRewardOrderMap<T>>::get(&miner);
		assert_eq!(1, reward_map.len());
		assert_eq!(reward_map[0].calculate_reward, 8000);
	}


	timed_task_award_table {
				let v in 1 .. 50;
				// let mut miner_name_list: Vec<Vec<u8>> = Vec::new();
				let mut miner_list: Vec<T::AccountId> = Vec::new();
				let reward: BalanceOf<T> = 8000u32.saturated_into();
				<CurrencyReward<T>>::put(reward);
				<TotalIdleSpace<T>>::put(1000 * v as u128);
				for i in 0 .. v {
					// let test = i.to_string().as_bytes().to_vec();
					// miner_name_list.push(test);
					let miner = add_miner::<T>(Box::leak(i.to_string().into_boxed_str()));
					// let miner = add_miner::<T>(std::str::from_utf8(&miner_name_list[i as usize]).unwrap());
					miner_list.push(miner.clone());
				<MinerItems<T>>::try_mutate(&miner, |s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.power = 1000;
			Ok(())
			})?;
				}
		Sminer::<T>::timed_increase_rewards(RawOrigin::Root.into())?;
	}: _(RawOrigin::Root)
	verify {
				let count = <CalculateRewardOrderMap<T>>::count();
		assert_eq!(count, v);
	}

	timed_user_receive_award1 {
		let v in 1 .. 50;
				// let mut miner_name_list: Vec<Vec<u8>> = Vec::new();
				let mut miner_list: Vec<T::AccountId> = Vec::new();
				let reward: BalanceOf<T> = 8000u32.saturated_into();
				<CurrencyReward<T>>::put(reward);
				<TotalIdleSpace<T>>::put(1000 * v as u128);
				for i in 0 .. v {
					// let test = i.to_string().as_bytes().to_vec();
					// miner_name_list.push(test);
					let miner = add_miner::<T>(Box::leak(i.to_string().into_boxed_str()));
					// let miner = add_miner::<T>(std::str::from_utf8(&miner_name_list[i as usize]).unwrap());
					miner_list.push(miner.clone());
				<MinerItems<T>>::try_mutate(&miner, |s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.power = 1000;
			Ok(())
			})?;
				}

		let acc = T::PalletId::get().into_account_truncating();
		T::Currency::make_free_balance_be(
			&acc,
			BalanceOf::<T>::max_value(),
		);
		Sminer::<T>::timed_increase_rewards(RawOrigin::Root.into())?;
		Sminer::<T>::timed_task_award_table(RawOrigin::Root.into())?;
	}: _(RawOrigin::Root)
	verify {
			for miner in miner_list.iter() {
				let info = RewardClaimMap::<T>::get(&miner).unwrap();
				let share = 8000 / v as u128;
				let reward: u128 = ((share -((share * 2 / 10))) / 180) + (share * 2 / 10);
				let reward: BalanceOf<T> = reward.try_into().map_err(|_e| "reward convert err")?;
				assert_eq!(info.have_to_receive, reward);
			}
	}

	timing_task_increase_power_rewards {}: _(RawOrigin::Root, 10u32.saturated_into(), 28800u32.saturated_into(), 365)
	verify {
		let id = "msminerA".as_bytes().to_vec();
		let next_block = T::SScheduler::next_dispatch_time(id).map_err(|_e| "next time err")?;
		assert_eq!(next_block, 10u32.saturated_into());
	}

	timing_task_award_table {}: _(RawOrigin::Root, 10u32.saturated_into(), 28800u32.saturated_into(), 365)
	verify {
		let id = "msminerB".as_bytes().to_vec();
		let next_block = T::SScheduler::next_dispatch_time(id).map_err(|_e| "next time err")?;
		assert_eq!(next_block, 10u32.saturated_into());
	}

	timing_user_receive_award {}: _(RawOrigin::Root, 10u32.saturated_into(), 28800u32.saturated_into(), 365)
	verify {
		let id = "msminerC".as_bytes().to_vec();
		let next_block = T::SScheduler::next_dispatch_time(id).map_err(|_e| "next time err")?;
		assert_eq!(next_block, 10u32.saturated_into());
	}

	// faucet_top_up {
	//     log::info!("point 1");
	//     let caller: T::AccountId = whitelisted_caller();
	//     let existential_deposit = T::Currency::minimum_balance();
	//     T::Currency::make_free_balance_be(
	// 		&caller,
	// 		BalanceOf::<T>::max_value(),
	// 	);
	//     let fa: BalanceOf<T> =  existential_deposit.checked_mul(&10u32.saturated_into()).ok_or("over flow")?;
	//     log::info!("point 2");
	// }: _(RawOrigin::Signed(caller), fa)
	// verify {
	//     assert_eq!(1, 1)
	// }
}
