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
use pallet_storage_handler::{Pallet as StorageHandler};
use sp_runtime::{
	traits::{Bounded, One, StaticLookup, TrailingZeroInput, Zero},
	Perbill, Percent, Digest, DigestItem,
};
use sp_std::prelude::*;
use scale_info::prelude::format;
use frame_system::RawOrigin;
use sp_runtime::traits::BlakeTwo256;
use cessp_consensus_rrsc::{Slot, RRSC_ENGINE_ID};
pub struct Pallet<T: Config>(FileBank<T>);
pub trait Config:
	crate::Config + pallet_sminer::benchmarking::Config + pallet_storage_handler::Config + pallet_rrsc::Config + pallet_grandpa::Config + pallet_session::Config
{
}
type SminerBalanceOf<T> = <<T as pallet_storage_handler::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

const SEED: u32 = 2190502;
const miner_list: [&'static str; 30] = [
	"miner1", "miner2", "miner3", "miner4", "miner5", "miner6", "miner7", "miner8", "miner9", "miner10",
	"miner11", "miner12", "miner13", "miner14", "miner15", "miner16", "miner17", "miner18", "miner19", "miner20",
	"miner21", "miner22", "miner23", "miner24", "miner25", "miner26", "miner27", "miner28", "miner29", "miner30",
];
// const MAX_SPANS: u32 = 100;
pub struct DealSubmitInfo<T: Config> {
	file_hash: Hash,
	user_brief: UserBrief<T>,
	segment_list: BoundedVec<SegmentList<T>, T::SegmentCount>,
	file_size: u128,
}

pub fn add_idle_space<T: Config>(miner: T::AccountId) -> Result<(), &'static str> {
	let idle_space = <T as crate::Config>::MinerControl::add_miner_idle_space(
		&miner,
		[8u8; 256], 
		10000,
		[8u8; 256],
	).expect("add idle space failed, func add_miner_idle_space()");

	<T as crate::Config>::StorageHandle::add_total_idle_space(idle_space).expect("add idle space failed, func add_total_idle_space()");

	Ok(())
}

pub fn buy_space<T: Config>(user: T::AccountId) -> Result<(), &'static str> {
	<T as pallet_storage_handler::Config>::Currency::make_free_balance_be(
		&user,
		SminerBalanceOf::<T>::max_value(),
	);

	StorageHandler::<T>::buy_space(RawOrigin::Signed(user).into(), 10)?;

	Ok(())
}

pub fn create_deal_info<T: Config>(acc: AccountOf<T>, length: u32) -> Result<DealSubmitInfo<T>, &'static str> {
	let mut deal_info: BoundedVec<SegmentList<T>, T::SegmentCount> = Default::default();
	let file_name = "test-file".as_bytes().to_vec();
	let bucket_name = "test-bucket1".as_bytes().to_vec();
	let file_hash: Hash = Hash([4u8; 64]);
	let user_brief = UserBrief::<T>{
		user: acc,
		file_name: file_name.try_into().map_err(|_e| "file name convert err")?,
		bucket_name: bucket_name.try_into().map_err(|_e| "bucket name convert err")?,
	};

	let mut seed = 0;
	for i in 0 .. length {
		let segment_list = SegmentList::<T> {
			hash: Hash([1u8; 64]),
			fragment_list: [
				Hash([97u8; 64]),
				Hash([98u8; 64]),
				Hash([99u8; 64]),
			].to_vec().try_into().unwrap(),
		};

		deal_info.try_push(segment_list).unwrap();
	}

	Ok(DealSubmitInfo::<T>{
		file_hash: file_hash,
		user_brief: user_brief,
		segment_list: deal_info,
		file_size: 123,
	})
}

pub fn create_new_bucket<T: Config>(caller: T::AccountId, name: Vec<u8>) -> Result<(), &'static str> {
	let name = name.try_into().map_err(|_| "create bucket convert error")?;
	FileBank::<T>::create_bucket(RawOrigin::Signed(caller.clone()).into(), caller, name)?;
	Ok(())
}

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
	// cert_idle_space {
	// 	let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
	// 	let miner = pallet_sminer::benchmarking::add_miner::<T>("miner1")?;

	// 	let pois_key = PoISKey {
    //         g: [2u8; 256],
    //         n: [3u8; 256],
    //     };

	// 	let space_proof_info = SpaceProofInfo::<AccountOf<T>, BlockNumberOf<T>> {
    //         last_operation_block: 5u32.saturated_into(),
    //         miner: miner.clone(),
    //         front: u64::MIN,
    //         rear: 1000,
    //         pois_key: pois_key.clone(),
    //         accumulator: pois_key.g,
    //     };

	// 	let original = space_proof_info.encode();
	// 	let original = sp_io::hashing::sha2_256(&original);

	// 	log::info!("original: {:?}", original);

	// 	let tee_sig = [30, 158, 52, 127, 32, 101, 53, 6, 105, 180, 114, 181, 75, 121, 182, 16, 185, 182, 120, 50, 218, 241, 222, 48, 162, 218, 67, 67, 254, 87, 153, 234, 249, 165, 250, 113, 36, 145, 137, 102, 251, 241, 219, 134, 93, 7, 22, 130, 64, 108, 13, 84, 60, 84, 224, 58, 219, 121, 153, 49, 60, 140, 107, 67, 8, 143, 132, 31, 1, 151, 208, 100, 18, 131, 157, 132, 159, 42, 213, 92, 248, 12, 219, 35, 118, 227, 149, 114, 155, 115, 149, 239, 29, 190, 150, 196, 107, 207, 144, 89, 181, 61, 132, 25, 31, 42, 0, 128, 243, 58, 9, 220, 221, 188, 20, 151, 24, 177, 51, 254, 205, 85, 173, 248, 250, 101, 189, 242, 114, 204, 5, 253, 244, 120, 48, 207, 153, 38, 84, 1, 53, 151, 113, 194, 224, 18, 77, 82, 190, 231, 144, 255, 188, 149, 134, 115, 207, 16, 194, 122, 75, 143, 243, 86, 210, 118, 80, 2, 252, 247, 96, 163, 103, 81, 99, 27, 39, 8, 224, 178, 67, 131, 163, 251, 183, 13, 166, 63, 7, 78, 38, 227, 178, 14, 89, 11, 221, 17, 155, 250, 196, 181, 119, 40, 245, 170, 168, 10, 164, 210, 207, 200, 88, 199, 229, 165, 47, 168, 75, 21, 90, 94, 162, 157, 185, 152, 68, 55, 45, 151, 224, 144, 222, 94, 171, 237, 28, 166, 230, 33, 109, 160, 74, 169, 108, 68, 168, 14, 135, 202, 92, 146, 15, 92, 165, 64];

	// }: _(RawOrigin::Signed(miner.clone()), space_proof_info, tee_sig)
	// verify {
	// 	let (idle, service) = T::MinerControl::get_power(&miner)?;
	// 	assert_eq!(idle, 1000 * 256 * 1024 * 1024);
	// }

	// upload_declaration {
	// 	let v in 1 .. 30;
	// 	// <pallet_rrsc::Pallet<T> as Hooks<T::BlockNumber>>::on_initialize(1u32.saturated_into());
	// 	// <frame_system::Pallet<T>>::set_block_number(1u32.saturated_into());
	// 	let caller: AccountOf<T> = account("user1", 100, SEED);
	// 	let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;

	// 	for i in 0 .. v {
	// 		let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
	// 		add_idle_space::<T>(miner.clone())?;
	// 	}
	// 	buy_space::<T>(caller.clone())?;
	// 	let mut deal_info: BoundedVec<SegmentList<T>, T::SegmentCount> = Default::default();
	// 	let file_name = "test-file".as_bytes().to_vec();
	// 	let bucket_name = "test-bucket1".as_bytes().to_vec();
	// 	let file_hash: Hash = Hash([4u8; 64]);
	// 	let user_brief = UserBrief::<T>{
	// 		user: caller.clone(),
	// 		file_name: file_name.try_into().map_err(|_e| "file name convert err")?,
	// 		bucket_name: bucket_name.try_into().map_err(|_e| "bucket name convert err")?,
	// 	};
	// 	let segment_list = SegmentList::<T> {
	// 		hash: Hash([1u8; 64]),
	// 		fragment_list: [
	// 			Hash([2u8; 64]),
	// 			Hash([3u8; 64]),
	// 			Hash([4u8; 64]),
	// 		].to_vec().try_into().unwrap(),
	// 	};
	// 	deal_info.try_push(segment_list).unwrap();
	// 	let segment_list = SegmentList::<T> {
	// 		hash: Hash([2u8; 64]),
	// 		fragment_list: [
	// 			Hash([2u8; 64]),
	// 			Hash([3u8; 64]),
	// 			Hash([4u8; 64]),
	// 		].to_vec().try_into().unwrap(),
	// 	};
	// 	deal_info.try_push(segment_list).unwrap();
	// 	let segment_list = SegmentList::<T> {
	// 		hash: Hash([3u8; 64]),
	// 		fragment_list: [
	// 			Hash([2u8; 64]),
	// 			Hash([3u8; 64]),
	// 			Hash([4u8; 64]),
	// 		].to_vec().try_into().unwrap(),
	// 	};
	// 	deal_info.try_push(segment_list).unwrap();
	// }: _(RawOrigin::Signed(caller), file_hash.clone(), deal_info, user_brief, 123)
	// verify {
	// 	assert!(DealMap::<T>::contains_key(&file_hash));
	// }

	// upload_declaration_expected_max {
	// 	let v in 1 .. 30;
	// 	let caller: AccountOf<T> = account("user1", 100, SEED);
	// 	let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
	// 	for i in 0 .. 30 {
	// 		let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
	// 		add_idle_space::<T>(miner.clone())?;
	// 		if i < 27 {
	// 			pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
	// 		}
	// 	}
	// 	buy_space::<T>(caller.clone())?;

	// 	let deal_info = create_deal_info::<T>(caller.clone(), v)?;
	// }: upload_declaration(RawOrigin::Signed(caller), deal_info.file_hash.clone(), deal_info.segment_list, deal_info.user_brief, deal_info.file_size)
	// verify {
	// 	assert!(DealMap::<T>::contains_key(&deal_info.file_hash));
	// }

	// transfer_report {
	// 	let v in 1 .. 30;
	// 	let caller: AccountOf<T> = account("user1", 100, SEED);
	// 	let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
	// 	for i in 0 .. 15 {
	// 		let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
	// 		add_idle_space::<T>(miner.clone())?;
	// 		if i < 12 {
	// 			pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
	// 		}
	// 	}
	// 	buy_space::<T>(caller.clone())?;

	// 	let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
	// 	FileBank::<T>::upload_declaration(
	// 		RawOrigin::Signed(caller).into(),
	// 		deal_submit_info.file_hash.clone(),
	// 		deal_submit_info.segment_list,
	// 		deal_submit_info.user_brief,
	// 		deal_submit_info.file_size,
	// 	)?;

	// 	let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
	// }: _(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()), vec![deal_submit_info.file_hash])
	// verify {
	// 	let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
	// 	assert_eq!(deal_info.complete_list.len(), 1);
	// }

	// transfer_report_last {
	// 	let v in 1 .. 30;
	// 	let caller: AccountOf<T> = account("user1", 100, SEED);
	// 	let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
	// 	for i in 0 .. 15 {
	// 		let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
	// 		add_idle_space::<T>(miner.clone())?;
	// 		if i < 12 {
	// 			pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
	// 		}
	// 	}
	// 	buy_space::<T>(caller.clone())?;

	// 	let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
	// 	FileBank::<T>::upload_declaration(
	// 		RawOrigin::Signed(caller).into(),
	// 		deal_submit_info.file_hash.clone(),
	// 		deal_submit_info.segment_list,
	// 		deal_submit_info.user_brief,
	// 		deal_submit_info.file_size,
	// 	)?;

	// 	let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
	// 	FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
	// 	FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
	// }: transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()), vec![deal_submit_info.file_hash])
	// verify {
	// 	assert!(File::<T>::contains_key(&deal_submit_info.file_hash));
	// }

	// upload_declaration_fly_upload {
	// 	let v in 1 .. 30;
	// 	let caller: AccountOf<T> = account("user1", 100, SEED);
	// 	let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
	// 	for i in 0 .. 15 {
	// 		let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
	// 		add_idle_space::<T>(miner.clone())?;
	// 		if i < 12 {
	// 			pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
	// 		}
	// 	}
	// 	buy_space::<T>(caller.clone())?;

	// 	let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
	// 	FileBank::<T>::upload_declaration(
	// 		RawOrigin::Signed(caller).into(),
	// 		deal_submit_info.file_hash.clone(),
	// 		deal_submit_info.segment_list,
	// 		deal_submit_info.user_brief,
	// 		deal_submit_info.file_size,
	// 	)?;

	// 	let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
	// 	FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
	// 	FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
	// 	FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
	// 	let caller: AccountOf<T> = account("user2", 100, SEED);
	// 	buy_space::<T>(caller.clone())?;
	// 	let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
	// }: upload_declaration(RawOrigin::Signed(caller.clone()), deal_submit_info.file_hash.clone(), deal_submit_info.segment_list, deal_submit_info.user_brief.clone(), deal_submit_info.file_size)
	// verify {
	// 	let file = File::<T>::get(deal_submit_info.file_hash.clone()).unwrap();
	// 	assert!(file.owner.contains(&deal_submit_info.user_brief))
	// }

	// deal_reassign_miner {
	// 	let v in 0 ..30;
	// 	let caller: AccountOf<T> = account("user1", 100, SEED);
	// 	let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
	// 	for i in 0 .. 30 {
	// 		let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
	// 		add_idle_space::<T>(miner.clone())?;
	// 		if i < 24 {
	// 			pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
	// 		}
	// 	}
	// 	buy_space::<T>(caller.clone())?;

	// 	let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
	// 	FileBank::<T>::upload_declaration(
	// 		RawOrigin::Signed(caller.clone()).into(),
	// 		deal_submit_info.file_hash.clone(),
	// 		deal_submit_info.segment_list,
	// 		deal_submit_info.user_brief,
	// 		deal_submit_info.file_size,
	// 	)?;

	// 	<T as crate::Config>::FScheduler::cancel_named([4u8; 64].to_vec()).expect("cancel scheduler failed");
	// }: _(RawOrigin::Root, deal_submit_info.file_hash.clone(), 1, 200)
	// verify {
	// 	let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
	// 	assert_eq!(deal_info.count, 1);
	// }

	// deal_reassign_miner_exceed_limit {
	// 	let v in 0 ..30;
	// 	let caller: AccountOf<T> = account("user1", 100, SEED);
	// 	let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
	// 	for i in 0 .. 30 {
	// 		let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
	// 		add_idle_space::<T>(miner.clone())?;
	// 		if i < 24 {
	// 			pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
	// 		}
	// 	}
	// 	buy_space::<T>(caller.clone())?;

	// 	let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
	// 	FileBank::<T>::upload_declaration(
	// 		RawOrigin::Signed(caller.clone()).into(),
	// 		deal_submit_info.file_hash.clone(),
	// 		deal_submit_info.segment_list,
	// 		deal_submit_info.user_brief,
	// 		deal_submit_info.file_size,
	// 	)?;
	// }: deal_reassign_miner(RawOrigin::Root, deal_submit_info.file_hash.clone(), 20, 200)
	// verify {
	// 	assert!(!DealMap::<T>::contains_key(&deal_submit_info.file_hash));
	// }

	// calculate_end {
	// 	let v in 1 .. 30;
	// 	let caller: AccountOf<T> = account("user1", 100, SEED);
	// 	let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
	// 	for i in 0 .. 5 {
	// 		let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
	// 		add_idle_space::<T>(miner.clone())?;
	// 	}
	// 	buy_space::<T>(caller.clone())?;

	// 	let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
	// 	FileBank::<T>::upload_declaration(
	// 		RawOrigin::Signed(caller).into(),
	// 		deal_submit_info.file_hash.clone(),
	// 		deal_submit_info.segment_list,
	// 		deal_submit_info.user_brief,
	// 		deal_submit_info.file_size,
	// 	)?;

	// 	let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
	// 	FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
	// 	FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
	// 	FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
	// 	let file_info = File::<T>::get(&deal_submit_info.file_hash).unwrap();
	// }: _(RawOrigin::Root, deal_submit_info.file_hash)
	// verify {
	// 	let file_info = File::<T>::get(&deal_submit_info.file_hash).unwrap();
	// 	assert_eq!(FileState::Active, file_info.stat);
	// }

	// replace_idle_space {
	// 	let caller: AccountOf<T> = account("user1", 100, SEED);
	// 	let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
	// 	for i in 0 .. 3 {
	// 		let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
	// 		add_idle_space::<T>(miner.clone())?;
	// 	}
	// 	buy_space::<T>(caller.clone())?;

	// 	let deal_submit_info = create_deal_info::<T>(caller.clone(), 5)?;
	// 	FileBank::<T>::upload_declaration(
	// 		RawOrigin::Signed(caller).into(),
	// 		deal_submit_info.file_hash.clone(),
	// 		deal_submit_info.segment_list,
	// 		deal_submit_info.user_brief,
	// 		deal_submit_info.file_size,
	// 	)?;
	// 	let miner: AccountOf<T> = account("miner1", 100, SEED);
	// 	let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
	// 	FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
	// 	FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
	// 	FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), vec![deal_submit_info.file_hash])?;

	// 	let front = deal_info.assigned_miner[0].fragment_list.len() as u128 * FRAGMENT_SIZE / IDLE_SEG_SIZE;

	// 	let pois_key = PoISKey {
	// 		g: [2u8; 256],
	// 		n: [3u8; 256],
	// 	};

	// 	let space_proof_info = SpaceProofInfo::<AccountOf<T>> {
	// 		miner: miner.clone(),
	// 		front: front as u64,
	// 		rear: 10000,
	// 		pois_key: pois_key.clone(),
	// 		accumulator: pois_key.g,
	// 	};
	// 	let encoding = space_proof_info.encode();

	// 	let tee_sig = [170, 163, 0, 251, 194, 156, 166, 134, 188, 113, 65, 133, 72, 173, 164, 190, 182, 110, 62, 65, 47, 115, 229, 46, 245, 235, 23, 77, 58, 49, 215, 226, 35, 68, 5, 214, 233, 134, 35, 63, 153, 28, 51, 207, 140, 112, 210, 165, 13, 67, 116, 36, 126, 115, 49, 201, 175, 69, 189, 161, 135, 174, 130, 14, 173, 140, 235, 151, 179, 51, 107, 145, 107, 84, 119, 155, 32, 228, 161, 242, 202, 127, 137, 153, 191, 78, 153, 242, 117, 150, 91, 213, 58, 20, 224, 171, 91, 237, 130, 53, 18, 23, 135, 155, 89, 199, 135, 154, 123, 166, 127, 61, 51, 24, 172, 245, 20, 195, 118, 35, 233, 126, 247, 219, 249, 167, 94, 222, 59, 84, 2, 3, 33, 125, 182, 9, 122, 3, 203, 5, 253, 118, 161, 179, 163, 254, 7, 95, 97, 56, 50, 47, 137, 155, 34, 250, 29, 121, 1, 101, 73, 41, 57, 34, 223, 26, 91, 174, 170, 23, 112, 228, 175, 243, 162, 244, 245, 107, 5, 247, 77, 155, 151, 36, 129, 56, 123, 55, 205, 223, 113, 62, 84, 40, 33, 1, 146, 58, 118, 252, 144, 245, 125, 210, 12, 210, 243, 16, 88, 22, 80, 0, 234, 146, 220, 88, 204, 50, 207, 186, 194, 167, 239, 222, 209, 74, 136, 77, 29, 16, 106, 214, 99, 90, 8, 85, 60, 216, 199, 213, 113, 243, 127, 155, 131, 11, 79, 227, 55, 70, 51, 78, 167, 105, 68, 106];
	// }: _(RawOrigin::Signed(miner.clone()), space_proof_info, tee_sig)
	// verify {
	// 	let space = PendingReplacements::<T>::get(&miner);
	// 	assert_eq!(space, 0);
	// }

	// delete_file {
	// 	let v in 0 ..30;
	// 	let caller: AccountOf<T> = account("user1", 100, SEED);
	// 	let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
	// 	for i in 0 .. 3 {
	// 		let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
	// 		add_idle_space::<T>(miner.clone())?;
	// 	}
	// 	buy_space::<T>(caller.clone())?;

	// 	let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
	// 	FileBank::<T>::upload_declaration(
	// 		RawOrigin::Signed(caller.clone()).into(),
	// 		deal_submit_info.file_hash.clone(),
	// 		deal_submit_info.segment_list,
	// 		deal_submit_info.user_brief,
	// 		deal_submit_info.file_size,
	// 	)?;

	// 	let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
	// 	FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
	// 	FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
	// 	FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
	// 	FileBank::<T>::calculate_end(RawOrigin::Root.into(), deal_submit_info.file_hash.clone());
	// }: _(RawOrigin::Signed(caller.clone()), caller.clone(), vec![deal_submit_info.file_hash.clone()])
	// verify {
	// 	assert!(!<File<T>>::contains_key(&deal_submit_info.file_hash.clone()));
	// }

	create_bucket {
		log::info!("start create_bucket");
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let name: Vec<u8> = "test-bucket1".as_bytes().to_vec();
		let name: BoundedVec<u8, T::NameStrLimit> = name.try_into().map_err(|_| "name convert error")?;
	}: _(RawOrigin::Signed(caller.clone()), caller.clone(), name.clone())
	verify {
		assert!(Bucket::<T>::contains_key(&caller, name));
	}

	delete_bucket {
		log::info!("start delete_bucket");
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let name: Vec<u8> = "test-bucket1".as_bytes().to_vec();
		let name_bound: BoundedVec<u8, T::NameStrLimit> = name.clone().try_into().map_err(|_| "bounded_vec convert err!")?;
		create_new_bucket::<T>(caller.clone(), name.clone())?;
		Bucket::<T>::contains_key(&caller, name_bound.clone());
	}: _(RawOrigin::Signed(caller.clone()), caller.clone(), name_bound.clone())
	verify {
		assert!(!Bucket::<T>::contains_key(&caller, name_bound));
	}
}



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
