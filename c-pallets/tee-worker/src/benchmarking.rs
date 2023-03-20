use super::*;
use crate::{Pallet as TeeWorker, testing_utils as FMTestUtils,*};
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

pub struct Pallet<T: Config>(TeeWorker<T>);
pub trait Config:
	crate::Config + pallet_cess_staking::Config
{
}

const USER_SEED: u32 = 999666;

benchmarks! {
    registration_scheduler {
        let caller: T::AccountId = whitelisted_caller();
        let (stash, controller) = pallet_cess_staking::testing_utils::create_stash_controller::<T>(USER_SEED, 100, Default::default())?;
    }: _(RawOrigin::Signed(controller.clone()), stash.clone(), IpAddress::IPV4([127,0,0,1], 15001))
    verify {
        let s_vec = SchedulerMap::<T>::get();
		let ip_bound = IpAddress::IPV4([127,0,0,1], 15001);
		let scheduler = SchedulerInfo::<T> {
			ip: ip_bound,
			stash_user: stash.clone(),
			controller_user: controller.clone(),
		};
		assert!(s_vec.to_vec().contains(&scheduler))
    }

    update_scheduler {
        let ip = IpAddress::IPV4([127,0,0,1], 15001);
        let (stash, controller) = pallet_cess_staking::testing_utils::create_stash_controller::<T>(USER_SEED, 100, Default::default())?;
        FMTestUtils::add_scheduler::<T>(controller.clone(), stash.clone(), ip.clone())?;
        let s_vec = SchedulerMap::<T>::get();
        let ip_bound = IpAddress::IPV4([127,0,0,1], 15001);
        let scheduler = SchedulerInfo::<T> {
			ip: ip_bound,
			stash_user: stash.clone(),
			controller_user: controller.clone(),
		};
        assert!(s_vec.to_vec().contains(&scheduler));
        let new_ip = IpAddress::IPV4([127,0,0,1], 15002);
    }: _(RawOrigin::Signed(controller.clone()), new_ip)
    verify {
        let s_vec = SchedulerMap::<T>::get();
        let ip_bound = IpAddress::IPV4([127,0,0,1], 15002);
        let scheduler = SchedulerInfo::<T> {
					ip: ip_bound,
					stash_user: stash.clone(),
					controller_user: controller.clone(),
				};
        assert!(s_vec.to_vec().contains(&scheduler))
    }


}
