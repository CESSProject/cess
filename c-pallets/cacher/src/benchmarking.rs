#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_benchmarking::{account, benchmarks};
use frame_system::RawOrigin;
use sp_runtime::traits::{Bounded, Hash};

#[allow(unused)]
use crate::Pallet as Cacher;

const SEED: u32 = 0;

benchmarks! {

	register {
		let alice: AccountOf<T> = account("alice", 0, SEED);
		let info = CacherInfo::<AccountOf<T>, BalanceOf<T>> {
			payee: alice.clone(),
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
		let alice: AccountOf<T> = account("alice", 0, SEED);
		let info = CacherInfo::<AccountOf<T>, BalanceOf<T>> {
			payee: alice.clone(),
			ip: IpAddress::IPV4([127,0,0,1], 8080),
			byte_price: 100u32.into(),
		};
		Cachers::<T>::insert(&alice, info);

		let new_info = CacherInfo::<AccountOf<T>, BalanceOf<T>> {
			payee: alice.clone(),
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
		let alice: AccountOf<T> = account("alice", 0, SEED);
		let info = CacherInfo::<AccountOf<T>, BalanceOf<T>> {
			payee: alice.clone(),
			ip: IpAddress::IPV4([127,0,0,1], 8080),
			byte_price: 100u32.into(),
		};
		Cachers::<T>::insert(&alice, info);
	}: _(RawOrigin::Signed(alice.clone()))
	verify {
		assert!(!Cachers::<T>::contains_key(&alice));
	}

	pay {
		let v in 0 .. 10;
		let alice: AccountOf<T> = account("alice", 0, SEED);
		T::Currency::make_free_balance_be(&alice, BalanceOf::<T>::max_value());
		let bob: AccountOf<T> = account("bob", 1, SEED);
		T::Currency::make_free_balance_be(&bob, T::Currency::minimum_balance());
		let s_file = String::from("file");
		let s_slice = String::from("slice");
		let mut bill_vec = Vec::new();
		for i in 0 .. v {
			let bill = Bill::<AccountOf<T>, BalanceOf<T>, T::Hash> {
				id: [i as u8; 16],
				to: bob.clone(),
				amount: 10000u32.into(),
				file_hash: T::Hashing::hash_of(&format!("{}{}", s_file, i)),
				slice_hash: T::Hashing::hash_of(&format!("{}{}", s_slice, i)),
				expiration_time: 1675900800u64,
			};
			bill_vec.push(bill);
		}
		let bills: BoundedVec<_, T::BillsLimit> = bill_vec.try_into().unwrap();
	}: _(RawOrigin::Signed(alice), bills)
	verify {
		
	}

	impl_benchmark_test_suite!(Cacher, crate::mock::new_test_ext(), crate::mock::Test)
}
