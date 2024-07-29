use crate::*;

impl<T: Config> Pallet<T> {
	pub fn check_time_unix(signing_time: &u64) -> bool {
		let expiration = 4 * 60 * 60 * 1000; // 4 hours
		let now = T::UnixTime::now().as_millis().saturated_into::<u64>();
		if signing_time < &now && now <= signing_time + expiration {
			return true
		} else {
			return false
		}
	}

	pub fn clear_mission(now: BlockNumberFor<T>) -> Weight {
		let mut weight: Weight = Weight::zero();

		let least = T::AtLeastWorkBlock::get();

		for (pubkey, last_block) in LastWork::<T>::iter() {
			weight = weight.saturating_add(T::DbWeight::get().reads(1));
			if last_block.saturating_add(least) < now {
				if let Ok(temp_weight) = Self::execute_exit(pubkey.clone()) {
					weight.saturating_add(temp_weight);
					LastWork::<T>::remove(pubkey.clone());
					LastRefresh::<T>::remove(pubkey.clone());
					Self::deposit_event(Event::<T>::ClearInvalidTee{ pubkey });
				}
			}
		}

		return weight
	}

	pub fn execute_exit(pbk: WorkerPublicKey) -> Result<Weight, DispatchError> {
		let mut weight: Weight = Weight::zero();

		if let Some(first_holder) = MasterKeyFirstHolder::<T>::get() {
			ensure!(first_holder != pbk, Error::<T>::CannotExitMasterKeyHolder);
		}
		weight = weight.saturating_add(T::DbWeight::get().reads(1));

		Workers::<T>::remove(&pbk);
		weight = weight.saturating_add(T::DbWeight::get().writes(1));

		WorkerAddedAt::<T>::remove(&pbk);
		weight = weight.saturating_add(T::DbWeight::get().writes(1));

		Endpoints::<T>::remove(&pbk);
		weight = weight.saturating_add(T::DbWeight::get().writes(1));

		ValidationTypeList::<T>::mutate(|puk_list| -> DispatchResult {
			puk_list.retain(|g| *g != pbk);
			Ok(())
		})?;
		weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

		Ok(weight)
	}
}
