#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;

use crate::Pallet as Cacher;

const SEED: u32 = 0;

benchmarks! {

	register {
		let alice: AccountOf<T> = account("alice", 100, SEED);
		let info = CacherInfo::<AccountOf<T>, BalanceOf<T>> {
			acc: alice.clone(),
			ip: IpAddress::IPV4([127,0,0,1], 8080),
			byte_price: 100u32.into(),
		};
	}: _(RawOrigin::Signed(alice.clone()), info.clone())
	verify {
		assert!(Cachers::<T>::contains_key(&alice));
		let cacher_info = Cachers::<T>::get(&alice).unwrap();
		assert_eq!(info, cacher_info);
	}

	update {
		let alice: AccountOf<T> = account("alice", 100, SEED);
		let info = CacherInfo::<AccountOf<T>, BalanceOf<T>> {
			acc: alice.clone(),
			ip: IpAddress::IPV4([127,0,0,1], 8080),
			byte_price: 100u32.into(),
		};
		Cachers::<T>::insert(&alice, info);

		let new_info = CacherInfo::<AccountOf<T>, BalanceOf<T>> {
			acc: alice.clone(),
			ip: IpAddress::IPV4([127,0,0,1], 80),
			byte_price: 200u32.into(),
		};
	}: _(RawOrigin::Signed(alice.clone()), new_info.clone())
	verify {
		assert!(Cachers::<T>::contains_key(&alice));
		let cacher_info = Cachers::<T>::get(&alice).unwrap();
		assert_eq!(new_info, cacher_info);
	}

	logout {
		let alice: AccountOf<T> = account("alice", 100, SEED);
		let info = CacherInfo::<AccountOf<T>, BalanceOf<T>> {
			acc: alice.clone(),
			ip: IpAddress::IPV4([127,0,0,1], 8080),
			byte_price: 100u32.into(),
		};
		Cachers::<T>::insert(&alice, info);
	}: _(RawOrigin::Signed(alice.clone()))
	verify {
		assert!(!Cachers::<T>::contains_key(&alice));
	}

	impl_benchmark_test_suite!(Cacher, crate::mock::new_test_ext(), crate::mock::Test)
}
