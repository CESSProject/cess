//! Tests for the module.

use super::*;
use frame_support::{assert_ok, assert_noop, bounded_vec};
use frame_benchmarking::account;
use crate::{mock::*, Event, Error, mock::System as Sys};
use pallet_file_bank::{SliceInfo, FillerInfo, UserBrief};
use cp_cess_common::{Hash, IpAddress, DataType};

// fn to_bounded_vec<T>(v: Vec<T>) -> BoundedVec<T, StringLimit> {
//     let bv: BoundedVec<T, StringLimit> = v.try_into().unwrap();
//     bv
// }

// fn to_bounded_vec2<T: Clone>(vv: Vec<Vec<T>>) -> BoundedVec<BoundedVec<T, StringLimit>, StringLimit> {
//     let mut r: BoundedVec<BoundedVec<T, StringLimit>, StringLimit> = Vec::new().try_into().unwrap();
//     // let mut t = &r;
//     // vv.iter().map(|v| {
//     //     t.push(to_bounded_vec::<T>(v.to_vec()))
//     // });
//     for v in vv {
//         r.try_push(v.try_into().unwrap()).unwrap();
//     }
//     r
// }

#[derive(Debug, Clone)]
pub struct MockingFileInfo {
    file_hash: Hash,
    file_size: u64,
		slice_info: Vec<SliceInfo<Test>>,
}

impl Default for MockingFileInfo {
	fn default() -> Self {
		let file_hash = Hash([5u8; 64]);
		let mut file_hash1: Vec<u8> = file_hash.0.to_vec();
		file_hash1.append("-001".as_bytes().to_vec().as_mut());
		let file_hash1: [u8; 68] = file_hash1.as_slice().try_into().unwrap();

		let mut file_hash2: Vec<u8> = file_hash.0.to_vec();
		file_hash2.append("-002".as_bytes().to_vec().as_mut());
		let file_hash2: [u8; 68] = file_hash1.as_slice().try_into().unwrap();

		let mut file_hash3: Vec<u8> = file_hash.0.to_vec();
		file_hash3.append("-002".as_bytes().to_vec().as_mut());
		let file_hash3: [u8; 68] = file_hash1.as_slice().try_into().unwrap();
		MockingFileInfo {
			file_hash: file_hash,
			file_size: 12,
			slice_info: vec![SliceInfo::<Test>{
				miner_id: 1,
				shard_size: 111,
				block_num: 8,
				shard_id: file_hash1,
				miner_ip: IpAddress::IPV4([127,0,0,1], 15000),
				miner_acc: mock::miner1(),
			},
			SliceInfo::<Test>{
				miner_id: 1,
				shard_size: 111,
				block_num: 8,
				shard_id: file_hash2,
				miner_ip: IpAddress::IPV4([127,0,0,1], 15000),
				miner_acc: mock::miner1(),
			},
			SliceInfo::<Test>{
				miner_id: 1,
				shard_size: 111,
				block_num: 8,
				shard_id: file_hash3,
				miner_ip: IpAddress::IPV4([127,0,0,1], 15000),
				miner_acc: mock::miner1(),
			},
			]
		}
	}
}

fn add_power_for_miner(controller: AccountId, miner: AccountId) -> DispatchResult {
	let max: u8 = 10;
	let mut filler_list: Vec<FillerInfo<Test>> = Vec::new();
	for i in 0 .. max {
		let filler_hash: Hash = Hash([i; 64]);
		filler_list.push(
			FillerInfo::<Test> {
				index: 1,
				filler_size: 8 * 1_048_576,
				block_num: 8,
				segment_size: 1_048_576,
				scan_size: 1_048_576,
				miner_address: miner.clone(),
				filler_hash: filler_hash.clone(),
			}
		)
	}
	FileBank::upload_filler(RuntimeOrigin::signed(controller), miner, filler_list)?;

	Ok(())
}

fn upload_declaration_alias(account: AccountId, file_name: Vec<u8>, file_hash: Hash, bucket_name: Vec<u8>) -> DispatchResult {
	let user_brief = UserBrief::<Test>{
		user: account.clone(),
		file_name: file_name.try_into().unwrap(),
		bucket_name: bucket_name.try_into().unwrap(),
	};
	FileBank::upload_declaration(
		RuntimeOrigin::signed(account.clone()),
		file_hash,
		user_brief,
	)
}

fn upload_file_alias(_account: AccountId, controller: AccountId, file_info: &MockingFileInfo) -> DispatchResult {
	let MockingFileInfo { file_hash, file_size, slice_info} = file_info.clone();
	FileBank::upload(
		RuntimeOrigin::signed(controller),
		file_hash,
		file_size,
		slice_info,
	)
}

fn create_new_bucket(acc: AccountId, bucket_name: Vec<u8>) -> DispatchResult {
	FileBank::create_bucket(
		RuntimeOrigin::signed(acc.clone()),
		acc,
		bucket_name.try_into().unwrap(),
	)
}

fn register_scheduler(stash: AccountId, controller: AccountId) -> DispatchResult {
	pallet_cess_staking::Bonded::<Test>::insert(&stash, controller.clone());
	FileMap::registration_scheduler(
		RuntimeOrigin::signed(controller),
		stash,
		IpAddress::IPV4([127,0,0,1], 15000),
	)

}

#[test]
fn submit_challenge_prove_works() {
    new_test_ext().execute_with(|| {
        let miner_acc = miner1();
        let file_id = Hash([5u8; 64]);
        let mu = bounded_vec![bounded_vec![2_u8]];
        let sigma = bounded_vec![1_u8];
				let name = bounded_vec![5_u8];
				let u = bounded_vec![bounded_vec![3_u8, 3u8]];
        let controller1 = controller1();
        let stash1 = stash1();
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));

        let mut prove_list: Vec<ProveInfo<Test>> = Vec::new();
        let challenge_info = ChallengeInfo::<Test>{
					file_size: 1024,
					file_type: DataType::Filler,
					block_list: bounded_vec![1,2,3],
					file_id: file_id.clone(),
					shard_id: [0u8; 68],
					random: bounded_vec![bounded_vec![1,3],bounded_vec![4,5]],
        };
        let prove_info = ProveInfo::<Test> {
            file_id: file_id.clone(),
            miner_acc: miner_acc.clone(),
            challenge_info: challenge_info.clone(),
            mu,
            sigma,
						name,
						u,
        };
        prove_list.push(prove_info);
        assert_noop!(SegmentBook::submit_challenge_prove(RuntimeOrigin::signed(miner_acc.clone()), prove_list.clone()), Error::<Test>::NoChallenge);
        assert_ok!(ChallengeMap::<Test>::try_mutate(miner_acc, |value| -> DispatchResult {
            value.try_push(challenge_info).map_err(|_| Error::<Test>::BoundedVecError)?;
            Ok(())
        }));

        assert_ok!(SegmentBook::submit_challenge_prove(RuntimeOrigin::signed(miner_acc.clone()), prove_list));

        assert!(UnVerifyProof::<Test>::contains_key(&controller1));
        let prove_info = UnVerifyProof::<Test>::try_get(&controller1).unwrap().pop().unwrap();
        assert_eq!(miner_acc, prove_info.miner_acc);
        assert_eq!(0, ChallengeMap::<Test>::try_get(miner_acc).unwrap().len());

        let event = Sys::events().pop().expect("Expected at least one ChallengeProof to be found").event;
        assert_eq!(mock::RuntimeEvent::from(Event::ChallengeProof { miner : miner_acc, file_id: file_id }), event);
    });
}

#[test]
fn verify_proof_works() {
    new_test_ext().execute_with(|| {
        let miner_acc = miner1();
				let file_id = Hash([5u8; 64]);
        let controller1 = controller1();
        let stash1 = stash1();
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));

				let challenge_info = ChallengeInfo::<Test>{
					file_size: 1024,
					file_type: DataType::Filler,
					block_list: bounded_vec![1,2,3],
					file_id: file_id.clone(),
					shard_id: [0u8; 68],
					random: bounded_vec![bounded_vec![1,3],bounded_vec![4,5]],
				};

        let mut verify_result_list: Vec<VerifyResult<Test>> = Vec::new();
        let verify_result = VerifyResult::<Test> {
            miner_acc: miner_acc.clone(),
            file_id: file_id.clone(),
						shard_id: [0u8; 68],
            result: true,
        };
        verify_result_list.push(verify_result);

        assert_ok!(UnVerifyProof::<Test>::try_mutate(controller1, |value| -> DispatchResult {
            value.try_push(ProveInfo {
                file_id: file_id.clone(),
                miner_acc,
                challenge_info,
                mu: bounded_vec![bounded_vec![1, 3], bounded_vec![4, 5]],
                sigma: bounded_vec![1],
								name: bounded_vec![1],
								u: bounded_vec![bounded_vec![1, 3], bounded_vec![4, 5]],
            }).map_err(|_| Error::<Test>::BoundedVecError)?;
            Ok(())
        }));

        assert_ok!(SegmentBook::verify_proof(RuntimeOrigin::signed(controller1.clone()), verify_result_list.clone()));
        assert_eq!(0, UnVerifyProof::<Test>::try_get(controller1.clone()).unwrap().len());

        let event = Sys::events().pop().expect("Expected at least one VerifyProof to be found").event;
        assert_eq!(mock::RuntimeEvent::from(Event::VerifyProof { miner: miner_acc, file_id: file_id }), event);
    });
}

#[test]
fn verify_proof_on_punish() {
    new_test_ext().execute_with(|| {
        let beneficiary = account::<mock::AccountId>("beneficiary", 0, 0);
        let stake_amount: u64 = 2_000_000;
        let ip = IpAddress::IPV4([127,0,0,1], 15000);
        assert_ok!(Sminer::regnstk(RuntimeOrigin::signed(miner1()), beneficiary, ip.clone(), stake_amount));
        let miner_acc = miner1();
        assert_ok!(Sminer::add_power(&miner_acc.clone(), 10_000));
        let mu = bounded_vec![bounded_vec![2_u8]];
        let sigma = bounded_vec![1_u8];
				let name = bounded_vec![5_u8];
				let u = bounded_vec![bounded_vec![3_u8, 3u8]];
        let controller1 = controller1();
        let stash1 = stash1();
        let mfi = MockingFileInfo::default();
        let acc1 = account1();
				let bucket_name = "cess-bucket".as_bytes().to_vec();

				assert_ok!(create_new_bucket(acc1, bucket_name.clone()));
        assert_ok!(upload_declaration_alias(acc1, "cess-book".as_bytes().to_vec(), mfi.file_hash, bucket_name.clone()));
        assert_ok!(Sminer::add_power(&miner_acc, 1_048_576 * 1024 * 50));
        assert_ok!(FileBank::buy_space(RuntimeOrigin::signed(acc1), 1));
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));
        assert_ok!(add_power_for_miner(controller1.clone(), miner_acc.clone()));
        assert_ok!(upload_file_alias(acc1.clone(), controller1.clone(), &mfi));

				let mut shard_id1: Vec<u8> = mfi.file_hash.0.to_vec();
				shard_id1.append("-001".as_bytes().to_vec().as_mut());
				let shard_id1: [u8; 68] = shard_id1.as_slice().try_into().unwrap();

        let mut prove_list: Vec<ProveInfo<Test>> = Vec::new();
        let challenge_info = ChallengeInfo::<Test>{
            file_type: DataType::File,
            file_id: mfi.file_hash,
						shard_id: shard_id1,
            file_size: 12,
            block_list: bounded_vec![1,2,3],
            random: bounded_vec![bounded_vec![1,3],bounded_vec![4,5]],
        };
        let prove_info = ProveInfo::<Test> {
            file_id: mfi.file_hash,
            miner_acc: miner_acc.clone(),
            challenge_info: challenge_info.clone(),
            mu,
            sigma,
						name,
						u,
        };
        prove_list.push(prove_info);

        let mut verify_result_list: Vec<VerifyResult<Test>> = Vec::new();
        let verify_result = VerifyResult::<Test> {
            miner_acc: miner_acc.clone(),
            file_id: mfi.file_hash,
						shard_id: shard_id1,
            result: false,
        };
        verify_result_list.push(verify_result);

        assert_ok!(ChallengeMap::<Test>::try_mutate(&miner_acc, |value| -> DispatchResult {
            value.try_push(challenge_info.clone()).map_err(|_| Error::<Test>::BoundedVecError)?;
            Ok(())
        }));

        assert_ok!(SegmentBook::submit_challenge_prove(RuntimeOrigin::signed(miner_acc.clone()), prove_list));
        assert_ok!(Sminer::add_reward_order1(&miner_acc, 1000000));
        //FIXME! the punish action is hard to test now, as it's depend concrete pallet: sminer. Suggest doing this use Trait instead.
        assert_ok!(SegmentBook::verify_proof(RuntimeOrigin::signed(controller1.clone()), verify_result_list));  // the last parameter indicate whether punish
        assert_eq!(0, UnVerifyProof::<Test>::try_get(controller1.clone()).unwrap().len());

        let event = Sys::events().pop().expect("Expected at least one VerifyProof to be found").event;
        assert_eq!(mock::RuntimeEvent::from(Event::VerifyProof { miner: miner_acc.clone(), file_id: mfi.file_hash }), event);
        <MinerTotalProof<Test>>::insert(&miner_acc, 1);
        <VerifyDuration<Test>>::put(10);
        <SegmentBook as Hooks<u64>>::on_initialize(10);
        Sys::set_block_number(11);

        let state = Sminer::get_miner_state(miner_acc.clone()).unwrap();
        assert_eq!(state, "frozen".as_bytes().to_vec());
    });
}
