use crate::*;

impl<T: Config> Pallet<T> {
	pub fn check_time_unix(signing_time: &u64) -> bool {
		let expiration = 4 * 60 * 60 * 1000; // 4 hours
		let now = T::UnixTime::now().as_millis().saturated_into::<u64>();
		if signing_time < &now && now <= signing_time + expiration {
			return true;
		} else {
			return false;
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
					Self::deposit_event(Event::<T>::ClearInvalidTee { pubkey });
				}
			}
		}

		return weight;
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

	pub fn clean_expired_master_key_postation(block_number: BlockNumberFor<T>) -> Weight {
		let mut weight: Weight = Weight::zero();
		for (key, value) in MasterKeyPostation::<T>::iter() {
			weight = weight.saturating_add(T::DbWeight::get().reads(1));
			if let Some((bn, _)) = value {
				if bn.saturating_add(3_u32.into()) > block_number {
					MasterKeyPostation::<T>::remove(key);
					weight = weight.saturating_add(T::DbWeight::get().writes(1));
				}
			}
		}
		return weight;
	}

	pub fn verify_signature(
		signature_slice: &[u8],
		msg: &[u8],
		pub_key: &sp_core::sr25519::Public,
	) -> Result<(), Error<T>> {
		ensure!(signature_slice.len() == 64, Error::<T>::InvalidSignatureLength);
		let sig = sp_core::sr25519::Signature::try_from(signature_slice).or(Err(Error::<T>::MalformedSignature))?;
		ensure!(sp_io::crypto::sr25519_verify(&sig, msg, pub_key), Error::<T>::InvalidSignature);
		Ok(())
	}

	pub fn verify_signing_time(signing_time_by_sec: u64, expiration_secs: u64) -> Result<(), Error<T>> {
		let now = T::UnixTime::now().as_secs().saturated_into::<u64>();
		ensure!(
			signing_time_by_sec < now && now <= signing_time_by_sec + expiration_secs,
			Error::<T>::InvalidMasterKeyApplySigningTime
		);
		Ok(())
	}
}
