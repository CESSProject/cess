use super::*;
use crate::{Pallet as FileMap, testing_utils as FMTestUtils,*};
use codec::{alloc::string::ToString, Decode};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_support::{
	dispatch::UnfilteredDispatchable,
	pallet_prelude::*,
	traits::{Currency, CurrencyToVote, Get, Imbalance},
};
use pallet_cess_staking::{
	testing_utils, Config as StakingConfig, Pallet as Staking, RewardDestination,
};
use frame_system::RawOrigin;

pub struct Pallet<T: Config>(FileMap<T>);
pub trait Config:
	crate::Config + pallet_cess_staking::Config
{
}

const USER_SEED: u32 = 999666;

benchmarks! {
    registration_scheduler {
        let caller: T::AccountId = whitelisted_caller();
        let (stash, controller) = pallet_cess_staking::testing_utils::create_stash_controller::<T>(USER_SEED, 100, Default::default())?;
    }: _(RawOrigin::Signed(controller.clone()), stash.clone(), "127.0.0.1:8888".as_bytes().to_vec())
    verify {
        let s_vec = SchedulerMap::<T>::get();
		let ip_bound = "127.0.0.1:8888".as_bytes().to_vec().try_into().map_err(|_e| "convert err!")?;
		let scheduler = SchedulerInfo::<T> {
			ip: ip_bound,
			stash_user: stash.clone(),
			controller_user: controller.clone(),
		};
		assert!(s_vec.to_vec().contains(&scheduler))
    }

    update_scheduler {
        let ip = "127.0.0.1:8888".as_bytes().to_vec();
        let (stash, controller) = pallet_cess_staking::testing_utils::create_stash_controller::<T>(USER_SEED, 100, Default::default())?;
        FMTestUtils::add_scheduler::<T>(controller.clone(), stash.clone(), ip.clone())?;
        let s_vec = SchedulerMap::<T>::get();
        let ip_bound = "127.0.0.1:8888".as_bytes().to_vec().try_into().map_err(|_e| "convert err!")?;
        let scheduler = SchedulerInfo::<T> {
			ip: ip_bound,
			stash_user: stash.clone(),
			controller_user: controller.clone(),
		};
        assert!(s_vec.to_vec().contains(&scheduler));
        let new_ip = "127.0.0.1:8889".as_bytes().to_vec();
    }: _(RawOrigin::Signed(controller.clone()), new_ip)
    verify {
        let s_vec = SchedulerMap::<T>::get();
        let ip_bound = "127.0.0.1:8889".as_bytes().to_vec().try_into().map_err(|_e| "convert err!")?;
        let scheduler = SchedulerInfo::<T> {
			ip: ip_bound,
			stash_user: stash.clone(),
			controller_user: controller.clone(),
		};
        assert!(s_vec.to_vec().contains(&scheduler))
    }


}