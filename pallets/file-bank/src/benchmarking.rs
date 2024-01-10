// use super::*;
// use crate::{Pallet as FileBank, *};
// use cp_cess_common::{Hash, IpAddress};
// use codec::{alloc::string::ToString, Decode};
// pub use frame_benchmarking::{
// 	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
// };
// use frame_support::{
// 	dispatch::UnfilteredDispatchable,
// 	pallet_prelude::*,
// 	traits::{Currency, CurrencyToVote, Get, Imbalance},
// };

// use pallet_cess_staking::{
// 	testing_utils, Config as StakingConfig, Pallet as Staking, RewardDestination,
// };
// use pallet_tee_worker::{Config as TeeWorkerConfig, Pallet as TeeWorker};
// use pallet_sminer::{Config as SminerConfig, Pallet as Sminer};
// use pallet_storage_handler::{Pallet as StorageHandler};
// use sp_runtime::{
// 	traits::{Bounded, One, StaticLookup, TrailingZeroInput, Zero},
// 	Perbill, Percent, Digest, DigestItem,
// };
// use sp_std::prelude::*;
// use scale_info::prelude::format;
// use frame_system::RawOrigin;
// use sp_runtime::traits::BlakeTwo256;
// use cessp_consensus_rrsc::{Slot, RRSC_ENGINE_ID};
// pub struct Pallet<T: Config>(FileBank<T>);
// pub trait Config:
// 	crate::Config + pallet_sminer::benchmarking::Config + pallet_storage_handler::Config + pallet_rrsc::Config + pallet_grandpa::Config + pallet_session::Config
// {
// }
// type SminerBalanceOf<T> = <<T as pallet_storage_handler::Config>::Currency as Currency<
// 	<T as frame_system::Config>::AccountId,
// >>::Balance;

// const SEED: u32 = 2190502;
// const miner_list: [&'static str; 30] = [
// 	"miner1", "miner2", "miner3", "miner4", "miner5", "miner6", "miner7", "miner8", "miner9", "miner10",
// 	"miner11", "miner12", "miner13", "miner14", "miner15", "miner16", "miner17", "miner18", "miner19", "miner20",
// 	"miner21", "miner22", "miner23", "miner24", "miner25", "miner26", "miner27", "miner28", "miner29", "miner30",
// ];
// // const MAX_SPANS: u32 = 100;
// pub struct DealSubmitInfo<T: Config> {
// 	file_hash: Hash,
// 	user_brief: UserBrief<T>,
// 	segment_list: BoundedVec<SegmentList<T>, T::SegmentCount>,
// 	file_size: u128,
// }

// pub fn add_idle_space<T: Config>(miner: T::AccountId) -> Result<(), &'static str> {
// 	let idle_space = <T as crate::Config>::MinerControl::add_miner_idle_space(
// 		&miner,
// 		[8u8; 256], 
// 		10000,
// 		[8u8; 256],
// 	).expect("add idle space failed, func add_miner_idle_space()");

// 	<T as crate::Config>::StorageHandle::add_total_idle_space(idle_space).expect("add idle space failed, func add_total_idle_space()");

// 	Ok(())
// }

// pub fn buy_space<T: Config>(user: T::AccountId) -> Result<(), &'static str> {
// 	<T as pallet_storage_handler::Config>::Currency::make_free_balance_be(
// 		&user,
// 		SminerBalanceOf::<T>::max_value(),
// 	);

// 	StorageHandler::<T>::buy_space(RawOrigin::Signed(user).into(), 10)?;

// 	Ok(())
// }

// pub fn create_deal_info<T: Config>(acc: AccountOf<T>, length: u32, hash_seed: u8) -> Result<DealSubmitInfo<T>, &'static str> {
// 	let mut deal_info: BoundedVec<SegmentList<T>, T::SegmentCount> = Default::default();
// 	let file_name = "test-file".as_bytes().to_vec();
// 	let bucket_name = "test-bucket1".as_bytes().to_vec();
// 	let file_hash: Hash = Hash([hash_seed; 64]);
// 	let user_brief = UserBrief::<T>{
// 		user: acc,
// 		file_name: file_name.try_into().map_err(|_e| "file name convert err")?,
// 		bucket_name: bucket_name.try_into().map_err(|_e| "bucket name convert err")?,
// 	};

// 	let mut seed = 0;
// 	for i in 0 .. length {
// 		let segment_list = SegmentList::<T> {
// 			hash: Hash([1u8; 64]),
// 			fragment_list: [
// 				Hash([97u8; 64]),
// 				Hash([98u8; 64]),
// 				Hash([99u8; 64]),
// 			].to_vec().try_into().unwrap(),
// 		};

// 		deal_info.try_push(segment_list).unwrap();
// 	}

// 	Ok(DealSubmitInfo::<T>{
// 		file_hash: file_hash,
// 		user_brief: user_brief,
// 		segment_list: deal_info,
// 		file_size: 123,
// 	})
// }

// pub fn create_new_bucket<T: Config>(caller: T::AccountId, name: Vec<u8>) -> Result<(), &'static str> {
// 	let name = name.try_into().map_err(|_| "create bucket convert error")?;
// 	FileBank::<T>::create_bucket(RawOrigin::Signed(caller.clone()).into(), caller, name)?;
// 	Ok(())
// }

// benchmarks! {
// 	cert_idle_space {
// 		log::info!("start cert_idle_space");
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		let miner = pallet_sminer::benchmarking::add_miner::<T>("miner1")?;

// 		let pois_key = PoISKey {
//             g: [2u8; 256],
//             n: [3u8; 256],
//         };

// 		let space_proof_info = SpaceProofInfo::<AccountOf<T>> {
//             miner: miner.clone(),
//             front: u64::MIN,
//             rear: 1000,
//             pois_key: pois_key.clone(),
//             accumulator: pois_key.g,
//         };

// 		let original = space_proof_info.encode();
// 		let original = sp_io::hashing::sha2_256(&original);

// 		log::info!("original: {:?}", original);

// 		let tee_sig = [19, 66, 60, 64, 120, 225, 105, 244, 158, 246, 90, 112, 6, 239, 63, 152, 116, 41, 57, 4, 4, 217, 206, 100, 138, 168, 193, 247, 62, 155, 76, 164, 179, 117, 202, 77, 182, 109, 223, 103, 50, 197, 214, 153, 233, 158, 124, 138, 64, 252, 54, 62, 250, 128, 177, 239, 248, 98, 31, 77, 208, 8, 91, 241, 66, 205, 89, 240, 211, 84, 250, 194, 206, 106, 35, 25, 154, 82, 174, 156, 14, 10, 201, 1, 45, 70, 50, 89, 202, 96, 42, 93, 227, 71, 119, 211, 11, 210, 192, 113, 249, 221, 217, 116, 181, 116, 206, 56, 43, 238, 147, 157, 38, 95, 79, 73, 221, 41, 87, 203, 162, 172, 32, 9, 173, 190, 225, 160, 118, 34, 77, 121, 73, 79, 199, 66, 32, 131, 174, 158, 210, 97, 172, 72, 251, 183, 247, 74, 207, 46, 201, 97, 90, 171, 251, 245, 202, 155, 8, 168, 199, 206, 64, 97, 66, 215, 6, 251, 111, 32, 37, 56, 60, 178, 2, 155, 167, 207, 224, 56, 20, 196, 162, 9, 188, 137, 225, 48, 117, 217, 229, 183, 122, 157, 93, 12, 142, 131, 239, 44, 12, 157, 55, 246, 59, 22, 216, 112, 84, 42, 54, 226, 99, 46, 246, 5, 110, 15, 122, 247, 46, 244, 132, 195, 60, 54, 98, 232, 53, 162, 82, 107, 134, 218, 70, 71, 148, 75, 31, 65, 33, 193, 221, 20, 124, 128, 42, 9, 116, 181, 125, 77, 33, 202, 152, 179];

// 	}: _(RawOrigin::Signed(miner.clone()), space_proof_info, tee_sig)
// 	verify {
// 		let (idle, service) = T::MinerControl::get_power(&miner)?;
// 		assert_eq!(idle, 1000 * IDLE_SEG_SIZE);
// 	}

// 	upload_declaration {
// 		let v in 1 .. 30;
// 		log::info!("start upload_declaration");
// 		// <pallet_rrsc::Pallet<T> as Hooks<T::BlockNumber>>::on_initialize(1u32.saturated_into());
// 		// <frame_system::Pallet<T>>::set_block_number(1u32.saturated_into());
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;

// 		for i in 0 .. v {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
// 			add_idle_space::<T>(miner.clone())?;
// 		}
// 		buy_space::<T>(caller.clone())?;
// 		let mut deal_info: BoundedVec<SegmentList<T>, T::SegmentCount> = Default::default();
// 		let file_name = "test-file".as_bytes().to_vec();
// 		let bucket_name = "test-bucket1".as_bytes().to_vec();
// 		let file_hash: Hash = Hash([4u8; 64]);
// 		let user_brief = UserBrief::<T>{
// 			user: caller.clone(),
// 			file_name: file_name.try_into().map_err(|_e| "file name convert err")?,
// 			bucket_name: bucket_name.try_into().map_err(|_e| "bucket name convert err")?,
// 		};
// 		let segment_list = SegmentList::<T> {
// 			hash: Hash([1u8; 64]),
// 			fragment_list: [
// 				Hash([2u8; 64]),
// 				Hash([3u8; 64]),
// 				Hash([4u8; 64]),
// 			].to_vec().try_into().unwrap(),
// 		};
// 		deal_info.try_push(segment_list).unwrap();
// 		let segment_list = SegmentList::<T> {
// 			hash: Hash([2u8; 64]),
// 			fragment_list: [
// 				Hash([2u8; 64]),
// 				Hash([3u8; 64]),
// 				Hash([4u8; 64]),
// 			].to_vec().try_into().unwrap(),
// 		};
// 		deal_info.try_push(segment_list).unwrap();
// 		let segment_list = SegmentList::<T> {
// 			hash: Hash([3u8; 64]),
// 			fragment_list: [
// 				Hash([2u8; 64]),
// 				Hash([3u8; 64]),
// 				Hash([4u8; 64]),
// 			].to_vec().try_into().unwrap(),
// 		};
// 		deal_info.try_push(segment_list).unwrap();
// 	}: _(RawOrigin::Signed(caller), file_hash.clone(), deal_info, user_brief, 123)
// 	verify {
// 		assert!(DealMap::<T>::contains_key(&file_hash));
// 	}

// 	upload_declaration_expected_max {
// 		let v in 1 .. 30;
// 		log::info!("start upload_declaration_expected_max");
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 30 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
// 			add_idle_space::<T>(miner.clone())?;
// 			if i < 27 {
// 				pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
// 			}
// 		}
// 		buy_space::<T>(caller.clone())?;

// 		let deal_info = create_deal_info::<T>(caller.clone(), v, 4)?;
// 	}: upload_declaration(RawOrigin::Signed(caller), deal_info.file_hash.clone(), deal_info.segment_list, deal_info.user_brief, deal_info.file_size)
// 	verify {
// 		assert!(DealMap::<T>::contains_key(&deal_info.file_hash));
// 	}

// 	transfer_report {
// 		let v in 1 .. 30;
// 		log::info!("start transfer_report");
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 3 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
// 			add_idle_space::<T>(miner.clone())?;
// 		}
// 		buy_space::<T>(caller.clone())?;

// 		let deal_submit_info = create_deal_info::<T>(caller.clone(), v, 4)?;
// 		FileBank::<T>::upload_declaration(
// 			RawOrigin::Signed(caller).into(),
// 			deal_submit_info.file_hash.clone(),
// 			deal_submit_info.segment_list,
// 			deal_submit_info.user_brief,
// 			deal_submit_info.file_size,
// 		)?;
// 		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();

// 	}: _(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()), deal_submit_info.file_hash)
// 	verify {
// 		let deal_info = DealMap::<T>::get(deal_submit_info.file_hash).unwrap();
// 		assert_eq!(deal_info.complete_list.len(), 1);
// 	}

// 	transfer_report_last {
// 		let v in 1 .. 30;
// 		log::info!("start transfer_report_last");
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 15 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
// 			add_idle_space::<T>(miner.clone())?;
// 			if i < 12 {
// 				pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
// 			}
// 		}
// 		buy_space::<T>(caller.clone())?;

// 		let deal_submit_info = create_deal_info::<T>(caller.clone(), v, 4)?;
// 		FileBank::<T>::upload_declaration(
// 			RawOrigin::Signed(caller).into(),
// 			deal_submit_info.file_hash.clone(),
// 			deal_submit_info.segment_list,
// 			deal_submit_info.user_brief,
// 			deal_submit_info.file_size,
// 		)?;

// 		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), deal_submit_info.file_hash)?;
// 	}: transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()), deal_submit_info.file_hash)
// 	verify {
// 		assert!(File::<T>::contains_key(&deal_submit_info.file_hash));
// 	}

// 	upload_declaration_fly_upload {
// 		let v in 1 .. 30;
// 		log::info!("start upload_declaration_fly_upload");
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 15 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
// 			add_idle_space::<T>(miner.clone())?;
// 			if i < 12 {
// 				pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
// 			}
// 		}
// 		buy_space::<T>(caller.clone())?;

// 		let deal_submit_info = create_deal_info::<T>(caller.clone(), v, 4)?;
// 		FileBank::<T>::upload_declaration(
// 			RawOrigin::Signed(caller).into(),
// 			deal_submit_info.file_hash.clone(),
// 			deal_submit_info.segment_list,
// 			deal_submit_info.user_brief,
// 			deal_submit_info.file_size,
// 		)?;

// 		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		let caller: AccountOf<T> = account("user2", 100, SEED);
// 		buy_space::<T>(caller.clone())?;
// 		let deal_submit_info = create_deal_info::<T>(caller.clone(), v, 4)?;
// 	}: upload_declaration(RawOrigin::Signed(caller.clone()), deal_submit_info.file_hash.clone(), deal_submit_info.segment_list, deal_submit_info.user_brief.clone(), deal_submit_info.file_size)
// 	verify {
// 		let file = File::<T>::get(deal_submit_info.file_hash.clone()).unwrap();
// 		assert!(file.owner.contains(&deal_submit_info.user_brief))
// 	}

// 	deal_reassign_miner {
// 		let v in 0 ..30;
// 		log::info!("start deal_reassign_miner");
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 30 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
// 			add_idle_space::<T>(miner.clone())?;
// 			if i < 24 {
// 				pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
// 			}
// 		}
// 		buy_space::<T>(caller.clone())?;

// 		let deal_submit_info = create_deal_info::<T>(caller.clone(), v, 4)?;
// 		FileBank::<T>::upload_declaration(
// 			RawOrigin::Signed(caller.clone()).into(),
// 			deal_submit_info.file_hash.clone(),
// 			deal_submit_info.segment_list,
// 			deal_submit_info.user_brief,
// 			deal_submit_info.file_size,
// 		)?;

// 		<T as crate::Config>::FScheduler::cancel_named([4u8; 64].to_vec()).expect("cancel scheduler failed");
// 	}: _(RawOrigin::Root, deal_submit_info.file_hash.clone(), 1, 200)
// 	verify {
// 		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
// 		assert_eq!(deal_info.count, 1);
// 	}

// 	deal_reassign_miner_exceed_limit {
// 		let v in 0 ..30;
// 		log::info!("start deal_reassign_miner_exceed_limit");
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 30 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
// 			add_idle_space::<T>(miner.clone())?;
// 			if i < 24 {
// 				pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
// 			}
// 		}
// 		buy_space::<T>(caller.clone())?;

// 		let deal_submit_info = create_deal_info::<T>(caller.clone(), v, 4)?;
// 		FileBank::<T>::upload_declaration(
// 			RawOrigin::Signed(caller.clone()).into(),
// 			deal_submit_info.file_hash.clone(),
// 			deal_submit_info.segment_list,
// 			deal_submit_info.user_brief,
// 			deal_submit_info.file_size,
// 		)?;
// 	}: deal_reassign_miner(RawOrigin::Root, deal_submit_info.file_hash.clone(), 20, 200)
// 	verify {
// 		assert!(!DealMap::<T>::contains_key(&deal_submit_info.file_hash));
// 	}

// 	calculate_end {
// 		let v in 1 .. 30;
// 		log::info!("start calculate_end");
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 5 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
// 			add_idle_space::<T>(miner.clone())?;
// 		}
// 		buy_space::<T>(caller.clone())?;

// 		let deal_submit_info = create_deal_info::<T>(caller.clone(), v, 4)?;
// 		FileBank::<T>::upload_declaration(
// 			RawOrigin::Signed(caller).into(),
// 			deal_submit_info.file_hash.clone(),
// 			deal_submit_info.segment_list,
// 			deal_submit_info.user_brief,
// 			deal_submit_info.file_size,
// 		)?;

// 		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		let file_info = File::<T>::get(&deal_submit_info.file_hash).unwrap();
// 	}: _(RawOrigin::Root, deal_submit_info.file_hash)
// 	verify {
// 		let file_info = File::<T>::get(&deal_submit_info.file_hash).unwrap();
// 		assert_eq!(FileState::Active, file_info.stat);
// 	}

// 	replace_idle_space {
// 		log::info!("start replace_idle_space");
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 3 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
// 			add_idle_space::<T>(miner.clone())?;
// 		}
// 		buy_space::<T>(caller.clone())?;

// 		let deal_submit_info = create_deal_info::<T>(caller.clone(), 50, 4)?;
// 		FileBank::<T>::upload_declaration(
// 			RawOrigin::Signed(caller).into(),
// 			deal_submit_info.file_hash.clone(),
// 			deal_submit_info.segment_list,
// 			deal_submit_info.user_brief,
// 			deal_submit_info.file_size,
// 		)?;
// 		let miner: AccountOf<T> = account("miner1", 100, SEED);
// 		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), deal_submit_info.file_hash)?;

// 		let avail_replace_space = deal_info.assigned_miner[0].fragment_list.len() as u128 * FRAGMENT_SIZE;
// 		let front = avail_replace_space / IDLE_SEG_SIZE;

// 		let pois_key = PoISKey {
// 			g: [2u8; 256],
// 			n: [3u8; 256],
// 		};

// 		let space_proof_info = SpaceProofInfo::<AccountOf<T>> {
// 			miner: miner.clone(),
// 			front: front as u64,
// 			rear: 10000,
// 			pois_key: pois_key.clone(),
// 			accumulator: pois_key.g,
// 		};
// 		let encoding = space_proof_info.encode();
// 		let hashing = sp_io::hashing::sha2_256(&encoding);
// 		log::info!("replace_idle_space hashing: {:?}", hashing);

// 		let tee_sig = [11, 155, 78, 85, 105, 233, 219, 127, 241, 166, 154, 21, 185, 76, 67, 196, 13, 233, 94, 42, 83, 110, 93, 124, 132, 244, 5, 233, 254, 238, 160, 73, 24, 130, 215, 13, 120, 203, 172, 10, 186, 82, 142, 3, 51, 151, 242, 175, 14, 77, 93, 167, 181, 105, 32, 108, 16, 44, 185, 159, 53, 38, 102, 134, 81, 3, 57, 100, 32, 117, 35, 157, 78, 215, 88, 41, 201, 102, 63, 28, 190, 206, 116, 140, 101, 3, 136, 177, 201, 188, 32, 56, 195, 36, 217, 238, 111, 54, 119, 70, 199, 55, 183, 252, 210, 204, 33, 29, 88, 253, 15, 29, 194, 18, 93, 170, 144, 25, 66, 120, 210, 253, 203, 236, 74, 191, 234, 84, 130, 164, 22, 142, 251, 105, 178, 69, 80, 111, 115, 54, 71, 67, 48, 215, 5, 132, 234, 26, 207, 102, 18, 62, 185, 184, 78, 9, 159, 158, 180, 78, 78, 242, 219, 229, 234, 238, 211, 103, 158, 132, 62, 219, 34, 242, 32, 254, 104, 136, 114, 210, 180, 52, 17, 17, 0, 119, 35, 129, 25, 175, 99, 37, 55, 195, 159, 157, 120, 82, 6, 38, 47, 105, 111, 114, 51, 47, 170, 123, 196, 78, 98, 128, 220, 246, 159, 237, 101, 111, 213, 219, 5, 177, 76, 150, 22, 34, 82, 59, 70, 204, 219, 38, 78, 50, 179, 36, 93, 132, 84, 24, 69, 176, 93, 230, 11, 1, 140, 53, 108, 72, 107, 200, 238, 197, 28, 59];
// 	}: _(RawOrigin::Signed(miner.clone()), space_proof_info, tee_sig)
// 	verify {
// 		let space = PendingReplacements::<T>::get(&miner);
// 		assert_eq!(space, avail_replace_space - (front * IDLE_SEG_SIZE));
// 	}

// 	delete_file {
// 		let v in 0 .. 30;
// 		log::info!("start delete_file");
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 3 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
// 			add_idle_space::<T>(miner.clone())?;
// 		}
// 		buy_space::<T>(caller.clone())?;

// 		let deal_submit_info = create_deal_info::<T>(caller.clone(), v, 4)?;
// 		FileBank::<T>::upload_declaration(
// 			RawOrigin::Signed(caller.clone()).into(),
// 			deal_submit_info.file_hash.clone(),
// 			deal_submit_info.segment_list,
// 			deal_submit_info.user_brief,
// 			deal_submit_info.file_size,
// 		)?;

// 		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::calculate_end(RawOrigin::Root.into(), deal_submit_info.file_hash.clone());
// 	}: _(RawOrigin::Signed(caller.clone()), caller.clone(), vec![deal_submit_info.file_hash.clone()])
// 	verify {
// 		assert!(!<File<T>>::contains_key(&deal_submit_info.file_hash.clone()));
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

// 	generate_restoral_order {
// 		log::info!("start generate_restoral_order");
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 4 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
// 			add_idle_space::<T>(miner.clone())?;
// 		}
// 		buy_space::<T>(caller.clone())?;

// 		let deal_submit_info = create_deal_info::<T>(caller.clone(), 50, 4)?;
// 		FileBank::<T>::upload_declaration(
// 			RawOrigin::Signed(caller.clone()).into(),
// 			deal_submit_info.file_hash.clone(),
// 			deal_submit_info.segment_list,
// 			deal_submit_info.user_brief,
// 			deal_submit_info.file_size,
// 		)?;

// 		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::calculate_end(RawOrigin::Root.into(), deal_submit_info.file_hash.clone());
// 	}: _(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()), deal_submit_info.file_hash, Hash([99u8; 64]))
// 	verify {
// 		assert!(<RestoralOrder<T>>::contains_key(&Hash([99u8; 64])))
// 	}

// 	claim_restoral_order {
// 		log::info!("start claim_restoral_order");
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 3 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
// 			add_idle_space::<T>(miner.clone())?;
// 		}
// 		buy_space::<T>(caller.clone())?;

// 		let deal_submit_info = create_deal_info::<T>(caller.clone(), 50, 4)?;
// 		FileBank::<T>::upload_declaration(
// 			RawOrigin::Signed(caller).into(),
// 			deal_submit_info.file_hash.clone(),
// 			deal_submit_info.segment_list,
// 			deal_submit_info.user_brief,
// 			deal_submit_info.file_size,
// 		)?;

// 		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::calculate_end(RawOrigin::Root.into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::generate_restoral_order(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), deal_submit_info.file_hash, Hash([99u8; 64]))?;
// 	}: _(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()), Hash([99u8; 64]))
// 	verify {
// 		let restoral_info = <RestoralOrder<T>>::try_get(&Hash([99u8; 64])).unwrap();
// 		assert_eq!(restoral_info.miner, deal_info.assigned_miner[1].miner);
// 	}

// 	restoral_order_complete {
// 		log::info!("start restoral_order_complete");
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 3 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
// 			add_idle_space::<T>(miner.clone())?;
// 		}
// 		buy_space::<T>(caller.clone())?;

// 		let deal_submit_info = create_deal_info::<T>(caller.clone(), 50, 4)?;
// 		FileBank::<T>::upload_declaration(
// 			RawOrigin::Signed(caller).into(),
// 			deal_submit_info.file_hash.clone(),
// 			deal_submit_info.segment_list,
// 			deal_submit_info.user_brief,
// 			deal_submit_info.file_size,
// 		)?;

// 		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::calculate_end(RawOrigin::Root.into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::generate_restoral_order(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), deal_submit_info.file_hash, Hash([99u8; 64]))?;
// 		frame_system::pallet::Pallet::<T>::set_block_number(2u32.saturated_into());
// 		FileBank::<T>::claim_restoral_order(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), Hash([99u8; 64]))?;
// 	}: _(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()), Hash([99u8; 64]))
// 	verify {
// 		assert!(!<RestoralOrder<T>>::contains_key(Hash([97u8; 64])))
// 	}

// 	claim_restoral_noexist_order {
// 		log::info!("start claim_restoral_noexist_order");
// 		let caller: AccountOf<T> = account("user1", 100, SEED);
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 3 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
// 			add_idle_space::<T>(miner.clone())?;
// 		}
// 		buy_space::<T>(caller.clone())?;

// 		let deal_submit_info = create_deal_info::<T>(caller.clone(), 50, 4)?;
// 		FileBank::<T>::upload_declaration(
// 			RawOrigin::Signed(caller).into(),
// 			deal_submit_info.file_hash.clone(),
// 			deal_submit_info.segment_list,
// 			deal_submit_info.user_brief,
// 			deal_submit_info.file_size,
// 		)?;

// 		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), deal_submit_info.file_hash)?;
// 		FileBank::<T>::calculate_end(RawOrigin::Root.into(), deal_submit_info.file_hash)?;
// 		pallet_sminer::benchmarking::bench_miner_exit::<T>(deal_info.assigned_miner[2].miner.clone())?;
// 	}: _(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()), deal_info.assigned_miner[2].miner.clone(), deal_submit_info.file_hash, Hash([99u8; 64]))
// 	verify {
// 		assert!(<RestoralOrder<T>>::contains_key(Hash([99u8; 64])))
// 	}
// }