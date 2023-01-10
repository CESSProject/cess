// use super::*;
use crate::{Pallet as FileBank, *};
// use sp_application_crypto::{
// 	ecdsa::Signature,
// };
// use cp_cess_common::{Hash, IpAddress};
// use codec::{alloc::string::ToString, Decode};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
// use frame_support::{
// 	dispatch::UnfilteredDispatchable,
// 	pallet_prelude::*,
// 	traits::{Currency, CurrencyToVote, Get, Imbalance},
// };
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
// type SminerBalanceOf<T> = <<T as pallet_sminer::Config>::Currency as Currency<
// 	<T as frame_system::Config>::AccountId,
// >>::Balance;

const SEED: u32 = 2190502;
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
	FileMap::<T>::registration_scheduler(RawOrigin::Signed(controller.clone()).into(), stash, IpAddress::IPV4([127,0,0,1], 15001))?;
	Ok(controller)
}

fn add_filler<T: Config>() -> Result<(), &'static str> {
	pallet_sminer::benchmarking::set_mrenclave_code::<T>();
	let miner = pallet_sminer::benchmarking::add_miner::<T>("miner1");

	let mut filler_list: Vec<FillerInfo<T>> = Vec::new();
	let mut index = 0;
	for i in 0 .. 63 {
		let mut hash = Hash([49u8; 64]);
		hash.0[index] = b'd';
		// log::info!("value len: {:?}", value.len());
		let new_filler = FillerInfo::<T> {
			filler_size: 1024 * 1024 * 512,
			miner_address: miner.clone(),
			filler_hash: hash,
		};
		// log::info!("filler_hash: {:?}", new_filler.filler_hash);
		filler_list.push(new_filler);
		if filler_list.len() == 10 {
			FileBank::<T>::upload_filler(RawOrigin::Signed(miner.clone()).into(), filler_list.clone())?;
			filler_list.clear()
		}

		index = index + 1;
	}

	Ok(())
}

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
	upload_filler {
		let v in 0 .. 10;

		pallet_sminer::benchmarking::set_mrenclave_code::<T>();
		let miner = pallet_sminer::benchmarking::add_miner::<T>("miner1");

		let mut filler_list: Vec<FillerInfo<T>> = Vec::new();

		for i in 0 .. v {
			let index = i + 48;
			let mut new_filler = FillerInfo::<T> {
				filler_size: SLICE_DEFAULT_BYTE as u64,
				miner_address: miner.clone(),
				filler_hash: Hash([index as u8; 64]),
			};
			if i == 10 {
				new_filler.filler_hash = Hash([b'a'; 64]);
			} 
			filler_list.push(new_filler);
		}
	}: _(RawOrigin::Signed(miner.clone()), filler_list)
	verify {
		for i in 0 .. v {
			let index = i + 48;
			let mut filler_id = Hash([index as u8; 64]);
			if i == 10 {
				filler_id = Hash([b'a'; 64]);
			} 
			assert!(FillerMap::<T>::contains_key(&miner, &filler_id));
		}
	}

	upload_autonomy_file {
		let v in 0 .. 9;

		pallet_sminer::benchmarking::set_mrenclave_code::<T>();
		let miner = pallet_sminer::benchmarking::add_miner::<T>("miner1");

		let file_hash = Hash([b'a'; 64]);
		let file_size = SLICE_DEFAULT_BYTE as u64 * v as u64;
		let mut slices: Vec<Hash> = Default::default();
		for i in 0..v {
			let index = i + 48;
			let slice_hash = Hash([index as u8; 64]);
			slices.push(slice_hash);
		}

	}: _(RawOrigin::Signed(miner.clone()), file_hash.clone(), file_size, slices.clone())
	verify {
		assert!(AutonomyFile::<T>::contains_key(&miner, &file_hash));
		assert!(UniqueHashMap::<T>::contains_key(&file_hash));
		for i in 0 .. v {
			let index = i + 48;
			let slice_hash = Hash([index as u8; 64]);
			assert!(UniqueHashMap::<T>::contains_key(&slice_hash));
		}
	}

	delete_autonomy_file {
		pallet_sminer::benchmarking::set_mrenclave_code::<T>();
		let miner = pallet_sminer::benchmarking::add_miner::<T>("miner1");

		let file_hash = Hash([b'a'; 64]);
		let file_size = SLICE_DEFAULT_BYTE;
		let mut slices: Vec<Hash> = Default::default();
		let slice_hash = Hash([b'b'; 64]);
		slices.push(slice_hash);

		FileBank::<T>::upload_autonomy_file(RawOrigin::Signed(miner.clone()).into(), file_hash.clone(), file_size as u64, slices)?;
		assert!(AutonomyFile::<T>::contains_key(&miner, &file_hash));
		assert!(UniqueHashMap::<T>::contains_key(&file_hash));
		assert!(UniqueHashMap::<T>::contains_key(&slice_hash));
	}: _(RawOrigin::Signed(miner.clone()), file_hash)
	verify {
		assert!(!AutonomyFile::<T>::contains_key(&miner, &file_hash));
		assert!(!UniqueHashMap::<T>::contains_key(&file_hash));
		assert!(!UniqueHashMap::<T>::contains_key(&slice_hash));
	}

	buy_space {
		let user: AccountOf<T> = account("user1", 100, SEED);
		let _ = add_filler::<T>()?;

		<T as crate::Config>::Currency::make_free_balance_be(
			&user,
			BalanceOf::<T>::max_value(),
		);

		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account();
		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
		<T as crate::Config>::Currency::make_free_balance_be(
			&acc,
			balance,
		);
	}: _(RawOrigin::Signed(user.clone()), 2)
	verify {
		assert!(<UserOwnedSpace<T>>::contains_key(&user));
		let info = <UserOwnedSpace<T>>::get(&user).unwrap();
		assert_eq!(info.total_space, G_BYTE * 2);
	}

	expansion_space {
		let user: AccountOf<T> = account("user1", 100, SEED);
		let _ = add_filler::<T>()?;
		<T as crate::Config>::Currency::make_free_balance_be(
			&user,
			BalanceOf::<T>::max_value(),
		);

		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account();
		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
		<T as crate::Config>::Currency::make_free_balance_be(
			&acc,
			balance,
		);

		FileBank::<T>::buy_space(RawOrigin::Signed(user.clone()).into(), 2);
		assert!(<UserOwnedSpace<T>>::contains_key(&user));
	}: _(RawOrigin::Signed(user.clone()), 1)
	verify {
		assert!(<UserOwnedSpace<T>>::contains_key(&user));
		let info = <UserOwnedSpace<T>>::get(&user).unwrap();
		assert_eq!(info.total_space, G_BYTE * 3);
	}

	renewal_space {
		let user: AccountOf<T> = account("user1", 100, SEED);
		let _ = add_filler::<T>()?;
		<T as crate::Config>::Currency::make_free_balance_be(
			&user,
			BalanceOf::<T>::max_value(),
		);

		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account();
		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
		<T as crate::Config>::Currency::make_free_balance_be(
			&acc,
			balance,
		);

		FileBank::<T>::buy_space(RawOrigin::Signed(user.clone()).into(), 2);
		assert!(<UserOwnedSpace<T>>::contains_key(&user));

	}: _(RawOrigin::Signed(user.clone()), 30)
	verify {
		assert!(<UserOwnedSpace<T>>::contains_key(&user));
		let info = <UserOwnedSpace<T>>::get(&user).unwrap();
		assert_eq!(info.start, 0u32.saturated_into());
		let day60 = T::OneDay::get() * 60u32.saturated_into();
		assert_eq!(info.deadline, day60);
	}

	create_bucket {
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let name: Vec<u8> = "test-bucket1".as_bytes().to_vec();
		let name: BoundedVec<u8, T::NameStrLimit> = name.try_into().map_err(|_| "name convert error")?;
	}: _(RawOrigin::Signed(caller.clone()), caller.clone(), name.clone())
	verify {
		assert!(Bucket::<T>::contains_key(&caller, name));
	}

	delete_bucket {
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let name: Vec<u8> = "test-bucket1".as_bytes().to_vec();
		let name_bound: BoundedVec<u8, T::NameStrLimit> = name.clone().try_into().map_err(|_| "bounded_vec convert err!")?;
		FileBank::<T>::create_bucket(RawOrigin::Signed(caller.clone()).into(), caller.clone(), name_bound.clone())?;
		Bucket::<T>::contains_key(&caller, name_bound.clone());
	}: _(RawOrigin::Signed(caller.clone()), caller.clone(), name_bound.clone())
	verify {
		assert!(!Bucket::<T>::contains_key(&caller, name_bound));
	}

	upload_deal {
		log::info!("start upload deal");
		// create account
		let caller: AccountOf<T> = account("user1", 100, SEED);
		// create scheduler
		let controller = add_scheduler::<T>()?;
		// create upload deal param
		let name: Vec<u8> = "test-bucket1".as_bytes().to_vec();
		let name_bound: BoundedVec<u8, T::NameStrLimit> = name.clone().try_into().map_err(|_| "bounded_vec convert err!")?;

		let file_size = 1000u64;
		let file_hash = Hash([b'f'; 64]);
		let slices = vec![Hash([b'c'; 64])];

		let user_details = Details::<T>{
			user: caller.clone(),
			file_name: name_bound.clone(),
			bucket_name: name_bound.clone(),
		};

		let _ = add_filler::<T>()?;

		<T as crate::Config>::Currency::make_free_balance_be(
			&caller,
			BalanceOf::<T>::max_value(),
		);

		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account();
		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
		<T as crate::Config>::Currency::make_free_balance_be(
			&acc,
			balance,
		);
		FileBank::<T>::buy_space(RawOrigin::Signed(caller.clone()).into(), 2);
	}: _(RawOrigin::Signed(caller.clone()), file_hash.clone(), file_size, slices, user_details)
	verify {
		assert!(DealMap::<T>::contains_key(&file_hash));
		assert!(UniqueHashMap::<T>::contains_key(&file_hash));
	}

	pack_deal {
		let v in 0 .. 9;
		// create account
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let miner: AccountOf<T> = account("miner1", 100, SEED);
		// create scheduler

		let controller = add_scheduler::<T>()?;

		let _ = add_filler::<T>()?;

		<T as crate::Config>::Currency::make_free_balance_be(
			&caller,
			BalanceOf::<T>::max_value(),
		);

		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account();
		let balance: BalanceOf<T> = <T as crate::Config>::Currency::minimum_balance().checked_mul(&2u32.saturated_into()).ok_or("over flow")?;
		<T as crate::Config>::Currency::make_free_balance_be(
			&acc,
			balance,
		);

		FileBank::<T>::buy_space(RawOrigin::Signed(caller.clone()).into(), 30)?;

		let name: Vec<u8> = "test-bucket1".as_bytes().to_vec();
		let name_bound: BoundedVec<u8, T::NameStrLimit> = name.clone().try_into().map_err(|_| "bounded_vec convert err!")?;

		let file_size = 1000u64;
		let file_hash = Hash([b'f'; 64]);

		let user_details = Details::<T>{
			user: caller.clone(),
			file_name: name_bound.clone(),
			bucket_name: name_bound.clone(),
		};

		let mut slices: Vec<Hash> = Default::default();
		for i in 0 .. v {
			let index = i + 48;
			let mut slice_hash = Hash([index as u8; 64]);
			slice_hash.0[63] = b'f';
			slices.push(slice_hash);
		}

		FileBank::<T>::upload_deal(RawOrigin::Signed(caller.clone()).into(), file_hash.clone(), file_size, slices, user_details)?;

		let msg1: Vec<Vec<u8>> = vec![
			"{\"shardId\":\"000000000000000000000000000000000000000000000000000000000000000f.101\",\"sliceHash\":\"000000000000000000000000000000000000000000000000000000000000000f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"111111111111111111111111111111111111111111111111111111111111111f.102\",\"sliceHash\":\"111111111111111111111111111111111111111111111111111111111111111f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"222222222222222222222222222222222222222222222222222222222222222f.103\",\"sliceHash\":\"222222222222222222222222222222222222222222222222222222222222222f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"333333333333333333333333333333333333333333333333333333333333333f.104\",\"sliceHash\":\"333333333333333333333333333333333333333333333333333333333333333f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"444444444444444444444444444444444444444444444444444444444444444f.105\",\"sliceHash\":\"444444444444444444444444444444444444444444444444444444444444444f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"555555555555555555555555555555555555555555555555555555555555555f.106\",\"sliceHash\":\"555555555555555555555555555555555555555555555555555555555555555f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"666666666666666666666666666666666666666666666666666666666666666f.107\",\"sliceHash\":\"666666666666666666666666666666666666666666666666666666666666666f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"777777777777777777777777777777777777777777777777777777777777777f.108\",\"sliceHash\":\"777777777777777777777777777777777777777777777777777777777777777f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"888888888888888888888888888888888888888888888888888888888888888f.109\",\"sliceHash\":\"888888888888888888888888888888888888888888888888888888888888888f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
		];

		let msg2: Vec<Vec<u8>> = vec![
			"{\"shardId\":\"000000000000000000000000000000000000000000000000000000000000000f.201\",\"sliceHash\":\"000000000000000000000000000000000000000000000000000000000000000f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"111111111111111111111111111111111111111111111111111111111111111f.202\",\"sliceHash\":\"111111111111111111111111111111111111111111111111111111111111111f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"222222222222222222222222222222222222222222222222222222222222222f.203\",\"sliceHash\":\"222222222222222222222222222222222222222222222222222222222222222f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"333333333333333333333333333333333333333333333333333333333333333f.204\",\"sliceHash\":\"333333333333333333333333333333333333333333333333333333333333333f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"444444444444444444444444444444444444444444444444444444444444444f.205\",\"sliceHash\":\"444444444444444444444444444444444444444444444444444444444444444f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"555555555555555555555555555555555555555555555555555555555555555f.206\",\"sliceHash\":\"555555555555555555555555555555555555555555555555555555555555555f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"666666666666666666666666666666666666666666666666666666666666666f.207\",\"sliceHash\":\"666666666666666666666666666666666666666666666666666666666666666f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"777777777777777777777777777777777777777777777777777777777777777f.208\",\"sliceHash\":\"777777777777777777777777777777777777777777777777777777777777777f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"888888888888888888888888888888888888888888888888888888888888888f.209\",\"sliceHash\":\"888888888888888888888888888888888888888888888888888888888888888f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
		];

		let msg3: Vec<Vec<u8>> = vec![
			"{\"shardId\":\"000000000000000000000000000000000000000000000000000000000000000f.301\",\"sliceHash\":\"000000000000000000000000000000000000000000000000000000000000000f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"111111111111111111111111111111111111111111111111111111111111111f.302\",\"sliceHash\":\"111111111111111111111111111111111111111111111111111111111111111f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"222222222222222222222222222222222222222222222222222222222222222f.303\",\"sliceHash\":\"222222222222222222222222222222222222222222222222222222222222222f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"333333333333333333333333333333333333333333333333333333333333333f.304\",\"sliceHash\":\"333333333333333333333333333333333333333333333333333333333333333f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"444444444444444444444444444444444444444444444444444444444444444f.305\",\"sliceHash\":\"444444444444444444444444444444444444444444444444444444444444444f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"555555555555555555555555555555555555555555555555555555555555555f.306\",\"sliceHash\":\"555555555555555555555555555555555555555555555555555555555555555f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"666666666666666666666666666666666666666666666666666666666666666f.307\",\"sliceHash\":\"666666666666666666666666666666666666666666666666666666666666666f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"777777777777777777777777777777777777777777777777777777777777777f.308\",\"sliceHash\":\"777777777777777777777777777777777777777777777777777777777777777f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
			"{\"shardId\":\"888888888888888888888888888888888888888888888888888888888888888f.309\",\"sliceHash\":\"888888888888888888888888888888888888888888888888888888888888888f\",\"minerIp\":\"112/112/112/112/15001\"}"
				.as_bytes().to_vec(),
		];

		let sig1: Vec<Signature> = vec![
			Signature([231, 158, 9, 168, 153, 115, 89, 41, 79, 140, 199, 178, 124, 76, 132, 101, 140, 79, 17, 75, 92, 249, 221, 201, 6, 40, 249, 43, 132, 254, 246, 183, 9, 104, 177, 51, 33, 79, 212, 145, 229, 139, 87, 72, 49, 17, 55, 45, 26, 234, 162, 35, 158, 130, 54, 73, 166, 137, 245, 167, 214, 158, 33, 49, 0]),
			Signature([1, 207, 59, 73, 7, 195, 113, 69, 59, 150, 214, 89, 68, 93, 93, 245, 149, 137, 238, 104, 115, 29, 57, 186, 108, 137, 189, 205, 156, 55, 122, 219, 110, 138, 174, 234, 7, 231, 241, 47, 11, 197, 47, 76, 253, 171, 20, 6, 157, 174, 69, 119, 245, 136, 215, 20, 193, 182, 21, 157, 169, 243, 222, 76, 1]),
			Signature([64, 121, 14, 242, 4, 109, 141, 161, 20, 150, 81, 184, 246, 6, 251, 75, 51, 70, 47, 8, 69, 137, 79, 46, 235, 128, 166, 150, 33, 17, 169, 12, 88, 206, 138, 239, 85, 128, 152, 188, 165, 28, 142, 148, 126, 178, 197, 120, 2, 82, 250, 48, 72, 122, 184, 160, 253, 10, 41, 247, 31, 154, 168, 77, 0]),
			Signature([198, 14, 205, 244, 98, 5, 17, 25, 86, 139, 42, 93, 89, 108, 47, 40, 148, 250, 124, 198, 41, 165, 4, 94, 82, 38, 246, 35, 104, 56, 161, 217, 125, 69, 162, 51, 133, 232, 32, 95, 102, 69, 187, 55, 106, 70, 112, 211, 36, 189, 31, 230, 125, 85, 97, 99, 229, 228, 184, 148, 78, 99, 149, 23, 0]),
			Signature([200, 61, 243, 123, 182, 11, 203, 195, 90, 219, 1, 177, 117, 86, 156, 4, 229, 124, 20, 157, 88, 22, 7, 74, 195, 154, 243, 161, 254, 13, 104, 19, 72, 122, 57, 60, 201, 58, 106, 196, 93, 86, 1, 209, 13, 249, 122, 99, 56, 205, 107, 37, 18, 68, 253, 37, 151, 169, 96, 60, 203, 88, 22, 68, 0]),
			Signature([168, 138, 44, 19, 248, 118, 200, 36, 231, 201, 105, 171, 160, 122, 7, 110, 44, 40, 191, 154, 42, 171, 190, 183, 49, 61, 88, 145, 217, 33, 2, 174, 21, 50, 104, 216, 206, 217, 54, 136, 71, 169, 177, 190, 252, 170, 240, 229, 76, 186, 72, 255, 67, 148, 123, 219, 174, 114, 179, 20, 66, 198, 41, 141, 1]),
			Signature([89, 250, 84, 102, 70, 162, 154, 103, 75, 131, 150, 170, 245, 237, 205, 61, 108, 126, 107, 29, 157, 106, 116, 194, 109, 248, 64, 171, 107, 184, 163, 46, 45, 101, 27, 162, 102, 10, 233, 185, 142, 132, 126, 97, 202, 171, 127, 227, 63, 122, 8, 73, 221, 250, 168, 9, 240, 202, 112, 236, 191, 114, 234, 99, 1]),
			Signature([21, 77, 171, 90, 107, 16, 22, 162, 126, 47, 0, 9, 118, 156, 243, 54, 75, 120, 185, 238, 60, 13, 9, 215, 174, 212, 130, 57, 72, 244, 172, 237, 85, 108, 13, 132, 112, 23, 126, 143, 127, 228, 134, 157, 34, 109, 215, 180, 91, 158, 137, 28, 138, 122, 5, 252, 12, 250, 76, 157, 57, 182, 232, 219, 0]),
			Signature([102, 228, 129, 81, 203, 210, 222, 8, 140, 91, 187, 188, 175, 111, 217, 89, 192, 138, 162, 63, 184, 16, 122, 64, 138, 154, 20, 131, 65, 10, 250, 121, 52, 169, 151, 9, 187, 97, 77, 116, 138, 225, 181, 164, 205, 108, 74, 234, 120, 193, 78, 25, 163, 198, 185, 150, 199, 135, 94, 123, 178, 172, 238, 50, 1]),
		];

		let sig2: Vec<Signature> = vec![
			Signature([238, 101, 253, 0, 32, 198, 240, 145, 92, 150, 12, 223, 79, 47, 154, 31, 69, 49, 207, 129, 138, 114, 191, 240, 27, 112, 155, 11, 84, 34, 102, 113, 50, 2, 254, 152, 111, 18, 181, 115, 181, 31, 234, 223, 14, 171, 236, 136, 148, 44, 26, 70, 23, 123, 122, 174, 182, 246, 67, 25, 89, 110, 92, 119, 0]),
			Signature([235, 84, 210, 28, 64, 163, 153, 137, 27, 19, 63, 29, 109, 143, 117, 210, 36, 153, 240, 94, 145, 251, 86, 5, 42, 206, 92, 117, 76, 6, 24, 148, 66, 113, 175, 36, 134, 149, 186, 132, 142, 135, 34, 180, 174, 14, 237, 226, 1, 164, 242, 62, 193, 52, 156, 125, 3, 50, 62, 178, 107, 185, 193, 35, 0]),
			Signature([179, 186, 100, 23, 33, 85, 5, 104, 191, 43, 112, 85, 98, 153, 230, 79, 227, 108, 244, 192, 95, 20, 33, 24, 251, 149, 129, 188, 213, 163, 149, 143, 20, 56, 110, 141, 250, 129, 216, 12, 117, 156, 43, 152, 180, 242, 80, 66, 5, 170, 113, 57, 168, 105, 15, 207, 47, 210, 223, 120, 251, 148, 3, 6, 0]),
			Signature([27, 73, 141, 124, 43, 129, 100, 36, 116, 233, 71, 85, 103, 254, 233, 112, 227, 50, 177, 160, 17, 134, 231, 163, 10, 172, 156, 118, 45, 63, 221, 122, 71, 183, 155, 218, 18, 155, 182, 75, 181, 206, 1, 167, 246, 10, 22, 224, 40, 167, 186, 114, 33, 133, 243, 58, 77, 247, 244, 250, 148, 107, 167, 118, 1]),
			Signature([70, 166, 216, 149, 220, 83, 254, 207, 13, 226, 93, 28, 44, 181, 25, 87, 57, 165, 223, 72, 75, 46, 165, 27, 167, 86, 37, 85, 179, 97, 105, 161, 98, 100, 133, 57, 61, 162, 85, 177, 61, 4, 208, 200, 93, 90, 97, 84, 118, 110, 23, 213, 174, 210, 136, 182, 106, 72, 71, 223, 188, 183, 171, 125, 0]),
			Signature([46, 59, 8, 195, 251, 152, 164, 36, 39, 24, 22, 198, 253, 141, 91, 219, 82, 203, 198, 66, 188, 183, 166, 64, 96, 63, 171, 234, 107, 103, 50, 83, 87, 215, 133, 244, 204, 0, 3, 141, 62, 93, 213, 168, 127, 97, 217, 189, 114, 75, 76, 146, 43, 114, 88, 224, 23, 207, 145, 53, 136, 175, 114, 137, 1]),
			Signature([208, 3, 53, 69, 163, 204, 59, 75, 243, 223, 231, 217, 187, 64, 152, 70, 146, 22, 134, 190, 6, 218, 46, 84, 41, 89, 207, 247, 179, 183, 67, 53, 117, 139, 238, 93, 215, 114, 192, 142, 203, 171, 38, 234, 56, 130, 125, 176, 46, 190, 33, 143, 141, 80, 88, 234, 230, 28, 41, 0, 47, 98, 134, 65, 0]),
			Signature([52, 220, 32, 231, 79, 42, 14, 27, 14, 44, 255, 144, 212, 176, 57, 6, 95, 32, 18, 39, 79, 227, 17, 92, 15, 109, 9, 144, 194, 246, 228, 40, 22, 250, 215, 111, 78, 176, 154, 29, 105, 56, 129, 240, 150, 115, 138, 72, 108, 41, 227, 166, 38, 195, 203, 153, 148, 181, 111, 252, 2, 79, 246, 107, 1]),
			Signature([133, 124, 200, 224, 83, 177, 255, 13, 60, 205, 51, 7, 39, 73, 96, 98, 196, 113, 26, 205, 53, 158, 248, 73, 11, 230, 243, 58, 222, 17, 49, 97, 62, 22, 28, 227, 231, 46, 216, 78, 219, 215, 90, 56, 244, 225, 233, 146, 70, 222, 76, 63, 113, 240, 197, 38, 226, 95, 242, 240, 50, 234, 183, 255, 1]),
		];

		let sig3: Vec<Signature> = vec![
			Signature([167, 37, 91, 224, 182, 90, 92, 216, 196, 131, 161, 194, 19, 71, 128, 131, 225, 175, 31, 205, 168, 103, 202, 48, 184, 94, 191, 121, 158, 93, 150, 241, 18, 145, 202, 174, 244, 115, 103, 254, 185, 135, 57, 15, 159, 141, 76, 229, 74, 111, 78, 163, 129, 65, 218, 88, 214, 70, 139, 159, 128, 136, 8, 106, 1]),
			Signature([189, 35, 152, 227, 247, 82, 124, 205, 82, 26, 11, 13, 146, 77, 212, 179, 96, 2, 63, 19, 104, 140, 38, 245, 106, 216, 24, 155, 215, 113, 28, 29, 113, 174, 8, 150, 21, 182, 2, 203, 22, 76, 109, 163, 82, 6, 30, 129, 140, 217, 9, 130, 228, 240, 198, 3, 243, 158, 17, 80, 68, 141, 230, 16, 1]),
			Signature([195, 181, 207, 19, 196, 241, 189, 218, 110, 41, 91, 30, 250, 237, 92, 123, 230, 80, 156, 82, 40, 137, 209, 162, 236, 76, 72, 203, 93, 79, 22, 251, 48, 192, 172, 87, 51, 3, 18, 87, 141, 163, 240, 14, 145, 149, 106, 92, 153, 202, 23, 220, 17, 131, 1, 216, 254, 42, 154, 89, 144, 186, 196, 114, 0]),
			Signature([96, 236, 194, 116, 143, 26, 152, 22, 17, 38, 56, 111, 103, 56, 168, 209, 115, 140, 126, 233, 171, 24, 80, 26, 162, 21, 129, 97, 56, 65, 141, 170, 124, 231, 49, 182, 82, 30, 23, 163, 172, 20, 83, 67, 252, 242, 241, 18, 70, 197, 123, 246, 244, 119, 138, 35, 6, 187, 221, 117, 129, 248, 56, 211, 1]),
			Signature([75, 20, 163, 100, 137, 125, 47, 16, 74, 11, 10, 218, 23, 100, 142, 201, 88, 252, 166, 204, 171, 12, 140, 215, 189, 197, 193, 49, 126, 99, 54, 96, 6, 144, 160, 233, 86, 187, 66, 123, 235, 10, 240, 175, 176, 78, 127, 151, 222, 64, 196, 238, 92, 36, 232, 87, 106, 203, 95, 245, 167, 119, 247, 92, 1]),
			Signature([78, 45, 152, 141, 33, 156, 85, 201, 98, 253, 202, 159, 35, 25, 233, 228, 75, 252, 40, 78, 48, 104, 14, 228, 37, 20, 178, 90, 191, 210, 220, 98, 87, 10, 234, 25, 36, 178, 211, 115, 7, 150, 113, 201, 160, 74, 62, 248, 205, 253, 161, 131, 126, 12, 74, 196, 75, 251, 216, 174, 219, 115, 232, 232, 1]),
			Signature([90, 121, 154, 182, 16, 233, 123, 169, 195, 190, 136, 36, 109, 235, 150, 66, 58, 188, 162, 195, 99, 114, 188, 204, 240, 47, 133, 19, 123, 2, 212, 77, 119, 250, 216, 69, 4, 211, 179, 29, 136, 35, 211, 27, 192, 241, 207, 218, 238, 221, 49, 25, 64, 75, 76, 53, 251, 139, 153, 236, 1, 29, 176, 247, 1]),
			Signature([147, 52, 233, 50, 196, 232, 212, 98, 26, 14, 188, 238, 40, 59, 177, 229, 192, 97, 244, 36, 195, 64, 81, 85, 32, 209, 108, 137, 151, 113, 8, 30, 43, 30, 95, 115, 202, 208, 14, 24, 32, 18, 195, 155, 41, 142, 232, 26, 124, 19, 172, 198, 79, 174, 72, 249, 250, 215, 49, 218, 47, 38, 147, 119, 0]),
			Signature([209, 222, 35, 131, 134, 186, 129, 138, 243, 239, 180, 180, 175, 247, 147, 123, 231, 157, 49, 128, 160, 1, 140, 93, 90, 131, 191, 106, 117, 186, 229, 32, 22, 198, 159, 233, 61, 120, 83, 239, 174, 140, 166, 208, 28, 20, 99, 111, 216, 28, 167, 136, 189, 224, 151, 22, 242, 195, 17, 237, 27, 159, 58, 171, 0]),
		];
		// let msg = 
		let mut slice_summary_list1: Vec<SliceSummary<T>> = Default::default();
		let mut slice_summary_list2: Vec<SliceSummary<T>> = Default::default();
		let mut slice_summary_list3: Vec<SliceSummary<T>> = Default::default();
		for i in 0 .. v {
			let slice_summary = SliceSummary::<T> {
				miner: miner.clone(),
				signature: sig1[i as usize].clone(),
				message: msg1[i as usize].clone().try_into().unwrap(),
			};
			slice_summary_list1.push(slice_summary);

			let slice_summary = SliceSummary::<T> {
				miner: miner.clone(),
				signature: sig2[i as usize].clone(),
				message: msg2[i as usize].clone().try_into().unwrap(),
			};
			slice_summary_list2.push(slice_summary);

			let slice_summary = SliceSummary::<T> {
				miner: miner.clone(),
				signature: sig3[i as usize].clone(),
				message: msg3[i as usize].clone().try_into().unwrap(),
			};
			slice_summary_list3.push(slice_summary);
		}

		let mut backup_summary: [Vec<SliceSummary<T>>; 3] = [slice_summary_list1, slice_summary_list2, slice_summary_list3];
		
	}: _(RawOrigin::Signed(controller.clone()), file_hash.clone(), backup_summary)
	verify {
		assert!(File::<T>::contains_key(&file_hash));
	}

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
// 		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account();
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
// 		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account();
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
// 		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account();
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

// 		let acc: AccountOf<T> = T::FilbakPalletId::get().into_account();
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
}
