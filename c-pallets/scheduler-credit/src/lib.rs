#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::traits::ValidatorCredits;
use log::{debug, warn};
use scale_info::TypeInfo;
use sp_runtime::RuntimeDebug;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use cp_scheduler_credit::{SchedulerCreditCounter, SchedulerStashAccountFinder};

pub use pallet::*;

pub type CreditScore = u32;

pub const FULL_CREDIT_SCORE: u32 = 1000;
const LOG_TARGET: &str = "scheduler-credit";

#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
pub struct SchedulerCounterEntry {
	pub proceed_block_size: u64,
	pub punishment_count: u32,
}

impl SchedulerCounterEntry {
	pub fn increase_block_size(&mut self, block_size: u64) {
		self.proceed_block_size += block_size;
	}

	pub fn increase_punishment_count(&mut self) {
		self.punishment_count += 1;
	}

	pub fn figure_credit_score(&self, total_block_size: u64) -> CreditScore {
		if total_block_size != 0 {
			let a = (FULL_CREDIT_SCORE as f64 *
				(self.proceed_block_size as f64 / total_block_size as f64)) as u32;
			return a.saturating_sub(self.punishment_part())
		}
		return 0
	}

	fn punishment_part(&self) -> u32 {
		if self.punishment_count != 0 {
			return (10 * self.punishment_count).pow(2)
		}
		return 0
	}
}

impl Default for SchedulerCounterEntry {
	fn default() -> Self {
		SchedulerCounterEntry { proceed_block_size: 0_u64, punishment_count: 0_u32 }
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(sp_std::marker::PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type StashAccountFinder: SchedulerStashAccountFinder<Self::AccountId>;
	}

	#[pallet::storage]
	#[pallet::getter(fn current_scheduler_credits)]
	pub(super) type CurrentCounters<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, SchedulerCounterEntry, ValueQuery>;
}

impl<T: Config> Pallet<T> {
	pub fn record_proceed_block_size(scheduler_id: &T::AccountId, block_size: u64) {
		<CurrentCounters<T>>::mutate(scheduler_id, |scb| scb.increase_block_size(block_size));
	}

	pub fn record_punishment(scheduler_id: &T::AccountId) {
		<CurrentCounters<T>>::mutate(scheduler_id, |scb| scb.increase_punishment_count());
	}

	pub fn figure_credits() -> BTreeMap<T::AccountId, CreditScore> {
		let mut credit_map = BTreeMap::new();
		let mut total_size = 0_u64;
		for (ctrl_account_id, counter_entry) in <CurrentCounters<T>>::iter() {
			total_size += counter_entry.proceed_block_size;
			if let Some(stash_account_id) =
				T::StashAccountFinder::find_stash_account_id(&ctrl_account_id)
			{
				credit_map.insert(stash_account_id, counter_entry);
			} else {
				warn!(
					target: LOG_TARGET,
					"can not find the scheduler stash account for the controller account: {:?}",
					ctrl_account_id
				);
			}
		}
		let mut result = BTreeMap::new();
		for (stash_account_id, counter_entry) in credit_map {
			let credit_score = counter_entry.figure_credit_score(total_size);
			debug!(
				target: LOG_TARGET,
				"scheduler stash account: {:?}, credit: {}",
				stash_account_id,
				credit_score
			);
			result.insert(stash_account_id, credit_score);
		}
		result
	}
}

impl<T: Config> SchedulerCreditCounter<T::AccountId> for Pallet<T> {
	fn record_proceed_block_size(scheduler_id: &T::AccountId, block_size: u64) {
		Pallet::<T>::record_proceed_block_size(scheduler_id, block_size);
	}

	fn record_punishment(scheduler_id: &T::AccountId) {
		Pallet::<T>::record_punishment(scheduler_id);
	}
}

impl<T: Config> ValidatorCredits<T::AccountId> for Pallet<T> {
	fn full_credit() -> u32 {
		FULL_CREDIT_SCORE
	}

	fn credits(epoch_index: u64) -> BTreeMap<T::AccountId, CreditScore> {
		debug!(target: LOG_TARGET, "begin fetch schedulers credit on epoch: {}", epoch_index);
		Pallet::<T>::figure_credits()
	}
}

#[cfg(test)]
mod test {
	use crate::SchedulerCounterEntry;

	#[test]
	fn scheduler_counter_works() {
		let mut sce = SchedulerCounterEntry::default();
		sce.increase_block_size(100);
		assert_eq!(100, sce.proceed_block_size);
		sce.increase_block_size(100);
		assert_eq!(200, sce.proceed_block_size);
		assert_eq!(0, sce.punishment_part());
		assert_eq!(100, sce.figure_credit_score(2000));

		sce.increase_punishment_count();
		assert_eq!(1, sce.punishment_count);
		assert_eq!(100, sce.figure_credit_score(1000));
		sce.increase_punishment_count();
		assert_eq!(2, sce.punishment_count);
		assert_eq!(0, sce.figure_credit_score(1000));
	}
}
