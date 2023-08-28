// use super::*;
use crate::{Pallet as Audit, *};
// use cp_cess_common::{IpAddress, Hash, DataType};
// use codec::{alloc::string::ToString, Decode};
use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_system::RawOrigin;
// use frame_support::{
// 	dispatch::UnfilteredDispatchable,
// 	pallet_prelude::*,
// 	traits::{Currency, CurrencyToVote, Get, Imbalance},
// };
// use pallet_cess_staking::{
// 	testing_utils, Config as StakingConfig, Pallet as Staking, RewardDestination,
// };
// use pallet_tee_worker::{Config as TeeWorkerConfig, testing_utils::add_scheduler, Pallet as TeeWorker};
// use pallet_sminer::{Config as SminerConfig, Pallet as Sminer};
// use sp_runtime::{
// 	traits::{Bounded, One, StaticLookup, TrailingZeroInput, Zero},
// 	Perbill, Percent,
// };
// use sp_std::prelude::*;

// use frame_system::RawOrigin;

pub struct Pallet<T: Config>(Audit<T>);
pub trait Config:
	crate::Config + pallet_cess_staking::Config + pallet_tee_worker::benchmarking::Config + pallet_sminer::benchmarking::Config + pallet_file_bank::benchmarking::Config
{
}

const USER_SEED: u32 = 999666;

const SEED: u32 = 2190502;

const MINER_LIST: [&'static str; 30] = [
	"miner1", "miner2", "miner3", "miner4", "miner5", "miner6", "miner7", "miner8", "miner9", "miner10",
	"miner11", "miner12", "miner13", "miner14", "miner15", "miner16", "miner17", "miner18", "miner19", "miner20",
	"miner21", "miner22", "miner23", "miner24", "miner25", "miner26", "miner27", "miner28", "miner29", "miner30",
];

pub fn bench_generate_challenge<T: Config>() {
	let space_challenge_param = [67_549_635, 67_864_236, 67_338_392, 67_130_229, 67_369_766, 67_193_409, 67_799_602, 67_425_292];
	let random_index_list = [691, 406, 838, 480, 996, 798, 362, 456, 144, 666, 1, 018, 568, 992, 650, 729, 808, 229, 623, 499, 671, 254, 24, 217, 698, 648, 781, 460, 298, 548, 742, 364, 183, 114, 309, 564, 127, 154, 815, 651, 397, 576, 697, 358, 880, 73, 629, 66];
	let random_list = [[55u8; 20]; 48];

	let mut miner_snapshot_list: BoundedVec<MinerSnapShot<AccountOf<T>, BlockNumberOf<T>>, <T as crate::Config>::ChallengeMinerMax> = Default::default();
	let mut total_idle_space: u128 = u128::MIN;
	let mut total_service_space: u128 = u128::MIN;
	let all_miner = <T as crate::Config>::MinerControl::get_all_miner().expect("get all miner error!");
	for miner in all_miner.into_iter() {
		let (idle_space, service_space, service_bloom_filter, space_proof_info, tee_signature) = <T as crate::Config>::MinerControl::get_miner_snapshot(&miner).expect("get miner snapshot failed");
		if (idle_space == 0) && (service_space == 0) {
			continue;
		}
		total_idle_space = total_idle_space.checked_add(idle_space).expect("overflow");
		total_service_space = total_service_space.checked_add(service_space).expect("overflow");
		let miner_snapshot = MinerSnapShot::<AccountOf<T>, BlockNumberOf<T>> {
			miner,
			idle_life: 200u32.saturated_into(),
			service_life: 200u32.saturated_into(),
			idle_space,
			service_space,
			idle_submitted: false,
			service_submitted: false,
			service_bloom_filter,
			space_proof_info,
			tee_signature,
		};
		miner_snapshot_list.try_push(miner_snapshot).unwrap();
	}

	let net_snap_shot = NetSnapShot::<BlockNumberOf<T>> {
		start: 1u32.saturated_into(),
		life: 2000u32.saturated_into(),
		total_reward: 1_000_000_000_000_000u128,
		total_idle_space,
		total_service_space,
		random_index_list: random_index_list.to_vec().try_into().expect("BoundedVec Error"),
		random_list: random_list.to_vec().try_into().expect("BoundedVec Error"),
		space_challenge_param,
	};

	let challenge_info = ChallengeInfo::<T> {
		net_snap_shot,
		miner_snapshot_list,
	};
	<ChallengeSnapShot<T>>::put(challenge_info);
	let duration: BlockNumberOf<T> = 5000u32.saturated_into();
	<ChallengeDuration<T>>::put(duration);
	let duration: BlockNumberOf<T> = 6000u32.saturated_into();
	<VerifyDuration<T>>::put(duration);
}

benchmarks! {
	submit_idle_proof {
		let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
		for i in 0 .. 5 {
			let miner = pallet_sminer::benchmarking::add_miner::<T>(MINER_LIST[i as usize])?;
			pallet_file_bank::benchmarking::add_idle_space::<T>(miner.clone())?;
		}
		bench_generate_challenge::<T>();
		let miner: AccountOf<T> = account("miner1", 100, SEED);
		let idle_total_hash: BoundedVec<u8, T::IdleTotalHashLength> = [5u8; 256].to_vec().try_into().unwrap();
	}: _(RawOrigin::Signed(miner.clone()), idle_total_hash)
	verify {
		let challenge_snap_shot = <ChallengeSnapShot<T>>::try_get().unwrap();
		let mut miner_list: Vec<AccountOf<T>> = Default::default();
		for (index, miner_snapshot) in challenge_snap_shot.miner_snapshot_list.iter().enumerate() {
			miner_list.push(miner_snapshot.miner.clone());
			if miner_snapshot.miner == miner {
				assert_eq!(miner_snapshot.idle_submitted, true);
			}
		}
		assert!(miner_list.contains(&miner));
	}
}
