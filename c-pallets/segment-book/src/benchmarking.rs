use super::*;
use crate::{Pallet as SegmentBook, *};
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
use pallet_file_map::{Config as FileMapConfig, testing_utils::add_scheduler, Pallet as FileMap};
use pallet_sminer::{Config as SminerConfig, Pallet as Sminer};
use sp_runtime::{
	traits::{Bounded, One, StaticLookup, TrailingZeroInput, Zero},
	Perbill, Percent,
};
use sp_std::prelude::*;

use frame_system::RawOrigin;

pub struct Pallet<T: Config>(SegmentBook<T>);
pub trait Config:
	crate::Config + pallet_cess_staking::Config + pallet_file_map::Config + pallet_sminer::Config
{
}

const USER_SEED: u32 = 999666;

benchmarks! {
    submit_challenge_prove {
        let v in 0 .. T::SubmitProofLimit::get() - 1;
        let ip = "127.0.0.1:8888".as_bytes().to_vec();
        let (stash, controller) = pallet_cess_staking::testing_utils::create_stash_controller::<T>(USER_SEED, 100, Default::default())?;
        pallet_file_map::testing_utils::add_scheduler::<T>(controller.clone(), stash.clone(), ip.clone())?;
        let miner: AccountOf<T> = account("miner1", 100, USER_SEED);
        Sminer::<T>::regnstk(
            RawOrigin::Signed(miner.clone()).into(),
            miner.clone(),
            ip.clone(),
            0u32.into(),
        )?;
        let mut challenge_list: Vec<ChallengeInfo<T>> = Vec::new();
				//ChallengeMaximum = 8000
        for i in 0 .. 7999 {
            let challenge_info = ChallengeInfo::<T>{
                file_size: 123,
                file_type: 1,
                block_list: vec![1,2].try_into().map_err(|_| "bounded convert err")?,
                file_id: (i as u32).to_string().as_bytes().to_vec().try_into().map_err(|_| "uint convert to BoundedVec Error")?,
                random: Default::default(),
            };
            challenge_list.push(challenge_info);
        }
        let challenge_list_bounded: BoundedVec<ChallengeInfo<T>, <T as crate::Config>::ChallengeMaximum> = challenge_list.try_into().map_err(|_| "challenge map convert err")?;
        ChallengeMap::<T>::insert(
            &miner.clone(),
            challenge_list_bounded.clone(),
        );
        let challenge_list = ChallengeMap::<T>::get(&miner);
        let mut prove_list: Vec<ProveInfo<T>> = Vec::new();
        for i in 0 .. v {
            let challenge_info = challenge_list[i as usize].clone();
            let prove_info = ProveInfo::<T>{
                file_id: challenge_info.file_id.clone(),
                miner_acc: miner.clone(),
                challenge_info: challenge_info.clone(),
                mu: Default::default(),
                sigma: Default::default(),
            };
            prove_list.push(prove_info);
        }
    }: _(RawOrigin::Signed(miner.clone()), prove_list.clone())
    verify {
        let unverify_prove = UnVerifyProof::<T>::get(controller.clone());
        for v in prove_list.iter() {
            assert!(unverify_prove.to_vec().contains(v));
        }
    }

    verify_proof {
        let v in 0 .. T::SubmitValidationLimit::get() - 1;
        let ip = "127.0.0.1:8888".as_bytes().to_vec();
        let (stash, controller) = pallet_cess_staking::testing_utils::create_stash_controller::<T>(USER_SEED, 100, Default::default())?;
        pallet_file_map::testing_utils::add_scheduler::<T>(controller.clone(), stash.clone(), ip.clone())?;
        let miner: AccountOf<T> = account("miner1", 100, USER_SEED);
        let mut challenge_list: Vec<ChallengeInfo<T>> = Vec::new();
				//ChallengeMaximum = 8000
        for i in 0 .. 7999 {
            let challenge_info = ChallengeInfo::<T>{
                file_size: 123,
                file_type: 1,
                block_list: vec![1,2].try_into().map_err(|_| "bounded convert err")?,
                file_id: (i as u32).to_string().as_bytes().to_vec().try_into().map_err(|_| "uint convert to BoundedVec Error")?,
                random: Default::default(),
            };
            challenge_list.push(challenge_info);
        }

        let mut prove_list: Vec<ProveInfo<T>> = Vec::new();
        for challenge_info in challenge_list.iter() {
            let prove_info = ProveInfo::<T>{
                file_id: challenge_info.file_id.clone(),
                miner_acc: miner.clone(),
                challenge_info: challenge_info.clone(),
                mu: Default::default(),
                sigma: Default::default(),
            };
            prove_list.push(prove_info);
        }
        let prove_list_bounded: BoundedVec<ProveInfo<T>, <T as crate::Config>::ChallengeMaximum> = prove_list.clone().try_into().map_err(|_| "prove list convert err")?;
        <UnVerifyProof<T>>::insert(
            &controller.clone(),
            prove_list_bounded.clone()
        );

        let mut verify_result_list: Vec<VerifyResult<T>> = Vec::new();
        let mut verify_info_list: Vec<ProveInfo<T>> = Vec::new();
        for i in 0 .. v {
            let prove_info = prove_list_bounded[i as usize].clone();
            let verify_result = VerifyResult::<T> {
                miner_acc: prove_info.clone().miner_acc,
                file_id: prove_info.clone().file_id,
                result: true,
            };
            verify_info_list.push(prove_info);
            verify_result_list.push(verify_result);
        }
    }: _(RawOrigin::Signed(controller.clone()), verify_result_list.clone())
    verify {
        let unverify_prove = UnVerifyProof::<T>::get(controller.clone());
        for value in verify_info_list.iter() {
            assert!(!unverify_prove.to_vec().contains(value));
        }
    }
}
