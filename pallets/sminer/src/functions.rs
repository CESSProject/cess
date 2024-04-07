use super::*;
use frame_system::pallet_prelude::BlockNumberFor;

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

			if miner_info.collaterals > punish_amount {
				T::Currency::unreserve(&miner_info.staking_account, punish_amount);
				T::CessTreasuryHandle::send_to_pid(miner.clone(), punish_amount)?;
				miner_info.collaterals = miner_info.collaterals.checked_sub(&punish_amount).ok_or(Error::<T>::Overflow)?;
			} else {
				T::Currency::unreserve(miner, miner_info.collaterals);
				T::CessTreasuryHandle::send_to_pid(miner.clone(), miner_info.collaterals)?;
				miner_info.collaterals = BalanceOf::<T>::zero();
				miner_info.debt = punish_amount.checked_sub(&miner_info.collaterals).ok_or(Error::<T>::Overflow)?;
			}

			let power = Self::calculate_power(miner_info.idle_space, miner_info.service_space);
			let limit: BalanceOf<T> = Self::calculate_limit_by_space(power)?
				.try_into().map_err(|_| Error::<T>::Overflow)?;

			if miner_info.collaterals < limit {
				miner_info.state = Self::str_to_bound(STATE_FROZEN)?;
			}

			Ok(())
		})?;

		Ok(())
	}

	pub(super) fn check_state(acc: &AccountOf<T>) -> Result<Vec<u8>, Error<T>> {
		Ok(<MinerItems<T>>::try_get(acc).map_err(|_e| Error::<T>::NotMiner)?.state.to_vec())
	}
	// Convert the miner state constant to boundedvec
	pub(super) fn str_to_bound(param: &str) -> Result<BoundedVec<u8, T::ItemLimit>, DispatchError> {
		let result: BoundedVec<u8, T::ItemLimit> = param.as_bytes().to_vec().try_into().map_err(|_| Error::<T>::Overflow)?;

		Ok(result)
	}
	// Note: that it is necessary to determine whether the state meets the exit conditions before use.
	pub(super) fn execute_exit(acc: &AccountOf<T>) -> DispatchResult {
		let mut miner_list = AllMiner::<T>::get();
		miner_list.retain(|s| s != acc);
		AllMiner::<T>::put(miner_list);
		MinerItems::<T>::try_mutate(acc, |miner_opt| -> DispatchResult {
			let miner = miner_opt.as_mut().ok_or(Error::<T>::Unexpected)?;
			miner.state = Self::str_to_bound(STATE_EXIT)?;

			Ok(())
		})?;
		
		Ok(())
	}

	pub(super) fn create_restoral_target(miner: &AccountOf<T>, service_space: u128) -> DispatchResult {
        let block: u32 = service_space
            .checked_div(T_BYTE).ok_or(Error::<T>::Overflow)?
            .checked_add(1).ok_or(Error::<T>::Overflow)?
            .checked_mul(T::OneDayBlock::get().try_into().map_err(|_| Error::<T>::Overflow)?).ok_or(Error::<T>::Overflow)? as u32;

        let now = <frame_system::Pallet<T>>::block_number();
        let block = now.checked_add(&block.saturated_into()).ok_or(Error::<T>::Overflow)?;

        let restoral_info = RestoralTargetInfo::<AccountOf<T>, BlockNumberFor<T>>{
            miner: miner.clone(),
            service_space,
            restored_space: u128::MIN,
            cooling_block: block,
        };

        <RestoralTarget<T>>::insert(&miner, restoral_info);

        Ok(())
    }
	
	// Note: that it is necessary to determine whether the state meets the exit conditions before use.
	pub(super) fn withdraw(miner: AccountOf<T>) -> DispatchResult {
		let miner_info = <MinerItems<T>>::try_get(&miner).map_err(|_| Error::<T>::NotMiner)?;
		T::Currency::unreserve(&miner_info.staking_account, miner_info.collaterals);
		let space_proof_info = miner_info.space_proof_info.ok_or(Error::<T>::NotpositiveState)?;
		let encoding = space_proof_info.pois_key.encode();
		let hashing = sp_io::hashing::sha2_256(&encoding);
		MinerPublicKey::<T>::remove(hashing);
		// <MinerItems<T>>::remove(miner.clone());
		<PendingReplacements<T>>::remove(miner);

		Ok(())
	}

	pub(super) fn calculate_limit_by_space(space: u128) -> Result<u128, DispatchError> {
		let mut tib_count = space.checked_div(T_BYTE).ok_or(Error::<T>::Overflow)?;
		if space % T_BYTE != 0 {
			tib_count = tib_count.checked_add(1).ok_or(Error::<T>::Overflow)?;
		}

		let collaterals_limit = tib_count.checked_mul(BASE_UNIT).ok_or(Error::<T>::Overflow)?;

		Ok(collaterals_limit)
	}
}