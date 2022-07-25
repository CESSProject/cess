use super::*;
use crate::{Pallet as Sminer, *};
use codec::{alloc::string::ToString, Decode};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_support::{
	dispatch::UnfilteredDispatchable,
	pallet_prelude::*,
	traits::{Currency, CurrencyToVote, Get, Imbalance},
};
use sp_runtime::{
	traits::{Bounded, One, StaticLookup, TrailingZeroInput, Zero},
	Perbill, Percent,
};
use frame_system::RawOrigin;

const SEED: u32 = 2190502;
const MAX_SPANS: u32 = 100;

pub fn add_miner<T: Config>() -> Result<T::AccountId, &'static str> {
	let miner: T::AccountId = account("miner1", 100, SEED);
	let ip = "1270008080".as_bytes().to_vec();
	let public_key = "thisisatestpublickey".as_bytes().to_vec();
	T::Currency::make_free_balance_be(
		&miner,
		BalanceOf::<T>::max_value(),
	);
	whitelist_account!(miner);
	Sminer::<T>::regnstk(
		RawOrigin::Signed(miner.clone()).into(),
		miner.clone(),
		ip,
		0u32.into(),
	)?;
	Ok(miner.clone())
}

// benchmarks! {

// }
