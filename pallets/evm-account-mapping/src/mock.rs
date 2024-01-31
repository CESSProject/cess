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

use crate as pallet_evm_account_mapping;
use frame_support::{
	derive_impl,
	dispatch::DispatchClass,
	pallet_prelude::*,
	parameter_types,
	traits::{
		fungible::Mutate, ConstU128, ConstU16, ConstU32, ConstU64, Get, Imbalance, OnUnbalanced,
	},
	weights::{Weight, WeightToFee as WeightToFeeT},
};
use pallet_transaction_payment::CurrencyAdapter;
use sp_runtime::{
	traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature, SaturatedConversion,
};

type Block = frame_system::mocking::MockBlock<Test>;

pub(crate) type Balance = u128;
pub(crate) type Signature = MultiSignature;
pub(crate) type AccountPublic = <Signature as Verify>::Signer;
pub(crate) type AccountId = <AccountPublic as IdentifyAccount>::AccountId;

pub(crate) const MILLI_CENTS: Balance = 1_000_000;
pub(crate) const CENTS: Balance = 1_000 * MILLI_CENTS;
pub(crate) const DOLLARS: Balance = 100 * CENTS;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub struct Test {
		System: frame_system,
		Balances: pallet_balances,
		TransactionPayment: pallet_transaction_payment,
		EvmAccountMapping: pallet_evm_account_mapping,
	}
);

parameter_types! {
	pub(crate) static ExtrinsicBaseWeight: Weight = Weight::zero();
}

pub struct BlockWeights;
impl Get<frame_system::limits::BlockWeights> for BlockWeights {
	fn get() -> frame_system::limits::BlockWeights {
		frame_system::limits::BlockWeights::builder()
			.base_block(Weight::zero())
			.for_class(DispatchClass::all(), |weights| {
				weights.base_extrinsic = ExtrinsicBaseWeight::get();
			})
			.for_class(DispatchClass::non_mandatory(), |weights| {
				weights.max_total = Weight::from_parts(1024, u64::MAX).into();
			})
			.build_or_panic()
	}
}

parameter_types! {
	pub static WeightToFee: Balance = 1;
	pub static TransactionByteFee: Balance = 1;
	pub static OperationalFeeMultiplier: u8 = 5;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = BlockWeights;
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = sp_core::H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type BlockHashCount = ConstU64<250>;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

impl pallet_balances::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type WeightInfo = ();
	type Balance = Balance;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU128<CENTS>;
	type AccountStore = System;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxLocks = ();
	type MaxReserves = ConstU32<50>;
	type MaxHolds = ();
	type MaxFreezes = ();
}

impl WeightToFeeT for WeightToFee {
	type Balance = Balance;

	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		Self::Balance::saturated_from(weight.ref_time())
			.saturating_mul(WEIGHT_TO_FEE.with(|v| *v.borrow()))
	}
}

impl WeightToFeeT for TransactionByteFee {
	type Balance = Balance;

	fn weight_to_fee(weight: &Weight) -> Self::Balance {
		Self::Balance::saturated_from(weight.ref_time())
			.saturating_mul(TRANSACTION_BYTE_FEE.with(|v| *v.borrow()))
	}
}

parameter_types! {
	pub(crate) static TipUnbalancedAmount: Balance = 0;
	pub(crate) static FeeUnbalancedAmount: Balance = 0;
}

pub struct DealWithFees;
impl OnUnbalanced<pallet_balances::NegativeImbalance<Test>> for DealWithFees {
	fn on_unbalanceds<B>(
		mut fees_then_tips: impl Iterator<Item = pallet_balances::NegativeImbalance<Test>>,
	) {
		if let Some(fees) = fees_then_tips.next() {
			FeeUnbalancedAmount::mutate(|a| *a += fees.peek());
			if let Some(tips) = fees_then_tips.next() {
				TipUnbalancedAmount::mutate(|a| *a += tips.peek());
			}
		}
	}
}

impl pallet_transaction_payment::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = CurrencyAdapter<Balances, DealWithFees>;
	type OperationalFeeMultiplier = OperationalFeeMultiplier;
	type WeightToFee = WeightToFee;
	type LengthToFee = TransactionByteFee;
	type FeeMultiplierUpdate = ();
}

parameter_types! {
	pub EIP712Name: Vec<u8> = b"Substrate".to_vec();
	pub EIP712Version: Vec<u8> = b"1".to_vec();
	pub EIP712ChainID: crate::EIP712ChainID = sp_core::U256::from(0);
	pub EIP712VerifyingContractAddress: crate::EIP712VerifyingContractAddress = sp_core::H160::from([0u8; 20]);
}

impl pallet_evm_account_mapping::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type AddressConverter = pallet_evm_account_mapping::SubstrateAddressConverter;
	type ServiceFee = ConstU128<1000>;
	type OnUnbalancedForServiceFee = ();
	type CallFilter = frame_support::traits::Everything;
	type EIP712Name = EIP712Name;
	type EIP712Version = EIP712Version;
	type EIP712ChainID = EIP712ChainID;
	type EIP712VerifyingContractAddress = EIP712VerifyingContractAddress;
	type WeightInfo = ();
}

#[allow(unused)]
pub(crate) fn set_balance(who: AccountId, new_free: Balance) {
	<Test as crate::Config>::Currency::set_balance(&who, new_free);
	assert_eq!(<Test as crate::Config>::Currency::free_balance(who), new_free);
}

// Build genesis storage according to the mock runtime.
pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
}

pub(crate) fn run_to_block(n: u64) {
	let current_block = System::block_number();
	assert!(n > current_block);
	while System::block_number() < n {
		Balances::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		Balances::on_initialize(System::block_number());
	}
}
