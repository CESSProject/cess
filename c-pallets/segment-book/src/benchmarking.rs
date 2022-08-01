// use super::*;
// use crate::{Pallet as FileBank, *};
// use codec::{alloc::string::ToString, Decode};
// pub use frame_benchmarking::{
// 	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
// };
// use frame_support::{
// 	dispatch::UnfilteredDispatchable,
// 	pallet_prelude::*,
// 	traits::{Currency, CurrencyToVote, Get, Imbalance},
// };
// use pallet_cess_staking::{
// 	testing_utils, Config as StakingConfig, Pallet as Staking, RewardDestination,
// };
// use pallet_file_map::{Config as FileMapConfig, Pallet as FileMap};
// use pallet_sminer::{Config as SminerConfig, Pallet as Sminer};
// use sp_runtime::{
// 	traits::{Bounded, One, StaticLookup, TrailingZeroInput, Zero},
// 	Perbill, Percent,
// };
// use sp_std::prelude::*;

// use frame_system::RawOrigin;

// pub struct Pallet<T: Config>(FileBank<T>);
// pub trait Config:
// 	crate::Config + pallet_cess_staking::Config + pallet_file_map::Config + pallet_sminer::Config
// {
// }
// type SminerBalanceOf<T> = <<T as pallet_sminer::Config>::Currency as Currency<
// 	<T as frame_system::Config>::AccountId,
// >>::Balance;


