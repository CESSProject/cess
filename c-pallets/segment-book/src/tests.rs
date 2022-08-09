// // This file is part of Substrate.

// // Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
// // SPDX-License-Identifier: Apache-2.0

// // Licensed under the Apache License, Version 2.0 (the "License");
// // you may not use this file except in compliance with the License.
// // You may obtain a copy of the License at
// //
// // 	http://www.apache.org/licenses/LICENSE-2.0
// //
// // Unless required by applicable law or agreed to in writing, software
// // distributed under the License is distributed on an "AS IS" BASIS,
// // WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// // See the License for the specific language governing permissions and
// // limitations under the License.

// //! Tests for the module.

use super::*;
use frame_support::{assert_ok, assert_noop};
use frame_benchmarking::account;
use crate::{mock::*, Event, Error, mock::System as Sys};
use pallet_file_bank::SliceInfo;

fn to_bounded_vec<T>(v: Vec<T>) -> BoundedVec<T, StringLimit> {
    let bv: BoundedVec<T, StringLimit> = v.try_into().unwrap();
    bv
}

fn to_bounded_vec2<T: Clone>(vv: Vec<Vec<T>>) -> BoundedVec<BoundedVec<T, StringLimit>, StringLimit> {
    let mut r: BoundedVec<BoundedVec<T, StringLimit>, StringLimit> = Vec::new().try_into().unwrap();
    // let mut t = &r;
    // vv.iter().map(|v| {
    //     t.push(to_bounded_vec::<T>(v.to_vec()))
    // });
    for v in vv {
        r.try_push(v.try_into().unwrap()).unwrap();
    }
    r
}

#[derive(Debug, Clone)]
pub struct MockingFileInfo {
    file_hash: Vec<u8>,
    file_size: u64,
    index: u32,
	file_state: Vec<u8>,
	file_name: Vec<u8>,
	slice_info: Vec<SliceInfo<Test>>,
}

impl Default for MockingFileInfo {
    fn default() -> Self {
        MockingFileInfo {
            file_hash: vec![5,45,23],
            file_size: 12,
            index: 1,
            file_state: "active".as_bytes().to_vec(),
            file_name: "testname".as_bytes().to_vec(),
            slice_info: vec![SliceInfo::<Test>{
                miner_id: 1,
                shard_size: 111,
                block_num: 8,
                shard_id: vec![5,45,23,2,19,5,1].try_into().ok().unwrap(),
                miner_ip: "192.168.1.1".as_bytes().to_vec().try_into().ok().unwrap(),
                miner_acc: mock::miner1(),
            },
            SliceInfo::<Test>{
                miner_id: 1,
                shard_size: 111,
                block_num: 8,
                shard_id: vec![5,45,23,2,19,5,2].try_into().ok().unwrap(),
                miner_ip: "192.168.1.1".as_bytes().to_vec().try_into().ok().unwrap(),
                miner_acc: mock::miner1(),
            },
            SliceInfo::<Test>{
                miner_id: 1,
                shard_size: 111,
                block_num: 8,
                shard_id: vec![5,45,23,2,19,5,3].try_into().ok().unwrap(),
                miner_ip: "192.168.1.1".as_bytes().to_vec().try_into().ok().unwrap(),
                miner_acc: mock::miner1(),
            },
            ]
        }
    }
}

fn add_power_for_miner(controller: AccountId, miner: AccountId) -> DispatchResult {
    let max: u8 = 10;
    let mut filler_list: Vec<pallet_file_bank::FillerInfo<Test>> = Vec::new();
    let filler_hash: BoundedVec<u8, StringLimit> = "hash".as_bytes().to_vec().try_into().unwrap();
    for i in 0 .. max {
        let filler_id: BoundedVec<u8, StringLimit> = i.to_string().as_bytes().to_vec().try_into().unwrap();
        filler_list.push(
            pallet_file_bank::FillerInfo::<Test> {
                filler_size: 8 * 1_048_576,
                index: 0,
                block_num: 8,
                segment_size: 1_048_576,
                scan_size: 1_048_576,
                miner_address: miner.clone(),
                filler_id: filler_id,
                filler_hash: filler_hash.clone(),
            }
        )
    }
    FileBank::upload_filler(Origin::signed(controller), miner, filler_list)?;

    Ok(())
}

fn upload_declaration_alias(account: AccountId, file_name: Vec<u8>, file_hash: Vec<u8>) -> DispatchResult {
    FileBank::upload_declaration(
        Origin::signed(account),
        file_hash,
        file_name,
    )
}

fn upload_file_alias(account: AccountId, controller: AccountId, file_info: &MockingFileInfo) -> DispatchResult {
    let MockingFileInfo { file_hash, file_size, index, file_state, file_name, slice_info } = file_info.clone();
    FileBank::upload(
        Origin::signed(controller),
        file_hash,
        file_size,
        slice_info,
        account.clone(),
    )
}

fn register_scheduler(stash: AccountId, controller: AccountId) -> DispatchResult {
    pallet_cess_staking::Bonded::<Test>::insert(&stash, controller.clone());
    FileMap::registration_scheduler(
        Origin::signed(controller),
        stash, 
        "132.168.191.67:3033".as_bytes().to_vec(),
    )

}

#[test]
fn submit_challenge_prove_works() {
    new_test_ext().execute_with(|| {
        let miner_acc = miner1();
        let file_id = vec![1_u8];
        let mu = vec![vec![2_u8]];
        let sigma = vec![1_u8];
        let controller1 = controller1();
        let stash1 = stash1(); 
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));

        let mut prove_list: Vec<ProveInfo<Test>> = Vec::new();
        let challenge_info = ChallengeInfo::<Test>{
            file_type: 1,
            file_id: to_bounded_vec(file_id.clone()),
            file_size: 1024,
            block_list: to_bounded_vec(vec![1,2,3]),
            random: to_bounded_vec2(vec![vec![1,3],vec![4,5]]),
        };
        let prove_info = ProveInfo::<Test> {
            file_id: to_bounded_vec(file_id.clone()),
            miner_acc: miner_acc.clone(),
            challenge_info: challenge_info,
            mu: to_bounded_vec2(mu),
            sigma: to_bounded_vec(sigma),
        };
        prove_list.push(prove_info);
        assert_noop!(SegmentBook::submit_challenge_prove(Origin::signed(miner_acc.clone()), prove_list.clone()), Error::<Test>::NoChallenge);
        assert_ok!(ChallengeMap::<Test>::try_mutate(miner_acc, |value| -> DispatchResult {
            let challenge_info = ChallengeInfo::<Test>{
                file_size: 1024,
                file_type: 1,
                block_list: to_bounded_vec(vec![1,2,3]),
                file_id: to_bounded_vec(file_id.clone()),
                random: to_bounded_vec2(vec![vec![1,3],vec![4,5]]),
            };
            value.try_push(challenge_info).map_err(|_| Error::<Test>::BoundedVecError)?;
            Ok(())
        }));

        assert_ok!(SegmentBook::submit_challenge_prove(Origin::signed(miner_acc.clone()), prove_list));

        assert!(UnVerifyProof::<Test>::contains_key(&controller1));
        let prove_info = UnVerifyProof::<Test>::try_get(&controller1).unwrap().pop().unwrap();
        assert_eq!(miner_acc, prove_info.miner_acc);
        assert_eq!(0, ChallengeMap::<Test>::try_get(miner_acc).unwrap().len());

        let event = Sys::events().pop().expect("Expected at least one ChallengeProof to be found").event;
        assert_eq!(mock::Event::from(Event::ChallengeProof { miner : miner_acc, file_id: file_id }), event);
    });
}

#[test]
fn verify_proof_works() {
    new_test_ext().execute_with(|| {
        let miner_acc = miner1();
        let file_id = vec![1_u8];
        let controller1 = controller1();
        let stash1 = stash1(); 
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));

        let challenge_info = ChallengeInfo::<Test>{
            file_type: 1,
            file_id: to_bounded_vec(file_id.clone()),
            file_size: 1024,
            block_list: to_bounded_vec(vec![1,2,3]),
            random: to_bounded_vec2(vec![vec![1,3],vec![4,5]]),
        };

        let mut verify_result_list: Vec<VerifyResult<Test>> = Vec::new();
        let verify_result = VerifyResult::<Test> {
            miner_acc: miner_acc.clone(),
            file_id: to_bounded_vec(file_id.clone()),
            result: true,
        };
        verify_result_list.push(verify_result);

        assert_ok!(UnVerifyProof::<Test>::try_mutate(controller1, |value| -> DispatchResult {
            value.try_push(ProveInfo {
                file_id: to_bounded_vec(file_id.clone()),
                miner_acc,
                challenge_info,
                mu: to_bounded_vec2(vec![vec![1, 3], vec![4, 5]]),
                sigma: to_bounded_vec(vec![1]),
            }).map_err(|_| Error::<Test>::BoundedVecError)?;
            Ok(())
        }));

        assert_ok!(SegmentBook::verify_proof(Origin::signed(controller1.clone()), verify_result_list.clone()));
        assert_eq!(0, UnVerifyProof::<Test>::try_get(controller1.clone()).unwrap().len());

        let event = Sys::events().pop().expect("Expected at least one VerifyProof to be found").event;
        assert_eq!(mock::Event::from(Event::VerifyProof { miner: miner_acc, file_id: file_id }), event);
    });
}

#[test]
fn verify_proof_on_punish() {
    new_test_ext().execute_with(|| {
        let beneficiary = account::<mock::AccountId>("beneficiary", 0, 0);
        let stake_amount: u64 = 2_000_000;
        let ip: Vec<u8> = Vec::from("192.168.0.1");
        assert_ok!(Sminer::regnstk(Origin::signed(miner1()), beneficiary, ip.clone(), stake_amount));
        let miner_acc = miner1();
        assert_ok!(Sminer::add_power(&miner_acc.clone(), 10_000));
        let file_id = vec![5,45,23,2,19,5,1];
        let mu = vec![vec![2_u8]];
        let sigma = vec![1_u8];
        let controller1 = controller1();
        let stash1 = stash1(); 
        let mfi = MockingFileInfo::default();
        let acc1 = account1();

        assert_ok!(upload_declaration_alias(acc1, "cess-book".as_bytes().to_vec(), mfi.file_hash.to_vec()));
        assert_ok!(Sminer::add_power(&miner_acc, 1_048_576 * 1024 * 50));
        assert_ok!(FileBank::update_price_for_tests());
        assert_ok!(FileBank::buy_package(Origin::signed(acc1), 1, 0));
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));
        assert_ok!(add_power_for_miner(controller1.clone(), miner_acc.clone()));
        assert_ok!(upload_file_alias(acc1.clone(), controller1.clone(), &mfi));

        

        let mut prove_list: Vec<ProveInfo<Test>> = Vec::new();
        let challenge_info = ChallengeInfo::<Test>{
            file_type: 2,
            file_id: to_bounded_vec(file_id.clone()),
            file_size: 12,
            block_list: to_bounded_vec(vec![1,2,3]),
            random: to_bounded_vec2(vec![vec![1,3],vec![4,5]]),
        };
        let prove_info = ProveInfo::<Test> {
            file_id: to_bounded_vec(file_id.clone()),
            miner_acc: miner_acc.clone(),
            challenge_info: challenge_info.clone(),
            mu: to_bounded_vec2(mu),
            sigma: to_bounded_vec(sigma),
        };
        prove_list.push(prove_info);

        let mut verify_result_list: Vec<VerifyResult<Test>> = Vec::new();
        let verify_result = VerifyResult::<Test> {
            miner_acc: miner_acc.clone(),
            file_id: to_bounded_vec(file_id.clone()),
            result: false,
        };
        verify_result_list.push(verify_result);

        assert_ok!(ChallengeMap::<Test>::try_mutate(&miner_acc, |value| -> DispatchResult {
            value.try_push(challenge_info.clone()).map_err(|_| Error::<Test>::BoundedVecError)?;
            Ok(())
        }));

        assert_ok!(SegmentBook::submit_challenge_prove(Origin::signed(miner_acc.clone()), prove_list));
        assert_ok!(Sminer::add_reward_order1(&miner_acc, 1000000));
        //FIXME! the punish action is hard to test now, as it's depend concrete pallet: sminer. Suggest doing this use Trait instead.
        assert_ok!(SegmentBook::verify_proof(Origin::signed(controller1.clone()), verify_result_list));  // the last parameter indicate whether punish
        assert_eq!(0, UnVerifyProof::<Test>::try_get(controller1.clone()).unwrap().len());
 
        let event = Sys::events().pop().expect("Expected at least one VerifyProof to be found").event;
        assert_eq!(mock::Event::from(Event::VerifyProof { miner: miner_acc.clone(), file_id: file_id }), event);
        <MinerTotalProof<Test>>::insert(&miner_acc, 1);
        <VerifyDuration<Test>>::put(10);
        <SegmentBook as Hooks<u64>>::on_initialize(10);
        Sys::set_block_number(11);

        if_std! { println!("miner_acc {:#?}", miner_acc.clone()) }

        let state = Sminer::get_miner_state(miner_acc.clone()).unwrap();
        assert_eq!(state, "frozen".as_bytes().to_vec());
    });
}