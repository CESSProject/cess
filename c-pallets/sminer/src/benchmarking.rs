use super::*;
use crate::{Pallet as Sminer, *};
use codec::{alloc::string::ToString, Decode};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_support::{
	dispatch::UnfilteredDispatchable,
	pallet_prelude::*,
	traits::{Currency, CurrencyToVote, Get, Imbalance},
};

use sp_runtime::{
	traits::{Bounded, One, StaticLookup, TrailingZeroInput, Zero, CheckedMul},
	Perbill, Percent,
};
use frame_system::RawOrigin;

const SEED: u32 = 2190502;
const MAX_SPANS: u32 = 100;

pub fn add_miner<T: Config>() -> Result<T::AccountId, &'static str> {
	let miner: T::AccountId = account("miner1", 100, SEED);
	let ip = "1270008080".as_bytes().to_vec();
	T::Currency::make_free_balance_be(
		&miner,
		BalanceOf::<T>::max_value(),
	);
	whitelist_account!(miner);
	Sminer::<T>::regnstk(
		RawOrigin::Signed(miner.clone()).into(),
		miner.clone(),
		ip,
		2_000u32.into(),
	)?;
	Ok(miner.clone())
}

benchmarks! {
    regnstk {
        let caller = account("user1", 100, SEED);
        let ip = "1270008080".as_bytes().to_vec();
        T::Currency::make_free_balance_be(
            &caller,
            BalanceOf::<T>::max_value(),
        );
    }: _(RawOrigin::Signed(caller.clone()), caller.clone(), ip, 2_000u32.into())
    verify {
        assert!(<MinerItems<T>>::contains_key(&caller));
    }

    increase_collateral {
        let miner = add_miner::<T>()?;
    }: _(RawOrigin::Signed(miner.clone()), 2_000u32.into())
    verify {
        let miner_info = <MinerItems<T>>::get(&miner).unwrap();
        assert_eq!(4000u32, miner_info.collaterals.saturated_into::<u32>());
    }

    updata_beneficiary {
        let miner = add_miner::<T>()?;
        let caller: AccountOf<T> = account("user1", 100, SEED);
    }: _(RawOrigin::Signed(miner.clone()), caller.clone())
    verify {
        let miner_info = <MinerItems<T>>::get(&miner).unwrap();
        assert_eq!(caller, miner_info.beneficiary);
    }

    updata_ip {
        let miner = add_miner::<T>()?;
        let new_ip = "192.168.1.1:8000".as_bytes().to_vec();
    }: _(RawOrigin::Signed(miner.clone()), new_ip.clone())
    verify {
        let miner_info = <MinerItems<T>>::get(&miner).unwrap();
        assert_eq!(new_ip, miner_info.ip.to_vec());
    }

    exit_miner {
        let miner = add_miner::<T>()?;
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
        let miner = add_miner::<T>()?;
        Sminer::<T>::exit_miner(RawOrigin::Signed(miner.clone()).into())?;
        let colling = MinerColling::<T>::get(&miner).unwrap();
        let colling_number: u32 = colling.saturated_into();
        // log::info!("{}", colling_number);
        <frame_system::Pallet<T>>::set_block_number((colling_number + 60000).saturated_into());
    }: _(RawOrigin::Signed(miner.clone()))
    verify {
        assert!(!<MinerItems<T>>::contains_key(&miner));
    }

    timed_increase_rewards {
        let miner = add_miner::<T>()?;
        let reward: BalanceOf<T> = 8000u32.saturated_into();
        <CurrencyReward<T>>::put(reward);
        <TotalPower<T>>::put(1000);
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
        let miner = add_miner::<T>()?;
        let reward: BalanceOf<T> = 8000u32.saturated_into();
        <CurrencyReward<T>>::put(reward);
        <TotalPower<T>>::put(1000);
        <MinerItems<T>>::try_mutate(&miner, |s_opt| -> DispatchResult {
            let s = s_opt.as_mut().unwrap();
            s.power = 1000;
            Ok(())
        })?;
        Sminer::<T>::timed_increase_rewards(RawOrigin::Root.into())?;
    }: _(RawOrigin::Root)
    verify {
        assert!(<RewardClaimMap<T>>::contains_key(miner.clone())); 
        let info = <RewardClaimMap<T>>::get(&miner).unwrap();
        let reward: BalanceOf<T> = 8000u128.try_into().map_err(|_e| "reward convert err")?;
        assert_eq!(info.total_reward, reward);
    }

    timed_user_receive_award1 {
        let miner = add_miner::<T>()?;
        let acc = T::PalletId::get().into_account();
        T::Currency::make_free_balance_be(
            &acc,
            BalanceOf::<T>::max_value(),
        );
        let reward: BalanceOf<T> = 8000u32.saturated_into();
        <CurrencyReward<T>>::put(reward);
        <TotalPower<T>>::put(1000);
        <MinerItems<T>>::try_mutate(&miner, |s_opt| -> DispatchResult {
            let s = s_opt.as_mut().unwrap();
            s.power = 1000;
            Ok(())
        })?;
        Sminer::<T>::timed_increase_rewards(RawOrigin::Root.into())?;
        Sminer::<T>::timed_task_award_table(RawOrigin::Root.into())?;
    }: _(RawOrigin::Root)
    verify {
        let info = RewardClaimMap::<T>::get(&miner).unwrap();
        let reward: u128 = ((8000 -((8000 * 2 / 10))) / 180) + (8000 * 2 / 10);
        let reward: BalanceOf<T> = reward.try_into().map_err(|_e| "reward convert err")?;
        assert_eq!(info.have_to_receive, reward);
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

    faucet_top_up {
        log::info!("point 1");
        let caller: T::AccountId = whitelisted_caller();
        let existential_deposit = T::Currency::minimum_balance();
        T::Currency::make_free_balance_be(
			&caller,
			BalanceOf::<T>::max_value(),
		);
        let fa: BalanceOf<T> =  existential_deposit.checked_mul(&10u32.saturated_into()).ok_or("over flow")?;
        log::info!("point 2");
    }: _(RawOrigin::Signed(caller), fa)
    verify {
        assert_eq!(1, 1)
    }
}
