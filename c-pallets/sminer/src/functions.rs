use super::*;

impl<T: Config> Pallet<T> {
	/// Sub computing power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn sub_miner_idle_space(acc: &AccountOf<T>, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerItems<T>>::contains_key(acc) {
			return Ok(());
		}

		let state = Self::check_state(acc)?; //read 1
		if state == STATE_EXIT.as_bytes().to_vec() {
			return Ok(());
		}
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> DispatchResult {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
			miner_info.idle_space =
				miner_info.idle_space.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?; //read 1 write 1

		Ok(())
	}

	pub(super) fn calculate_power(idle_space: u128, service_space: u128) -> u128 {
		let service_power = SERVICE_MUTI.mul_floor(service_space);

        let idle_power = IDLE_MUTI.mul_floor(idle_space);

		let power: u128 = idle_power + service_power;

		power
	}

	pub(super) fn deposit_punish(miner: &AccountOf<T>, punish_amount: BalanceOf<T>) -> DispatchResult {
		<MinerItems<T>>::try_mutate(miner, |miner_info_opt| -> DispatchResult {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::NotMiner)?;
			
			let reward_pot = T::PalletId::get().into_account_truncating();

			if miner_info.collaterals > punish_amount {
				T::Currency::unreserve(miner, punish_amount);
				T::Currency::transfer(miner, &reward_pot, punish_amount, KeepAlive)?;
				<CurrencyReward<T>>::mutate(|reward| {
					*reward = *reward + punish_amount;
				});
				miner_info.collaterals = miner_info.collaterals.checked_sub(&punish_amount).ok_or(Error::<T>::Overflow)?;
			} else {
				T::Currency::unreserve(miner, miner_info.collaterals);
				T::Currency::transfer(miner, &reward_pot, miner_info.collaterals, KeepAlive)?;
				<CurrencyReward<T>>::mutate(|reward| {
					*reward = *reward + miner_info.collaterals;
				});
				miner_info.collaterals = BalanceOf::<T>::zero();
				miner_info.debt = punish_amount.checked_sub(&miner_info.collaterals).ok_or(Error::<T>::Overflow)?;
			}

			let power = Self::calculate_power(miner_info.idle_space, miner_info.service_space);
			let limit = Self::check_collateral_limit(power)?;

			if miner_info.collaterals < limit {
				miner_info.state = Self::str_to_bound(STATE_FROZEN)?;
			}

			Ok(())
		})?;

		Ok(())
	}

	pub(super) fn check_collateral_limit(power: u128) -> Result<BalanceOf<T>, Error<T>> {
		let limit = 1 + power.checked_div(T_BYTE).ok_or(Error::<T>::Overflow)?;
		let limit = BASE_LIMIT.checked_mul(limit).ok_or(Error::<T>::Overflow)?;
		let limit: BalanceOf<T> = limit.try_into().map_err(|_| Error::<T>::Overflow)?;

		Ok(limit)
	}

	pub(super) fn check_state(acc: &AccountOf<T>) -> Result<Vec<u8>, Error<T>> {
		Ok(<MinerItems<T>>::try_get(acc).map_err(|_e| Error::<T>::NotMiner)?.state.to_vec())
	}
	// Convert the miner state constant to boundedvec
	pub(super) fn str_to_bound(param: &str) -> Result<BoundedVec<u8, T::ItemLimit>, DispatchError> {
		let result: BoundedVec<u8, T::ItemLimit> = param.as_bytes().to_vec().try_into().map_err(|_| Error::<T>::Overflow)?;

		Ok(result)
	}
}