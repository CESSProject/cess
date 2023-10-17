use super::*;

impl<T: Config> Pallet<T> {
	pub(super) fn add_miner_idle_space(
		acc: &AccountOf<T>, 
		accumulator: Accumulator, 
		rear: u64, 
		tee_sig: TeeRsaSignature,
	) -> Result<u128, DispatchError> {
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> Result<u128, DispatchError> {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::NotMiner)?;

			// check state 
			ensure!(miner_info.state.to_vec() == STATE_POSITIVE.as_bytes().to_vec(), Error::<T>::NotpositiveState);

			ensure!(miner_info.space_proof_info.rear < rear, Error::<T>::CountError);

			let count = rear.checked_sub(miner_info.space_proof_info.rear).ok_or(Error::<T>::Overflow)?;
			let idle_space = IDLE_SEG_SIZE.checked_mul(count as u128).ok_or(Error::<T>::Overflow)?;

			miner_info.space_proof_info.rear = rear;

			miner_info.space_proof_info.accumulator = accumulator;

			miner_info.idle_space =
				miner_info.idle_space.checked_add(idle_space).ok_or(Error::<T>::Overflow)?;

			miner_info.tee_signature = tee_sig;

			Ok(idle_space)
		})
	}

    pub(super) fn delete_idle_update_accu(
		acc: &AccountOf<T>, 
		accumulator: Accumulator, 
		front: u64, 
		tee_sig: TeeRsaSignature,
	) -> Result<u64, DispatchError> {
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> Result<u64, DispatchError> {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::NotMiner)?;

			ensure!(miner_info.space_proof_info.front < front, Error::<T>::CountError);

			let count = front - miner_info.space_proof_info.front;

			miner_info.space_proof_info.front = front;

			miner_info.space_proof_info.accumulator = accumulator;

			miner_info.tee_signature = tee_sig;

			Ok(count)
		})
	}

    pub(super) fn delete_idle_update_space(acc: &AccountOf<T>, idle_space: u128) -> DispatchResult {
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> DispatchResult {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::NotMiner)?;

			miner_info.idle_space = miner_info.idle_space.checked_sub(idle_space).ok_or(Error::<T>::Overflow)?;

			Ok(())
		})
	}

	/// Add space calculation power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub(super) fn add_miner_service_space(acc: &AccountOf<T>, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerItems<T>>::contains_key(acc) {
			return Ok(());
		}

		let state = Self::check_state(acc)?;
		if state == STATE_EXIT.as_bytes().to_vec() {
			return Ok(());
		}
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> DispatchResult {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
			miner_info.service_space =
				miner_info.service_space.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	/// Sub space calculation power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub(super) fn sub_miner_service_space(acc: &AccountOf<T>, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerItems<T>>::contains_key(acc) {
			return Ok(());
		}

		let state = Self::check_state(acc)?;
		if state == STATE_EXIT.as_bytes().to_vec() {
			return Ok(());
		}
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> DispatchResult {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
			miner_info.service_space =
				miner_info.service_space.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	pub(super) fn insert_service_bloom(acc: &AccountOf<T>, hash_list: Vec<Box<[u8; 256]>>) -> DispatchResult {
		<MinerItems<T>>::try_mutate(&acc, |opt_m_info| -> DispatchResult{
			let m_info = opt_m_info.as_mut().ok_or(Error::<T>::NotMiner)?;
			for elem in hash_list {
				m_info.service_bloom_filter.insert(*elem).map_err(|_| Error::<T>::BloomElemPushError)?;
			}
			Ok(())
		})?;

		Ok(())
	}

	pub(super) fn delete_service_bloom(acc: &AccountOf<T>, hash_list: Vec<Box<[u8; 256]>>) -> DispatchResult {
		<MinerItems<T>>::try_mutate(&acc, |opt_m_info| -> DispatchResult{
			let m_info = opt_m_info.as_mut().ok_or(Error::<T>::NotMiner)?;
			for elem in hash_list {
				m_info.service_bloom_filter.delete(*elem).map_err(|_| Error::<T>::BloomElemPushError)?;
			}
			Ok(())
		})?;

		Ok(())
	}

    pub(super) fn calculate_miner_reward(
		miner: &AccountOf<T>,
		total_idle_space: u128,
		total_service_space: u128,
		miner_idle_space: u128,
		miner_service_space: u128,
	) -> DispatchResult {
		let total_reward = T::RewardPool::get_reward();
		let total_power = Self::calculate_power(total_idle_space, total_service_space);
		let miner_power = Self::calculate_power(miner_idle_space, miner_service_space);

		let miner_prop = Perbill::from_rational(miner_power, total_power);
		let this_round_reward = miner_prop.mul_floor(total_reward);
		let each_share = EACH_SHARE_MUTI.mul_floor(this_round_reward);
		let each_share = each_share.checked_div(&RELEASE_NUMBER.into()).ok_or(Error::<T>::Overflow)?;
		let issued: BalanceOf<T> = ISSUE_MUTI.mul_floor(this_round_reward).try_into().map_err(|_| Error::<T>::Overflow)?;

		let order = RewardOrder::<BalanceOf<T>>{
			order_reward: this_round_reward.try_into().map_err(|_| Error::<T>::Overflow)?,
			each_share: each_share.try_into().map_err(|_| Error::<T>::Overflow)?,
			award_count: 1,
			has_issued: true,
		};
		// calculate available reward
		RewardMap::<T>::try_mutate(miner, |opt_reward_info| -> DispatchResult {
			let reward_info = opt_reward_info.as_mut().ok_or(Error::<T>::Unexpected)?;
			// traverse the order list
			for order_temp in reward_info.order_list.iter_mut() {
				// skip if the order has been issued for 180 times
				if order_temp.award_count == RELEASE_NUMBER {
					continue;
				}
				reward_info.currently_available_reward = reward_info.currently_available_reward
					.checked_add(&order_temp.each_share).ok_or(Error::<T>::Overflow)?;

				order_temp.award_count += 1;
			}

			if reward_info.order_list.len() == RELEASE_NUMBER as usize {
				reward_info.order_list.remove(0);
			}

			reward_info.currently_available_reward = reward_info.currently_available_reward
				.checked_add(&issued).ok_or(Error::<T>::Overflow)?
				.checked_add(&order.each_share).ok_or(Error::<T>::Overflow)?;
			reward_info.total_reward = reward_info.total_reward
				.checked_add(&order.order_reward).ok_or(Error::<T>::Overflow)?;
			reward_info.order_list.try_push(order.clone()).map_err(|_| Error::<T>::BoundedVecError)?;

			Ok(())
		})?;
		T::RewardPool::sub_reward(order.order_reward)?;
		
		Ok(())
	}

	pub(super) fn clear_punish(miner: &AccountOf<T>, level: u8, idle_space: u128, service_space: u128) -> DispatchResult {
		let power = Self::calculate_power(idle_space, service_space);
		let limit = Self::check_collateral_limit(power)?;
		// FOR TESTING
		let punish_amount = match level {
			1 => Perbill::from_percent(30).mul_floor(limit),
			2 => Perbill::from_percent(60).mul_floor(limit),
			3 | 4 | 5 | 6 => limit,
			_ => return Err(Error::<T>::Unexpected)?,
		};

		Self::deposit_punish(miner, punish_amount)?;

		Ok(())
    }

	pub(super) fn idle_punish(miner: &AccountOf<T>, idle_space: u128, service_space: u128) -> DispatchResult {
		let power = Self::calculate_power(idle_space, service_space);
		let limit = Self::check_collateral_limit(power)?;

		let punish_amount = IDLE_PUNI_MUTI.mul_floor(limit);

		Self::deposit_punish(miner, punish_amount)?;

		Ok(())
	}

	pub(super) fn service_punish(miner: &AccountOf<T>, idle_space: u128, service_space: u128) -> DispatchResult {
		let power = Self::calculate_power(idle_space, service_space);
		let limit = Self::check_collateral_limit(power)?;

		let punish_amount = SERVICE_PUNI_MUTI.mul_floor(limit);

		Self::deposit_punish(miner, punish_amount)?;

		Ok(())
	}
    // Note: that it is necessary to determine whether the state meets the exit conditions before use.
	pub(super) fn force_miner_exit(acc: &AccountOf<T>) -> DispatchResult {
		if let Ok(reward_info) = <RewardMap<T>>::try_get(acc).map_err(|_| Error::<T>::NotExisted) {
			let reward = reward_info.total_reward
				.checked_sub(&reward_info.reward_issued).ok_or(Error::<T>::Overflow)?;
			T::RewardPool::add_reward(reward)?;
		}
		let mut miner_list = AllMiner::<T>::get();
		miner_list.retain(|s| s != acc);
		AllMiner::<T>::put(miner_list);

		<RewardMap<T>>::remove(acc);

		<MinerItems<T>>::try_mutate(acc, |miner_opt| -> DispatchResult {
			let miner = miner_opt.as_mut().ok_or(Error::<T>::Unexpected)?;
			T::StorageHandle::sub_total_idle_space(miner.idle_space)?;
			Self::create_restoral_target(acc, miner.service_space)?;
			miner.state = Self::str_to_bound(STATE_OFFLINE)?;
			let encoding = miner.space_proof_info.pois_key.encode();
			let hashing = sp_io::hashing::sha2_256(&encoding);
			MinerPublicKey::<T>::remove(hashing);
			Ok(())
		})?;

		Ok(())
	}

    pub(super) fn update_restoral_target(miner: &AccountOf<T>, service_space: u128) -> DispatchResult {
        <RestoralTarget<T>>::try_mutate(miner, |info_opt| -> DispatchResult {
            let info = info_opt.as_mut().ok_or(Error::<T>::NotExisted)?;

            info.restored_space = info.restored_space
                .checked_add(service_space).ok_or(Error::<T>::Overflow)?;

            Ok(())
        })
    }
}