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

#[test]
fn submit_challenge_prove_works() {
    new_test_ext().execute_with(|| {
        let miner_id = 1_u64;
        let file_id = vec![1_u8];
        let mu = vec![vec![2_u8]];
        let sigma = vec![1_u8];

        assert_noop!(SegmentBook::submit_challenge_prove(Origin::signed(ACCOUNT1.0), miner_id, file_id.clone(), mu.clone(), sigma.clone()), Error::<Test>::NoChallenge);
        assert_ok!(ChallengeMap::<Test>::try_mutate(miner_id, |value| -> DispatchResult {
            let list: Vec<u8> = vec![1];
            let challenge_info = ChallengeInfo::<Test>{
                file_type: 1,
                file_id: to_bounded_vec(file_id.clone()),
                file_size: 1024,
                segment_size: 256,
                block_list: to_bounded_vec2(vec![list]),
                random: to_bounded_vec2(vec![vec![1,3],vec![4,5]]),
            };
            value.try_push(challenge_info).map_err(|_| Error::<Test>::BoundedVecError)?;
            Ok(())
        }));

        assert_ok!(SegmentBook::submit_challenge_prove(Origin::signed(ACCOUNT1.0), miner_id, file_id.clone(), mu, sigma));

        assert!(UnVerifyProof::<Test>::contains_key(&ACCOUNT1.0));
        let prove_info = UnVerifyProof::<Test>::try_get(ACCOUNT1.0).unwrap().pop().unwrap();
        assert_eq!(miner_id, prove_info.miner_id);
        assert_eq!(0, ChallengeMap::<Test>::try_get(miner_id).unwrap().len());

        let event = Sys::events().pop().expect("Expected at least one ChallengeProof to be found").event;
        assert_eq!(mock::Event::from(Event::ChallengeProof { peer_id: miner_id, file_id: file_id }), event);
    });
}

#[test]
fn verify_proof_works() {
    new_test_ext().execute_with(|| {
        let miner_id = 1_u64;
        let file_id = vec![1_u8];

        assert_noop!(SegmentBook::verify_proof(Origin::signed(ACCOUNT1.0), miner_id, file_id.clone(), true), Error::<Test>::NonProof);
        let challenge_info = ChallengeInfo::<Test> {
            file_type: 1,
            file_id: to_bounded_vec(file_id.clone()),
            file_size: 1024,
            segment_size: 256,
            block_list: to_bounded_vec2(vec![vec![10]]),
            random: to_bounded_vec2(vec![vec![10, 3], vec![4, 5]]),
        };

        assert_ok!(UnVerifyProof::<Test>::try_mutate(ACCOUNT1.0, |value| -> DispatchResult {
            value.try_push(ProveInfo {
                miner_id,
                challenge_info,
                mu: to_bounded_vec2(vec![vec![1, 3], vec![4, 5]]),
                sigma: to_bounded_vec(vec![1]),
            }).map_err(|_| Error::<Test>::BoundedVecError)?;
            Ok(())
        }));

        assert_ok!(SegmentBook::verify_proof(Origin::signed(ACCOUNT1.0), miner_id, file_id.clone(), true));
        assert_eq!(0, UnVerifyProof::<Test>::try_get(ACCOUNT1.0).unwrap().len());

        let event = Sys::events().pop().expect("Expected at least one VerifyProof to be found").event;
        assert_eq!(mock::Event::from(Event::VerifyProof { peer_id: miner_id, file_id: file_id }), event);
    });
}

#[test]
fn verify_proof_on_punish() {
    new_test_ext().execute_with(|| {
        let beneficiary = account::<mock::AccountId>("beneficiary", 0, 0);
        let stake_amount: u128 = 2000;
        let ip: Vec<u8> = Vec::from("192.168.0.1");
        assert_ok!(Sminer::regnstk(Origin::signed(ACCOUNT1.0), beneficiary, ip.clone(), stake_amount, Vec::from("public_key")));
        let miner_id = Sminer::get_peerid(&ACCOUNT1.0);
        assert_ok!(Sminer::add_power(miner_id, 10_000));
        let file_id = vec![1_u8];
        let mu = vec![vec![2_u8]];
        let sigma = vec![1_u8];
        let challenge_info = ChallengeInfo::<Test> {
            file_type: 1,
            file_id: to_bounded_vec(file_id.clone()),
            file_size: 1024,
            segment_size: 256,
            block_list: to_bounded_vec2(vec![vec![10]]),
            random: to_bounded_vec2(vec![vec![10, 5], vec![4, 5]]),
        };

        assert_ok!(ChallengeMap::<Test>::try_mutate(miner_id, |value| -> DispatchResult {
            value.try_push(challenge_info.clone()).map_err(|_| Error::<Test>::BoundedVecError)?;
            Ok(())
        }));

        assert_ok!(SegmentBook::submit_challenge_prove(Origin::signed(ACCOUNT1.0), miner_id, file_id.clone(), mu, sigma));

        //FIXME! the punish action is hard to test now, as it's depend concrete pallet: sminer. Suggest doing this use Trait instead.
        assert_ok!(SegmentBook::verify_proof(Origin::signed(ACCOUNT1.0), miner_id, file_id.clone(), false));  // the last parameter indicate whether punish
        assert_eq!(0, UnVerifyProof::<Test>::try_get(ACCOUNT1.0).unwrap().len());

        let event = Sys::events().pop().expect("Expected at least one VerifyProof to be found").event;
        assert_eq!(mock::Event::from(Event::VerifyProof { peer_id: miner_id, file_id: file_id }), event);
    });
}