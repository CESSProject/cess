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
use frame_support::{assert_ok, assert_noop};
use mock::{
	new_test_ext, Origin, SegmentBook,
};
use frame_benchmarking::account;
use crate::mock::Sminer;
use crate::{Error, mock::*};

#[test]
fn vpa_vpb_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::initi(Origin::root()));
		assert_ok!(Sminer::regnstk(Origin::signed(1),account("source", 0, 0),0,0,0,0));
		assert_ok!(SegmentBook::intent_submit(Origin::signed(1),1,1,1,vec![vec![5]],vec![6],vec![7]));	
		assert_ok!(SegmentBook::submit_to_vpa(Origin::signed(1),1,1,vec![4],vec![5]));	
		assert_ok!(SegmentBook::verify_in_vpa(Origin::signed(1),1,1,true));	
	});
}

#[test]
fn vpc_vpd_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::initi(Origin::root()));
		assert_ok!(Sminer::regnstk(Origin::signed(1),account("source", 0, 0),0,0,0,0));
		assert_ok!(SegmentBook::intent_submit(Origin::signed(1),1,2,1,vec![vec![5]],vec![6],vec![7]));	
		assert_ok!(SegmentBook::submit_to_vpc(Origin::signed(1),1,1,vec![vec![4]],vec![vec![5]]));	
		assert_ok!(SegmentBook::verify_in_vpc(Origin::signed(1),1,1,vec![vec![4]],true));	
	});
}

#[test]
fn size_type_error_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::initi(Origin::root()));
		assert_ok!(Sminer::regnstk(Origin::signed(1),account("source", 0, 0),0,0,0,0));

        assert_noop!(
            SegmentBook::intent_submit(Origin::signed(1),3,1,1,vec![vec![5]],vec![6],vec![7]),
            Error::<Test>::SizeTypeError
        );
	});
}

#[test]
fn submit_type_error_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::initi(Origin::root()));
		assert_ok!(Sminer::regnstk(Origin::signed(1),account("source", 0, 0),0,0,0,0));

        assert_noop!(
            SegmentBook::intent_submit(Origin::signed(1),1,3,1,vec![vec![5]],vec![6],vec![7]),
            Error::<Test>::SubmitTypeError
        );
	});
}

#[test]
fn no_intent_submit_yet_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::initi(Origin::root()));
		assert_ok!(Sminer::regnstk(Origin::signed(1),account("source", 0, 0),0,0,0,0));
	
        assert_noop!(
            SegmentBook::submit_to_vpa(Origin::signed(1),1,1,vec![4],vec![5]),
            Error::<Test>::NoIntentSubmitYet
        );
		assert_noop!(
            SegmentBook::submit_to_vpb(Origin::signed(1),1,1,vec![4],vec![5]),
            Error::<Test>::NoIntentSubmitYet
        );
		assert_noop!(
			SegmentBook::submit_to_vpc(Origin::signed(1),1,1,vec![vec![4]],vec![vec![5]]),
            Error::<Test>::NoIntentSubmitYet
        );
		assert_noop!(
            SegmentBook::submit_to_vpd(Origin::signed(1),1,1,vec![vec![4]],vec![vec![5]]),
            Error::<Test>::NoIntentSubmitYet
        );
	});
}

#[test]
fn not_exist_in_vpa_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::initi(Origin::root()));
		assert_ok!(Sminer::regnstk(Origin::signed(1),account("source", 0, 0),0,0,0,0));
	
        assert_noop!(
            SegmentBook::verify_in_vpa(Origin::signed(1),1,1,true),
            Error::<Test>::NotExistInVPA
        );
	});
}

#[test]
fn not_exist_in_vpc_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::initi(Origin::root()));
		assert_ok!(Sminer::regnstk(Origin::signed(1),account("source", 0, 0),0,0,0,0));
	
        assert_noop!(
            SegmentBook::verify_in_vpc(Origin::signed(1),1,1,vec![vec![4]],true),
            Error::<Test>::NotExistInVPC
        );
	});
}

#[test]
fn not_exist_in_vpd_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::initi(Origin::root()));
		assert_ok!(Sminer::regnstk(Origin::signed(1),account("source", 0, 0),0,0,0,0));
	
        assert_noop!(
            SegmentBook::verify_in_vpd(Origin::signed(1),1,1,true),
            Error::<Test>::NotExistInVPD
        );
	});
}

#[test]
fn not_ready_in_vpa_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::initi(Origin::root()));
		assert_ok!(Sminer::regnstk(Origin::signed(1),account("source", 0, 0),0,0,0,0));
		assert_ok!(SegmentBook::intent_submit(Origin::signed(1),1,1,1,vec![vec![5]],vec![6],vec![7]));	
		
        assert_noop!(
            SegmentBook::verify_in_vpa(Origin::signed(1),1,1,true),
            Error::<Test>::NotReadyInVPA
        );
	});
}

#[test]
fn not_ready_in_vpc_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Sminer::initi(Origin::root()));
		assert_ok!(Sminer::regnstk(Origin::signed(1),account("source", 0, 0),0,0,0,0));
		assert_ok!(SegmentBook::intent_submit(Origin::signed(1),1,2,1,vec![vec![5]],vec![6],vec![7]));	
		
        assert_noop!(
            SegmentBook::verify_in_vpc(Origin::signed(1),1,1,vec![vec![4]],true),
            Error::<Test>::NotReadyInVPC
        );
	});
}