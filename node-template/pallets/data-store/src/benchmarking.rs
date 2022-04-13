//! Benchmarking setup for pallet-template

use super::*;

#[allow(unused)]
use crate::Pallet as DataStore;
use frame_benchmarking::{benchmarks, whitelisted_caller, vec};
use frame_system::RawOrigin;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

fn add_file<T: Config>(origin: T::AccountId, randsize: u128) -> Vec<u8> {
	let caller = RawOrigin::Signed(origin);
	let fileid: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
	let filename: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
	let keywords:Vec<Vec<u8>> = vec![vec![0,1,2,3,4,5,6,7,8,9]];
	assert!(DataStore::<T>::store(
		caller.clone().into(),
		fileid.clone(),
		filename,
		randsize,
		keywords
	)
	.is_ok());

	fileid
}

benchmarks! {
	store {
		let s in 0 .. 100000;
		let fileid: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
		let filename: Vec<u8> = vec![0,1,2,3,4,5,6,7,8];
		let keywords:Vec<Vec<u8>> = vec![vec![0,1,2,3,4,5,6,7,8,9]];
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Signed(caller.clone()), fileid.clone(), filename, s.into(), keywords)
	verify {
		assert_last_event::<T>(Event::Store{acc: caller, fileid: fileid}.into());
	}

	
	retrieve {
		let s in 0 .. 100000;
		let caller: T::AccountId = whitelisted_caller();
		let fileid = add_file::<T>(caller.clone(), s.into());
	}: _(RawOrigin::Signed(caller.clone()), fileid.clone())
	verify {
		assert_last_event::<T>(Event::IsOwner{acc: caller, fileid: fileid}.into());
	}


	replace {
		let s in 0 .. 100000;
		let caller: T::AccountId = whitelisted_caller();
		let fileid = add_file::<T>(caller.clone(), s.into());
		let new_fileid = vec![5,6,4,7,8,9,2,4,56,78,95];
		let filename: Vec<u8> = vec![88,16,24,35,43,54,65,76,80];
		let keywords:Vec<Vec<u8>> = vec![vec![0,1,2,3,4,5,6,7,8,9],vec![45,65,8,48,99,78,23,54,65,66,87,58,49,91]];
	}: _(RawOrigin::Signed(caller.clone()), fileid.clone(), new_fileid.clone(), filename, s.into(), keywords)
	verify {
		assert_last_event::<T>(Event::Replace{acc: caller, old_fileid: fileid, new_fileid: new_fileid}.into());
	}


	delete {
		let s in 0 .. 100000;
		let caller: T::AccountId = whitelisted_caller();
		let fileid = add_file::<T>(caller.clone(), s.into());
	}: _(RawOrigin::Signed(caller.clone()), fileid.clone())
	verify {
		assert_last_event::<T>(Event::Delete{acc: caller, fileid: fileid}.into());
	}


	edit {
		let s in 0 .. 100000;
		let caller: T::AccountId = whitelisted_caller();
		let fileid = add_file::<T>(caller.clone(), s.into());
		let filename: Vec<u8> = vec![88,16,24,35,43,54,65,76,80];
		let keywords: Vec<Vec<u8>> = vec![vec![0,1,2,3,4,5,6,7,8,9],vec![45,65,8,48,99,78,23,54,65,66,87,58,49,91]];
	}: _(RawOrigin::Signed(caller.clone()), fileid.clone(), filename.clone(), keywords.clone())
	verify {
		assert_last_event::<T>(Event::Edit{acc: caller, fileid: fileid, new_filename: filename, new_keywords: keywords}.into());
	}

	impl_benchmark_test_suite!(DataStore, crate::mock::new_test_ext(), crate::mock::Test);
}
