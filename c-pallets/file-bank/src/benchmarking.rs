use super::*;
use crate::{Pallet as FileBank, *};
use cp_cess_common::{Hash, IpAddress};
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
use pallet_tee_worker::{Config as TeeWorkerConfig, Pallet as TeeWorker};
use pallet_sminer::{Config as SminerConfig, Pallet as Sminer};
use sp_runtime::{
	traits::{Bounded, One, StaticLookup, TrailingZeroInput, Zero},
	Perbill, Percent,
};
use sp_std::prelude::*;

use frame_system::RawOrigin;

pub struct Pallet<T: Config>(FileBank<T>);
pub trait Config:
	crate::Config + pallet_sminer::benchmarking::Config
{
}
type SminerBalanceOf<T> = <<T as pallet_sminer::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

// const SEED: u32 = 2190502;
// const MAX_SPANS: u32 = 100;

// pub fn create_new_bucket<T: Config>(caller: T::AccountId, name: Vec<u8>) -> Result<(), &'static str> {
// 	let name = name.try_into().map_err(|_| "create bucket convert error")?;
// 	FileBank::<T>::create_bucket(RawOrigin::Signed(caller.clone()).into(), caller, name)?;

// 	Ok(())
// }

// pub fn add_file<T: Config>(file_hash: Hash) -> Result<(Hash, T::AccountId, T::AccountId, T::AccountId), &'static str> {
// 	let caller: T::AccountId = whitelisted_caller();
// 	let (caller, miner, controller) = bench_buy_space::<T>(caller, 1000)?;
// 	let file_name = "test-file".as_bytes().to_vec();
// 	let bucket_name = "test-bucket1".as_bytes().to_vec();
// 	create_new_bucket::<T>(caller.clone(), bucket_name.clone())?;
// 	let user_brief = UserBrief::<T>{
// 		user: caller.clone(),
// 		file_name: file_name.try_into().map_err(|_e| "file name convert err")?,
// 		bucket_name: bucket_name.try_into().map_err(|_e| "bucket name convert err")?,
// 	};
// 	FileBank::<T>::upload_declaration(RawOrigin::Signed(caller.clone()).into(), file_hash.clone(), user_brief)?;

// 	let file_size: u64 = 333;
// 	let mut slice_info_list: Vec<SliceInfo<T>> = Vec::new();
// 	let mut file_hash1: Vec<u8> = file_hash.0.to_vec();
// 	file_hash1.append("-001".as_bytes().to_vec().as_mut());
// 	let file_hash1: [u8; 68] = file_hash1.as_slice().try_into().map_err(|_| "vec to u8; 68 error")?;
// 	let slice_info = SliceInfo::<T>{
// 		miner_id: 1,
// 		shard_size: 111,
// 		block_num: 8,
// 		shard_id: file_hash1,
// 		miner_ip: IpAddress::IPV4([127,0,0,1], 15001),
// 		miner_acc: miner.clone(),
// 	};
// 	slice_info_list.push(slice_info);

// 	let mut file_hash2: Vec<u8> = file_hash.0.to_vec();
// 	file_hash2.append("-002".as_bytes().to_vec().as_mut());
// 	let file_hash2: [u8; 68] = file_hash2.as_slice().try_into().map_err(|_| "vec to u8; 68 error")?;
// 	let slice_info2 = SliceInfo::<T>{
// 		miner_id: 1,
// 		shard_size: 111,
// 		block_num: 8,
// 		shard_id: file_hash2,
// 		miner_ip: IpAddress::IPV4([127,0,0,1], 15001),
// 		miner_acc: miner.clone(),
// 	};
// 	slice_info_list.push(slice_info2);

// 	let mut file_hash3: Vec<u8> = file_hash.0.to_vec();
// 	file_hash3.append("-003".as_bytes().to_vec().as_mut());
// 	let file_hash3: [u8; 68] = file_hash3.as_slice().try_into().map_err(|_| "vec to u8; 68 error")?;
// 	let slice_info3 = SliceInfo::<T>{
// 		miner_id: 1,
// 		shard_size: 111,
// 		block_num: 8,
// 		shard_id: file_hash3,
// 		miner_ip: IpAddress::IPV4([127,0,0,1], 15001),
// 		miner_acc: miner.clone(),
// 	};
// 	slice_info_list.push(slice_info3);

// 	FileBank::<T>::upload(RawOrigin::Signed(controller.clone()).into(), file_hash.clone(), file_size, slice_info_list)?;
// 	Ok((file_hash, caller, miner, controller))
// }

// pub fn add_scheduler<T: Config>() -> Result<T::AccountId, &'static str> {
// 	let controller = testing_utils::create_funded_user::<T>("controller", SEED, 100);
// 	let stash = testing_utils::create_funded_user::<T>("stash", SEED, 100);
// 	let controller_lookup: <T::Lookup as StaticLookup>::Source =
// 		T::Lookup::unlookup(controller.clone());
// 	let reward_destination = RewardDestination::Staked;
// 	let amount = <T as pallet_cess_staking::Config>::Currency::minimum_balance() * 10u32.into();
// 	whitelist_account!(stash);
// 	Staking::<T>::bond(
// 		RawOrigin::Signed(stash.clone()).into(),
// 		controller_lookup,
// 		amount,
// 		reward_destination,
// 	)?;
// 	whitelist_account!(controller);
// 	TeeWorker::<T>::registration_scheduler(RawOrigin::Signed(controller.clone()).into(), stash, IpAddress::IPV4([127,0,0,1], 15001))?;
// 	Ok(controller)
// }

// fn add_filler<T: Config>(len: u32, index: u32, controller: AccountOf<T>) -> Result<u32, &'static str> {
// 	let miner: AccountOf<T> = account("miner1", 100, SEED);
// 	let mut filler_list: Vec<FillerInfo<T>> = Vec::new();
// 	for i in 0..len {
// 		let mut value = (index * 10005 + i + 1).to_string().as_bytes().to_vec();
// 		// log::info!("value: {:?}", value);
// 		let fill_zero = 64 - value.len();
// 		for v in 0 .. fill_zero {
// 			value.push(0);
// 		}
// 		// log::info!("value len: {:?}", value.len());
// 		let new_filler = FillerInfo::<T> {
// 			filler_size: 1024 * 1024 * 8,
// 			index: i,
// 			block_num: 8,
// 			segment_size: 1024 * 1024,
// 			scan_size: 16,
// 			miner_address: miner.clone(),
// 			filler_hash: Hash(value.try_into().map_err(|_| "filler hash convert err!")?),
// 		};
// 		// log::info!("filler_hash: {:?}", new_filler.filler_hash);
// 		filler_list.push(new_filler);
// 	}
// 	FileBank::<T>::upload_filler(RawOrigin::Signed(controller.clone()).into(), miner.clone(), filler_list)?;

// 	Ok(1)
// }

// pub fn add_miner<T: Config>() -> Result<T::AccountId, &'static str> {
// 	let miner: T::AccountId = account("miner1", 100, SEED);
// 	let ip = IpAddress::IPV4([127,0,0,1], 15001);
// 	<T as pallet_sminer::Config>::Currency::make_free_balance_be(
// 		&miner,
// 		SminerBalanceOf::<T>::max_value(),
// 	);
// 	whitelist_account!(miner);
// 	Sminer::<T>::regnstk(
// 		RawOrigin::Signed(miner.clone()).into(),
// 		miner.clone(),
// 		ip,
// 		0u32.into(),
// 	)?;
// 	Ok(miner.clone())
// }

// pub fn bench_buy_space<T: Config>(caller: AccountOf<T>, len: u32) -> Result<(T::AccountId, T::AccountId, T::AccountId), &'static str> {
// 	<T as crate::Config>::Currency::make_free_balance_be(
// 		&caller,
// 		BalanceOf::<T>::max_value(),
// 	);
// 	let miner = add_miner::<T>()?;
// 	let controller = add_scheduler::<T>()?;
// 	for i in 0 .. len {
// 		let _ = add_filler::<T>(10, i, controller.clone())?;
// 	}
// 	FileBank::<T>::buy_space(RawOrigin::Signed(caller.clone()).into(), 10)?;
// 	Ok((caller.clone(), miner.clone(), controller.clone()))
// }

benchmarks! {
	cert_idle_space {
		let miner = pallet_sminer::benchmarking::add_miner::<T>("miner1")?;

		let idle_sig_info = IdleSigInfo::<T> {
			miner: miner.clone(),
			count: 1000,
			accumulator: [0u8; 256],
			last_operation_block: 10u32.into(),
		};

		let original = idle_sig_info.encode();
		let original = sp_io::hashing::sha2_256(&original);

		let tee_sig = [9, 145, 113, 138, 73, 244, 229, 35, 53, 150, 162, 241, 244, 34, 219, 176, 175, 110, 9, 247, 250, 148, 225, 98, 185, 68, 202, 4, 222, 105, 9, 108, 12, 175, 31, 141, 8, 253, 0, 75, 201, 246, 209, 160, 177, 186, 135, 2, 66, 247, 84, 16, 220, 169, 210, 116, 90, 192, 127, 120, 195, 91, 8, 44, 205, 55, 149, 22, 72, 56, 149, 182, 94, 143, 115, 20, 5, 156, 191, 63, 189, 206, 86, 254, 181, 167, 245, 7, 72, 160, 156, 86, 46, 78, 96, 122, 127, 65, 153, 108, 169, 119, 186, 228, 48, 100, 194, 192, 153, 158, 120, 214, 97, 52, 193, 21, 245, 189, 56, 179, 226, 36, 226, 109, 2, 174, 127, 50, 110, 155, 10, 164, 25, 103, 115, 0, 214, 149, 144, 152, 188, 194, 117, 253, 115, 255, 170, 25, 245, 86, 136, 25, 121, 33, 232, 10, 4, 194, 139, 167, 103, 220, 55, 183, 206, 108, 168, 109, 91, 91, 3, 188, 166, 231, 127, 213, 27, 13, 146, 43, 237, 88, 178, 48, 241, 214, 8, 123, 38, 228, 73, 190, 29, 232, 32, 109, 189, 127, 242, 137, 105, 154, 73, 238, 90, 205, 169, 20, 118, 6, 234, 178, 192, 91, 232, 182, 207, 207, 180, 175, 127, 216, 246, 218, 179, 144, 17, 230, 93, 65, 212, 1, 204, 146, 83, 195, 61, 68, 16, 140, 23, 243, 107, 95, 170, 82, 88, 120, 204, 97, 219, 189, 186, 202, 222, 214];

	}: _(RawOrigin::Signed(miner.clone()), idle_sig_info, tee_sig)
	verify {
		let (idle, service) = T::MinerControl::get_power(&miner)?;
		assert_eq!(idle, 1000 * 8 * 1024 * 1024);
	}
}

// 	buy_space {
// 		log::info!("start buy_space");
// 		let user: AccountOf<T> = account("user1", 100, SEED);
// 		let miner = add_miner::<T>()?;
// 		let controller = add_scheduler::<T>()?;
// 		for i in 0 .. 1000 {
// 			let _ = add_filler::<T>(10, i, controller.clone())?;
// 		}
// 		<T as crate::Config>::Currency::make_free_balance_be(
// 		&user,
// 		BalanceOf::<T>::max_value(),
// 	);
// 		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account_truncating();
// 		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
// 		<T as crate::Config>::Currency::make_free_balance_be(
// 			&acc,
// 			balance,
// 		);
// 	}: _(RawOrigin::Signed(user.clone()), 10)
// 	verify {
// 		assert!(<UserOwnedSpace<T>>::contains_key(&user));
// 		let info = <UserOwnedSpace<T>>::get(&user).unwrap();
// 		assert_eq!(info.total_space, G_BYTE * 10);
// 	}

// 	expansion_space {
// 		log::info!("start expansion_space");
// 		let caller: T::AccountId = whitelisted_caller();
// 		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account_truncating();
// 		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
// 		<T as crate::Config>::Currency::make_free_balance_be(
// 			&acc,
// 			balance,
// 		);
// 		let (user, miner, controller) = bench_buy_space::<T>(caller.clone(), 10000)?;
// 		let value: u32 = BalanceOf::<T>::max_value().saturated_into();
// 		let free_balance: u32 = <T as pallet::Config>::Currency::free_balance(&user).saturated_into();

// 		log::info!("free_balance: {}",free_balance);
// 	}: _(RawOrigin::Signed(caller), 490)
// 	verify {
// 		assert!(<UserOwnedSpace<T>>::contains_key(&user));
// 		let info = <UserOwnedSpace<T>>::get(&user).unwrap();
// 		assert_eq!(info.total_space, G_BYTE * 500);
// 	}

// 	renewal_space {
// 		log::info!("start renewal_space");
// 		let caller: T::AccountId = whitelisted_caller();
// 		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account_truncating();
// 		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
// 		<T as crate::Config>::Currency::make_free_balance_be(
// 			&acc,
// 			balance,
// 		);
// 		let (user, miner, controller) = bench_buy_space::<T>(caller.clone(), 1000)?;

// 	}: _(RawOrigin::Signed(caller), 30)
// 	verify {
// 		assert!(<UserOwnedSpace<T>>::contains_key(&user));
// 		let info = <UserOwnedSpace<T>>::get(&user).unwrap();
// 		assert_eq!(info.start, 0u32.saturated_into());
// 		let day60 = T::OneDay::get() * 60u32.saturated_into();
// 		assert_eq!(info.deadline, day60);
// 	}

// 	upload_declaration {
// 		log::info!("start upload_declaration");
// 		let caller: T::AccountId = account("user1", 100, SEED);
// 		let file_hash = Hash([5u8; 64]);
// 		let file_name = "test-file".as_bytes().to_vec();
// 		let bucket_name = "test-bucket1".as_bytes().to_vec();
// 		create_new_bucket::<T>(caller.clone(), bucket_name.clone())?;
// 		let user_brief = UserBrief::<T>{
// 			user: caller.clone(),
// 			file_name: file_name.try_into().map_err(|_e| "file name convert err")?,
// 			bucket_name: bucket_name.try_into().map_err(|_e| "bucket name convert err")?,
// 		};
// 	}: _(RawOrigin::Signed(caller), file_hash.clone(), user_brief)
// 	verify {
// 		let file_hash = Hash([5u8; 64]);
// 		assert!(<File<T>>::contains_key(file_hash));
// 	}

// 	upload {
// 		let v in 0 .. 30;
// 		log::info!("start upload");

// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account_truncating();
// 		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
// 		<T as crate::Config>::Currency::make_free_balance_be(
// 			&acc,
// 			balance,
// 		);
// 		let (user, miner, controller) = bench_buy_space::<T>(caller.clone(), 1000)?;
// 		let miner: T::AccountId = account("miner1", 100, SEED);
// 		let file_hash = Hash([3u8; 64]);
// 		let file_name = "test-file".as_bytes().to_vec();
// 		let bucket_name = "test-bucket1".as_bytes().to_vec();
// 		create_new_bucket::<T>(caller.clone(), bucket_name.clone())?;
// 		let user_brief = UserBrief::<T>{
// 			user: caller.clone(),
// 			file_name: file_name.try_into().map_err(|_e| "file name convert err")?,
// 			bucket_name: bucket_name.try_into().map_err(|_e| "bucket name convert err")?,
// 		};
// 		FileBank::<T>::upload_declaration(RawOrigin::Signed(caller.clone()).into(), file_hash.clone(), user_brief.clone())?;
// 		let file_size: u64 = 333;
// 		let mut slice_info_list: Vec<SliceInfo<T>> = Vec::new();
// 		for i in 0 .. v {
// 				let slice_info = SliceInfo::<T>{
// 				miner_id: 1,
// 				shard_size: 333 / v as u64,
// 				block_num: 8,
// 				shard_id: [i as u8; 68],
// 				miner_ip: IpAddress::IPV4([127,0,0,1], 15001),
// 				miner_acc: miner.clone(),
// 			};
// 			slice_info_list.push(slice_info);
// 		}
// 	}: _(RawOrigin::Signed(controller), file_hash.clone(), file_size, slice_info_list)
// 	verify {
// 		assert!(<File<T>>::contains_key(&file_hash));
// 		let info = <File<T>>::get(&file_hash).unwrap();
// 		assert_eq!(info.slice_info.len(), v as usize);
// 		let package_info = <UserOwnedSpace<T>>::get(&user).unwrap();
// 		assert_eq!(package_info.used_space, 333);
// 	}

// 	delete_file {
// 		log::info!("start delete_file");
// 		let file_hash = Hash([5u8; 64]);
// 		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account_truncating();
// 		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
// 		<T as crate::Config>::Currency::make_free_balance_be(
// 			&acc,
// 			balance,
// 		);
// 		let (file_hash, caller, _, _) = add_file::<T>(file_hash)?;
// 		assert!(File::<T>::contains_key(&file_hash));
// 	}: _(RawOrigin::Signed(caller.clone()), caller.clone(), file_hash)
// 	verify {
// 	    assert!(!File::<T>::contains_key(&file_hash));
// 	}

// 	recover_file {
// 		let v in 0 .. 50;
// 		log::info!("start recover_file");
// 		let mut avai = true;
// 		if v % 2 == 0 {
// 			avai = false;
// 		}
// 		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account_truncating();
// 		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
// 		<T as crate::Config>::Currency::make_free_balance_be(
// 			&acc,
// 			balance,
// 		);
// 		<frame_system::Pallet<T>>::set_block_number(1u32.saturated_into());
// 		let file_hash = Hash([5u8; 64]);

// 		let (file_hash, caller, miner, controller) = add_file::<T>(file_hash)?;

// 		assert!(File::<T>::contains_key(&file_hash));

// 		let info = File::<T>::get(&file_hash).unwrap();
// 		let re_shard = info.slice_info[0].shard_id.clone();

// 		<File<T>>::try_mutate(&file_hash, |opt| -> DispatchResult {
// 			let o = opt.as_mut().unwrap();
// 			o.slice_info.retain(|x| x.shard_id != re_shard.clone());
// 			Ok(())
// 		})?;
// 		<FileRecovery<T>>::try_mutate(&controller, |o| -> DispatchResult {
// 			o.try_push(re_shard.clone())
// 				.map_err(|_e| Error::<T>::StorageLimitReached)?;
// 			Ok(())
// 		})?;

// 		assert!(FileRecovery::<T>::contains_key(&controller));

// 		let re_list = FileRecovery::<T>::get(&controller);

// 		assert_eq!(re_list.len(), 1);

// 		let mut file_hash1: Vec<u8> = file_hash.0.to_vec();
// 		file_hash1.append("-004".as_bytes().to_vec().as_mut());

// 		let slice_info = SliceInfo::<T>{
// 			miner_id: 1,
// 			shard_size: 111,
// 			block_num: 8,
// 			shard_id: file_hash1.as_slice().try_into().map_err(|_e| "shar_id convert err")?,
// 			miner_ip: IpAddress::IPV4([127,0,0,1], 15001),
// 			miner_acc: miner.clone(),
// 		};

// 		let info = File::<T>::get(&file_hash).unwrap();
// 		assert_eq!(info.slice_info.len(), 2);
// 	}: _(RawOrigin::Signed(controller.clone()), re_shard, slice_info, avai)
// 	verify {
// 		if avai {
// 			let info = File::<T>::get(&file_hash).unwrap();
// 			assert_eq!(info.slice_info.len(), 3);
// 		} else {
// 			assert!(!File::<T>::contains_key(&file_hash));
// 		}
// 	}

// 	clear_invalid_file {
// 		log::info!("start clear_invalid_file");
// 		let miner: T::AccountId = account("miner1", 100, SEED);
// 		let file_hash: Hash = Hash([1u8; 64]);
// 		let mut file_hash_list: Vec<Hash> = Default::default();
// 		file_hash_list.push(file_hash.clone());
// 		let file_hash_list: BoundedVec<Hash, T::InvalidLimit> = file_hash_list.try_into().map_err(|_| "vec to boundedvec error")?;
// 		<InvalidFile<T>>::insert(&miner, file_hash_list);
// 	}: _(RawOrigin::Signed(miner.clone()), file_hash)
// 	verify {
// 		let list = <InvalidFile<T>>::get(&miner);
// 		assert_eq!(list.len(), 0);
// 	}

// 	create_bucket {
// 		log::info!("start create_bucket");
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let name: Vec<u8> = "test-bucket1".as_bytes().to_vec();
// 		let name: BoundedVec<u8, T::NameStrLimit> = name.try_into().map_err(|_| "name convert error")?;
// 	}: _(RawOrigin::Signed(caller.clone()), caller.clone(), name.clone())
// 	verify {
// 		assert!(Bucket::<T>::contains_key(&caller, name));
// 	}

// 	delete_bucket {
// 		log::info!("start delete_bucket");
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let name: Vec<u8> = "test-bucket1".as_bytes().to_vec();
// 		let name_bound: BoundedVec<u8, T::NameStrLimit> = name.clone().try_into().map_err(|_| "bounded_vec convert err!")?;
// 		create_new_bucket::<T>(caller.clone(), name.clone())?;
// 		Bucket::<T>::contains_key(&caller, name_bound.clone());
// 	}: _(RawOrigin::Signed(caller.clone()), caller.clone(), name_bound.clone())
// 	verify {
// 		assert!(!Bucket::<T>::contains_key(&caller, name_bound));
// 	}


// 	ownership_transfer {
// 		log::info!("start ownership_transfer");
// 		let target: AccountOf<T> = account("user2", 100, SEED);
// 		let bucket_name1: Vec<u8> = "test-bucket1".as_bytes().to_vec();
// 		let bucket_name2: Vec<u8> = "test-bucket2".as_bytes().to_vec();
// 		let file_hash = Hash([5u8; 64]);

// 		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
// 		<T as crate::Config>::Currency::make_free_balance_be(
// 			&target,
// 			balance,
// 		);

// 		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account_truncating();
// 		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
// 		<T as crate::Config>::Currency::make_free_balance_be(
// 			&acc,
// 			balance,
// 		);

// 		let (file_hash, caller, _, _) = add_file::<T>(file_hash.clone())?;
// 		log::info!("-----------1-------------");
// 		FileBank::<T>::buy_space(RawOrigin::Signed(target.clone()).into(), 10)?;
// 		log::info!("-----------2-------------");
// 		let target_brief = UserBrief::<T>{
// 			user: target.clone(),
// 			file_name: "test-name".as_bytes().to_vec().try_into().map_err(|_| "bounded_vec convert err!")?,
// 			bucket_name: bucket_name2.clone().try_into().map_err(|_| "bounded_vec convert err!")?,
// 		};
// 		create_new_bucket::<T>(target.clone(), bucket_name2.clone())?;
// 		let file = <File<T>>::get(&file_hash).unwrap();
// 		assert_eq!(file.user_brief_list[0].user, caller.clone());
// 		let bounded_bucket_name1: BoundedVec<u8, T::NameStrLimit> = bucket_name1.try_into().map_err(|_| "bounded_vec convert err!")?;
// 	}: _(RawOrigin::Signed(caller.clone()), bounded_bucket_name1.clone(), target_brief, file_hash.clone())
// 	verify {
// 		let file = <File<T>>::get(&file_hash).unwrap();
// 		assert_eq!(file.user_brief_list.len(), 1);
// 		assert_eq!(file.user_brief_list[0].user, target.clone());
// 	}
// }
