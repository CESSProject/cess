#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use codec::{Decode, Encode, MaxEncodedLen};

use frame_support::{
	pallet_prelude::*,
	storage::child::KillStorageResult,
	traits::ValidatorCredits,
	weights::Weight,
};
use log::{debug, warn};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{SaturatedConversion, Zero},
	Percent,
	RuntimeDebug, Perbill,
};

use sp_std::{collections::btree_map::BTreeMap, prelude::*};

use cp_scheduler_credit::{SchedulerCreditCounter, SchedulerStashAccountFinder};

pub use pallet::*;

pub type CreditScore = u32;

pub const FULL_CREDIT_SCORE: u32 = 1000;
const LOG_TARGET: &str = "scheduler-credit";
/// The weight of credit value when figure credit score.
/// period n-1 to period n-5
const PERIOD_WEIGHT: [Percent; 5] = [
	Percent::from_percent(50),
	Percent::from_percent(20),
	Percent::from_percent(15),
	Percent::from_percent(10),
	Percent::from_percent(5),
];

#[derive(PartialEq, Eq, Clone, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
pub struct SchedulerCounterEntry {
	pub proceed_block_size: u64,
	pub punishment_count: u32,
}

impl SchedulerCounterEntry {
	pub fn increase_block_size<T: Config>(&mut self, block_size: u64) -> DispatchResult {
		self.proceed_block_size = self.proceed_block_size.checked_add(block_size).ok_or(Error::<T>::Overflow)?;
		Ok(())
	}

	pub fn increase_punishment_count<T: Config>(&mut self) -> DispatchResult {
		self.punishment_count = self.punishment_count.checked_add(1).ok_or(Error::<T>::Overflow)?;
		Ok(())
	}

	pub fn figure_credit_value(&self, total_block_size: u64) -> CreditScore {
		if total_block_size != 0 {
			let a = Perbill::from_rational(self.proceed_block_size, total_block_size) * FULL_CREDIT_SCORE;
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

	#[pallet::pallet]
	pub struct Pallet<T>(sp_std::marker::PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {

		#[pallet::constant]
		type PeriodDuration: Get<Self::BlockNumber>;

		type StashAccountFinder: SchedulerStashAccountFinder<Self::AccountId>;
	}

	#[pallet::error]
	pub enum Error<T> {
		Overflow,
	}

	#[pallet::storage]
	#[pallet::getter(fn current_scheduler_credits)]
	pub(super) type CurrentCounters<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, SchedulerCounterEntry, ValueQuery>;

	#[pallet::storage]
	pub(super) type HistoryCreditValues<T: Config> =
		StorageDoubleMap<_, Twox64Concat, u32, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {
		fn on_initialize(now: T::BlockNumber) -> Weight {
			let period_duration = T::PeriodDuration::get();
			if now % period_duration == T::BlockNumber::zero() {
				let period: u32 = (now / period_duration).saturated_into();
				Self::figure_credit_values(period.saturating_sub(1))
			} else {
				Weight::from_ref_time(0)
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn record_proceed_block_size(scheduler_id: &T::AccountId, block_size: u64) -> DispatchResult {
		<CurrentCounters<T>>::mutate(scheduler_id, |scb| -> DispatchResult {
			scb.increase_block_size::<T>(block_size)?;
			Ok(())
		})?;
		Ok(())
	}

	pub fn record_punishment(scheduler_id: &T::AccountId) -> DispatchResult {
		<CurrentCounters<T>>::mutate(scheduler_id, |scb| -> DispatchResult {
			scb.increase_punishment_count::<T>()?;
			Ok(())
		})?;
		Ok(())
	}

	pub fn figure_credit_values(period: u32) -> Weight {
		let mut weight: Weight = Weight::from_ref_time(0);
		let mut total_size = 0_u64;
		for (_, counter_entry) in <CurrentCounters<T>>::iter() {
			total_size += counter_entry.proceed_block_size;
			weight = weight.saturating_add(T::DbWeight::get().reads(1));
		}

		for (ctrl_account_id, counter_entry) in <CurrentCounters<T>>::iter() {
			let credit_value = counter_entry.figure_credit_value(total_size);
			debug!(
				target: LOG_TARGET,
				"scheduler control account: {:?}, credit value: {}",
				ctrl_account_id,
				credit_value
			);
			HistoryCreditValues::<T>::insert(&period, &ctrl_account_id, credit_value);
			weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
		}

		// Clear CurrentCounters
		#[allow(deprecated)]
		let cc_outcome = CurrentCounters::<T>::remove_all(None);
		let cc_keys_removed= match cc_outcome {
			KillStorageResult::AllRemoved(count) => count,
			KillStorageResult::SomeRemaining(count) => count,
		};
		weight = weight.saturating_add(T::DbWeight::get().reads(cc_keys_removed.into()));

		// Remove `period - history_depth` credit values in history.
		let history_depth = PERIOD_WEIGHT.len() as u32;
		if period >= history_depth {
			#[allow(deprecated)]
			let hcv_outcome = HistoryCreditValues::<T>::remove_prefix(&period.saturating_sub(history_depth), None);
			let hcv_keys_removed= match hcv_outcome {
				KillStorageResult::AllRemoved(count) => count,
				KillStorageResult::SomeRemaining(count) => count,
			};
			weight = weight.saturating_add(T::DbWeight::get().reads(hcv_keys_removed.into()));
		}
		weight
	}

	pub fn figure_credit_scores() -> BTreeMap<T::AccountId, CreditScore> {
		let mut result = BTreeMap::new();
		let now = <frame_system::Pallet<T>>::block_number();
		let period_duration = T::PeriodDuration::get();
		let period: u32 = (now / period_duration).saturated_into();

		if period == 0 {
			return result;
		}

		let last_period = period.saturating_sub(1);
		HistoryCreditValues::<T>::iter_key_prefix(&last_period)
			.for_each(|ctrl_account_id| {
				if let Some(stash_account_id) =
					T::StashAccountFinder::find_stash_account_id(&ctrl_account_id)
				{
					let mut credit_score = 0_u32;
					for (index, weight) in PERIOD_WEIGHT.into_iter().enumerate() {
						if last_period >= index as u32 {
							let credit_value = HistoryCreditValues::<T>::try_get(&last_period.saturating_sub(index as u32), &ctrl_account_id)
								.unwrap_or(0);
							credit_score += weight * credit_value;
						}
					}
					debug!(
						target: LOG_TARGET,
						"scheduler stash account: {:?}, credit value: {}",
						stash_account_id,
						credit_score
					);
					result.insert(stash_account_id, credit_score);
				} else {
					warn!(
						target: LOG_TARGET,
						"can not find the scheduler stash account for the controller account: {:?}",
						ctrl_account_id
					);
				}
		});
		result
	}
}

impl<T: Config> SchedulerCreditCounter<T::AccountId> for Pallet<T> {
	fn record_proceed_block_size(scheduler_id: &T::AccountId, block_size: u64) -> DispatchResult {
		Pallet::<T>::record_proceed_block_size(scheduler_id, block_size)?;
		Ok(())
	}

	fn record_punishment(scheduler_id: &T::AccountId) -> DispatchResult {
		Pallet::<T>::record_punishment(scheduler_id)?;
		Ok(())
	}
}

impl<T: Config> ValidatorCredits<T::AccountId> for Pallet<T> {
	fn full_credit() -> u32 {
		FULL_CREDIT_SCORE
	}

	fn credits(epoch_index: u64) -> BTreeMap<T::AccountId, CreditScore> {
		debug!(target: LOG_TARGET, "begin fetch schedulers credit on epoch: {}", epoch_index);
		Pallet::<T>::figure_credit_scores()
	}
}

#[cfg(test)]
mod test {
	use crate::SchedulerCounterEntry;
	use crate::mock::Test;
	#[test]
	fn scheduler_counter_works() {
		let mut sce = SchedulerCounterEntry::default();
		sce.increase_block_size::<Test>(100);
		assert_eq!(100, sce.proceed_block_size);
		sce.increase_block_size::<Test>(100);
		assert_eq!(200, sce.proceed_block_size);
		assert_eq!(0, sce.punishment_part());
		assert_eq!(100, sce.figure_credit_value(2000));

		sce.increase_punishment_count::<Test>();
		assert_eq!(1, sce.punishment_count);

		assert_eq!(100, sce.figure_credit_value(1000));
		sce.increase_punishment_count::<Test>();

		assert_eq!(2, sce.punishment_count);
		assert_eq!(0, sce.figure_credit_value(1000));
	}
}
