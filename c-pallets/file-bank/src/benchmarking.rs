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
// pub fn add_idle_space(name: T::AccountId) -> Result<(), &'static str> {
	
// }
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

		let pois_key = PoISKey {
            g: [2u8; 256],
            n: [3u8; 256],
        };

		let space_proof_info = SpaceProofInfo::<AccountOf<T>, BlockNumberOf<T>> {
            last_operation_block: u32::MIN.saturated_into(),
            miner: miner.clone(),
            front: u64::MIN,
            rear: 1000,
            pois_key: pois_key.clone(),
            accumulator: pois_key.g,
        };

		let original = space_proof_info.encode();
		let original = sp_io::hashing::sha2_256(&original);

		log::info!("original: {:?}", original);

		let tee_sig = [218, 77, 206, 141, 156, 234, 61, 85, 171, 227, 3, 196, 139, 81, 138, 186, 176, 28, 71, 251, 185, 142, 162, 68, 82, 77, 142, 165, 29, 29, 157, 141, 240, 59, 145, 63, 152, 83, 11, 171, 110, 24, 140, 8, 135, 37, 107, 131, 108, 69, 31, 206, 230, 87, 0, 20, 163, 215, 245, 153, 183, 230, 94, 212, 38, 195, 169, 182, 129, 22, 209, 18, 125, 194, 24, 168, 132, 166, 66, 206, 39, 67, 100, 91, 250, 86, 52, 190, 121, 189, 56, 210, 21, 143, 209, 41, 163, 37, 186, 87, 200, 133, 50, 58, 93, 13, 205, 24, 249, 110, 107, 196, 102, 15, 172, 233, 223, 67, 54, 161, 170, 167, 95, 154, 91, 47, 247, 228, 103, 52, 53, 61, 19, 214, 162, 49, 88, 219, 13, 105, 24, 158, 163, 101, 95, 219, 133, 244, 217, 151, 45, 25, 254, 161, 223, 233, 120, 177, 192, 237, 165, 242, 248, 203, 69, 125, 164, 51, 117, 70, 223, 125, 169, 73, 212, 59, 254, 66, 13, 247, 189, 215, 119, 157, 206, 233, 141, 224, 75, 101, 128, 103, 94, 31, 15, 14, 177, 173, 105, 47, 169, 240, 62, 31, 151, 77, 206, 64, 66, 99, 85, 238, 252, 104, 140, 177, 224, 68, 226, 81, 90, 229, 212, 224, 214, 73, 76, 13, 151, 237, 213, 120, 8, 21, 200, 67, 115, 215, 94, 8, 89, 0, 228, 60, 38, 110, 223, 160, 179, 107, 64, 128, 137, 205, 173, 18];

	}: _(RawOrigin::Signed(miner.clone()), space_proof_info, tee_sig)
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
