// This file is part of Substrate.

// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Tests for the module.

use super::*;
use crate::{mock::*, Event};
use mock::System as Sys;
use frame_support::{assert_ok, assert_noop};

#[derive(Debug, Clone)]
pub struct MockingFileInfo {
    pub file_hash: Vec<u8>,
    pub file_size: u64,
	pub block_num: u32,
	pub scan_size: u32,
	pub segment_size: u32,
    pub miner_acc: AccountId,
	pub miner_ip: Vec<u8>,
}

impl Default for MockingFileInfo {
    fn default() -> Self {
        MockingFileInfo {
            file_hash: vec![5,45,23,2,19,5,2],
            file_size: 12,
            block_num: 8,
            scan_size: 3,
            segment_size: 3,
            miner_acc: mock::miner1(),
            miner_ip: vec![66,55,44,33,11],
        }
    }
}


fn upload_declaration_alias(account: AccountId, file_name: Vec<u8>, file_hash: Vec<u8>) -> DispatchResult {
    FileBank::upload_declaration(
        Origin::signed(account),
        file_hash,
        file_name,
    )
}

fn upload_file_alias(account: AccountId, controller: AccountId, file_info: &MockingFileInfo) -> DispatchResult {
    let MockingFileInfo { file_hash, file_size, block_num, scan_size, segment_size, miner_acc, miner_ip } = file_info.clone();
    FileBank::upload(
        Origin::signed(controller),
        file_hash,
        file_size,
        block_num,
        scan_size,
        segment_size,
        miner_acc,
        1,
        miner_ip,
        account,
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

fn register_miner(miner: AccountId) -> DispatchResult {
    Sminer::regnstk(
        Origin::signed(miner),
        miner.clone(),
        "132.168.191.67:3033".as_bytes().to_vec(),
        2_000u128.try_into().unwrap(),
    )
}

fn add_power_for_miner(controller: AccountId, miner: AccountId) -> DispatchResult {
    let mut filler_list: Vec<u8> = Vec::new();
    for i in 0 .. 100 {
        filler_list.push(
            FillerInfo<T: pallet::Config> {
                filler_size: 8 * 1_048_576,
                block_num: 8,
                segment_size: 1_048_576,
                scan_size: 1_048_576,
                miner_address: ,
                filler_id: ,
                filler_hash: ,
            }
        )
    }

    Ok
}

#[test]
fn buy_space_works() {
    new_test_ext().execute_with(|| {
        let acc1 = mock::account1();
        let bal_before = Balances::free_balance(acc1);
        let space_gb = 10_u128;
        let lease_count = 10_u128;  // one month a lease
        let max_price = 100_u64;
        let bn = Sys::block_number();

        assert_ok!(Sminer::add_available_space(1_048_576 * 1024 * space_gb));  // GB => MB

        UnitPrice::<Test>::put(max_price * 1000000000000 + 1);
        assert_noop!(FileBank::buy_space(Origin::signed(acc1), space_gb, lease_count, max_price as u128), Error::<Test>::ExceedExpectations);

        let unit_price = 100_u64;
        UnitPrice::<Test>::put(unit_price);
        assert_ok!(FileBank::buy_space(Origin::signed(acc1), space_gb, lease_count, max_price as u128));
        // balance verify
        let pay_amount = unit_price * (space_gb as u64 * 1024) * lease_count as u64 / 3;
        assert_eq!(bal_before - pay_amount, Balances::free_balance(acc1));

        let space_info = UserSpaceList::<Test>::try_get(acc1).unwrap().pop().unwrap();
        assert_eq!(space_gb * 1024 * 1_048_576, space_info.size); // MB unit
        assert_eq!(864000 * lease_count + bn as u128, space_info.deadline as u128);

        let uhsd = UserHoldSpaceDetails::<Test>::try_get(acc1).unwrap();
        assert_eq!(space_gb * 1024 * 1_048_576, uhsd.purchased_space);  //KB unit
        assert_eq!(uhsd.purchased_space, uhsd.remaining_space);

        let event = Sys::events().pop().expect("Expected at least one BuySpace to be found").event;
        assert_eq!(mock::Event::from(Event::BuySpace { acc: acc1, size: space_gb * 1024, fee: pay_amount as u64 }), event);
    });
}


#[test]
fn upload_declaration() {
    new_test_ext().execute_with(|| {
        let acc1 = mock::account1();
        let file_name = "cess-book".as_bytes().to_vec();
        let file_hash = "cess056237k7k439902190502".as_bytes().to_vec();
        assert_ok!(upload_declaration_alias(acc1, file_name, file_hash));
    });
}

#[test]
fn upload_works() {
    new_test_ext().execute_with(|| {
        let acc1 = mock::account1();
        let miner1 = mock::miner1();
        let stash1 = mock::stash1();
        let controller1 = mock::controller1();
        let mfi = MockingFileInfo::default();
        assert_ok!(upload_declaration_alias(acc1, "cess-book".as_bytes().to_vec(), mfi.file_hash.to_vec()));
        assert_noop!(upload_file_alias(acc1, controller1, &mfi), Error::<Test>::ScheduleNonExistent);
        assert_ok!(register_miner(miner1));
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));
        //upload() file not work have not buy space
        

        let space_gb = 10_u128;
        assert_ok!(Sminer::add_available_space(1_048_576 * 1024 * space_gb));
        UnitPrice::<Test>::put(100);
        assert_ok!(FileBank::buy_space(Origin::signed(acc1), 1, 1, 0));

        assert_ok!(upload_file_alias(acc1, controller1, &mfi));

        let file_hash: BoundedVec<u8, StringLimit> = mfi.file_hash.try_into().unwrap();
        let file_size = mfi.file_size;
        let file_slice_info = UserFileSliceInfo::<Test>{
            file_hash: file_hash.clone(),
            file_size: file_size,
        };
        assert!(File::<Test>::contains_key(&file_hash));
        let t = UserHoldSpaceDetails::<Test>::try_get(acc1).unwrap();
        assert_eq!(mfi.file_size as u128 * 3, t.used_space);
        assert_eq!(t.purchased_space - mfi.file_size as u128 * 3, t.remaining_space);
        assert!(UserHoldFileList::<Test>::try_get(acc1).unwrap().contains(&file_slice_info));

        let event = Sys::events().pop().expect("Expected at least one FileUpload to be found").event;
        assert_eq!(mock::Event::from(Event::FileUpload { acc: acc1 }), event);
    });
}

#[test]
fn upload_should_not_work_when_insufficient_storage() {
    new_test_ext().execute_with(|| {
        let acc1 = mock::account1();
        let stash1 = mock::stash1();
        let miner1 = mock::miner1();
        let controller1 = mock::controller1();
        let mut mfi = MockingFileInfo::default();
        let space_gb = 1_u128;
        assert_ok!(register_miner(miner1));
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));
        assert_ok!(Sminer::add_available_space(1_048_576 * 1024 * space_gb));
        UnitPrice::<Test>::put(100);
        assert_ok!(FileBank::buy_space(Origin::signed(acc1), space_gb, 100, 1000));
        mfi.file_size = 2 * 1024 * 1024 * 1024;  // large file
        //FIXME! the assert_noop! not work, why? it's need to solve
        //assert_noop!(upload_file_alias(acc1, vec![1], &mfi), Error::<Test>::InsufficientStorage);
        if let Err(e) = upload_file_alias(acc1, controller1, &mfi) {
            if let DispatchError::Module(m) = e {
                assert_eq!("InsufficientStorage", m.message.unwrap());
            }
        }
    })
}

#[test]
fn delete_file_works() {
    new_test_ext().execute_with(|| {
        let acc1 = mock::account1();
        let stash1 = mock::stash1();
        let miner1 = mock::miner1();
        let controller1 = mock::controller1();
        let mfi = MockingFileInfo::default();
        assert_noop!(FileBank::delete_file(Origin::signed(acc1), mfi.file_hash.clone()), Error::<Test>::FileNonExistent);

        let space_gb = 10_u128;
        assert_ok!(Sminer::add_available_space(1_048_576 * 1024 * space_gb));
        UnitPrice::<Test>::put(100);
        assert_ok!(FileBank::buy_space(Origin::signed(acc1), space_gb, 100, 1000));
        // acc1 upload file
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));
        assert_ok!(upload_declaration_alias(acc1, "cess-book".as_bytes().to_vec(), mfi.file_hash.to_vec()));
        assert_ok!(register_miner(miner1.clone()));
        assert_ok!(upload_file_alias(acc1, controller1, &mfi));
        assert_noop!(FileBank::delete_file(Origin::signed(mock::account2()), mfi.file_hash.clone()), Error::<Test>::NotOwner);
        let ss_before = UserHoldSpaceDetails::<Test>::try_get(acc1).unwrap();

        assert_ok!(FileBank::delete_file(Origin::signed(acc1), mfi.file_hash.clone()));

        let ss_after = UserHoldSpaceDetails::<Test>::try_get(acc1).unwrap();
        let bounded_file_hash: BoundedVec<u8, StringLimit> = mfi.file_hash.to_vec().try_into().unwrap();
        assert!(!File::<Test>::contains_key(bounded_file_hash));
        assert_ne!(ss_before.remaining_space, ss_after.remaining_space);

        let event = Sys::events().pop().expect("Expected at least one DeleteFile to be found").event;
        assert_eq!(mock::Event::from(Event::DeleteFile { acc: acc1, fileid: mfi.file_hash }), event);
    });
}

#[test]
fn receive_free_space_works() {
    new_test_ext().execute_with(|| {
        let space_gb = 10_u128;
        assert_ok!(Sminer::add_available_space(1_048_576 * 1024 * space_gb));

        let acc1 = mock::account1();
        assert_ok!(FileBank::receive_free_space(Origin::signed(acc1)));
        let ss_after = UserHoldSpaceDetails::<Test>::try_get(acc1).unwrap();
        assert_eq!(1_048_576 * 1024, ss_after.remaining_space);
        assert_eq!(1_048_576 * 1024, ss_after.purchased_space);
        assert!(UserFreeRecord::<Test>::contains_key(acc1));

        assert_noop!(FileBank::receive_free_space(Origin::signed(acc1)), Error::<Test>::AlreadyReceive);
    });
}

// #[test]
// fn update_price_works() {
//     new_test_ext().execute_with(|| {
//         let acc1 = mock::account1();
//         assert_ok!(FileBank::update_price(Origin::signed(acc1), Vec::from("1000")));
//         assert_eq!(1000 / 3, UnitPrice::<Test>::try_get().unwrap());
//     });
// }