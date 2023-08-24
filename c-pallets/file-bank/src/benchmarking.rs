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

benchmarks! {
	cert_idle_space {
		log::info!("start cert_idle_space");
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
		let miner = pallet_sminer::benchmarking::add_miner::<T>("miner1")?;

		let pois_key = PoISKey {
            g: [2u8; 256],
            n: [3u8; 256],
        };

		let space_proof_info = SpaceProofInfo::<AccountOf<T>> {
            miner: miner.clone(),
            front: u64::MIN,
            rear: 1000,
            pois_key: pois_key.clone(),
            accumulator: pois_key.g,
        };

		let original = space_proof_info.encode();
		let original = sp_io::hashing::sha2_256(&original);

		log::info!("original: {:?}", original);

		let tee_sig = [40, 184, 234, 22, 220, 221, 5, 31, 62, 245, 212, 216, 231, 188, 235, 150, 73, 189, 73, 38, 124, 227, 107, 163, 241, 3, 213, 25, 35, 245, 176, 145, 173, 18, 171, 18, 76, 103, 112, 64, 46, 19, 129, 212, 13, 42, 248, 218, 159, 38, 172, 121, 253, 252, 252, 181, 115, 110, 39, 122, 231, 249, 121, 121, 219, 96, 153, 180, 207, 193, 58, 248, 191, 255, 151, 223, 13, 179, 23, 153, 236, 234, 214, 254, 73, 142, 0, 56, 89, 245, 238, 26, 211, 246, 152, 12, 79, 2, 30, 21, 124, 168, 72, 100, 204, 7, 107, 71, 127, 209, 3, 55, 148, 240, 64, 143, 1, 190, 228, 24, 192, 46, 34, 254, 252, 105, 253, 95, 8, 143, 93, 177, 132, 73, 42, 182, 141, 226, 1, 167, 187, 47, 41, 176, 44, 140, 141, 150, 160, 246, 23, 251, 241, 23, 97, 58, 44, 11, 116, 116, 93, 182, 247, 58, 132, 44, 50, 224, 105, 39, 108, 84, 31, 113, 22, 35, 218, 163, 114, 240, 162, 80, 8, 183, 74, 181, 122, 132, 241, 196, 54, 61, 245, 108, 253, 45, 222, 211, 98, 147, 213, 53, 101, 169, 209, 139, 214, 64, 228, 10, 179, 204, 168, 231, 91, 23, 75, 38, 97, 4, 12, 78, 197, 166, 79, 40, 105, 162, 110, 251, 102, 158, 108, 223, 148, 208, 205, 226, 134, 29, 131, 135, 130, 152, 244, 177, 208, 44, 158, 17, 131, 249, 34, 74, 89, 216];

	}: _(RawOrigin::Signed(miner.clone()), space_proof_info, tee_sig)
	verify {
		let (idle, service) = T::MinerControl::get_power(&miner)?;
		assert_eq!(idle, 1000 * 256 * 1024 * 1024);
	}

	upload_declaration {
		let v in 1 .. 30;
		log::info!("start upload_declaration");
		// <pallet_rrsc::Pallet<T> as Hooks<T::BlockNumber>>::on_initialize(1u32.saturated_into());
		// <frame_system::Pallet<T>>::set_block_number(1u32.saturated_into());
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;

		for i in 0 .. v {
			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
			add_idle_space::<T>(miner.clone())?;
		}
		buy_space::<T>(caller.clone())?;
		let mut deal_info: BoundedVec<SegmentList<T>, T::SegmentCount> = Default::default();
		let file_name = "test-file".as_bytes().to_vec();
		let bucket_name = "test-bucket1".as_bytes().to_vec();
		let file_hash: Hash = Hash([4u8; 64]);
		let user_brief = UserBrief::<T>{
			user: caller.clone(),
			file_name: file_name.try_into().map_err(|_e| "file name convert err")?,
			bucket_name: bucket_name.try_into().map_err(|_e| "bucket name convert err")?,
		};
		let segment_list = SegmentList::<T> {
			hash: Hash([1u8; 64]),
			fragment_list: [
				Hash([2u8; 64]),
				Hash([3u8; 64]),
				Hash([4u8; 64]),
			].to_vec().try_into().unwrap(),
		};
		deal_info.try_push(segment_list).unwrap();
		let segment_list = SegmentList::<T> {
			hash: Hash([2u8; 64]),
			fragment_list: [
				Hash([2u8; 64]),
				Hash([3u8; 64]),
				Hash([4u8; 64]),
			].to_vec().try_into().unwrap(),
		};
		deal_info.try_push(segment_list).unwrap();
		let segment_list = SegmentList::<T> {
			hash: Hash([3u8; 64]),
			fragment_list: [
				Hash([2u8; 64]),
				Hash([3u8; 64]),
				Hash([4u8; 64]),
			].to_vec().try_into().unwrap(),
		};
		deal_info.try_push(segment_list).unwrap();
	}: _(RawOrigin::Signed(caller), file_hash.clone(), deal_info, user_brief, 123)
	verify {
		assert!(DealMap::<T>::contains_key(&file_hash));
	}

	upload_declaration_expected_max {
		let v in 1 .. 30;
		log::info!("start upload_declaration_expected_max");
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
		for i in 0 .. 30 {
			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
			add_idle_space::<T>(miner.clone())?;
			if i < 27 {
				pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
			}
		}
		buy_space::<T>(caller.clone())?;

		let deal_info = create_deal_info::<T>(caller.clone(), v)?;
	}: upload_declaration(RawOrigin::Signed(caller), deal_info.file_hash.clone(), deal_info.segment_list, deal_info.user_brief, deal_info.file_size)
	verify {
		assert!(DealMap::<T>::contains_key(&deal_info.file_hash));
	}

	transfer_report {
		let v in 1 .. 30;
		log::info!("start transfer_report");
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
		for i in 0 .. 15 {
			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
			add_idle_space::<T>(miner.clone())?;
			if i < 12 {
				pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
			}
		}
		buy_space::<T>(caller.clone())?;

		let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
		FileBank::<T>::upload_declaration(
			RawOrigin::Signed(caller).into(),
			deal_submit_info.file_hash.clone(),
			deal_submit_info.segment_list,
			deal_submit_info.user_brief,
			deal_submit_info.file_size,
		)?;

		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
	}: _(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()), vec![deal_submit_info.file_hash])
	verify {
		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
		assert_eq!(deal_info.complete_list.len(), 1);
	}

	transfer_report_last {
		let v in 1 .. 30;
		log::info!("start transfer_report_last");
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
		for i in 0 .. 15 {
			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
			add_idle_space::<T>(miner.clone())?;
			if i < 12 {
				pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
			}
		}
		buy_space::<T>(caller.clone())?;

		let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
		FileBank::<T>::upload_declaration(
			RawOrigin::Signed(caller).into(),
			deal_submit_info.file_hash.clone(),
			deal_submit_info.segment_list,
			deal_submit_info.user_brief,
			deal_submit_info.file_size,
		)?;

		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
	}: transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()), vec![deal_submit_info.file_hash])
	verify {
		assert!(File::<T>::contains_key(&deal_submit_info.file_hash));
	}

	upload_declaration_fly_upload {
		let v in 1 .. 30;
		log::info!("start upload_declaration_fly_upload");
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
		for i in 0 .. 15 {
			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
			add_idle_space::<T>(miner.clone())?;
			if i < 12 {
				pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
			}
		}
		buy_space::<T>(caller.clone())?;

		let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
		FileBank::<T>::upload_declaration(
			RawOrigin::Signed(caller).into(),
			deal_submit_info.file_hash.clone(),
			deal_submit_info.segment_list,
			deal_submit_info.user_brief,
			deal_submit_info.file_size,
		)?;

		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		let caller: AccountOf<T> = account("user2", 100, SEED);
		buy_space::<T>(caller.clone())?;
		let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
	}: upload_declaration(RawOrigin::Signed(caller.clone()), deal_submit_info.file_hash.clone(), deal_submit_info.segment_list, deal_submit_info.user_brief.clone(), deal_submit_info.file_size)
	verify {
		let file = File::<T>::get(deal_submit_info.file_hash.clone()).unwrap();
		assert!(file.owner.contains(&deal_submit_info.user_brief))
	}

	deal_reassign_miner {
		let v in 0 ..30;
		log::info!("start deal_reassign_miner");
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
		for i in 0 .. 30 {
			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
			add_idle_space::<T>(miner.clone())?;
			if i < 24 {
				pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
			}
		}
		buy_space::<T>(caller.clone())?;

		let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
		FileBank::<T>::upload_declaration(
			RawOrigin::Signed(caller.clone()).into(),
			deal_submit_info.file_hash.clone(),
			deal_submit_info.segment_list,
			deal_submit_info.user_brief,
			deal_submit_info.file_size,
		)?;

		<T as crate::Config>::FScheduler::cancel_named([4u8; 64].to_vec()).expect("cancel scheduler failed");
	}: _(RawOrigin::Root, deal_submit_info.file_hash.clone(), 1, 200)
	verify {
		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
		assert_eq!(deal_info.count, 1);
	}

	deal_reassign_miner_exceed_limit {
		let v in 0 ..30;
		log::info!("start deal_reassign_miner_exceed_limit");
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
		for i in 0 .. 30 {
			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
			add_idle_space::<T>(miner.clone())?;
			if i < 24 {
				pallet_sminer::benchmarking::freeze_miner::<T>(miner.clone())?;
			}
		}
		buy_space::<T>(caller.clone())?;

		let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
		FileBank::<T>::upload_declaration(
			RawOrigin::Signed(caller.clone()).into(),
			deal_submit_info.file_hash.clone(),
			deal_submit_info.segment_list,
			deal_submit_info.user_brief,
			deal_submit_info.file_size,
		)?;
	}: deal_reassign_miner(RawOrigin::Root, deal_submit_info.file_hash.clone(), 20, 200)
	verify {
		assert!(!DealMap::<T>::contains_key(&deal_submit_info.file_hash));
	}

	calculate_end {
		let v in 1 .. 30;
		log::info!("start calculate_end");
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
		for i in 0 .. 5 {
			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
			add_idle_space::<T>(miner.clone())?;
		}
		buy_space::<T>(caller.clone())?;

		let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
		FileBank::<T>::upload_declaration(
			RawOrigin::Signed(caller).into(),
			deal_submit_info.file_hash.clone(),
			deal_submit_info.segment_list,
			deal_submit_info.user_brief,
			deal_submit_info.file_size,
		)?;

		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		let file_info = File::<T>::get(&deal_submit_info.file_hash).unwrap();
	}: _(RawOrigin::Root, deal_submit_info.file_hash)
	verify {
		let file_info = File::<T>::get(&deal_submit_info.file_hash).unwrap();
		assert_eq!(FileState::Active, file_info.stat);
	}

	replace_idle_space {
		log::info!("start replace_idle_space");
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
		for i in 0 .. 3 {
			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
			add_idle_space::<T>(miner.clone())?;
		}
		buy_space::<T>(caller.clone())?;

		let deal_submit_info = create_deal_info::<T>(caller.clone(), 50)?;
		FileBank::<T>::upload_declaration(
			RawOrigin::Signed(caller).into(),
			deal_submit_info.file_hash.clone(),
			deal_submit_info.segment_list,
			deal_submit_info.user_brief,
			deal_submit_info.file_size,
		)?;
		let miner: AccountOf<T> = account("miner1", 100, SEED);
		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), vec![deal_submit_info.file_hash])?;

		let avail_replace_space = deal_info.assigned_miner[0].fragment_list.len() as u128 * FRAGMENT_SIZE;
		let front = avail_replace_space / IDLE_SEG_SIZE;
		log::info!("deal_info.assigned_miner[0].fragment_list.len(): {:?}", deal_info.assigned_miner[0].fragment_list.len());

		let pois_key = PoISKey {
			g: [2u8; 256],
			n: [3u8; 256],
		};

		let space_proof_info = SpaceProofInfo::<AccountOf<T>> {
			miner: miner.clone(),
			front: front as u64,
			rear: 10000,
			pois_key: pois_key.clone(),
			accumulator: pois_key.g,
		};
		let encoding = space_proof_info.encode();
		let hashing = sp_io::hashing::sha2_256(&encoding);
		log::info!("replace_idle_space hashing: {:?}", hashing);

		let tee_sig = [92, 125, 38, 132, 147, 27, 79, 14, 12, 32, 237, 138, 183, 67, 18, 196, 163, 224, 179, 213, 78, 70, 226, 36, 197, 160, 7, 149, 95, 96, 243, 188, 178, 75, 242, 32, 158, 203, 204, 181, 80, 196, 107, 101, 49, 89, 168, 7, 231, 211, 192, 201, 132, 153, 206, 153, 237, 214, 110, 81, 74, 200, 136, 178, 67, 212, 123, 180, 234, 183, 126, 170, 145, 219, 121, 3, 224, 246, 92, 232, 243, 176, 37, 95, 137, 249, 137, 73, 240, 113, 123, 84, 168, 2, 233, 251, 141, 19, 214, 124, 82, 219, 6, 75, 228, 97, 102, 72, 144, 84, 117, 250, 6, 97, 230, 2, 16, 204, 31, 159, 197, 153, 95, 107, 240, 31, 184, 58, 3, 239, 248, 191, 133, 1, 191, 199, 215, 55, 125, 234, 105, 151, 218, 50, 110, 35, 208, 129, 166, 151, 255, 3, 229, 23, 239, 241, 221, 53, 216, 155, 86, 140, 194, 115, 13, 255, 218, 122, 186, 2, 55, 125, 237, 151, 200, 119, 16, 13, 112, 130, 188, 151, 223, 152, 28, 20, 136, 119, 142, 8, 93, 55, 95, 203, 126, 47, 128, 230, 0, 121, 191, 177, 104, 65, 91, 32, 76, 71, 43, 79, 118, 98, 131, 91, 152, 132, 179, 161, 112, 184, 207, 44, 205, 145, 61, 233, 75, 133, 214, 226, 213, 144, 29, 4, 86, 94, 67, 139, 255, 93, 161, 23, 95, 140, 176, 165, 175, 108, 44, 163, 44, 125, 252, 229, 141, 181];
	}: _(RawOrigin::Signed(miner.clone()), space_proof_info, tee_sig)
	verify {
		let space = PendingReplacements::<T>::get(&miner);
		assert_eq!(space, avail_replace_space - (front * IDLE_SEG_SIZE));
	}

	delete_file {
		let v in 0 ..30;
		log::info!("start delete_file");
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
		for i in 0 .. 3 {
			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
			add_idle_space::<T>(miner.clone())?;
		}
		buy_space::<T>(caller.clone())?;

		let deal_submit_info = create_deal_info::<T>(caller.clone(), v)?;
		FileBank::<T>::upload_declaration(
			RawOrigin::Signed(caller.clone()).into(),
			deal_submit_info.file_hash.clone(),
			deal_submit_info.segment_list,
			deal_submit_info.user_brief,
			deal_submit_info.file_size,
		)?;

		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::calculate_end(RawOrigin::Root.into(), deal_submit_info.file_hash.clone());
	}: _(RawOrigin::Signed(caller.clone()), caller.clone(), vec![deal_submit_info.file_hash.clone()])
	verify {
		assert!(!<File<T>>::contains_key(&deal_submit_info.file_hash.clone()));
	}

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

	generate_restoral_order {
		log::info!("start generate_restoral_order");
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
		for i in 0 .. 4 {
			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
			add_idle_space::<T>(miner.clone())?;
		}
		buy_space::<T>(caller.clone())?;

		let deal_submit_info = create_deal_info::<T>(caller.clone(), 50)?;
		FileBank::<T>::upload_declaration(
			RawOrigin::Signed(caller.clone()).into(),
			deal_submit_info.file_hash.clone(),
			deal_submit_info.segment_list,
			deal_submit_info.user_brief,
			deal_submit_info.file_size,
		)?;

		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::calculate_end(RawOrigin::Root.into(), deal_submit_info.file_hash.clone());
	}: _(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()), deal_submit_info.file_hash, Hash([99u8; 64]))
	verify {
		assert!(<RestoralOrder<T>>::contains_key(&Hash([99u8; 64])))
	}

	claim_restoral_order {
		log::info!("start claim_restoral_order");
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
		for i in 0 .. 3 {
			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
			add_idle_space::<T>(miner.clone())?;
		}
		buy_space::<T>(caller.clone())?;

		let deal_submit_info = create_deal_info::<T>(caller.clone(), 50)?;
		FileBank::<T>::upload_declaration(
			RawOrigin::Signed(caller).into(),
			deal_submit_info.file_hash.clone(),
			deal_submit_info.segment_list,
			deal_submit_info.user_brief,
			deal_submit_info.file_size,
		)?;

		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::calculate_end(RawOrigin::Root.into(), deal_submit_info.file_hash)?;
		FileBank::<T>::generate_restoral_order(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), deal_submit_info.file_hash, Hash([99u8; 64]))?;
	}: _(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()), Hash([99u8; 64]))
	verify {
		let restoral_info = <RestoralOrder<T>>::try_get(&Hash([99u8; 64])).unwrap();
		assert_eq!(restoral_info.miner, deal_info.assigned_miner[1].miner);
	}

	restoral_order_complete {
		log::info!("start restoral_order_complete");
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
		for i in 0 .. 3 {
			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
			add_idle_space::<T>(miner.clone())?;
		}
		buy_space::<T>(caller.clone())?;

		let deal_submit_info = create_deal_info::<T>(caller.clone(), 50)?;
		FileBank::<T>::upload_declaration(
			RawOrigin::Signed(caller).into(),
			deal_submit_info.file_hash.clone(),
			deal_submit_info.segment_list,
			deal_submit_info.user_brief,
			deal_submit_info.file_size,
		)?;

		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::calculate_end(RawOrigin::Root.into(), deal_submit_info.file_hash)?;
		FileBank::<T>::generate_restoral_order(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), deal_submit_info.file_hash, Hash([99u8; 64]))?;
		frame_system::pallet::Pallet::<T>::set_block_number(2u32.saturated_into());
		FileBank::<T>::claim_restoral_order(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), Hash([99u8; 64]))?;
	}: _(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()), Hash([99u8; 64]))
	verify {
		assert!(!<RestoralOrder<T>>::contains_key(Hash([97u8; 64])))
	}

	claim_restoral_noexist_order {
		log::info!("start claim_restoral_noexist_order");
		let caller: AccountOf<T> = account("user1", 100, SEED);
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
		for i in 0 .. 3 {
			let miner = pallet_sminer::benchmarking::add_miner::<T>(miner_list[i as usize])?;
			add_idle_space::<T>(miner.clone())?;
		}
		buy_space::<T>(caller.clone())?;

		let deal_submit_info = create_deal_info::<T>(caller.clone(), 50)?;
		FileBank::<T>::upload_declaration(
			RawOrigin::Signed(caller).into(),
			deal_submit_info.file_hash.clone(),
			deal_submit_info.segment_list,
			deal_submit_info.user_brief,
			deal_submit_info.file_size,
		)?;

		let deal_info = DealMap::<T>::get(&deal_submit_info.file_hash).unwrap();
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[0].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::transfer_report(RawOrigin::Signed(deal_info.assigned_miner[2].miner.clone()).into(), vec![deal_submit_info.file_hash])?;
		FileBank::<T>::calculate_end(RawOrigin::Root.into(), deal_submit_info.file_hash)?;
		pallet_sminer::benchmarking::bench_miner_exit::<T>(deal_info.assigned_miner[2].miner.clone())?;
	}: _(RawOrigin::Signed(deal_info.assigned_miner[1].miner.clone()), deal_info.assigned_miner[2].miner.clone(), deal_submit_info.file_hash, Hash([99u8; 64]))
	verify {
		assert!(<RestoralOrder<T>>::contains_key(Hash([99u8; 64])))
	}
}