// // use super::*;
// use crate::{Pallet as Audit, *};
// // use cp_cess_common::{IpAddress, Hash, DataType};
// // use codec::{alloc::string::ToString, Decode};
// use frame_benchmarking::{
// 	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
// };
// use frame_system::RawOrigin;
// // use frame_support::{
// // 	dispatch::UnfilteredDispatchable,
// // 	pallet_prelude::*,
// // 	traits::{Currency, CurrencyToVote, Get, Imbalance},
// // };
// // use pallet_cess_staking::{
// // 	testing_utils, Config as StakingConfig, Pallet as Staking, RewardDestination,
// // };
// // use pallet_tee_worker::{Config as TeeWorkerConfig, testing_utils::add_scheduler, Pallet as
// // TeeWorker}; use pallet_sminer::{Config as SminerConfig, Pallet as Sminer};
// // use sp_runtime::{
// // 	traits::{Bounded, One, StaticLookup, TrailingZeroInput, Zero},
// // 	Perbill, Percent,
// // };
// // use sp_std::prelude::*;

// // use frame_system::RawOrigin;
// pub struct Pallet<T: Config>(Audit<T>);
// pub trait Config:
// 	crate::Config
// 	+ pallet_cess_staking::Config
// 	+ pallet_tee_worker::benchmarking::Config
// 	+ pallet_sminer::benchmarking::Config
// 	+ pallet_file_bank::benchmarking::Config
// {
// }

// const USER_SEED: u32 = 999666;

// const SEED: u32 = 2190502;

// const MINER_LIST: [&'static str; 30] = [
// 	"miner1", "miner2", "miner3", "miner4", "miner5", "miner6", "miner7", "miner8", "miner9",
// 	"miner10", "miner11", "miner12", "miner13", "miner14", "miner15", "miner16", "miner17",
// 	"miner18", "miner19", "miner20", "miner21", "miner22", "miner23", "miner24", "miner25",
// 	"miner26", "miner27", "miner28", "miner29", "miner30",
// ];

// pub fn bench_generate_challenge<T: Config>() {
// 	let space_challenge_param = [
// 		67_549_635, 67_864_236, 67_338_392, 67_130_229, 67_369_766, 67_193_409, 67_799_602,
// 		67_425_292,
// 	];
// 	let random_index_list = [
// 		691, 406, 838, 480, 996, 798, 362, 456, 144, 666, 1, 018, 568, 992, 650, 729, 808, 229,
// 		623, 499, 671, 254, 24, 217, 698, 648, 781, 460, 298, 548, 742, 364, 183, 114, 309, 564,
// 		127, 154, 815, 651, 397, 576, 697, 358, 880, 73, 629, 66,
// 	];
// 	let random_list = [[55u8; 20]; 48];
// pub fn generate_pair_key() {
//     let pair = sp_core::sr25519::Pair::from_string(&format!("{}//Alice", DEV_PHRASE), None).unwrap();
//     let pubkey = pair.public().unwrap();
    
// }

// 	let mut miner_snapshot_list: BoundedVec<
// 		MinerSnapShot<AccountOf<T>, BlockNumberOf<T>>,
// 		<T as crate::Config>::ChallengeMinerMax,
// 	> = Default::default();
// 	let mut total_idle_space: u128 = u128::MIN;
// 	let mut total_service_space: u128 = u128::MIN;
// 	let all_miner =
// 		<T as crate::Config>::MinerControl::get_all_miner().expect("get all miner error!");
// 	for miner in all_miner.into_iter() {
// 		let (idle_space, service_space, service_bloom_filter, space_proof_info, tee_signature) =
// 			<T as crate::Config>::MinerControl::get_miner_snapshot(&miner)
// 				.expect("get miner snapshot failed");
// 		if (idle_space == 0) && (service_space == 0) {
// 			continue
// 		}
// 		total_idle_space = total_idle_space.checked_add(idle_space).expect("overflow");
// 		total_service_space = total_service_space.checked_add(service_space).expect("overflow");
// 		let miner_snapshot = MinerSnapShot::<AccountOf<T>, BlockNumberOf<T>> {
// 			miner,
// 			idle_life: 200u32.saturated_into(),
// 			service_life: 200u32.saturated_into(),
// 			idle_space,
// 			service_space,
// 			idle_submitted: false,
// 			service_submitted: false,
// 			service_bloom_filter,
// 			space_proof_info,
// 			tee_signature,
// 		};
// 		miner_snapshot_list.try_push(miner_snapshot).unwrap();
// 	}

// 	let _ = pallet_sminer::benchmarking::increase_reward::<T>(1_000_000_000_000_000u128);

// 	let net_snap_shot = NetSnapShot::<BlockNumberOf<T>> {
// 		start: 1u32.saturated_into(),
// 		life: 2000u32.saturated_into(),
// 		total_reward: 1_000_000_000_000_000u128,
// 		total_idle_space,
// 		total_service_space,
// 		random_index_list: random_index_list.to_vec().try_into().expect("BoundedVec Error"),
// 		random_list: random_list.to_vec().try_into().expect("BoundedVec Error"),
// 		space_challenge_param,
// 	};

// 	let challenge_info = ChallengeInfo::<T> { net_snap_shot, miner_snapshot_list };
// 	<ChallengeSnapShot<T>>::put(challenge_info);
// 	let duration: BlockNumberOf<T> = 5000u32.saturated_into();
// 	<ChallengeDuration<T>>::put(duration);
// 	let duration: BlockNumberOf<T> = 6000u32.saturated_into();
// 	<VerifyDuration<T>>::put(duration);
// }

// benchmarks! {
// 	submit_idle_proof {
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 5 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(MINER_LIST[i as usize])?;
// 			pallet_file_bank::benchmarking::add_idle_space::<T>(miner.clone())?;
// 		}
// 		bench_generate_challenge::<T>();
// 		let miner: AccountOf<T> = account("miner1", 100, SEED);
// 		let idle_total_hash: BoundedVec<u8, T::IdleTotalHashLength> = [5u8; 256].to_vec().try_into().unwrap();
// 	}: _(RawOrigin::Signed(miner.clone()), idle_total_hash)
// 	verify {
// 		let challenge_snap_shot = <ChallengeSnapShot<T>>::try_get().unwrap();
// 		let mut miner_list: Vec<AccountOf<T>> = Default::default();
// 		for (index, miner_snapshot) in challenge_snap_shot.miner_snapshot_list.iter().enumerate() {
// 			miner_list.push(miner_snapshot.miner.clone());
// 			if miner_snapshot.miner == miner {
// 				assert_eq!(miner_snapshot.idle_submitted, true);
// 			}
// 		}
// 		assert!(miner_list.contains(&miner));
// 	}

// 	submit_service_proof {
// 		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 5 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(MINER_LIST[i as usize])?;
// 			pallet_file_bank::benchmarking::add_idle_space::<T>(miner.clone())?;
// 		}
// 		bench_generate_challenge::<T>();
// 		let miner: AccountOf<T> = account("miner1", 100, SEED);
// 		let sigma: BoundedVec<u8, T::SigmaMax> = [5u8; 2048].to_vec().try_into().unwrap();
// 	}: _(RawOrigin::Signed(miner.clone()), sigma)
// 	verify {
// 		let challenge_snap_shot = <ChallengeSnapShot<T>>::try_get().unwrap();
// 		let mut miner_list: Vec<AccountOf<T>> = Default::default();
// 		for (index, miner_snapshot) in challenge_snap_shot.miner_snapshot_list.iter().enumerate() {
// 			miner_list.push(miner_snapshot.miner.clone());
// 			if miner_snapshot.miner == miner {
// 				assert_eq!(miner_snapshot.service_submitted, true);
// 			}
// 		}
// 		assert!(miner_list.contains(&miner));
// 	}

// 	submit_verify_idle_result {
// 		let (_, controller_acc) = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 5 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(MINER_LIST[i as usize])?;
// 			pallet_file_bank::benchmarking::add_idle_space::<T>(miner.clone())?;
// 		}
// 		bench_generate_challenge::<T>();
// 		let miner: AccountOf<T> = account("miner1", 100, SEED);
// 		let idle_total_hash: BoundedVec<u8, T::IdleTotalHashLength> = [5u8; 256].to_vec().try_into().unwrap();
// 		let sigma: BoundedVec<u8, T::SigmaMax> = [5u8; 2048].to_vec().try_into().unwrap();
// 		Audit::<T>::submit_idle_proof(RawOrigin::Signed(miner.clone()).into(), idle_total_hash.clone())?;
// 		Audit::<T>::submit_service_proof(RawOrigin::Signed(miner.clone()).into(), sigma.clone())?;

// 		let random_index_list = [691, 406, 838, 480, 996, 798, 362, 456, 144, 666, 1, 018, 568, 992, 650, 729, 808, 229, 623, 499, 671, 254, 24, 217, 698, 648, 781, 460, 298, 548, 742, 364, 183, 114, 309, 564, 127, 154, 815, 651, 397, 576, 697, 358, 880, 73, 629, 66];
// 		let random_list = [[55u8; 20]; 48];
// 		let verify_service_result = VerifyServiceResultInfo::<T>{
// 			miner: miner.clone(),
// 			tee_acc: controller_acc.clone(),
// 			miner_prove: sigma.clone(),
// 			result: true,
// 			chal: QElement {
// 				random_index_list: random_index_list.to_vec().try_into().unwrap(),
// 				random_list: random_list.to_vec().try_into().unwrap(),
// 			},
// 			service_bloom_filter: cp_bloom_filter::BloomFilter([0u64; 256]),
// 		};

// 		let sig = [51, 231, 229, 255, 95, 218, 131, 76, 213, 104, 14, 3, 79, 205, 33, 157, 39, 190, 5, 169, 27, 210, 250, 86, 103, 116, 218, 23, 255, 252, 221, 247, 82, 109, 185, 200, 134, 188, 81, 199, 219, 245, 30, 42, 228, 193, 208, 135, 209, 24, 232, 30, 104, 90, 156, 215, 174, 98, 143, 143, 119, 217, 72, 24, 247, 232, 191, 246, 205, 57, 221, 124, 234, 135, 110, 184, 208, 77, 60, 64, 169, 75, 168, 123, 244, 134, 187, 187, 65, 62, 237, 10, 246, 102, 44, 235, 68, 175, 91, 36, 58, 157, 213, 63, 184, 32, 38, 242, 57, 255, 190, 251, 214, 41, 53, 36, 190, 66, 47, 100, 153, 110, 88, 6, 221, 250, 183, 169, 76, 203, 9, 188, 187, 79, 149, 16, 145, 155, 97, 89, 56, 94, 187, 65, 197, 133, 121, 0, 73, 115, 166, 148, 177, 62, 121, 137, 96, 86, 3, 111, 153, 108, 230, 241, 190, 114, 169, 249, 118, 50, 130, 46, 190, 127, 131, 142, 253, 217, 138, 102, 215, 162, 47, 94, 83, 36, 117, 25, 24, 125, 11, 91, 112, 211, 201, 28, 64, 141, 129, 14, 117, 2, 124, 217, 229, 179, 192, 34, 0, 37, 187, 44, 38, 170, 116, 176, 175, 96, 207, 181, 144, 30, 246, 170, 98, 206, 88, 176, 141, 118, 180, 99, 107, 196, 201, 206, 144, 81, 154, 138, 115, 38, 88, 219, 8, 55, 221, 20, 222, 106, 134, 98, 184, 92, 253, 185];
// 		Audit::<T>::submit_verify_service_result(RawOrigin::Signed(miner.clone()).into(), verify_service_result.result, sig, verify_service_result.service_bloom_filter, verify_service_result.tee_acc)?;

// 		let verify_idle_result = VerifyIdleResultInfo::<T>{
// 			miner: miner.clone(),
// 			miner_prove: idle_total_hash.clone(),
// 			front: 0,
// 			rear: 10000,
// 			accumulator: [8u8; 256],
// 			space_challenge_param: [67_549_635, 67_864_236, 67_338_392, 67_130_229, 67_369_766, 67_193_409, 67_799_602, 67_425_292],
// 			result: true,
// 			tee_acc: controller_acc.clone(),
// 		};

// 		let sig = [105, 84, 152, 186, 134, 202, 80, 152, 159, 223, 251, 150, 106, 226, 63, 179, 168, 182, 237, 158, 111, 17, 68, 195, 39, 255, 75, 98, 67, 232, 10, 129, 175, 254, 222, 147, 45, 182, 91, 235, 26, 246, 88, 16, 106, 100, 241, 54, 236, 205, 224, 173, 113, 54, 71, 0, 95, 157, 184, 238, 54, 114, 40, 145, 44, 87, 163, 77, 177, 34, 211, 247, 51, 174, 53, 20, 151, 186, 173, 135, 83, 150, 197, 172, 121, 181, 203, 164, 10, 104, 163, 51, 110, 231, 241, 26, 112, 214, 127, 139, 247, 33, 158, 252, 146, 157, 47, 53, 172, 50, 111, 45, 249, 109, 145, 173, 10, 228, 133, 34, 163, 4, 254, 140, 64, 252, 136, 40, 87, 242, 36, 83, 15, 23, 182, 77, 9, 197, 223, 36, 129, 3, 41, 247, 102, 143, 192, 42, 251, 16, 215, 193, 1, 19, 154, 245, 212, 209, 52, 1, 80, 248, 118, 47, 114, 202, 238, 173, 244, 154, 218, 209, 71, 185, 76, 15, 0, 56, 213, 201, 228, 193, 105, 160, 48, 247, 134, 58, 91, 121, 75, 144, 163, 89, 105, 176, 29, 30, 169, 70, 205, 202, 188, 230, 207, 171, 95, 253, 14, 103, 249, 144, 213, 44, 57, 142, 82, 252, 103, 201, 183, 218, 31, 131, 219, 108, 83, 115, 198, 3, 14, 244, 178, 48, 196, 128, 100, 53, 1, 10, 31, 37, 205, 163, 20, 181, 71, 152, 171, 180, 102, 208, 1, 241, 174, 28];
// 	}: _(RawOrigin::Signed(miner.clone()), verify_idle_result.miner_prove, verify_idle_result.front, verify_idle_result.rear, verify_idle_result.accumulator, verify_idle_result.result, sig, verify_idle_result.tee_acc)
// 	verify {
// 		let unverify_list = <UnverifyIdleProof<T>>::try_get(&controller_acc).unwrap();
// 		assert_eq!(0, unverify_list.len());
// 	}

// 	submit_verify_service_result {
// 		let (_, controller_acc) = pallet_tee_worker::benchmarking::tee_register::<T>()?;
// 		for i in 0 .. 5 {
// 			let miner = pallet_sminer::benchmarking::add_miner::<T>(MINER_LIST[i as usize])?;
// 			pallet_file_bank::benchmarking::add_idle_space::<T>(miner.clone())?;
// 		}
// 		bench_generate_challenge::<T>();
// 		let miner: AccountOf<T> = account("miner1", 100, SEED);
// 		let idle_total_hash: BoundedVec<u8, T::IdleTotalHashLength> = [5u8; 256].to_vec().try_into().unwrap();
// 		let sigma: BoundedVec<u8, T::SigmaMax> = [5u8; 2048].to_vec().try_into().unwrap();
// 		Audit::<T>::submit_idle_proof(RawOrigin::Signed(miner.clone()).into(), idle_total_hash.clone())?;
// 		Audit::<T>::submit_service_proof(RawOrigin::Signed(miner.clone()).into(), sigma.clone())?;

// 		let verify_idle_result = VerifyIdleResultInfo::<T>{
// 			miner: miner.clone(),
// 			miner_prove: idle_total_hash.clone(),
// 			front: 0,
// 			rear: 10000,
// 			accumulator: [8u8; 256],
// 			space_challenge_param: [67_549_635, 67_864_236, 67_338_392, 67_130_229, 67_369_766, 67_193_409, 67_799_602, 67_425_292],
// 			result: true,
// 			tee_acc: controller_acc.clone(),
// 		};
// 		let sig = [105, 84, 152, 186, 134, 202, 80, 152, 159, 223, 251, 150, 106, 226, 63, 179, 168, 182, 237, 158, 111, 17, 68, 195, 39, 255, 75, 98, 67, 232, 10, 129, 175, 254, 222, 147, 45, 182, 91, 235, 26, 246, 88, 16, 106, 100, 241, 54, 236, 205, 224, 173, 113, 54, 71, 0, 95, 157, 184, 238, 54, 114, 40, 145, 44, 87, 163, 77, 177, 34, 211, 247, 51, 174, 53, 20, 151, 186, 173, 135, 83, 150, 197, 172, 121, 181, 203, 164, 10, 104, 163, 51, 110, 231, 241, 26, 112, 214, 127, 139, 247, 33, 158, 252, 146, 157, 47, 53, 172, 50, 111, 45, 249, 109, 145, 173, 10, 228, 133, 34, 163, 4, 254, 140, 64, 252, 136, 40, 87, 242, 36, 83, 15, 23, 182, 77, 9, 197, 223, 36, 129, 3, 41, 247, 102, 143, 192, 42, 251, 16, 215, 193, 1, 19, 154, 245, 212, 209, 52, 1, 80, 248, 118, 47, 114, 202, 238, 173, 244, 154, 218, 209, 71, 185, 76, 15, 0, 56, 213, 201, 228, 193, 105, 160, 48, 247, 134, 58, 91, 121, 75, 144, 163, 89, 105, 176, 29, 30, 169, 70, 205, 202, 188, 230, 207, 171, 95, 253, 14, 103, 249, 144, 213, 44, 57, 142, 82, 252, 103, 201, 183, 218, 31, 131, 219, 108, 83, 115, 198, 3, 14, 244, 178, 48, 196, 128, 100, 53, 1, 10, 31, 37, 205, 163, 20, 181, 71, 152, 171, 180, 102, 208, 1, 241, 174, 28];
// 		Audit::<T>::submit_verify_idle_result(RawOrigin::Signed(miner.clone()).into(), verify_idle_result.miner_prove, verify_idle_result.front, verify_idle_result.rear, verify_idle_result.accumulator, verify_idle_result.result, sig, verify_idle_result.tee_acc)?;

// 		let random_index_list = [691, 406, 838, 480, 996, 798, 362, 456, 144, 666, 1, 018, 568, 992, 650, 729, 808, 229, 623, 499, 671, 254, 24, 217, 698, 648, 781, 460, 298, 548, 742, 364, 183, 114, 309, 564, 127, 154, 815, 651, 397, 576, 697, 358, 880, 73, 629, 66];
// 		let random_list = [[55u8; 20]; 48];
// 		let verify_service_result = VerifyServiceResultInfo::<T>{
// 			miner: miner.clone(),
// 			tee_acc: controller_acc.clone(),
// 			miner_prove: sigma.clone(),
// 			result: true,
// 			chal: QElement {
// 				random_index_list: random_index_list.to_vec().try_into().unwrap(),
// 				random_list: random_list.to_vec().try_into().unwrap(),
// 			},
// 			service_bloom_filter: cp_bloom_filter::BloomFilter([0u64; 256]),
// 		};

// 		let sig = [51, 231, 229, 255, 95, 218, 131, 76, 213, 104, 14, 3, 79, 205, 33, 157, 39, 190, 5, 169, 27, 210, 250, 86, 103, 116, 218, 23, 255, 252, 221, 247, 82, 109, 185, 200, 134, 188, 81, 199, 219, 245, 30, 42, 228, 193, 208, 135, 209, 24, 232, 30, 104, 90, 156, 215, 174, 98, 143, 143, 119, 217, 72, 24, 247, 232, 191, 246, 205, 57, 221, 124, 234, 135, 110, 184, 208, 77, 60, 64, 169, 75, 168, 123, 244, 134, 187, 187, 65, 62, 237, 10, 246, 102, 44, 235, 68, 175, 91, 36, 58, 157, 213, 63, 184, 32, 38, 242, 57, 255, 190, 251, 214, 41, 53, 36, 190, 66, 47, 100, 153, 110, 88, 6, 221, 250, 183, 169, 76, 203, 9, 188, 187, 79, 149, 16, 145, 155, 97, 89, 56, 94, 187, 65, 197, 133, 121, 0, 73, 115, 166, 148, 177, 62, 121, 137, 96, 86, 3, 111, 153, 108, 230, 241, 190, 114, 169, 249, 118, 50, 130, 46, 190, 127, 131, 142, 253, 217, 138, 102, 215, 162, 47, 94, 83, 36, 117, 25, 24, 125, 11, 91, 112, 211, 201, 28, 64, 141, 129, 14, 117, 2, 124, 217, 229, 179, 192, 34, 0, 37, 187, 44, 38, 170, 116, 176, 175, 96, 207, 181, 144, 30, 246, 170, 98, 206, 88, 176, 141, 118, 180, 99, 107, 196, 201, 206, 144, 81, 154, 138, 115, 38, 88, 219, 8, 55, 221, 20, 222, 106, 134, 98, 184, 92, 253, 185];
// 	}: _(RawOrigin::Signed(miner.clone()), verify_service_result.result, sig, verify_service_result.service_bloom_filter, verify_service_result.tee_acc)
// 	verify {
// 		let unverify_list = <UnverifyServiceProof<T>>::try_get(&controller_acc).unwrap();
// 		assert_eq!(0, unverify_list.len());
// 	}
// }
