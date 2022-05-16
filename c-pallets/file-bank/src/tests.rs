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

// use super::*;
// use crate::{mock::*, Error};
// use frame_support::{assert_ok, assert_noop};
// use mock::{
// 	new_test_ext, Origin, FileBank,
// };
// use frame_benchmarking::account;
// use pallet_sminer::Error as OtherError;
// pub struct FileInfoTest {
// 	filename: Vec<u8>,
// 	filehash: Vec<u8>,
// 	backups: u8,
// 	filesize: u128,
// }

// #[test]
// fn upload_works() {
// 	new_test_ext().execute_with(|| {
// 		let fileid = vec![1];
// 		let address = vec![1];
// 		let downloadfee = 1;
// 		let fit = create();

// 		init();
// 		add_space();
// 		assert_ok!(FileBank::buy_space(Origin::signed(1), 1, 0));
// 		assert_eq!(File::<Test>::contains_key(&fileid), false);

// 		assert_ok!(FileBank::upload(
// 			Origin::signed(1),
// 			address,
// 			fit.filename,
// 			fileid.clone(),
// 			fit.filehash,
// 			fit.backups,
// 			fit.filesize,
// 			downloadfee
// 		));

// 		assert_eq!(File::<Test>::contains_key(&fileid), true);
// 	});
// }

// #[test]
// fn upload_works_when_not_buy_space() {
// 	new_test_ext().execute_with(|| {
// 		let fileid = vec![1];
// 		let address = vec![1];
// 		let downloadfee = 1;
// 		let fit = create();

// 		assert_noop!(
// 			FileBank::upload(
// 				Origin::signed(1), 
// 				address, 
// 				fit.filename, 
// 				fileid.clone(),
// 				fit.filehash,
// 				fit.backups,
// 				fit.filesize,
// 				downloadfee
// 			),
// 			Error::<Test>::NotPurchasedSpace
// 		);
// 	})
// }

// #[test]
// fn upload_works_when_insufficient_storage() {
// 	new_test_ext().execute_with(|| {
// 		let fileid = vec![1];
// 		let address = vec![1];
// 		let downloadfee = 1;
// 		let fit = create();

// 		init();
// 		add_space();
// 		assert_ok!(FileBank::buy_space(Origin::signed(1), 1, 0));
// 		assert_eq!(File::<Test>::contains_key(&fileid), false);

// 		assert_noop!(
// 			FileBank::upload(
// 				Origin::signed(1),
// 				address,
// 				fit.filename,
// 				fileid.clone(),
// 				fit.filehash,
// 				fit.backups,
// 				171 * 1024,
// 				downloadfee
// 			),
// 			Error::<Test>::InsufficientStorage
// 		);
// 	})
// }

// #[test]
// fn buy_space_works() {
// 	new_test_ext().execute_with(|| {
// 		init();
// 		add_space();
// 		assert_ok!(FileBank::buy_space(Origin::signed(1), 1, 0));
// 	})
// }

// #[test]
// fn buy_space_works_when_exceed() {
// 	new_test_ext().execute_with(|| {
// 		init();
// 		add_space();
// 		assert_noop!(
// 			FileBank::buy_space(Origin::signed(1), 1, 1),
// 			Error::<Test>::ExceedExpectations
// 		);
// 	})
// }

// #[test]
// fn buy_space_works_when_insufficient_space() {
// 	new_test_ext().execute_with(|| {
// 		init();
// 		add_space();
// 		assert_noop!(
// 			FileBank::buy_space(Origin::signed(1), 2, 0),
// 			OtherError::<Test>::InsufficientAvailableSpace
// 		);
// 	})
// }

// #[test]
// fn buy_file_work() {
// 	new_test_ext().execute_with(|| {
// 		let fileid = vec![1];
// 		let address = vec![1];
// 		let address2 = vec![2];
// 		let downloadfee = 1;
// 		let fit = create();

// 		init();
// 		add_space();
// 		assert_ok!(FileBank::buy_space(Origin::signed(1), 1, 0));
// 		assert_eq!(File::<Test>::contains_key(&fileid), false);

// 		assert_ok!(FileBank::upload(
// 			Origin::signed(1),
// 			address,
// 			fit.filename,
// 			fileid.clone(),
// 			fit.filehash,
// 			fit.backups,
// 			fit.filesize,
// 			downloadfee
// 		));

// 		assert_eq!(File::<Test>::contains_key(&fileid), true);

// 		assert_ok!(FileBank::buyfile(Origin::signed(2), fileid, address2));
// 	})
// }

// #[test]
// fn buy_file_when_file_none_exis() {
// 	new_test_ext().execute_with(|| {
// 		let fileid = vec![1];
// 		let address = vec![1];
// 		let address2 = vec![2];
// 		let downloadfee = 1;
// 		let fit = create();
		
// 		assert_noop!(
// 			FileBank::buyfile(Origin::signed(2), [2].to_vec(), address2),
// 			Error::<Test>::FileNonExistent
// 		);
// 	})
// }

// pub fn create() -> FileInfoTest {
// 	FileInfoTest {
// 		filename: vec![1],
// 		filehash: vec![1],
// 		backups: 3,
// 		filesize: 1,
// 	}
// }

// //Registered miners to submit certificates
// pub fn init() {
// 	assert_ok!(Sminer::initi(Origin::root()));
// 	assert_ok!(Sminer::regnstk(Origin::signed(1),account("source", 0, 0),0,0,0,0));
// }
// //Only when the miner submits the certificate can the available space 
// //be increased and later purchased by the user
// pub fn add_space() {
// 	let value: Vec<u8> = [0].to_vec();
// 	let value1: Vec<u8> = [0].to_vec();
// 	let value2: Vec<u8> = [0].to_vec();
// 	let value3: Vec<u8> = [0].to_vec();
// 	let value4: Vec<u8> = [0].to_vec();
// 	let mut v: Vec<Vec<u8>> = vec![];
// 	v.push(value);
// 	assert_ok!(SegmentBook::intent_submit(Origin::signed(1), 2, 1, 1, v, value1, value4));
// 	assert_ok!(SegmentBook::submit_to_vpa(Origin::signed(1), 1, 1, value2, value3));
// 	assert_ok!(SegmentBook::verify_in_vpa(Origin::signed(1), 1, 1, true));
// }