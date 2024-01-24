// This file is part of EVM Account Mapping Pallet.

// Copyright (C) HashForest Technology Pte. Ltd.
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

//! Benchmarking setup for the pallet
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as ThisPallet;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

use codec::Decode;
use sp_std::prelude::*;
use sp_core::crypto::Ss58Codec;
use sp_runtime::traits::{Bounded, TrailingZeroInput};

#[allow(dead_code)]
fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

#[benchmarks( where PaymentBalanceOf<T>: FixedPointOperand, <T as frame_system::Config>::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>, T: frame_system::Config<AccountId = sp_runtime::AccountId32>,)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn meta_call() -> Result<(), BenchmarkError> {
		let account =
			T::AccountId::from_ss58check("5DT96geTS2iLpkH8fAhYAAphNpxddKCV36s5ShVFavf1xQiF")
				.unwrap();
		let call_data = hex::decode("00071448656c6c6f").expect("Valid"); // system.remarkWithEvent("Hello")
		let call = <T as frame_system::Config>::RuntimeCall::decode(&mut TrailingZeroInput::new(
			&call_data,
		))
		.expect("Valid");
		let nonce: u64 = 0;
		let signature: [u8; 65] = hex::decode("37cb6ff8e296d7e476ee13a6cfababe788217519d428fcc723b482dc97cb4d1359a8d1c020fe3cebc1d06a67e61b1f0e296739cecacc640b0ba48e8a7555472e1b").expect("Decodable").try_into().expect("Valid");

		T::Currency::make_free_balance_be(&account, BalanceOf::<T>::max_value() / 2u32.into());

		#[extrinsic_call]
		_(RawOrigin::None, account, Box::new(call.into()), nonce, signature, None);

		Ok(())
	}

	impl_benchmark_test_suite!(ThisPallet, crate::mock::new_test_ext(), crate::mock::Test);
}
