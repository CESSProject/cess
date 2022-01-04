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
use crate::{mock::*, Error};
use frame_support::{assert_ok, assert_noop};
use mock::{
	new_test_ext, Origin, FileBank,
};

pub struct FileInfoTest {
	filename: Vec<u8>,
	filehash: Vec<u8>,
	similarityhash: Vec<u8>,
	ispublic: u8,
	backups: u8,
	creator: Vec<u8>,
	filesize: u128,
	keywords: Vec<u8>,
	email: Vec<u8>,
	deadline: u128,
	
}

#[test]
fn upload_works() {
	new_test_ext().execute_with(|| {
		let fileid = vec![1];
		let address = vec![1];
		let uploadfee = 1;
		let downloadfee = 1;
		let fit = create();

		assert_eq!(File::<Test>::contains_key(&fileid), false);

		assert_ok!(FileBank::upload(
			Origin::signed(1),
			fit.filename,
			address,
			fileid.clone(),
			fit.filehash,
			fit.similarityhash,
			fit.ispublic,
			fit.backups,
			fit.creator,
			fit.filesize,
			fit.keywords,
			fit.email,
			uploadfee,
			downloadfee,
			fit.deadline
		));

		assert_eq!(File::<Test>::contains_key(&fileid), true);
	});
}

#[test]
fn upload_transfer_works() {
	new_test_ext().execute_with(|| {
		let fileid = vec![1];
		let address = vec![1];
		let uploadfee = 10;
		let downloadfee = 1;
		let fit = create();

		assert_eq!(Balances::free_balance(&2), 100);

		assert_ok!(FileBank::upload(
			Origin::signed(2),
			fit.filename,
			address,
			fileid.clone(),
			fit.filehash,
			fit.similarityhash,
			fit.ispublic,
			fit.backups,
			fit.creator,
			fit.filesize,
			fit.keywords,
			fit.email,
			uploadfee,
			downloadfee,
			fit.deadline
		));

		assert_eq!(Balances::free_balance(&2), 90);
	});
}

#[test]
fn update_works() {
	new_test_ext().execute_with(|| {
		let fileid = vec![1];
		let address = vec![1];
		let uploadfee = 10;
		let downloadfee = 1;
		let fit = create();

		assert_ok!(FileBank::upload(
			Origin::signed(2),
			fit.filename,
			address,
			fileid.clone(),
			fit.filehash,
			fit.similarityhash,
			fit.ispublic,
			fit.backups,
			fit.creator,
			fit.filesize,
			fit.keywords,
			fit.email,
			uploadfee,
			downloadfee,
			fit.deadline
		));

		let mut file_info = File::<Test>::get(&fileid).unwrap();
		assert_eq!(file_info.ispublic, 0);
		assert_eq!(file_info.similarityhash, vec![1]);

		let new_ispublic = 1;
		let new_similarityhash = vec![2];

		assert_ok!(FileBank::update(Origin::signed(2), fileid.clone(), new_ispublic, new_similarityhash));

		file_info = File::<Test>::get(&fileid).unwrap();
		assert_eq!(file_info.ispublic, 1);
		assert_eq!(file_info.similarityhash, vec![2]);
	});
}

#[test]
fn update_failed_when_file_non_exists() {
	new_test_ext().execute_with(|| {
		let fileid = vec![1];
		let new_ispublic = 1;
		let new_similarityhash = vec![2];

		assert_noop!(
			FileBank::update(Origin::signed(2), fileid.clone(), new_ispublic, new_similarityhash),
			Error::<Test>::FileNonExistent
		);
	});
}

#[test]
fn buyfile_works() {
	new_test_ext().execute_with(|| {
		let fileid = vec![1];
		let address = vec![1];
		let uploadfee = 10;
		let downloadfee = 1;
		let fit = create();

		assert_ok!(FileBank::upload(
			Origin::signed(1),
			fit.filename,
			address,
			fileid.clone(),
			fit.filehash,
			fit.similarityhash,
			fit.ispublic,
			fit.backups,
			fit.creator,
			fit.filesize,
			fit.keywords,
			fit.email,
			uploadfee,
			downloadfee,
			fit.deadline
		));

		let invoice = vec![1, 1];
		assert_eq!(Invoice::<Test>::contains_key(&invoice), true);

		let new_address = vec![2];
		assert_ok!(FileBank::buyfile(Origin::signed(2), fileid.clone(), new_address));

		let new_invoice = vec![1, 2];
		assert_eq!(Invoice::<Test>::contains_key(&new_invoice), true);
	});
}

#[test]
fn buyfile_failed_when_file_non_exists() {
	new_test_ext().execute_with(|| {
		let fileid = vec![1];
		let address = vec![1];

		assert_noop!(
			FileBank::buyfile(Origin::signed(2), fileid.clone(), address),
			Error::<Test>::FileNonExistent
		);
	});
}

pub fn create() -> FileInfoTest {
	FileInfoTest {
		filename: vec![1],
		filehash: vec![1],
		similarityhash: vec![1],
		ispublic: 0,
		backups: 3,
		creator: vec![1],
		filesize: 1,
		keywords: vec![1],
		email: vec![1],
		deadline: 100,
	}
}