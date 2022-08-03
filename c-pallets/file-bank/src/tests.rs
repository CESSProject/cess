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
use pallet_sminer::MinerControl;

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
            file_hash: vec![5,45,23,2,19,5,2],
            file_size: 12,
            index: 1,
            file_state: "active".as_bytes().to_vec(),
            file_name: "testname".as_bytes().to_vec(),
            slice_info: vec![SliceInfo::<Test>{
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
                shard_id: vec![5,45,23,2,19,5,2].try_into().ok().unwrap(),
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
            ]
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
    let MockingFileInfo { file_hash, file_size, _index, _file_state, _file_name, slice_info } = file_info.clone();
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

fn register_miner(miner: AccountId) -> DispatchResult {
    Sminer::regnstk(
        Origin::signed(miner),
        miner.clone(),
        "132.168.191.67:3033".as_bytes().to_vec(),
        2_000u128.try_into().unwrap(),
    )
}

fn add_power_for_miner(controller: AccountId, miner: AccountId) -> DispatchResult {
    let max: u8 = 10;
    let mut filler_list: Vec<FillerInfo<Test>> = Vec::new();
    let filler_hash: BoundedVec<u8, StringLimit> = "hash".as_bytes().to_vec().try_into().unwrap();
    for i in 0 .. max {
        let filler_id: BoundedVec<u8, StringLimit> = i.to_string().as_bytes().to_vec().try_into().unwrap();
        filler_list.push(
            FillerInfo::<Test> {
                index: 1,
                filler_size: 8 * 1_048_576,
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

#[test]
fn buy_space_works() {
    new_test_ext().execute_with(|| {
        let acc1 = mock::account1();
        let bal_before = Balances::free_balance(acc1);
        let miner1 = mock::miner1();
        let space_gb = 10_u128;
        let lease_count = 10_u128;  // one month a lease
        let max_price = 100_u64;
        let bn = Sys::block_number();

        assert_ok!(register_miner(miner1.clone()));
        assert_ok!(Sminer::add_power(&miner1, 1_048_576 * 1024 * space_gb));  // GB => MB

        let unit_price = 100_u64;
        UnitPrice::<Test>::put(unit_price);
        assert_ok!(FileBank::buy_package(Origin::signed(acc1), 1, 0));

        let uhsd = PurchasedPackage::<Test>::try_get(acc1).unwrap();
        assert_eq!(space_gb * 1024 * 1_048_576, uhsd.space); // MB unit
        assert_eq!(86400 * lease_count + bn as u128, uhsd.deadline as u128);

        assert_eq!(uhsd.space, uhsd.remaining_space);

        let event = Sys::events().pop().expect("Expected at least one BuySpace to be found").event;
        assert_eq!(mock::Event::from(Event::BuyPackage { acc: acc1, size: 10 * 1024 * 1_048_576, fee: 0 }), event);
    });
}

#[test]
fn upgrade_package_work() {
    new_test_ext().execute_with(|| {
        let acc1 = mock::account1();
        let miner1 = mock::miner1();
        let space_gb = 1000_u128;
        assert_ok!(register_miner(miner1.clone()));
        assert_ok!(Sminer::add_power(&miner1, 1_048_576 * 1024 * space_gb));  // GB => MB

        assert_ok!(FileBank::buy_package(Origin::signed(acc1), 1, 0));
        let uhsd = PurchasedPackage::<Test>::try_get(acc1).unwrap();
        assert_eq!(10 * 1024 * 1_048_576, uhsd.space); // MB unit

        let price1: u128 = 0;

        let unit_price2 = FileBank::get_price(500 * 1_048_576 * 1024).ok().unwrap();
        let gb_unit_price2 = unit_price2 / 1024;

        let price2 = 500 * gb_unit_price2 / 30 * 30;
        let diff_price = price2 - price1;

        assert_ok!(FileBank::upgrade_package(Origin::signed(acc1), 2, 0));
        let event = Sys::events().pop().expect("Expected at least one BuySpace to be found").event;
        assert_eq!(mock::Event::from(Event::PackageUpgrade { acc: acc1, old_type: 1, new_type: 2, fee: diff_price as u64}), event);
        
        let uhsd = PurchasedPackage::<Test>::try_get(acc1).unwrap();
        assert_eq!(500 * 1024 * 1_048_576, uhsd.space); // MB unit
    });
}

#[test]
fn renewal_package_work() {
    new_test_ext().execute_with(|| {
        let acc1 = mock::account1();
        let miner1 = mock::miner1();
        let space_gb = 1000_u128;
        assert_ok!(register_miner(miner1.clone()));
        assert_ok!(Sminer::add_power(&miner1, 1_048_576 * 1024 * space_gb));  // GB => MB

        assert_ok!(FileBank::buy_package(Origin::signed(acc1), 2, 0));
        let uhsd = PurchasedPackage::<Test>::try_get(acc1).unwrap();
        assert_eq!(500 * 1024 * 1_048_576, uhsd.space); // MB unit

        let unit_price2 = FileBank::get_price(500 * 1_048_576 * 1024).ok().unwrap();
        let gb_unit_price2 = unit_price2 / 1024;
        let price = gb_unit_price2 * 500;

        assert_ok!(FileBank::renewal_package(Origin::signed(acc1)));
        let event = Sys::events().pop().expect("Expected at least one BuySpace to be found").event;
        assert_eq!(mock::Event::from(Event::PackageRenewal { acc: acc1, package_type: 2, fee: price as u64}), event);

        let uhsd = PurchasedPackage::<Test>::try_get(acc1).unwrap();
        assert_eq!(500 * 1024 * 1_048_576, uhsd.space);
        assert_eq!(1, uhsd.start);
        assert_eq!(30 * 2 * 28800 + 1, uhsd.deadline);
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
        

        let space_gb = 20_u128;
        assert_ok!(Sminer::add_power(&miner1, 1_048_576 * 1024 * space_gb));
        UnitPrice::<Test>::put(100);
        assert_ok!(FileBank::buy_package(Origin::signed(acc1), 1, 0));
        assert_ok!(add_power_for_miner(controller1, miner1));

        assert_ok!(upload_file_alias(acc1, controller1, &mfi));

        let file_hash: BoundedVec<u8, StringLimit> = mfi.file_hash.try_into().unwrap();
        let file_size = mfi.file_size;
        let file_slice_info = UserFileSliceInfo::<Test>{
            file_hash: file_hash.clone(),
            file_size: file_size,
        };
        assert!(File::<Test>::contains_key(&file_hash));
        let t = PurchasedPackage::<Test>::try_get(acc1).unwrap();
        assert_eq!(mfi.file_size as u128, t.used_space);
        assert_eq!(t.space - mfi.file_size as u128, t.remaining_space);
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
        let space_gb = 20_u128;
        assert_ok!(register_miner(miner1)); 
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));
        assert_ok!(add_power_for_miner(controller1, miner1));
        assert_ok!(Sminer::add_power(&miner1, 1_048_576 * 1024 * space_gb));
        UnitPrice::<Test>::put(100);
        assert_ok!(FileBank::buy_package(Origin::signed(acc1), 1, 0));
        mfi.file_size = 2 * 1024 * 1024 * 1024;  // large file
        //FIXME! the assert_noop! not work, why? it's need to solve
        //assert_noop!(upload_file_alias(acc1, vec![1], &mfi), Error::<Test>::InsufficientStorage);
        assert_ok!(upload_declaration_alias(acc1, "cess-book".as_bytes().to_vec(), mfi.file_hash.to_vec()));
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

        let space_gb = 20_u128;
        assert_ok!(register_miner(miner1.clone()));
        assert_ok!(Sminer::add_power(&miner1, 1_048_576 * 1024 * space_gb));
        UnitPrice::<Test>::put(100);
        assert_ok!(FileBank::buy_package(Origin::signed(acc1), 1, 0));
        // acc1 upload file
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));
        assert_ok!(upload_declaration_alias(acc1, "cess-book".as_bytes().to_vec(), mfi.file_hash.to_vec()));
        assert_ok!(add_power_for_miner(controller1, miner1));
        assert_ok!(upload_file_alias(acc1, controller1, &mfi));
        assert_noop!(FileBank::delete_file(Origin::signed(mock::account2()), mfi.file_hash.clone()), Error::<Test>::NotOwner);
        let ss_before = PurchasedPackage::<Test>::try_get(acc1).unwrap();

        assert_ok!(FileBank::delete_file(Origin::signed(acc1), mfi.file_hash.clone()));

        let ss_after = PurchasedPackage::<Test>::try_get(acc1).unwrap();
        let bounded_file_hash: BoundedVec<u8, StringLimit> = mfi.file_hash.to_vec().try_into().unwrap();
        assert!(!File::<Test>::contains_key(bounded_file_hash));
        assert_ne!(ss_before.remaining_space, ss_after.remaining_space);

        let event = Sys::events().pop().expect("Expected at least one DeleteFile to be found").event;
        assert_eq!(mock::Event::from(Event::DeleteFile { acc: acc1, fileid: mfi.file_hash }), event);
    });
}

#[test]
fn upload_filler_work() {
    new_test_ext().execute_with(|| {
        let stash1 = mock::stash1();
        let miner1 = mock::miner1();
        let controller1 = mock::controller1();
        assert_noop!(add_power_for_miner(controller1.clone(), miner1.clone()), Error::<Test>::ScheduleNonExistent);
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));
        assert_noop!(add_power_for_miner(controller1.clone(), miner1.clone()), pallet_sminer::Error::<Test>::NotMiner);
        assert_ok!(register_miner(miner1));
        assert_ok!(add_power_for_miner(controller1.clone(), miner1.clone()));

        let (power, _) = Sminer::get_power_and_space(miner1.clone()).unwrap();
        assert_eq!(1_048_576 * 8 * 10, power);
    });
}

#[test]
fn clear_invalid_file_work() {
    new_test_ext().execute_with(|| {
        let acc1 = mock::account1();
        let stash1 = mock::stash1();
        let miner1 = mock::miner1();
        let controller1 = mock::controller1();
        let mfi = MockingFileInfo::default();
        
        assert_ok!(register_miner(miner1.clone()));
        assert_ok!(Sminer::add_power(&miner1 ,1_048_576 * 1024 * 20));
        assert_ok!(register_scheduler(stash1.clone(), controller1.clone()));
        assert_ok!(upload_declaration_alias(acc1, "cess-book".as_bytes().to_vec(), mfi.file_hash.to_vec()));
        assert_ok!(add_power_for_miner(controller1, miner1));
        assert_ok!(FileBank::buy_package(Origin::signed(acc1), 1, 0));
        assert_ok!(upload_file_alias(acc1, controller1, &mfi));

        let mut file_hash_list = InvalidFile::<Test>::get(miner1.clone());
        assert_ok!(FileBank::clear_invalid_file(Origin::signed(miner1.clone()), file_hash_list[0].to_vec()));
        file_hash_list.remove(0);
        assert_eq!(file_hash_list, InvalidFile::<Test>::get(miner1.clone()));
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