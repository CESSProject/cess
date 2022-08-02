use super::*;
use crate::{Pallet as FileBank, *};
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
use pallet_file_map::{Config as FileMapConfig, Pallet as FileMap};
use pallet_sminer::{Config as SminerConfig, Pallet as Sminer};
use sp_runtime::{
	traits::{Bounded, One, StaticLookup, TrailingZeroInput, Zero},
	Perbill, Percent,
};
use sp_std::prelude::*;

use frame_system::RawOrigin;

pub struct Pallet<T: Config>(FileBank<T>);
pub trait Config:
	crate::Config + pallet_cess_staking::Config + pallet_file_map::Config + pallet_sminer::Config
{
}
type SminerBalanceOf<T> = <<T as pallet_sminer::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

const SEED: u32 = 2190502;
const MAX_SPANS: u32 = 100;

pub fn add_file<T: Config>(file_hash: Vec<u8>) -> Result<(BoundedString<T>, T::AccountId, T::AccountId, T::AccountId), &'static str> {
	let caller: T::AccountId = whitelisted_caller();
	let (caller, miner, controller) = bench_buy_package::<T>(caller, 1000)?;
	let file_hash = file_hash.clone();
	let file_name = "test-file".as_bytes().to_vec();
	FileBank::<T>::upload_declaration(RawOrigin::Signed(caller.clone()).into(), file_hash.clone(), file_name.clone())?;

	let file_size: u64 = 333;
	let mut slice_info_list: Vec<SliceInfo<T>> = Vec::new();
	let mut file_hash1: Vec<u8> = file_hash.clone();
	file_hash1.append("-001".as_bytes().to_vec().as_mut());
	let slice_info = SliceInfo::<T>{
		miner_id: 1,
		shard_size: 111,
		block_num: 8,
		shard_id: file_hash1.try_into().map_err(|_e| "shar_id convert err")?,
		miner_ip: "192.168.1.1".as_bytes().to_vec().try_into().map_err(|_e| "miner ip convert err")?,
		miner_acc: miner.clone(),
	};
	slice_info_list.push(slice_info);

	let mut file_hash2: Vec<u8> = file_hash.clone();
	file_hash2.append("-002".as_bytes().to_vec().as_mut());
	let slice_info2 = SliceInfo::<T>{
		miner_id: 1,
		shard_size: 111,
		block_num: 8,
		shard_id: file_hash2.try_into().map_err(|_e| "shar_id convert err")?,
		miner_ip: "192.168.1.1".as_bytes().to_vec().try_into().map_err(|_e| "miner ip convert err")?,
		miner_acc: miner.clone(),
	};
	slice_info_list.push(slice_info2);

	let mut file_hash3: Vec<u8> = file_hash.clone();
	file_hash3.append("-003".as_bytes().to_vec().as_mut());
	let slice_info3 = SliceInfo::<T>{
		miner_id: 1,
		shard_size: 111,
		block_num: 8,
		shard_id: file_hash3.try_into().map_err(|_e| "shar_id convert err")?,
		miner_ip: "192.168.1.1".as_bytes().to_vec().try_into().map_err(|_e| "miner ip convert err")?,
		miner_acc: miner.clone(),
	};
	slice_info_list.push(slice_info3);

	FileBank::<T>::upload(RawOrigin::Signed(controller.clone()).into(), file_hash.clone(), file_size, slice_info_list, caller.clone())?;
	let file_hash: BoundedString<T> = file_hash.try_into().map_err(|_e| "file_hash Vec convert BoundedVec err")?;
	Ok((file_hash, caller, miner, controller))
}

pub fn add_scheduler<T: Config>() -> Result<T::AccountId, &'static str> {
	let controller = testing_utils::create_funded_user::<T>("controller", SEED, 100);
	let stash = testing_utils::create_funded_user::<T>("stash", SEED, 100);
	let controller_lookup: <T::Lookup as StaticLookup>::Source =
		T::Lookup::unlookup(controller.clone());
	let reward_destination = RewardDestination::Staked;
	let amount = <T as pallet_cess_staking::Config>::Currency::minimum_balance() * 10u32.into();
	whitelist_account!(stash);
	Staking::<T>::bond(
		RawOrigin::Signed(stash.clone()).into(),
		controller_lookup,
		amount,
		reward_destination,
	)?;
	whitelist_account!(controller);
	FileMap::<T>::registration_scheduler(RawOrigin::Signed(controller.clone()).into(), stash, "127.0.0.1:8080".as_bytes().to_vec())?;
	Ok(controller)
}

fn add_filler<T: Config>(len: u32, index: u32, controller: AccountOf<T>) -> Result<u32, &'static str> {
	let miner: AccountOf<T> = account("miner1", 100, SEED);
	let mut filler_list: Vec<FillerInfo<T>> = Vec::new();
	for i in 0..len {
		let new_filler = FillerInfo::<T> {
			filler_size: 1024 * 1024 * 8,
			index: i,
			block_num: 8,
			segment_size: 1024 * 1024,
			scan_size: 16,
			miner_address: miner.clone(),
			filler_id: (index * 100 + i)
				.to_string()
				.as_bytes()
				.to_vec()
				.try_into()
				.map_err(|_| "uint convert to BoundedVec Error")?,
			filler_hash: "fillerhashhashhashhash"
				.as_bytes()
				.to_vec()
				.try_into()
				.map_err(|_| "Vec convert to BoundedVec Error")?,
		};
		filler_list.push(new_filler);
	}
	FileBank::<T>::upload_filler(RawOrigin::Signed(controller.clone()).into(), miner.clone(), filler_list)?;

	Ok(1)
}

pub fn add_miner<T: Config>() -> Result<T::AccountId, &'static str> {
	let miner: T::AccountId = account("miner1", 100, SEED);
	let ip = "1270008080".as_bytes().to_vec();
	<T as pallet_sminer::Config>::Currency::make_free_balance_be(
		&miner,
		SminerBalanceOf::<T>::max_value(),
	);
	whitelist_account!(miner);
	Sminer::<T>::regnstk(
		RawOrigin::Signed(miner.clone()).into(),
		miner.clone(),
		ip,
		0u32.into(),
	)?;
	Ok(miner.clone())
}

pub fn bench_buy_package<T: Config>(caller: AccountOf<T>, len: u32) -> Result<(T::AccountId, T::AccountId, T::AccountId), &'static str> {
	<T as crate::Config>::Currency::make_free_balance_be(
		&caller,
		BalanceOf::<T>::max_value(),
	);
	let miner = add_miner::<T>()?;
	let controller = add_scheduler::<T>()?;
	for i in 0 .. len {
		let _ = add_filler::<T>(10, i, controller.clone())?;
	}
	FileBank::<T>::buy_package(RawOrigin::Signed(caller.clone()).into(), 1, 0)?;
	Ok((caller.clone(), miner.clone(), controller.clone()))
}

benchmarks! {
	upload_filler {
		let v in 0 .. 10;

		let controller = testing_utils::create_funded_user::<T>("controller", SEED, 100);
		let stash = testing_utils::create_funded_user::<T>("stash", SEED, 100);
		let controller_lookup: <T::Lookup as StaticLookup>::Source
			= T::Lookup::unlookup(controller.clone());
		let reward_destination = RewardDestination::Staked;
		let amount = <T as pallet_cess_staking::Config>::Currency::minimum_balance() * 10u32.into();
		whitelist_account!(stash);
		Staking::<T>::bond(RawOrigin::Signed(stash.clone()).into(), controller_lookup, amount, reward_destination)?;
		whitelist_account!(controller);

		FileMap::<T>::registration_scheduler(RawOrigin::Signed(controller.clone()).into(), stash, "127.0.0.1:8080".as_bytes().to_vec())?;

		let mut filler_list: Vec<FillerInfo<T>> = Vec::new();
		let miner = add_miner::<T>()?;
		for i in 0 .. v {
			let new_filler = FillerInfo::<T> {
				filler_size: 1024 * 1024 * 8,
				index: 1,
				block_num: 8,
				segment_size: 1024 * 1024,
				scan_size: 16,
				miner_address: miner.clone(),
				filler_id: i.to_string().as_bytes().to_vec().try_into().map_err(|_| "uint convert to BoundedVec Error")?,
				filler_hash: "fillerhashhashhashhash".as_bytes().to_vec().try_into().map_err(|_| "Vec convert to BoundedVec Error")?,
			};
			filler_list.push(new_filler);
		}
	}: _(RawOrigin::Signed(controller), miner.clone(), filler_list)
	verify {
		for i in 0 .. v {
			let filler_id: BoundedString<T> = i.to_string().as_bytes().to_vec().try_into().map_err(|_| "uint convert to BoundedVec Error")?;
			assert!(FillerMap::<T>::contains_key(&miner, &filler_id));
		}
	}

	buy_package {
		let user: AccountOf<T> = account("user1", 100, SEED);
		let miner = add_miner::<T>()?;
		let controller = add_scheduler::<T>()?;
		for i in 0 .. 1000 {
			let _ = add_filler::<T>(10, i, controller.clone())?;
		}
	}: _(RawOrigin::Signed(user.clone()), 1, 0)
	verify {
		assert!(<PurchasedPackage<T>>::contains_key(&user));
		let info = <PurchasedPackage<T>>::get(&user).unwrap();
		assert_eq!(info.space, G_BYTE * 10);
	}

	upgrade_package {
		let caller: T::AccountId = whitelisted_caller();
		let (user, miner, controller) = bench_buy_package::<T>(caller.clone(), 10000)?;	
		let value: u32 = BalanceOf::<T>::max_value().saturated_into();
		let free_balance: u32 = <T as pallet::Config>::Currency::free_balance(&user).saturated_into();
		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account();
		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
		<T as crate::Config>::Currency::make_free_balance_be(
			&acc,
			balance,
		);
		log::info!("free_balance: {}",free_balance);
	}: _(RawOrigin::Signed(caller), 2, 0)
	verify {
		assert!(<PurchasedPackage<T>>::contains_key(&user));
		let info = <PurchasedPackage<T>>::get(&user).unwrap();
		assert_eq!(info.space, G_BYTE * 500);
	}

	renewal_package {
		let caller: T::AccountId = whitelisted_caller();
		let (user, miner, controller) = bench_buy_package::<T>(caller.clone(), 1000)?;
		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account();
		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
		<T as crate::Config>::Currency::make_free_balance_be(
			&acc,
			balance,
		);
	}: _(RawOrigin::Signed(caller))
	verify {
		assert!(<PurchasedPackage<T>>::contains_key(&user));
		let info = <PurchasedPackage<T>>::get(&user).unwrap();
		assert_eq!(info.start, 0u32.saturated_into());
		let day60 = T::OneDay::get() * 60u32.saturated_into();
		assert_eq!(info.deadline, day60);
	}

	upload_declaration {
		let caller = account("user1", 100, SEED);
		let file_hash = "cess-13540202190502".as_bytes().to_vec();
		let file_name = "test-file".as_bytes().to_vec();
	}: _(RawOrigin::Signed(caller), file_hash.clone(), file_name)
	verify {
		let file_hash: BoundedString<T> = file_hash.try_into().map_err(|_e| "convert err")?;
		assert!(<File<T>>::contains_key(file_hash));
	}

	upload {
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let (user, miner, controller) = bench_buy_package::<T>(caller.clone(), 1000)?;
		let miner: T::AccountId = account("miner1", 100, SEED);
		let file_hash = "cess-13540202190502".as_bytes().to_vec();
		let file_name = "test-file".as_bytes().to_vec();
		FileBank::<T>::upload_declaration(RawOrigin::Signed(caller.clone()).into(), file_hash.clone(), file_name.clone())?;
		let file_size: u64 = 333;
		let mut slice_info_list: Vec<SliceInfo<T>> = Vec::new();
		let slice_info = SliceInfo::<T>{
			miner_id: 1,
			shard_size: 111,
			block_num: 8,
			shard_id: "1".as_bytes().to_vec().try_into().map_err(|_e| "shar_id convert err")?,
			miner_ip: "192.168.1.1".as_bytes().to_vec().try_into().map_err(|_e| "miner ip convert err")?,
			miner_acc: miner.clone(),
		};
		slice_info_list.push(slice_info);
	
		let slice_info2 = SliceInfo::<T>{
			miner_id: 1,
			shard_size: 111,
			block_num: 8,
			shard_id: "2".as_bytes().to_vec().try_into().map_err(|_e| "shar_id convert err")?,
			miner_ip: "192.168.1.1".as_bytes().to_vec().try_into().map_err(|_e| "miner ip convert err")?,
			miner_acc: miner.clone(),
		};
		slice_info_list.push(slice_info2);
	
		let slice_info3 = SliceInfo::<T>{
			miner_id: 1,
			shard_size: 111,
			block_num: 8,
			shard_id: "3".as_bytes().to_vec().try_into().map_err(|_e| "shar_id convert err")?,
			miner_ip: "192.168.1.1".as_bytes().to_vec().try_into().map_err(|_e| "miner ip convert err")?,
			miner_acc: miner.clone(),
		};
		slice_info_list.push(slice_info3);
	}: _(RawOrigin::Signed(controller), file_hash.clone(), file_size, slice_info_list, caller.clone())
	verify {
		let file_hash: BoundedString<T> = file_hash.try_into().map_err(|_e| "file_hash Vec convert BoundedVec err")?;
		assert!(<File<T>>::contains_key(&file_hash));
		let info = <File<T>>::get(&file_hash).unwrap();
		assert_eq!(info.slice_info.len(), 3);
		let package_info = <PurchasedPackage<T>>::get(&user).unwrap();
		assert_eq!(package_info.used_space, 333);
	}

	delete_file {
		let file_hash = "cess_05020219520".as_bytes().to_vec();
	    let (file_hash, caller, _, _) = add_file::<T>(file_hash)?;
		assert!(File::<T>::contains_key(&file_hash));
	}: _(RawOrigin::Signed(caller), file_hash.to_vec())
	verify {
	    assert!(!File::<T>::contains_key(&file_hash));
	}

	recover_file {
		let v in 0 .. 50;
		let mut avai = true;
		if v % 2 == 0 {
			avai = false;
		}
		<frame_system::Pallet<T>>::set_block_number(1u32.saturated_into());
		let file_hash = "cess_05020219520".as_bytes().to_vec();
	    let (file_hash, caller, miner, controller) = add_file::<T>(file_hash)?;
		assert!(File::<T>::contains_key(&file_hash));
		let info = File::<T>::get(&file_hash).unwrap();
		let re_shard = info.slice_info[0].shard_id.clone();

		<File<T>>::try_mutate(&file_hash, |opt| -> DispatchResult {
			let o = opt.as_mut().unwrap();
			o.slice_info.retain(|x| x.shard_id != re_shard.clone());
			Ok(())
		})?;
		<FileRecovery<T>>::try_mutate(&controller, |o| -> DispatchResult {
			o.try_push(re_shard.clone().try_into().map_err(|_e| Error::<T>::BoundedVecError)?)
				.map_err(|_e| Error::<T>::StorageLimitReached)?;
			Ok(())
		})?;

		assert!(FileRecovery::<T>::contains_key(&controller));
		let re_list = FileRecovery::<T>::get(&controller);
		assert_eq!(re_list.len(), 1);
		let slice_info = SliceInfo::<T>{
			miner_id: 1,
			shard_size: 111,
			block_num: 8,
			shard_id: "4".as_bytes().to_vec().try_into().map_err(|_e| "shar_id convert err")?,
			miner_ip: "192.168.1.1".as_bytes().to_vec().try_into().map_err(|_e| "miner ip convert err")?,
			miner_acc: miner.clone(),
		};
		let info = File::<T>::get(&file_hash).unwrap();
		assert_eq!(info.slice_info.len(), 2);
	}: _(RawOrigin::Signed(controller.clone()), re_shard.to_vec(), slice_info, avai)
	verify {
		if avai {
			let info = File::<T>::get(&file_hash).unwrap();
			assert_eq!(info.slice_info.len(), 3);
		} else {
			assert!(!File::<T>::contains_key(&file_hash));
		}
	}


}
