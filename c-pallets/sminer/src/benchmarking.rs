#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{
	whitelisted_caller, benchmarks,
};
use frame_system::RawOrigin as SystemOrigin;
use sp_runtime::traits::Bounded;
type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
use sp_std::convert::TryInto;
use crate::Pallet as Sminer;

fn new_miner<T: Config>() {
	let caller: T::AccountId = whitelisted_caller();
    assert!(Pallet::<T>::regnstk(
        SystemOrigin::Signed(caller.clone()).into(),
        
    )
    .is_ok());  
}

fn add_owner<T: Config>() {
    let caller: T::AccountId = whitelisted_caller();
    EtcdRegisterOwner::<T>::mutate(|s|{
        s.push(caller.clone());
    });
    EtcdOwner::<T>::put(caller.clone());
}

fn add_calculate<T: Config>() {
    let caller: T::AccountId = whitelisted_caller();
    let now = T::BlockNumber::from(500u32);
    let deadline = now + T::BlockNumber::from(18000u32);
    let reward: u128 = 128731209;
    let order: Vec<CalculateRewardOrder<T>> = vec![CalculateRewardOrder::<T>{
        calculate_reward: reward,
        start_t: now,
        deadline: deadline,
    }];
    <CalculateRewardOrderMap<T>>::insert(
        caller,
        order,
    );
}

fn add_rewardclaimmap<T: Config>() {
    let caller: T::AccountId = whitelisted_caller();
    let reward2: BalanceOf<T> = BalanceOf::<T>::from(5000u32);
    let currently_available: BalanceOf<T> = BalanceOf::<T>::from(300u32);
    <RewardClaimMap<T>>::insert(
        &caller, 
        RewardClaim::<T> {
            total_reward: reward2,
            total_rewards_currently_available: reward2,
            have_to_receive: BalanceOf::<T>::from(3000u32),
            current_availability: currently_available,
            total_not_receive: reward2,
        }
    );
}

fn init<T: Config>() {
    let _caller: T::AccountId = whitelisted_caller();
    let value = BalanceOf::<T>::from(0 as u32);
    let mst = MinerStatInfo::<T> {
        total_miners: 0u64,
        active_miners: 0u64,
        staking: value,
        miner_reward: value,
        sum_files: 0u128,
    };
    <MinerStatValue<T>>::put(mst);
}

benchmarks! {
    regnstk {
        init::<T>();
        let caller: T::AccountId = whitelisted_caller();
        let ip = 15646984;
        let port = 1651654;
        let fileport = 35535;
        let staking_val: BalanceOf<T> = BalanceOf::<T>::from(0u32);
        let beneficiary = T::Lookup::unlookup(caller.clone());
    }: _(SystemOrigin::Signed(caller.clone()), beneficiary, ip, staking_val)
    verify {
        assert_eq!(<MinerItems<T>>::get(&caller).unwrap().peerid, 1);
    }

    redeem {
        new_miner::<T>();
        let caller: T::AccountId = whitelisted_caller();
    }: _(SystemOrigin::Signed(caller.clone()))
    verify {

    }

    claim {
        new_miner::<T>();
        let caller: T::AccountId = whitelisted_caller();
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let money: BalanceOf<T> = BalanceOf::<T>::from(10000000u32);
        Sminer::<T>::faucet_top_up(SystemOrigin::Signed(caller.clone()).into(), money)?;
        MinerItems::<T>::mutate(&caller, |s_opt|{
            let s = s_opt.as_mut().unwrap();
            s.earnings = BalanceOf::<T>::from(50u32);
        });
    }: _(SystemOrigin::Signed(caller.clone()))
    verify {

    }

    initi {
        let caller: T::AccountId = whitelisted_caller();
    }: _(SystemOrigin::Signed(caller.clone()))
    verify {
        assert_eq!(<MinerStatValue<T>>::get().unwrap().total_miners, 0);
    }

    add_available_storage {
        let caller: T::AccountId = whitelisted_caller();
        let increment: u128 = 496841651;
    }: _(SystemOrigin::Signed(caller.clone()), increment)
    verify {
        assert_eq!(<StorageInfoValue<T>>::get().available_storage, increment);
    }

    add_used_storage {
        let caller: T::AccountId = whitelisted_caller();
        let increment: u128 = 496841651;
    }: _(SystemOrigin::Signed(caller.clone()), increment)
    verify {
        assert_eq!(<StorageInfoValue<T>>::get().used_storage, increment);
    }

    timing_storage_space {
        let caller: T::AccountId = whitelisted_caller();
    }: _(SystemOrigin::Signed(caller.clone()))
    verify {

    }

    timing_task_storage_space {
        let caller: T::AccountId = whitelisted_caller();
        let when: T::BlockNumber = 80u32.into();
        let cycle: T::BlockNumber = 10u32.into();
        let degree: u32 = 10;
    }: _(SystemOrigin::Signed(caller.clone()), when, cycle, degree)
    verify {

    }

    timing_storage_space_thirty_days {
        let caller: T::AccountId = whitelisted_caller();
    }: _(SystemOrigin::Signed(caller.clone()))
    verify {

    }

    timed_increase_rewards {
        init::<T>();
        let caller: T::AccountId = whitelisted_caller();
        <TotalPower<T>>::put(1200);
    }: _(SystemOrigin::Signed(caller.clone()))
    verify {

    }

    timing_task_increase_power_rewards {
        init::<T>();
        let caller: T::AccountId = whitelisted_caller();
        let when: T::BlockNumber = 80u32.into();
        let cycle: T::BlockNumber = 10u32.into();
        let degree: u32 = 10;
        <TotalPower<T>>::put(1200);
    }: _(SystemOrigin::Signed(caller.clone()), when, cycle, degree)
    verify {

    }

    del_reward_order {
        add_calculate::<T>();
        let caller: T::AccountId = whitelisted_caller();
        let calculate: u128 = 0;
    }: _(SystemOrigin::Signed(caller.clone()), caller.clone(), calculate)
    verify {

    }

    timed_user_receive_award1 {
        new_miner::<T>();
        add_rewardclaimmap::<T>();
        let caller: T::AccountId = whitelisted_caller();    
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let money: BalanceOf<T> = 100000000000000000u128.try_into().map_err(|_e| "Error")?;
        Sminer::<T>::faucet_top_up(SystemOrigin::Signed(caller.clone()).into(), money)?;
    }: _(SystemOrigin::Signed(caller.clone()))
    verify {

    }

    timing_user_receive_award {
        new_miner::<T>();
        add_calculate::<T>();
        add_rewardclaimmap::<T>();
        let caller: T::AccountId = whitelisted_caller();
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let money: BalanceOf<T> = 100000000000000000u128.try_into().map_err(|_e| "Error")?;
        Sminer::<T>::faucet_top_up(SystemOrigin::Signed(caller.clone()).into(), money)?;
        let when: T::BlockNumber = 80u32.into();
        let cycle: T::BlockNumber = 10u32.into();
        let degree: u32 = 10;
    }: _(SystemOrigin::Signed(caller.clone()), when, cycle, degree)
    verify {

    }
    
    timed_task_award_table {
        new_miner::<T>();
        add_calculate::<T>();
        add_rewardclaimmap::<T>();
        let caller: T::AccountId = whitelisted_caller();
    }: _(SystemOrigin::Signed(caller.clone()))
    verify {

    }

    timing_task_award_table {
        new_miner::<T>();
        add_calculate::<T>();
        add_rewardclaimmap::<T>();
        let caller: T::AccountId = whitelisted_caller();
        let when: T::BlockNumber = 80u32.into();
        let cycle: T::BlockNumber = 10u32.into();
        let degree: u32 = 10;
    }: _(SystemOrigin::Signed(caller.clone()), when, cycle, degree)
    verify {

    }

    punishment {
        let caller: T::AccountId = whitelisted_caller();
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let money: BalanceOf<T> = 100000000000000000u128.try_into().map_err(|_e| "Error")?;
        Sminer::<T>::faucet_top_up(SystemOrigin::Signed(caller.clone()).into(), money)?;
    }: _(SystemOrigin::Signed(caller.clone()), caller.clone())
    verify {

    }

    faucet_top_up {
        let caller: T::AccountId = whitelisted_caller();
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let money: BalanceOf<T> = 100000000000000000u128.try_into().map_err(|_e| "Error")?;
    }: _(SystemOrigin::Signed(caller.clone()), money)
    verify {

    }

    faucet {
        let caller: T::AccountId = whitelisted_caller();
        T::Currency::make_free_balance_be(&caller, BalanceOf::<T>::max_value());
        let money: BalanceOf<T> = 100000000000000000u128.try_into().map_err(|_e| "Error")?;
        Sminer::<T>::faucet_top_up(SystemOrigin::Signed(caller.clone()).into(), money)?;
    }: _(SystemOrigin::Signed(caller.clone()), caller.clone())
    verify {

    }

}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::{new_test_ext, Test};
	use frame_support::assert_ok;

	#[test]
	fn test_benchmarks() {
	new_test_ext().execute_with(|| {
		assert_ok!(Pallet::<Test>::test_benchmark_faucet_top_up());
        assert_ok!(Pallet::<Test>::test_benchmark_faucet());
        assert_ok!(Pallet::<Test>::test_benchmark_regnstk());
        assert_ok!(Pallet::<Test>::test_benchmark_redeem());
	});
	}
}

