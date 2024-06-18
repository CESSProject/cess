use sp_runtime::traits::CheckedDiv;

use super::*;

impl<T: Config> Pallet<T> {
	pub(super) fn add_miner_idle_space(
		acc: &AccountOf<T>, 
		accumulator: Accumulator,
		check_front: u64,
		rear: u64, 
		tee_sig: TeeSig,
	) -> Result<u128, DispatchError> {
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> Result<u128, DispatchError> {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::NotMiner)?;

			// check state 
			ensure!(miner_info.state.to_vec() == STATE_POSITIVE.as_bytes().to_vec(), Error::<T>::NotpositiveState);

			let mut space_proof_info = miner_info.space_proof_info.clone().ok_or(Error::<T>::NotpositiveState)?;

			ensure!(check_front == space_proof_info.front, Error::<T>::CountError);
			ensure!(space_proof_info.rear < rear, Error::<T>::CountError);

			let count = rear.checked_sub(space_proof_info.rear).ok_or(Error::<T>::Overflow)?;
			let idle_space = IDLE_SEG_SIZE.checked_mul(count as u128).ok_or(Error::<T>::Overflow)?;

			space_proof_info.rear = rear;

			space_proof_info.accumulator = accumulator;

			miner_info.idle_space =
				miner_info.idle_space.checked_add(idle_space).ok_or(Error::<T>::Overflow)?;
			
			let currency_cert_space = miner_info.idle_space
				.checked_add(miner_info.service_space).ok_or(Error::<T>::Overflow)?
				.checked_add(miner_info.lock_space).ok_or(Error::<T>::Overflow)?;

			ensure!(currency_cert_space <= miner_info.declaration_space, Error::<T>::ExceedingDeclarationSpace);

			miner_info.tee_signature = tee_sig;

			miner_info.space_proof_info = Some(space_proof_info);

			Ok(idle_space)
		})
	}

    pub(super) fn delete_idle_update_accu(
		acc: &AccountOf<T>, 
		accumulator: Accumulator, 
		front: u64,
		check_rear: u64,
		tee_sig: TeeSig,
	) -> Result<u64, DispatchError> {
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> Result<u64, DispatchError> {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::NotMiner)?;

			let mut space_proof_info = miner_info.space_proof_info.clone().ok_or(Error::<T>::NotpositiveState)?;

			ensure!(check_rear == space_proof_info.rear, Error::<T>::CountError);
			ensure!(space_proof_info.front < front, Error::<T>::CountError);

			let count = front - space_proof_info.front;

			space_proof_info.front = front;

			space_proof_info.accumulator = accumulator;

			miner_info.tee_signature = tee_sig;

			miner_info.space_proof_info = Some(space_proof_info);

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
	) -> DispatchResult {
		let order_list = <CompleteMinerSnapShot<T>>::mutate(&miner, |snap_shot_list| -> Result<Vec<RewardOrder::<BalanceOf<T>, BlockNumberFor<T>>>, DispatchError> {
			if snap_shot_list.len() == 0 {
				return Ok(Default::default());
			}

			let mut order_list: Vec<RewardOrder::<BalanceOf<T>, BlockNumberFor<T>>> = Default::default();

			for snap_shot in snap_shot_list.into_iter() {
				if snap_shot.issued == false {
					let cur_era = T::Staking::current_era();
					if snap_shot.era_index >= cur_era {
						continue;
					}

					let total_power = <CompleteSnapShot<T>>::get(snap_shot.era_index).total_power;
					let total_reward = T::RewardPool::get_round_reward(snap_shot.era_index);
					if total_reward == BalanceOf::<T>::zero() {
						Err(Error::<T>::Unexpected)?;
					}
					let miner_prop = Perbill::from_rational(snap_shot.power, total_power);
					let this_round_reward = miner_prop.mul_floor(total_reward);
					T::RewardPool::sub_round_reward(snap_shot.era_index, this_round_reward)?;
					let each_reward = AOIR_PERCENT
						.mul_floor(this_round_reward)
						.checked_div(&RELEASE_NUMBER.into()).ok_or(Error::<T>::Overflow)?;
					let order = RewardOrder::<BalanceOf<T>, BlockNumberFor<T>> {
						receive_count: 0,
						max_count: RELEASE_NUMBER,
						atonce: false,
						order_reward: this_round_reward,
						each_amount: each_reward,
						last_receive_block: snap_shot.finsh_block,
					};
					order_list.push(order);
					snap_shot.issued = true;
				}
			}

			snap_shot_list.retain(|snap_shot| snap_shot.issued == false);

			Ok(order_list)
		})?;

		if order_list.len() == 0 {
			return Ok(());
		}
		
		// calculate available reward
		RewardMap::<T>::try_mutate(miner, |opt_reward_info| -> DispatchResult {
			let reward_info = opt_reward_info.as_mut().ok_or(Error::<T>::Unexpected)?;
			// traverse the order list

			let mut flag = true;
			if reward_info.order_list.len() == RELEASE_NUMBER as usize {
				flag = false;
			}

			let mut new_reward = BalanceOf::<T>::zero();
			for order in order_list {
				if flag {
					reward_info.total_reward = reward_info.total_reward
						.checked_add(&order.order_reward).ok_or(Error::<T>::Overflow)?;
					new_reward = new_reward.checked_add(&order.order_reward).ok_or(Error::<T>::Overflow)?;
					reward_info.order_list.try_push(order.clone()).map_err(|_| Error::<T>::BoundedVecError)?;
				} else {
					new_reward = new_reward.checked_add(&order.order_reward).ok_or(Error::<T>::Overflow)?;
					T::RewardPool::reward_reserve(order.order_reward)?;
				}
			}
			
			T::RewardPool::sub_reward(new_reward)?;

			Ok(())
		})?;
		
		Ok(())
	}

	pub(super) fn distribute_rewards(miner: &AccountOf<T>, beneficiary: AccountOf<T>) -> DispatchResult {
		<RewardMap<T>>::try_mutate(miner, |opt_reward| -> DispatchResult {
			let reward = opt_reward.as_mut().ok_or(Error::<T>::Unexpected)?;
			let one_day = T::OneDayBlock::get();
			let now = <frame_system::Pallet<T>>::block_number();
			let mut avail_reward: BalanceOf<T> = BalanceOf::<T>::zero(); 

			for order in reward.order_list.iter_mut() {
				let diff = now.checked_sub(&order.last_receive_block).ok_or(Error::<T>::Overflow)?;
				if diff >= one_day {
					let count = diff.checked_div(&one_day).ok_or(Error::<T>::Overflow)?;
					let avail_count: u8;
					if order.receive_count.saturating_add(count.saturated_into()) > order.max_count {
						avail_count = order.max_count.checked_sub(order.receive_count).ok_or(Error::<T>::Unexpected)?;
					} else {
						avail_count = count.saturated_into();
					}

					if avail_count > 0 {
						let order_avail_reward = order.each_amount.checked_mul(&avail_count.into()).ok_or(Error::<T>::Overflow)?;
						avail_reward = avail_reward.checked_add(&order_avail_reward).ok_or(Error::<T>::Overflow)?;
						order.receive_count = order.receive_count.checked_add(avail_count).ok_or(Error::<T>::Overflow)?;
						order.last_receive_block = now;
					}
				}

				if !order.atonce {
					avail_reward = avail_reward.checked_add(
						&(AOIR_PERCENT.mul_floor(order.order_reward))
					).ok_or(Error::<T>::Overflow)?;
					order.atonce = true;
				}
			}

			reward.order_list.retain(|order| order.max_count != order.receive_count);

			reward.reward_issued = reward.reward_issued.checked_add(&avail_reward).ok_or(Error::<T>::Overflow)?;

			T::RewardPool::send_reward_to_miner(beneficiary.clone(), avail_reward)?;

			Self::deposit_event(Event::<T>::Receive { acc: beneficiary, reward: avail_reward });

			Ok(())
		})
	}

	pub(super) fn clear_punish(miner: &AccountOf<T>, idle_space: u128, service_space: u128, count: u8) -> DispatchResult {
		let power = Self::calculate_power(idle_space, service_space);
		let limit: BalanceOf<T> = Self::calculate_limit_by_space(power)?
			.try_into().map_err(|_| Error::<T>::Overflow)?;
		let miner_reward = <RewardMap<T>>::try_get(&miner).map_err(|_| Error::<T>::NotMiner)?;
			
		let reward: u128 = miner_reward.total_reward.try_into().map_err(|_| Error::<T>::Overflow)?;
		let punish_amount = match reward {
			0 => 100u128.try_into().map_err(|_| Error::<T>::Overflow)?,
			_ => {
				let punish_amount = match count {
					1 => BalanceOf::<T>::zero(),
					2 => Perbill::from_percent(5).mul_floor(limit),
					3 => Perbill::from_percent(15).mul_floor(limit),
					_ => Perbill::from_percent(15).mul_floor(limit),
				};
				punish_amount
			},
		};

		Self::deposit_punish(miner, punish_amount)?;

		Ok(())
    }

	pub(super) fn idle_punish(miner: &AccountOf<T>, idle_space: u128, service_space: u128) -> DispatchResult {
		let power = Self::calculate_power(idle_space, service_space);
		let limit: BalanceOf<T> = Self::calculate_limit_by_space(power)?
			.try_into().map_err(|_| Error::<T>::Overflow)?;

		let punish_amount = IDLE_PUNI_MUTI.mul_floor(limit);

		Self::deposit_punish(miner, punish_amount)?;

		Ok(())
	}

	pub(super) fn service_punish(miner: &AccountOf<T>, idle_space: u128, service_space: u128) -> DispatchResult {
		let power = Self::calculate_power(idle_space, service_space);
		let limit: BalanceOf<T> = Self::calculate_limit_by_space(power)?
			.try_into().map_err(|_| Error::<T>::Overflow)?;

		let punish_amount = SERVICE_PUNI_MUTI.mul_floor(limit);

		Self::deposit_punish(miner, punish_amount)?;

		Ok(())
	}
    // Note: that it is necessary to determine whether the state meets the exit conditions before use.
	pub(super) fn force_miner_exit(acc: &AccountOf<T>) -> DispatchResult {
		let mut miner_list = AllMiner::<T>::get();
		miner_list.retain(|s| s != acc);
		AllMiner::<T>::put(miner_list);

		<MinerItems<T>>::try_mutate(acc, |miner_opt| -> DispatchResult {
			let miner = miner_opt.as_mut().ok_or(Error::<T>::Unexpected)?;
			if let Ok(reward_info) = <RewardMap<T>>::try_get(acc).map_err(|_| Error::<T>::NotExisted) {
				// T::RewardPool::send_reward_to_miner(miner.beneficiary.clone(), reward_info.total_reward)?;
				if reward_info.total_reward == BalanceOf::<T>::zero() {
					let spec_acc = T::ReservoirGate::get_reservoir_acc();
					if spec_acc == miner.staking_account {
						T::ReservoirGate::redeem(acc, miner.collaterals, false)?;
					}
					T::Currency::unreserve(&miner.staking_account, miner.collaterals);
				} else {
					Self::calculate_miner_reward(acc)?;
					Self::distribute_rewards(acc, miner.beneficiary.clone())?;
					let residue_reward = reward_info.total_reward.checked_sub(&reward_info.reward_issued).ok_or(Error::<T>::Overflow)?;
					T::RewardPool::add_reward(residue_reward)?;
					let start_block = <StakingStartBlock<T>>::try_get(&acc).map_err(|_| Error::<T>::BugInvalid)?;
					let staking_lock_block = T::StakingLockBlock::get();
					let exec_block = start_block.checked_add(&staking_lock_block).ok_or(Error::<T>::Overflow)?;
					<ReturnStakingSchedule<T>>::try_mutate(&exec_block, |miner_list| -> DispatchResult {
						miner_list
							.try_push((acc.clone(), miner.staking_account.clone(), miner.collaterals.clone()))
							.map_err(|_| Error::<T>::BoundedVecError)?;

						Ok(())
					})?;
				}
			}
			T::StorageHandle::sub_total_idle_space(miner.idle_space + miner.lock_space)?;
			Self::create_restoral_target(acc, miner.service_space + miner.lock_space)?;
			miner.state = Self::str_to_bound(STATE_OFFLINE)?;
			let space_proof_info = miner.space_proof_info.clone().ok_or(Error::<T>::NotpositiveState)?;
			let encoding = space_proof_info.pois_key.encode();
			let hashing = sp_io::hashing::sha2_256(&encoding);
			MinerPublicKey::<T>::remove(hashing);
			Ok(())
		})?;

		<CompleteMinerSnapShot<T>>::remove(acc);
		<RewardMap<T>>::remove(acc);
		<PendingReplacements<T>>::remove(acc);

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