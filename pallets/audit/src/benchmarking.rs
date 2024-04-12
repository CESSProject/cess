use super::*;
use crate::{Pallet as Audit, *};
// // use cp_cess_common::{IpAddress, Hash, DataType};
// // use codec::{alloc::string::ToString, Decode};
use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_system::RawOrigin;
use frame_system::pallet_prelude::BlockNumberFor;
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

pub struct Pallet<T: Config>(Audit<T>);
pub trait Config:
	crate::Config
    + pallet_tee_worker::Config
	+ pallet_sminer::benchmarking::Config
	+ pallet_file_bank::benchmarking::Config
{
}

// const USER_SEED: u32 = 999666;

const SEED: u32 = 2190502;

// const MINER_LIST: [&'static str; 30] = [
// 	"miner1", "miner2", "miner3", "miner4", "miner5", "miner6", "miner7", "miner8", "miner9",
// 	"miner10", "miner11", "miner12", "miner13", "miner14", "miner15", "miner16", "miner17",
// 	"miner18", "miner19", "miner20", "miner21", "miner22", "miner23", "miner24", "miner25",
// 	"miner26", "miner27", "miner28", "miner29", "miner30",
// ];

pub fn bench_generate_challenge<T: Config>(miner: AccountOf<T>) -> Result<(), &'static str>{
    let service_param: QElement= QElement {
        random_index_list: [568,208,340,638,1,006,206,737,844,283,887,890,187,728,342,623,425,224,358,985,543,719,102,579,170,841,978,400,738,899,248,831,772,880,662,367,532,993,270,287,269,606,681,657,106,347,377,422].to_vec().try_into().unwrap(),
        random_list: [
            [0x7d,0xdb,0x2f,0x4e,0x31,0x52,0x76,0x27,0x04,0x96,0x22,0x1b,0x2b,0x21,0x2e,0xba,0xe9,0x5a,0x0d,0x33],
            [0x67,0x9a,0x77,0xcd,0xda,0xe7,0x2d,0x9e,0x09,0x18,0x21,0x07,0x62,0xca,0x55,0x52,0xde,0x37,0x57,0x8e],
            [0x2c,0x1b,0xe7,0x93,0x65,0xfd,0x26,0x5c,0x23,0x4a,0x5c,0x40,0xfe,0xe9,0x1f,0x3a,0x0f,0x0d,0x9c,0x1e],
            [0x7d,0x3f,0xe8,0x17,0x88,0xd1,0x1d,0x44,0x1f,0x78,0xe5,0x07,0x2b,0x3a,0x69,0x76,0xf1,0xcf,0x30,0x33],
            [0xb5,0x44,0xa8,0xac,0xfb,0x27,0x19,0xef,0x39,0xf9,0xd5,0xc8,0x37,0xb9,0x75,0xc3,0x45,0x98,0xca,0xd1],
            [0xf9,0x84,0x96,0xad,0xdc,0x1e,0x1c,0xe0,0x5a,0x58,0xda,0xa9,0x45,0x52,0x73,0x26,0x3f,0x48,0x28,0xe2],
            [0x98,0x4e,0xed,0x71,0x58,0x68,0x67,0xe8,0xb1,0x7a,0x2a,0x14,0xba,0x4e,0x12,0x5e,0x5e,0x18,0x81,0x85],
            [0xf3,0x49,0x6b,0x88,0x54,0xd0,0x6a,0xb5,0xbc,0x4e,0x0b,0x48,0x11,0x3f,0xbf,0x7f,0xc3,0xff,0xb5,0x7b],
            [0x0d,0x2d,0x12,0x2f,0xd6,0x84,0x7d,0x5a,0x5b,0x0f,0x2f,0xe5,0x06,0xc3,0x20,0x75,0xf5,0x7f,0x35,0x76],
            [0x56,0x60,0xe4,0xc4,0xb6,0x82,0x1d,0x4f,0x15,0xec,0x8b,0x18,0x41,0x23,0x92,0x97,0x46,0x5a,0x35,0xa2],
            [0x10,0x88,0x5f,0xbb,0x5d,0xfa,0x49,0xa1,0x40,0xff,0xeb,0xa6,0x4f,0x85,0x8a,0xd3,0x8a,0x17,0xc6,0x75],
            [0x91,0x93,0xa6,0x05,0x74,0x9c,0x64,0xea,0x42,0x03,0x12,0x79,0xc5,0xe0,0xcd,0xcf,0x66,0x07,0x09,0x3b],
            [0x9c,0x24,0x0a,0x54,0xb6,0x32,0x70,0x1f,0x6f,0x86,0x91,0x4b,0x0d,0x85,0x81,0x11,0x15,0x95,0xff,0xb0],
            [0xe8,0x3c,0xa1,0x78,0xe0,0x2a,0x9c,0xf4,0x4c,0x6a,0x9a,0x6d,0xc7,0x43,0xc0,0xfd,0x93,0x66,0x2e,0x3b],
            [0xc0,0x8c,0x22,0xe8,0xe9,0x58,0x42,0xf1,0x78,0xde,0x63,0xc0,0x50,0x28,0x34,0x0d,0xb1,0xba,0xea,0x66],
            [0x27,0x34,0x85,0x03,0x3a,0x65,0x5a,0xd3,0x33,0x8c,0x83,0x05,0x31,0xdf,0xf8,0x4f,0xde,0x96,0x8c,0x27],
            [0xa2,0x07,0x8c,0x6b,0xc6,0x71,0xfd,0x5d,0x56,0x53,0xef,0x98,0x2a,0x95,0xdd,0xc8,0xda,0xc0,0x75,0xd7],
            [0xf8,0xc5,0xeb,0x28,0x8c,0xce,0xab,0x8a,0x2c,0x65,0x21,0x68,0xd7,0x79,0xd3,0xbb,0xae,0xaf,0xca,0x99],
            [0x42,0x8f,0xd9,0xa7,0xef,0x3a,0x4a,0x79,0x62,0x32,0xb3,0x0f,0xc9,0x26,0x6c,0xa6,0x33,0x6c,0x2d,0x89],
            [0x1d,0x83,0x6f,0x29,0x73,0x13,0x8c,0xda,0xda,0x86,0x0c,0x60,0x91,0xf7,0xcd,0x12,0xd8,0xc2,0xec,0xcd],
            [0x9a,0x30,0x11,0x3a,0xea,0x9f,0x83,0xc1,0x34,0x9e,0xd6,0xf0,0x0e,0x04,0x6b,0x1b,0x96,0x6f,0xf9,0x00],
            [0xea,0xae,0xaf,0x8d,0xd9,0xce,0x0e,0x34,0xa4,0x80,0x0d,0x38,0x12,0xb4,0x3e,0x5a,0x92,0xab,0xa9,0x36],
            [0xea,0xe4,0x54,0x59,0xf2,0x97,0x5d,0xe3,0x80,0x0d,0x1a,0xf4,0xc8,0xf0,0x0d,0xed,0xcf,0x2f,0xb8,0xd0],
            [0xd1,0x56,0x37,0x4c,0xe8,0x17,0xab,0x64,0x25,0x97,0x3a,0x00,0x81,0x4a,0xf2,0x91,0x23,0xda,0x6f,0x44],
            [0x90,0xc2,0x95,0xda,0x27,0xcb,0x14,0xe1,0x1b,0x33,0xa2,0x63,0x60,0x5b,0x11,0x68,0xac,0x44,0xce,0x04],
            [0x13,0x00,0xd1,0xb1,0xd7,0x56,0xf4,0xa0,0x87,0x73,0x3c,0x0c,0x41,0x9b,0x0d,0x1c,0x0d,0xbf,0xcb,0x8d],
            [0xe4,0x78,0xad,0xd8,0x6e,0x90,0xab,0xbb,0xfa,0xca,0x47,0xa6,0xb6,0x65,0xae,0xe6,0x88,0x15,0xa3,0x5e],
            [0xd6,0x44,0xd1,0x15,0xc4,0x61,0x43,0x4b,0x9b,0x36,0x63,0x61,0x86,0x10,0x4e,0x44,0x53,0x85,0x07,0x6a],
            [0xa4,0x6c,0x81,0xc5,0xf3,0x67,0xfe,0x10,0xb4,0x88,0xff,0x01,0x59,0xeb,0x57,0x8b,0x80,0x40,0xdd,0x57],
            [0x7f,0x0e,0xcf,0x53,0xa8,0x92,0x36,0xa9,0x40,0xda,0x1f,0xd2,0x8a,0x40,0x4a,0x63,0xb8,0x81,0x1b,0xff],
            [0xa9,0x63,0x31,0x21,0x99,0x85,0xe9,0x99,0x64,0x78,0xc2,0x63,0xfe,0x90,0x5f,0x0a,0x84,0x04,0x9f,0xe3],
            [0x1e,0xed,0x37,0x33,0x90,0xb7,0xfe,0x75,0xca,0xcc,0x8f,0xaf,0xdd,0xf9,0x12,0x44,0x78,0x48,0xc7,0x37],
            [0x3b,0x6b,0x03,0x59,0xb3,0xba,0x23,0x5d,0xff,0xcc,0xe2,0xf1,0x62,0x6d,0x21,0x02,0x9f,0xdb,0x31,0xf4],
            [0xb1,0x33,0x15,0xf6,0xbf,0xa9,0x48,0xeb,0x8b,0xdd,0x94,0xe9,0x8a,0x78,0x1e,0x14,0x08,0x94,0xba,0x1b],
            [0xf9,0xee,0xf6,0x63,0xbd,0x00,0x39,0xd7,0x1d,0xc0,0xa2,0xf0,0x3f,0xe8,0x3d,0xe5,0xba,0x9f,0x92,0xab],
            [0x88,0xdc,0xa5,0x47,0xab,0x8e,0xbb,0x03,0x17,0xcf,0xe1,0xc2,0x17,0x88,0x58,0xbf,0x45,0x9c,0xde,0xea],
            [0x64,0xdb,0x28,0x31,0xa5,0xfb,0xf5,0x38,0x2b,0xd0,0x40,0xcb,0xaa,0x18,0x2f,0xe6,0xa8,0x9a,0x98,0x7d],
            [0xae,0xc7,0x09,0x44,0x23,0xb8,0xcc,0x22,0xc2,0x16,0x59,0xf3,0x02,0xea,0x64,0x79,0x25,0x2b,0xdb,0x4e],
            [0xa5,0xb3,0xd5,0x23,0x61,0x04,0xe2,0x33,0x69,0x52,0xfa,0x6f,0xb0,0xd4,0x4b,0xbe,0x01,0xd1,0x5a,0x6d],
            [0x81,0x52,0x18,0x8f,0x4c,0x6f,0x14,0xdf,0x26,0x0c,0x5d,0x7e,0xbe,0x7f,0xd1,0xa4,0xa1,0x26,0x8b,0xaa],
            [0x75,0xdb,0xf4,0xa0,0xdf,0x88,0x1c,0x2b,0xf7,0xb9,0xbb,0xba,0xdc,0x81,0x89,0x89,0x6f,0xbc,0x84,0x60],
            [0xf3,0x1b,0x4d,0xf9,0x2e,0x9c,0x34,0x17,0x47,0x98,0xaf,0x14,0xd3,0x6b,0x8e,0xdb,0xc6,0x22,0x37,0xf8],
            [0x4a,0xa1,0xb4,0x9b,0x97,0xe1,0x68,0x5f,0xfb,0x20,0xce,0x28,0xf2,0x6e,0x5b,0x82,0x6a,0xe9,0xca,0x8b],
            [0x39,0xa8,0x92,0x9c,0x90,0x6e,0xe2,0x25,0xbd,0xea,0x3b,0xeb,0x50,0x40,0x5e,0xe3,0xb9,0x9c,0x71,0x1f],
            [0x4e,0x6f,0x9b,0xaa,0xb2,0x14,0xec,0x52,0x49,0x33,0x42,0x24,0x2c,0x9e,0x15,0xbf,0x38,0x2f,0x20,0x01],
            [0xa7,0x85,0x1d,0x85,0x4e,0xa6,0x17,0x40,0xe8,0x9d,0x51,0xd0,0x83,0x5b,0xf2,0x07,0x30,0xa3,0xcc,0xc4],
            [0xcb,0x21,0xc4,0x89,0xca,0x49,0xc8,0x81,0x7e,0xbe,0x2e,0x6b,0x2c,0xa5,0x42,0x99,0x85,0x81,0x28,0x8f],
        ].to_vec().try_into().unwrap(),
    };
    let space_param = [83_929_270,84_623_930,83_957_872,84_132_815,84_117_982,84_347_592,84_507_076,84_417_887];

    let idle_slip: BlockNumberFor<T> = 10u32.saturated_into();
    let service_slip: BlockNumberFor<T> = 10u32.saturated_into();
    let verify_slip: BlockNumberFor<T> = 10u32.saturated_into();

    let (idle_space, service_space, service_bloom_filter, space_proof_info, tee_signature) = <T as pallet::Config>::MinerControl::get_miner_snapshot(&miner).unwrap();

    let challenge_info = ChallengeInfo::<T> {
        miner_snapshot: MinerSnapShot::<T> {
            idle_space,
            service_space,
            service_bloom_filter,
            space_proof_info,
            tee_signature,
        },
        challenge_element: ChallengeElement::<T> {
            start: 1u32.saturated_into(),
            idle_slip,
            service_slip,
            verify_slip,
            space_param,
            service_param,
        },
        prove_info: ProveInfo::<T> {
            assign: u8::MIN,
            idle_prove: None,
            service_prove: None,
        },
    };

	<ChallengeSnapShot<T>>::insert(&miner, challenge_info);
	<ChallengeSlip<T>>::insert(&verify_slip, &miner, true);
	<VerifySlip<T>>::insert(&verify_slip, &miner, true);

    Ok(())
}

benchmarks! {
    submit_idle_proof {
        log::info!("start submit_idle_proof");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
        let miner: AccountOf<T> = account("miner1", 100, SEED);
        pallet_sminer::benchmarking::register_positive_miner::<T>(miner.clone())?;
        let _ = pallet_file_bank::benchmarking::cert_idle_for_miner::<T>(miner.clone())?;
        bench_generate_challenge::<T>(miner.clone())?;
        let idle_prove: BoundedVec<u8, T::IdleTotalHashLength> = [8u8; 64].to_vec().try_into().unwrap();
    }: _(RawOrigin::Signed(miner.clone()), idle_prove)
    verify {
        let challenge_info = <ChallengeSnapShot<T>>::try_get(&miner).unwrap();
        assert!(challenge_info.prove_info.idle_prove.is_some());
    }

    submit_service_proof {
        log::info!("start submit_service_proof");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
        let miner: AccountOf<T> = account("miner1", 100, SEED);
        pallet_sminer::benchmarking::register_positive_miner::<T>(miner.clone())?;
        let _ = pallet_file_bank::benchmarking::cert_idle_for_miner::<T>(miner.clone())?;
        bench_generate_challenge::<T>(miner.clone())?;
        let service_prove: BoundedVec<u8, T::SigmaMax> = [8u8; 2048].to_vec().try_into().unwrap();
    }: _(RawOrigin::Signed(miner.clone()), service_prove)
    verify {
        let challenge_info = <ChallengeSnapShot<T>>::try_get(&miner).unwrap();
        assert!(challenge_info.prove_info.service_prove.is_some());
    }

    submit_verify_idle_result {
        log::info!("start submit_verify_idle_result");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
        let miner: AccountOf<T> = account("miner1", 100, SEED);
        pallet_sminer::benchmarking::register_positive_miner::<T>(miner.clone())?;
        let _ = pallet_file_bank::benchmarking::cert_idle_for_miner::<T>(miner.clone())?;
        bench_generate_challenge::<T>(miner.clone())?;
        let idle_prove: BoundedVec<u8, T::IdleTotalHashLength> = [8u8; 64].to_vec().try_into().unwrap();
        Audit::<T>::submit_idle_proof(RawOrigin::Signed(miner.clone()).into(), idle_prove.clone())?;

        let (idle_space, service_space, service_bloom_filter, space_proof_info, tee_signature) = <T as pallet::Config>::MinerControl::get_miner_snapshot(&miner).unwrap();
        let challenge_info = <ChallengeSnapShot<T>>::try_get(&miner).unwrap();
        let tee_puk = pallet_tee_worker::benchmarking::get_pubkey::<T>();

        let verify_idle_info = VerifyIdleResultInfo::<T> {
            miner: miner.clone(),
            miner_prove: idle_prove.clone(),
            front: space_proof_info.front,
            rear: space_proof_info.rear,
            accumulator: space_proof_info.accumulator,
            space_challenge_param: challenge_info.challenge_element.space_param,
            result: true,
            tee_puk: tee_puk.clone(),
        };

        let encoding = verify_idle_info.encode();
        let hashing = sp_io::hashing::sha2_256(&encoding);
        let sig = pallet_tee_worker::benchmarking::sign_message::<T>(&hashing);
        let sig: BoundedVec<u8, ConstU32<64>> = sig.0.to_vec().try_into().map_err(|_| "bounded convert error")?;
    }: _(RawOrigin::Signed(miner.clone()), idle_prove.clone(), space_proof_info.front, space_proof_info.rear, space_proof_info.accumulator, true, sig, tee_puk)
    verify {
        let challenge_info = <ChallengeSnapShot<T>>::try_get(&miner).unwrap();
        match challenge_info.prove_info.idle_prove {
            Some(idle_prove) => assert!(idle_prove.verify_result.is_some()),
            None => assert!(false),
        };
    }

    submit_verify_service_result {
        log::info!("start submit_verify_service_result");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
        let miner: AccountOf<T> = account("miner1", 100, SEED);
        pallet_sminer::benchmarking::register_positive_miner::<T>(miner.clone())?;
        let _ = pallet_file_bank::benchmarking::cert_idle_for_miner::<T>(miner.clone())?;
        bench_generate_challenge::<T>(miner.clone())?;
        let service_prove: BoundedVec<u8, T::SigmaMax> = [8u8; 2048].to_vec().try_into().unwrap();
        Audit::<T>::submit_service_proof(RawOrigin::Signed(miner.clone()).into(), service_prove.clone())?;

        let (idle_space, service_space, service_bloom_filter, space_proof_info, tee_signature) = <T as pallet::Config>::MinerControl::get_miner_snapshot(&miner).unwrap();
        let challenge_info = <ChallengeSnapShot<T>>::try_get(&miner).unwrap();
        let tee_puk = pallet_tee_worker::benchmarking::get_pubkey::<T>();

        let verify_service_info = VerifyServiceResultInfo::<T> {
            miner: miner.clone(),
            tee_puk: tee_puk.clone(),
            miner_prove: service_prove.clone(),
            result: true,
            chal: QElement {
                random_index_list: challenge_info
                    .challenge_element
                    .service_param
                    .random_index_list
                    .clone(),
                random_list: challenge_info
                    .challenge_element
                    .service_param
                    .random_list
                    .clone(),
            },
            service_bloom_filter: service_bloom_filter.clone(),
        };
        let encoding = verify_service_info.encode();
        let hashing = sp_io::hashing::sha2_256(&encoding);
        let sig = pallet_tee_worker::benchmarking::sign_message::<T>(&hashing);
        let sig: BoundedVec<u8, ConstU32<64>> = sig.0.to_vec().try_into().map_err(|_| "bounded convert error")?;

    }: _(RawOrigin::Signed(miner.clone()), true, sig, service_bloom_filter, tee_puk)
    verify {
        let challenge_info = <ChallengeSnapShot<T>>::try_get(&miner).unwrap();
        match challenge_info.prove_info.service_prove {
            Some(service_proof) => assert!(service_proof.verify_result.is_some()),
            None => assert!(false),
        };
    }

    submit_verify_idle_result_reward {
        log::info!("start submit_verify_idle_result_reward");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
        let miner: AccountOf<T> = account("miner1", 100, SEED);
        pallet_sminer::benchmarking::register_positive_miner::<T>(miner.clone())?;
        let _ = pallet_file_bank::benchmarking::cert_idle_for_miner::<T>(miner.clone())?;
        bench_generate_challenge::<T>(miner.clone())?;
        let idle_prove: BoundedVec<u8, T::IdleTotalHashLength> = [8u8; 64].to_vec().try_into().unwrap();
        Audit::<T>::submit_idle_proof(RawOrigin::Signed(miner.clone()).into(), idle_prove.clone())?;
        let service_prove: BoundedVec<u8, T::SigmaMax> = [8u8; 2048].to_vec().try_into().unwrap();
        Audit::<T>::submit_service_proof(RawOrigin::Signed(miner.clone()).into(), service_prove.clone())?;

        let (idle_space, service_space, service_bloom_filter, space_proof_info, tee_signature) = <T as pallet::Config>::MinerControl::get_miner_snapshot(&miner).unwrap();
        let challenge_info = <ChallengeSnapShot<T>>::try_get(&miner).unwrap();
        let tee_puk = pallet_tee_worker::benchmarking::get_pubkey::<T>();

        let verify_service_info = VerifyServiceResultInfo::<T> {
            miner: miner.clone(),
            tee_puk: tee_puk.clone(),
            miner_prove: service_prove.clone(),
            result: true,
            chal: QElement {
                random_index_list: challenge_info
                    .challenge_element
                    .service_param
                    .random_index_list
                    .clone(),
                random_list: challenge_info
                    .challenge_element
                    .service_param
                    .random_list
                    .clone(),
            },
            service_bloom_filter: service_bloom_filter.clone(),
        };
        let encoding = verify_service_info.encode();
        let hashing = sp_io::hashing::sha2_256(&encoding);
        let sig = pallet_tee_worker::benchmarking::sign_message::<T>(&hashing);
        let sig: BoundedVec<u8, ConstU32<64>> = sig.0.to_vec().try_into().map_err(|_| "bounded convert error")?;

        Audit::<T>::submit_verify_service_result(RawOrigin::Signed(miner.clone()).into(), true, sig, service_bloom_filter, tee_puk.clone())?;

        let verify_idle_info = VerifyIdleResultInfo::<T> {
            miner: miner.clone(),
            miner_prove: idle_prove.clone(),
            front: space_proof_info.front,
            rear: space_proof_info.rear,
            accumulator: space_proof_info.accumulator,
            space_challenge_param: challenge_info.challenge_element.space_param,
            result: true,
            tee_puk: tee_puk.clone(),
        };

        let encoding = verify_idle_info.encode();
        let hashing = sp_io::hashing::sha2_256(&encoding);
        let sig = pallet_tee_worker::benchmarking::sign_message::<T>(&hashing);
        let sig: BoundedVec<u8, ConstU32<64>> = sig.0.to_vec().try_into().map_err(|_| "bounded convert error")?;
    }: submit_verify_idle_result(RawOrigin::Signed(miner.clone()), idle_prove.clone(), space_proof_info.front, space_proof_info.rear, space_proof_info.accumulator, true, sig, tee_puk)
    verify {
        let challenge_info = <ChallengeSnapShot<T>>::try_get(&miner).unwrap();
        match challenge_info.prove_info.idle_prove {
            Some(idle_prove) => assert!(idle_prove.verify_result.is_some()),
            None => assert!(false),
        };
    }

    submit_verify_service_result_reward {
        log::info!("start submit_verify_service_result_reward");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
        let miner: AccountOf<T> = account("miner1", 100, SEED);
        pallet_sminer::benchmarking::register_positive_miner::<T>(miner.clone())?;
        let _ = pallet_file_bank::benchmarking::cert_idle_for_miner::<T>(miner.clone())?;
        bench_generate_challenge::<T>(miner.clone())?;
        let idle_prove: BoundedVec<u8, T::IdleTotalHashLength> = [8u8; 64].to_vec().try_into().unwrap();
        Audit::<T>::submit_idle_proof(RawOrigin::Signed(miner.clone()).into(), idle_prove.clone())?;
        let service_prove: BoundedVec<u8, T::SigmaMax> = [8u8; 2048].to_vec().try_into().unwrap();
        Audit::<T>::submit_service_proof(RawOrigin::Signed(miner.clone()).into(), service_prove.clone())?;

        let (idle_space, service_space, service_bloom_filter, space_proof_info, tee_signature) = <T as pallet::Config>::MinerControl::get_miner_snapshot(&miner).unwrap();
        let challenge_info = <ChallengeSnapShot<T>>::try_get(&miner).unwrap();
        let tee_puk = pallet_tee_worker::benchmarking::get_pubkey::<T>();

        let verify_idle_info = VerifyIdleResultInfo::<T> {
            miner: miner.clone(),
            miner_prove: idle_prove.clone(),
            front: space_proof_info.front,
            rear: space_proof_info.rear,
            accumulator: space_proof_info.accumulator,
            space_challenge_param: challenge_info.challenge_element.space_param,
            result: true,
            tee_puk: tee_puk.clone(),
        };

        let encoding = verify_idle_info.encode();
        let hashing = sp_io::hashing::sha2_256(&encoding);
        let sig = pallet_tee_worker::benchmarking::sign_message::<T>(&hashing);
        let sig: BoundedVec<u8, ConstU32<64>> = sig.0.to_vec().try_into().map_err(|_| "bounded convert error")?;
        Audit::<T>::submit_verify_idle_result(
            RawOrigin::Signed(miner.clone()).into(), 
            idle_prove.clone(), 
            space_proof_info.front.clone(), 
            space_proof_info.rear.clone(), 
            space_proof_info.accumulator.clone(),
            true, 
            sig, 
            tee_puk.clone(),
        )?;

        let verify_service_info = VerifyServiceResultInfo::<T> {
            miner: miner.clone(),
            tee_puk: tee_puk.clone(),
            miner_prove: service_prove.clone(),
            result: true,
            chal: QElement {
                random_index_list: challenge_info
                    .challenge_element
                    .service_param
                    .random_index_list
                    .clone(),
                random_list: challenge_info
                    .challenge_element
                    .service_param
                    .random_list
                    .clone(),
            },
            service_bloom_filter: service_bloom_filter.clone(),
        };
        let encoding = verify_service_info.encode();
        let hashing = sp_io::hashing::sha2_256(&encoding);
        let sig = pallet_tee_worker::benchmarking::sign_message::<T>(&hashing);
        let sig: BoundedVec<u8, ConstU32<64>> = sig.0.to_vec().try_into().map_err(|_| "bounded convert error")?;
    }: submit_verify_service_result(RawOrigin::Signed(miner.clone()), true, sig, service_bloom_filter, tee_puk)
    verify {
        let challenge_info = <ChallengeSnapShot<T>>::try_get(&miner).unwrap();
        match challenge_info.prove_info.service_prove {
            Some(service_proof) => assert!(service_proof.verify_result.is_some()),
            None => assert!(false),
        };
    }
}
