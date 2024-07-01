use super::*;
use crate::{Pallet as FileBank, *};
// use cp_cess_common::{Hash, IpAddress};
// use codec::{alloc::string::ToString, Decode};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_support::traits::Currency;
// use frame_support::{
// 	dispatch::UnfilteredDispatchable,
// 	pallet_prelude::*,
// 	traits::{Currency, CurrencyToVote, Get, Imbalance},
// };

// use pallet_cess_staking::{
// 	testing_utils, Config as StakingConfig, Pallet as Staking, RewardDestination,
// };
// use pallet_tee_worker::{Config as TeeWorkerConfig, Pallet as TeeWorker};
use pallet_sminer::{Config as SminerConfig, Pallet as Sminer};
// use pallet_storage_handler::{Pallet as StorageHandler};
// use sp_runtime::{
// 	traits::{Bounded, One, StaticLookup, TrailingZeroInput, Zero},
// 	Perbill, Percent, Digest, DigestItem,
// };
// use sp_std::prelude::*;
// use scale_info::prelude::format;
use frame_system::RawOrigin;
// use sp_runtime::traits::BlakeTwo256;
// use cessp_consensus_rrsc::{Slot, RRSC_ENGINE_ID};
pub struct Pallet<T: Config>(FileBank<T>);
pub trait Config:
	crate::Config + pallet_sminer::benchmarking::Config + pallet_storage_handler::Config + pallet_tee_worker::Config
{
}
// type SminerBalanceOf<T> = <<T as pallet_storage_handler::Config>::Currency as Currency<
// 	<T as frame_system::Config>::AccountId,
// >>::Balance;

const SEED: u32 = 2190502;
const miner_list: [&'static str; 30] = [
	"miner1", "miner2", "miner3", "miner4", "miner5", "miner6", "miner7", "miner8", "miner9", "miner10",
	"miner11", "miner12", "miner13", "miner14", "miner15", "miner16", "miner17", "miner18", "miner19", "miner20",
	"miner21", "miner22", "miner23", "miner24", "miner25", "miner26", "miner27", "miner28", "miner29", "miner30",
];
// // const MAX_SPANS: u32 = 100;
// pub struct DealSubmitInfo<T: Config> {
// 	file_hash: Hash,
// 	user_brief: UserBrief<T>,
// 	segment_list: BoundedVec<SegmentList<T>, T::SegmentCount>,
// 	file_size: u128,
// }

pub fn cert_idle_for_miner<T: Config>(miner: T::AccountId) -> Result<(), &'static str> {
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

	let tee_puk = pallet_tee_worker::benchmarking::get_pubkey::<T>();
	let tee_puk_encode = tee_puk.encode();
	let idle_sig_info_encode = space_proof_info.encode();
	let mut original = Vec::new();
	original.extend_from_slice(&idle_sig_info_encode);
	original.extend_from_slice(&tee_puk_encode);
	let original = sp_io::hashing::sha2_256(&original);
	let sig = pallet_tee_worker::benchmarking::sign_message::<T>(&original);
	let sig: BoundedVec<u8, ConstU32<64>> = sig.0.to_vec().try_into().map_err(|_| "bounded convert error")?;

	FileBank::<T>::cert_idle_space(RawOrigin::Signed(miner.clone()).into(), space_proof_info, sig.clone(), sig, tee_puk)?;

	Ok(())
}

pub fn buy_space<T: Config>(user: T::AccountId) -> Result<(), &'static str> {
	let territory_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
    <T as pallet_sminer::Config>::Currency::make_free_balance_be(
		&user, 
		365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!"),
	);

	pallet_storage_handler::Pallet::<T>::mint_territory(RawOrigin::Signed(user).into(), 10, territory_name)?;

	Ok(())
}

pub fn initialize_file_from_scratch<T: Config>() -> Result<(), &'static str> {
	pallet_tee_worker::benchmarking::generate_workers::<T>();
	let user: AccountOf<T> = account("user1", 100, SEED);
	let mut positive_miner: Vec<AccountOf<T>> = Default::default();
	for i in 0 .. 12 {
		let miner: AccountOf<T> = account(miner_list[i as usize], 100, SEED);
		positive_miner.push(miner.clone());
		let _ = pallet_sminer::benchmarking::register_positive_miner::<T>(miner.clone())?;
		let _ = cert_idle_for_miner::<T>(miner)?;
	}

	let _ = buy_space::<T>(user.clone())?;
	
	let file_name = "test-file".as_bytes().to_vec();
	let bucket_name = "test-bucket1".as_bytes().to_vec();
	let file_hash: Hash = Hash([80u8; 64]);
	let file_size: u128 = SEGMENT_SIZE * 3;
	let territory_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
	let user_brief = UserBrief::<T> {
		user: user.clone(),
		file_name: file_name.try_into().map_err(|_e| "file name convert err")?,
		bucket_name: bucket_name.try_into().map_err(|_e| "bucket name convert err")?,
		territory_name,
	};

	let mut deal_info: BoundedVec<SegmentList<T>, T::SegmentCount> = Default::default();
	let segment_list = SegmentList::<T> {
		hash: Hash([65u8; 64]),
		fragment_list: [
			Hash([97u8; 64]),
			Hash([97u8; 64]),
			Hash([97u8; 64]),
			Hash([97u8; 64]),
			Hash([97u8; 64]),
			Hash([97u8; 64]),
			Hash([97u8; 64]),
			Hash([97u8; 64]),
			Hash([97u8; 64]),
			Hash([97u8; 64]),
			Hash([97u8; 64]),
			Hash([97u8; 64]),
		].to_vec().try_into().unwrap(),
	};
	deal_info.try_push(segment_list).unwrap();
	FileBank::<T>::upload_declaration(RawOrigin::Signed(user.clone()).into(), file_hash.clone(), deal_info, user_brief, file_size)?;

	for i in 0 .. 12 {
		FileBank::<T>::transfer_report(RawOrigin::Signed(positive_miner[i as usize].clone()).into(), i + 1, file_hash.clone())?;
	}

	Ok(())
}

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

benchmarks! {
	cert_idle_space {
    	log::info!("start cert_idle_space");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
		let miner: AccountOf<T> = account("miner1", 100, SEED);
		let _ = pallet_sminer::benchmarking::register_positive_miner::<T>(miner.clone())?;

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

		let tee_puk = pallet_tee_worker::benchmarking::get_pubkey::<T>();
		let tee_puk_encode = tee_puk.encode();
		let idle_sig_info_encode = space_proof_info.encode();
		let mut original = Vec::new();
		original.extend_from_slice(&idle_sig_info_encode);
		original.extend_from_slice(&tee_puk_encode);
		let original = sp_io::hashing::sha2_256(&original);
        let sig = pallet_tee_worker::benchmarking::sign_message::<T>(&original);
        let sig: BoundedVec<u8, ConstU32<64>> = sig.0.to_vec().try_into().map_err(|_| "bounded convert error")?;
	}: _(RawOrigin::Signed(miner.clone()), space_proof_info, sig.clone(), sig, tee_puk)
	verify {
		let (idle, service) = T::MinerControl::get_power(&miner)?;
		assert_eq!(idle, 1000 * IDLE_SEG_SIZE);
	}

	upload_declaration {
		let v in 1 .. 30;
		log::info!("start upload_declaration");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
		let user: AccountOf<T> = account("user1", 100, SEED);
		let miner: AccountOf<T> = account("miner1", 100, SEED);
		let _ = pallet_sminer::benchmarking::register_positive_miner::<T>(miner.clone())?;
		let _ = cert_idle_for_miner::<T>(miner)?;
		let _ = buy_space::<T>(user.clone())?;

		let file_name = "test-file".as_bytes().to_vec();
		let bucket_name = "test-bucket1".as_bytes().to_vec();
		let file_hash: Hash = Hash([80u8; 64]);
		let file_size: u128 = SEGMENT_SIZE * 3;
		let territory_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
		let user_brief = UserBrief::<T> {
			user: user.clone(),
			file_name: file_name.try_into().map_err(|_e| "file name convert err")?,
			bucket_name: bucket_name.try_into().map_err(|_e| "bucket name convert err")?,
			territory_name,
		};
		
		let mut deal_info: BoundedVec<SegmentList<T>, T::SegmentCount> = Default::default();
		for i in 0 .. v {
			let segment_list = SegmentList::<T> {
				hash: Hash([65u8; 64]),
				fragment_list: [
					Hash([66u8; 64]),
					Hash([67u8; 64]),
					Hash([68u8; 64]),
					Hash([69u8; 64]),
					Hash([70u8; 64]),
					Hash([71u8; 64]),
					Hash([72u8; 64]),
					Hash([73u8; 64]),
					Hash([74u8; 64]),
					Hash([75u8; 64]),
					Hash([76u8; 64]),
					Hash([77u8; 64]),
				].to_vec().try_into().unwrap(),
			};
			deal_info.try_push(segment_list).unwrap();
		}
	}: _(RawOrigin::Signed(user), file_hash.clone(), deal_info, user_brief, file_size)
	verify {
		assert!(DealMap::<T>::contains_key(&file_hash));
	}

	transfer_report {
		let v in 1 .. 30;
		log::info!("start transfer_report");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
		let user: AccountOf<T> = account("user1", 100, SEED);
		let mut positive_miner: Vec<AccountOf<T>> = Default::default();
		for i in 0 .. 12 {
			let miner: AccountOf<T> = account(miner_list[i as usize], 100, SEED);
			positive_miner.push(miner.clone());
			let _ = pallet_sminer::benchmarking::register_positive_miner::<T>(miner.clone())?;
			let _ = cert_idle_for_miner::<T>(miner)?;
		}

		let _ = buy_space::<T>(user.clone())?;
		
		let file_name = "test-file".as_bytes().to_vec();
		let bucket_name = "test-bucket1".as_bytes().to_vec();
		let file_hash: Hash = Hash([80u8; 64]);
		let file_size: u128 = SEGMENT_SIZE * 3;
		let territory_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
		let user_brief = UserBrief::<T> {
			user: user.clone(),
			file_name: file_name.try_into().map_err(|_e| "file name convert err")?,
			bucket_name: bucket_name.try_into().map_err(|_e| "bucket name convert err")?,
			territory_name,
		};
		
		let mut deal_info: BoundedVec<SegmentList<T>, T::SegmentCount> = Default::default();
		for i in 0 .. v {
			let segment_list = SegmentList::<T> {
				hash: Hash([65u8; 64]),
				fragment_list: [
					Hash([66u8; 64]),
					Hash([67u8; 64]),
					Hash([68u8; 64]),
					Hash([69u8; 64]),
					Hash([70u8; 64]),
					Hash([71u8; 64]),
					Hash([72u8; 64]),
					Hash([73u8; 64]),
					Hash([74u8; 64]),
					Hash([75u8; 64]),
					Hash([76u8; 64]),
					Hash([77u8; 64]),
				].to_vec().try_into().unwrap(),
			};
			deal_info.try_push(segment_list).unwrap();
		}

		FileBank::<T>::upload_declaration(RawOrigin::Signed(user).into(), file_hash.clone(), deal_info, user_brief, file_size)?;

		for i in 0 .. 11 {
			FileBank::<T>::transfer_report(RawOrigin::Signed(positive_miner[i as usize].clone()).into(), i + 1, file_hash.clone())?;
		}
	}: _(RawOrigin::Signed(positive_miner[11].clone()), 12, file_hash.clone())
	verify {
		assert!(<File<T>>::contains_key(&file_hash));
	}

	calculate_report {
		log::info!("start calculate_report");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
		let user: AccountOf<T> = account("user1", 100, SEED);
		let mut positive_miner: Vec<AccountOf<T>> = Default::default();
		for i in 0 .. 12 {
			let miner: AccountOf<T> = account(miner_list[i as usize], 100, SEED);
			positive_miner.push(miner.clone());
			let _ = pallet_sminer::benchmarking::register_positive_miner::<T>(miner.clone())?;
			let _ = cert_idle_for_miner::<T>(miner)?;
		}

		let _ = buy_space::<T>(user.clone())?;
		
		let file_name = "test-file".as_bytes().to_vec();
		let bucket_name = "test-bucket1".as_bytes().to_vec();
		let file_hash: Hash = Hash([80u8; 64]);
		let file_size: u128 = SEGMENT_SIZE * 3;
		let territory_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
		let user_brief = UserBrief::<T> {
			user: user.clone(),
			file_name: file_name.try_into().map_err(|_e| "file name convert err")?,
			bucket_name: bucket_name.try_into().map_err(|_e| "bucket name convert err")?,
			territory_name,
		};
		
		let mut deal_info: BoundedVec<SegmentList<T>, T::SegmentCount> = Default::default();

		let segment_list = SegmentList::<T> {
			hash: Hash([65u8; 64]),
			fragment_list: [
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
			].to_vec().try_into().unwrap(),
		};
		deal_info.try_push(segment_list).unwrap();

		FileBank::<T>::upload_declaration(RawOrigin::Signed(user).into(), file_hash.clone(), deal_info, user_brief, file_size)?;

		for i in 0 .. 12 {
			FileBank::<T>::transfer_report(RawOrigin::Signed(positive_miner[i as usize].clone()).into(), i + 1, file_hash.clone())?;
		}

		let tee_puk = pallet_tee_worker::benchmarking::get_pubkey::<T>();
		let mut digest_list: BoundedVec<DigestInfo, ConstU32<1000>> = Default::default();
		let digest_info = DigestInfo {
			fragment: Hash([97u8; 64]),
			tee_puk : tee_puk,
		};
		digest_list.try_push(digest_info).unwrap();
		let tag_sig_info = TagSigInfo::<AccountOf<T>> {
			miner: positive_miner[0].clone(),
			digest: digest_list,
			file_hash: file_hash.clone(),
		};

		let idle_sig_info_encode = tag_sig_info.encode();
		let original = sp_io::hashing::sha2_256(&idle_sig_info_encode);
		let sig = pallet_tee_worker::benchmarking::sign_message::<T>(&original);
        let sig: BoundedVec<u8, ConstU32<64>> = sig.0.to_vec().try_into().map_err(|_| "bounded convert error")?;
		assert!(<File<T>>::contains_key(&file_hash));
		let (_, service) = T::MinerControl::get_power(&positive_miner[0]).unwrap();
		assert_eq!(service, 0);
	}: _(RawOrigin::Signed(positive_miner[0].clone()), sig, tag_sig_info)
	verify {
		let (_, service) = T::MinerControl::get_power(&positive_miner[0]).unwrap();
		assert_eq!(service, FRAGMENT_SIZE * 1);
	}

	replace_idle_space {
		let v in 8 .. 30;
		log::info!("start replace_idle_space");
		pallet_tee_worker::benchmarking::generate_workers::<T>();
		let user: AccountOf<T> = account("user1", 100, SEED);
		let mut positive_miner: Vec<AccountOf<T>> = Default::default();
		for i in 0 .. 12 {
			let miner: AccountOf<T> = account(miner_list[i as usize], 100, SEED);
			positive_miner.push(miner.clone());
			let _ = pallet_sminer::benchmarking::register_positive_miner::<T>(miner.clone())?;
			let _ = cert_idle_for_miner::<T>(miner)?;
		}

		let _ = buy_space::<T>(user.clone())?;
		
		let file_name = "test-file".as_bytes().to_vec();
		let bucket_name = "test-bucket1".as_bytes().to_vec();
		let file_hash: Hash = Hash([80u8; 64]);
		let file_size: u128 = SEGMENT_SIZE * 3;
		let territory_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
		let user_brief = UserBrief::<T> {
			user: user.clone(),
			file_name: file_name.try_into().map_err(|_e| "file name convert err")?,
			bucket_name: bucket_name.try_into().map_err(|_e| "bucket name convert err")?,
			territory_name,
		};

		let mut deal_info: BoundedVec<SegmentList<T>, T::SegmentCount> = Default::default();
		for i in 0 .. v {
			let segment_list = SegmentList::<T> {
				hash: Hash([65u8; 64]),
				fragment_list: [
					Hash([97u8; 64]),
					Hash([97u8; 64]),
					Hash([97u8; 64]),
					Hash([97u8; 64]),
					Hash([97u8; 64]),
					Hash([97u8; 64]),
					Hash([97u8; 64]),
					Hash([97u8; 64]),
					Hash([97u8; 64]),
					Hash([97u8; 64]),
					Hash([97u8; 64]),
					Hash([97u8; 64]),
				].to_vec().try_into().unwrap(),
			};
			deal_info.try_push(segment_list).unwrap();
		}

		FileBank::<T>::upload_declaration(RawOrigin::Signed(user).into(), file_hash.clone(), deal_info, user_brief, file_size)?;

		for i in 0 .. 12 {
			FileBank::<T>::transfer_report(RawOrigin::Signed(positive_miner[i as usize].clone()).into(), i + 1, file_hash.clone())?;
		}

		let tee_puk = pallet_tee_worker::benchmarking::get_pubkey::<T>();
		let mut digest_list: BoundedVec<DigestInfo, ConstU32<1000>> = Default::default();
		for i in 0 .. v {
			let digest_info = DigestInfo {
				fragment: Hash([97u8; 64]),
				tee_puk : tee_puk,
			};
			digest_list.try_push(digest_info).unwrap();
		}
		let tag_sig_info = TagSigInfo::<AccountOf<T>> {
			miner: positive_miner[0].clone(),
			digest: digest_list,
			file_hash: file_hash.clone(),
		};
		let idle_sig_info_encode = tag_sig_info.encode();
		let original = sp_io::hashing::sha2_256(&idle_sig_info_encode);
		let sig = pallet_tee_worker::benchmarking::sign_message::<T>(&original);
        let sig: BoundedVec<u8, ConstU32<64>> = sig.0.to_vec().try_into().map_err(|_| "bounded convert error")?;
		FileBank::<T>::calculate_report(RawOrigin::Signed(positive_miner[0].clone()).into(), sig, tag_sig_info)?;

		let pois_key = PoISKey {
            g: [2u8; 256],
            n: [3u8; 256],
        };

		let space_proof_info = SpaceProofInfo::<AccountOf<T>> {
            miner: positive_miner[0].clone(),
            front: 1,
            rear: 1000,
            pois_key: pois_key.clone(),
            accumulator: pois_key.g,
        };

		let idle_sig_info_encode = space_proof_info.encode();
		let tee_puk_encode = tee_puk.encode();
		let mut original = Vec::new();
		original.extend_from_slice(&idle_sig_info_encode);
		original.extend_from_slice(&tee_puk_encode);
		let original = sp_io::hashing::sha2_256(&original);

		let sig = pallet_tee_worker::benchmarking::sign_message::<T>(&original);
        let sig: BoundedVec<u8, ConstU32<64>> = sig.0.to_vec().try_into().map_err(|_| "bounded convert error")?;
		let replace_space = pallet_sminer::benchmarking::get_replace_space::<T>(positive_miner[0].clone()).unwrap();
		assert_eq!(replace_space, FRAGMENT_SIZE * v as u128);
	}: _(RawOrigin::Signed(positive_miner[0].clone()), space_proof_info, sig.clone(), sig, tee_puk)
	verify {
		let replace_space = pallet_sminer::benchmarking::get_replace_space::<T>(positive_miner[0].clone()).unwrap();
		assert_eq!(replace_space, FRAGMENT_SIZE * v as u128 - IDLE_SEG_SIZE);
	}

	delete_file {
		log::info!("start delete_file");
		pallet_tee_worker::benchmarking::generate_workers::<T>();
		let user: AccountOf<T> = account("user1", 100, SEED);
		let mut positive_miner: Vec<AccountOf<T>> = Default::default();
		for i in 0 .. 12 {
			let miner: AccountOf<T> = account(miner_list[i as usize], 100, SEED);
			positive_miner.push(miner.clone());
			let _ = pallet_sminer::benchmarking::register_positive_miner::<T>(miner.clone())?;
			let _ = cert_idle_for_miner::<T>(miner)?;
		}

		let _ = buy_space::<T>(user.clone())?;
		
		let file_name = "test-file".as_bytes().to_vec();
		let bucket_name = "test-bucket1".as_bytes().to_vec();
		let file_hash: Hash = Hash([80u8; 64]);
		let file_size: u128 = SEGMENT_SIZE * 3;
		let territory_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
		let user_brief = UserBrief::<T> {
			user: user.clone(),
			file_name: file_name.try_into().map_err(|_e| "file name convert err")?,
			bucket_name: bucket_name.try_into().map_err(|_e| "bucket name convert err")?,
			territory_name,
		};

		let mut deal_info: BoundedVec<SegmentList<T>, T::SegmentCount> = Default::default();
		let segment_list = SegmentList::<T> {
			hash: Hash([65u8; 64]),
			fragment_list: [
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
				Hash([97u8; 64]),
			].to_vec().try_into().unwrap(),
		};
		deal_info.try_push(segment_list).unwrap();
		FileBank::<T>::upload_declaration(RawOrigin::Signed(user.clone()).into(), file_hash.clone(), deal_info, user_brief, file_size)?;

		for i in 0 .. 12 {
			FileBank::<T>::transfer_report(RawOrigin::Signed(positive_miner[i as usize].clone()).into(), i + 1, file_hash.clone())?;
		}
	}: _(RawOrigin::Signed(user.clone()), user.clone(), file_hash.clone())
	verify {
		assert!(!<File<T>>::contains_key(&file_hash));
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
		FileBank::<T>::create_bucket(RawOrigin::Signed(caller.clone()).into(), caller.clone(), name_bound.clone())?;
		Bucket::<T>::contains_key(&caller, name_bound.clone());
	}: _(RawOrigin::Signed(caller.clone()), caller.clone(), name_bound.clone())
	verify {
		assert!(!Bucket::<T>::contains_key(&caller, name_bound));
	}

	generate_restoral_order {
		log::info!("start generate_restoral_order");
		initialize_file_from_scratch::<T>()?;
		let miner = account(miner_list[0], 100, SEED);
	}: _(RawOrigin::Signed(miner), Hash([80u8; 64]), Hash([97u8; 64]))
	verify {
		assert!(<RestoralOrder<T>>::contains_key(&Hash([97u8; 64])));
	}

	claim_restoral_order {
		log::info!("start claim_restoral_order");
		initialize_file_from_scratch::<T>()?;
		let miner: AccountOf<T> = account(miner_list[0], 100, SEED);
		FileBank::<T>::generate_restoral_order(RawOrigin::Signed(miner.clone()).into(), Hash([80u8; 64]), Hash([97u8; 64]))?;
		assert!(<RestoralOrder<T>>::contains_key(&Hash([97u8; 64])));
		let miner2: AccountOf<T> = account(miner_list[12], 100, SEED);
		let _ = pallet_sminer::benchmarking::register_positive_miner::<T>(miner2.clone())?;
		let _ = cert_idle_for_miner::<T>(miner2.clone())?;
	}: _(RawOrigin::Signed(miner2.clone()), Hash([97u8; 64]))
	verify {
		let info = <RestoralOrder<T>>::try_get(&Hash([97u8; 64])).unwrap();
		assert_eq!(info.miner, miner2);
	}

	claim_restoral_noexist_order {
		log::info!("start claim_restoral_noexist_order");
		initialize_file_from_scratch::<T>()?;
		let miner: AccountOf<T> = account(miner_list[0], 100, SEED);

		let tee_puk = pallet_tee_worker::benchmarking::get_pubkey::<T>();
		let mut digest_list: BoundedVec<DigestInfo, ConstU32<1000>> = Default::default();
		let digest_info = DigestInfo {
			fragment: Hash([97u8; 64]),
			tee_puk : tee_puk,
		};
		digest_list.try_push(digest_info).unwrap();
		let tag_sig_info = TagSigInfo::<AccountOf<T>> {
			miner: miner.clone(),
			digest: digest_list,
			file_hash: Hash([80u8; 64]),
		};
		let idle_sig_info_encode = tag_sig_info.encode();
		let original = sp_io::hashing::sha2_256(&idle_sig_info_encode);
		let sig = pallet_tee_worker::benchmarking::sign_message::<T>(&original);
        let sig: BoundedVec<u8, ConstU32<64>> = sig.0.to_vec().try_into().map_err(|_| "bounded convert error")?;
		FileBank::<T>::calculate_report(RawOrigin::Signed(miner.clone()).into(), sig, tag_sig_info)?;

		frame_system::Pallet::<T>::set_block_number(28805001u32.into());
        Sminer::<T>::miner_exit_prep(RawOrigin::Signed(miner.clone()).into(), miner.clone())?;
		Sminer::<T>::miner_exit(RawOrigin::Root.into(), miner.clone())?;

		let miner2: AccountOf<T> = account(miner_list[12], 100, SEED);
		let _ = pallet_sminer::benchmarking::register_positive_miner::<T>(miner2.clone())?;
		let _ = cert_idle_for_miner::<T>(miner2.clone())?;
	}: _(RawOrigin::Signed(miner2.clone()), miner.clone(), Hash([80u8; 64]), Hash([97u8; 64]))
	verify {
		assert!(<RestoralOrder<T>>::contains_key(&Hash([97u8; 64])));
	}

	restoral_order_complete {
		log::info!("start restoral_order_complete");
		initialize_file_from_scratch::<T>()?;
		let miner: AccountOf<T> = account(miner_list[0], 100, SEED);
		FileBank::<T>::generate_restoral_order(RawOrigin::Signed(miner.clone()).into(), Hash([80u8; 64]), Hash([97u8; 64]))?;
		assert!(<RestoralOrder<T>>::contains_key(&Hash([97u8; 64])));
		let miner2: AccountOf<T> = account(miner_list[12], 100, SEED);
		let _ = pallet_sminer::benchmarking::register_positive_miner::<T>(miner2.clone())?;
		let _ = cert_idle_for_miner::<T>(miner2.clone())?;
		frame_system::Pallet::<T>::set_block_number(100u32.into());
		FileBank::<T>::claim_restoral_order(RawOrigin::Signed(miner2.clone()).into(), Hash([97u8; 64]))?;
		assert!(<RestoralOrder<T>>::contains_key(&Hash([97u8; 64])));
	}: _(RawOrigin::Signed(miner2.clone()), Hash([97u8; 64]))
	verify {
		assert!(!<RestoralOrder<T>>::contains_key(&Hash([97u8; 64])));
		let (_, space) = T::MinerControl::get_power(&miner2)?;
		assert_eq!(space, FRAGMENT_SIZE);
	}
}