use super::*;

impl<T: Config> Pallet<T> {
    pub fn check_file_spec(seg_list: &BoundedVec<SegmentList<T>, T::SegmentCount>) -> bool {
        let spec_len = T::FragmentCount::get();

        for segment in seg_list {
            if segment.fragment_list.len() as u32 != spec_len {
                return false
            }
        }

        true 
    }

    pub fn generate_file(
        file_hash: &Hash,
        deal_info: BoundedVec<SegmentList<T>, T::SegmentCount>,
        complete_list: BoundedVec<CompleteInfo<T>, T::FragmentCount>,
        user_brief: UserBrief<T>,
        stat: FileState,
        file_size: u128,
    ) -> DispatchResult {
        let mut segment_info_list: BoundedVec<SegmentInfo<T>, T::SegmentCount> = Default::default();
        ensure!(complete_list.len() == FRAGMENT_COUNT as usize, Error::<T>::Unexpected);
        let mut complete_list = complete_list;
        complete_list.sort_by_key(|info| info.index);
        for segment in deal_info.iter() {
            let mut segment_info = SegmentInfo::<T> {
                hash: segment.hash,
                fragment_list: Default::default(),
            };
            for (index, fragment_hash) in segment.fragment_list.iter().enumerate() {
                let frag_info = FragmentInfo::<T> {
                    hash:  *fragment_hash,
                    avail: true,
                    tag: None,
                    miner: complete_list[index as usize].miner.clone(),
                };

                segment_info.fragment_list.try_push(frag_info).map_err(|_e| Error::<T>::BoundedVecError)?;
            }

            segment_info_list.try_push(segment_info).map_err(|_e| Error::<T>::BoundedVecError)?;
        }
        
        let cur_block = <frame_system::Pallet<T>>::block_number();

        let file_info = FileInfo::<T> {
            segment_list: segment_info_list,
            owner: vec![user_brief].try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
            file_size,
            completion: cur_block,
            stat: stat,
        };

        <File<T>>::insert(file_hash, file_info);

        Ok(())
    }

    pub(super) fn generate_deal(
        file_hash: Hash, 
        file_info: BoundedVec<SegmentList<T>, T::SegmentCount>, 
        user_brief: UserBrief<T>,
        file_size: u128,
    ) -> DispatchResult {

        let deal = DealInfo::<T> {
            file_size,
            segment_list: file_info.clone(),
            user: user_brief,
            complete_list: Default::default(),
        };

        DealMap::insert(&file_hash, deal);

        Ok(())
    }

    pub fn remove_deal(deal_hash: &Hash) -> DispatchResult {
        let deal_info = <DealMap<T>>::try_get(deal_hash).map_err(|_| Error::<T>::NonExistent)?;
        let segment_len = deal_info.segment_list.len() as u128;
		let needed_space = Self::cal_file_size(segment_len);
		T::StorageHandle::unlock_user_space(&deal_info.user.user, &deal_info.user.territory_name, needed_space)?;
		// unlock mienr space
		for complete_info in deal_info.complete_list {
            T::MinerControl::unlock_space(&complete_info.miner, FRAGMENT_SIZE * segment_len)?;
		}

		<DealMap<T>>::remove(deal_hash);

        Ok(())
    }

    pub(super) fn cal_file_size(len: u128) -> u128 {
        len * (SEGMENT_SIZE * 30 / 10)
    }

    pub(super) fn delete_user_file(file_hash: &Hash, acc: &AccountOf<T>, file: &FileInfo<T>) -> Result<Weight, DispatchError> {
        let mut weight: Weight = Weight::zero();

		for user_brief in file.owner.iter() {
            if &user_brief.user == acc {
                if file.owner.len() > 1 {
                    Self::remove_file_owner(&file_hash, &acc, true)?;
                    weight = weight.saturating_add(T::DbWeight::get().reads_writes(2, 2));
                 } else {
                    let temp_weight  = Self::remove_file_last_owner(&file_hash, &acc, true)?;
                    weight = weight.saturating_add(temp_weight);
                }
            }
		}

        Ok(weight)
    }

    pub(super) fn remove_user_hold_file_list(
        file_hash: &Hash, 
        acc: &AccountOf<T>,
    ) -> DispatchResult {
        <UserHoldFileList<T>>::try_mutate(acc, |file_list| -> DispatchResult {
            file_list.retain(|temp_file| &temp_file.file_hash != file_hash);
            Ok(())
        })
    }


    // The status of the file must be confirmed before use.
    pub(super) fn remove_file_owner(file_hash: &Hash, acc: &AccountOf<T>, user_clear: bool) -> DispatchResult {
        <File<T>>::try_mutate(file_hash, |file_opt| -> DispatchResult {
            let file = file_opt.as_mut().ok_or(Error::<T>::NonExistent)?;
            for (index, user_brief) in file.owner.iter().enumerate() {
                if acc == &user_brief.user {
                    let file_size = Self::cal_file_size(file.segment_list.len() as u128);
                    if user_clear {
                        T::StorageHandle::sub_territory_used_space(acc, &user_brief.territory_name, file_size)?;
                    }
                    file.owner.remove(index);
                    break;
                }
            }
            Ok(())
        })?;

        Ok(())
    }

    // The status of the file must be confirmed before use.
    pub(super) fn remove_file_last_owner(file_hash: &Hash, acc: &AccountOf<T>, user_clear: bool) -> Result<Weight, DispatchError> {
        let mut weight = Weight::zero();

        let file = <File<T>>::try_get(file_hash).map_err(|_| Error::<T>::NonExistent)?; // reads 1
        weight = weight.saturating_add(T::DbWeight::get().reads(1));
        // Record the total number of fragments that need to be deleted.
        let mut total_fragment_dec = 0;
        // Used to record and store the amount of service space that miners need to reduce, 
        // and read changes once through counting
        let mut miner_list: BTreeMap<AccountOf<T>, Vec<(Hash, bool)>> = Default::default();
        // Traverse every segment
        for segment_info in file.segment_list.iter() {
            for fragment_info in segment_info.fragment_list.iter() {
                // The total number of fragments in a file should never exceed u32
                total_fragment_dec += 1;
                let tag_avail = match fragment_info.tag {
                    Some(_) => true,
                    None => false,
                };
                if miner_list.contains_key(&fragment_info.miner) {
                    let temp_list = miner_list.get_mut(&fragment_info.miner).ok_or(Error::<T>::BugInvalid)?;
                    // The total number of fragments in a file should never exceed u32
                    temp_list.push((fragment_info.hash, tag_avail));
                } else {
                    miner_list.insert(fragment_info.miner.clone(), vec![(fragment_info.hash, tag_avail)]);
                }
            }
        }

        for (miner, hash_list) in miner_list.iter() {
            let count = hash_list.len() as u128;
            
            if T::MinerControl::restoral_target_is_exist(miner) {
                T::MinerControl::update_restoral_target(miner, FRAGMENT_SIZE * count)?;
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
            } else {
                let mut count: u128 = 0;
                let mut unlock_count: u128 = 0;
                let mut binary_list: Vec<Box<[u8; 256]>> = Default::default(); 
                for (fragment_hash, tag_avail) in hash_list {
                    if *tag_avail {
                        let binary_temp = fragment_hash.binary().map_err(|_| Error::<T>::BugInvalid)?;
                        binary_list.push(binary_temp);
                        count = count + 1;
                    } else {
                        unlock_count = unlock_count + 1;
                    }
                }
                T::MinerControl::sub_miner_service_space(miner, FRAGMENT_SIZE * count)?;
                T::MinerControl::delete_service_bloom(miner, binary_list)?;
                T::MinerControl::unlock_space_direct(miner, FRAGMENT_SIZE * unlock_count)?;
                weight = weight.saturating_add(T::DbWeight::get().reads_writes(3, 3));
            }
            
        }

        if user_clear {
            for user_brief in file.owner {
                if &user_brief.user == acc {
                    T::StorageHandle::sub_territory_used_space(acc, &user_brief.territory_name, total_fragment_dec as u128 * FRAGMENT_SIZE)?;
                    weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
                    break;
                }
            }
        }
        T::StorageHandle::sub_total_service_space(total_fragment_dec as u128 * FRAGMENT_SIZE)?;
        weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

        <File<T>>::remove(file_hash);
        weight = weight.saturating_add(T::DbWeight::get().writes(1));

        Ok(weight)
    }
    /// helper: generate random number.
    ///
    /// Get a random number.
    ///
    /// Parameters:
    /// - `seed`: random seed.
    /// Result:
    /// - `u32`: random number.
    pub fn generate_random_number(seed: u32) -> Result<u32, DispatchError> {
        let mut counter = 0;
        loop {           
            let (random_seed, _) =
                T::MyRandomness::random(&(T::FilbakPalletId::get(), seed + counter).encode());
            let random_seed = match random_seed {
                Some(v) => v,
                None => {
                    #[cfg(feature = "runtime-benchmarks")]
                    return Ok(seed);

                    Default::default()
                },
            };
            let random_number = <u32>::decode(&mut random_seed.as_ref()).unwrap_or(0);
            if random_number != 0 {
                return Ok(random_number)
            }
            counter = counter.checked_add(1).ok_or(Error::<T>::Overflow)?;
        }
    }
    /// helper: add user hold fileslice.
    ///
    /// Add files held by users.
    ///
    /// Parameters:
    /// - `user`: AccountId.
    /// - `file_hash_bound`: file hash.
    /// - `file_size`: file size.
    ///
    /// Result:
    /// - DispatchResult
    pub(super) fn add_user_hold_fileslice(
        user: &AccountOf<T>,
        file_hash: Hash,
        file_size: u128,
        territory_name: TerrName,
    ) -> DispatchResult {
        let file_info = UserFileSliceInfo { 
            territory_name, 
            file_hash: file_hash, 
            file_size,
        };
        <UserHoldFileList<T>>::try_mutate(user, |v| -> DispatchResult {
            ensure!(!v.contains(&file_info), Error::<T>::Existed);
            v.try_push(file_info).map_err(|_| Error::<T>::StorageLimitReached)?;
            Ok(())
        })?;

        Ok(())
    }
    /// helper: get current scheduler.
    ///
    /// Get the current block consensus.
    ///
    /// Parameters:
    ///
    /// Result:
    /// - AccountOf: consensus
    // pub(super) fn get_current_scheduler() -> Result<AccountOf<T>, DispatchError> {
    //     let digest = <frame_system::Pallet<T>>::digest();
    //     let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
    //     let acc = T::FindAuthor::find_author(pre_runtime_digests).map(|a| a);
    //     let acc = match acc {
    //         Some(e) => T::Scheduler::get_controller_acc(e),
    //         None => T::Scheduler::get_first_controller()?,
    //     };
    //     Ok(acc)
    // }
    /// helper: check_is_file_owner.
    ///
    /// Check whether the user is the owner of the file.
    ///
    /// Parameters:
    ///
    /// Result:
    /// - acc: Inspected user.
    /// - file_hash: File hash, the unique identifier of the file.
    pub fn check_is_file_owner(acc: &AccountOf<T>, file_hash: &Hash) -> bool {
        if let Some(file) = <File<T>>::get(file_hash) {
            for user_brief in file.owner.iter() {
                if &user_brief.user == acc {
                    return true;
                }
            }
        }
        false
    }
    /// helper: Permission check method.
    /// Check whether the origin has the owner's authorization
    /// or whether the origin is the owner
    ///
    /// Parameters:
    /// - `acc`: AccountId.
    ///
    /// Result:
    /// - bool: True means there is permission, false means there is no permission.
    pub fn check_permission(operator: AccountOf<T>, owner: AccountOf<T>) -> bool {
        if owner == operator || T::OssFindAuthor::is_authorized(owner, operator) {
            return true;
        }
        false
    }

    // FIXME: Will this function still be used?
    #[allow(dead_code)]
    pub(super) fn check_name_spec(name: Vec<u8>) -> bool {
        let mut point_flag: bool = false;
        let mut count = 0;
        for elem in &name {
            if !BUCKET_ALLOW_CHAR.contains(elem) {
                return false;
            }

            if elem == &BUCKET_ALLOW_CHAR[64] {
                count += 1;
                if point_flag {
                    return false;
                }
                point_flag = true;
            } else {
                point_flag = false
            }
        }

        if count == 3 {
            let arr_iter = name.split(|num| num == &BUCKET_ALLOW_CHAR[64]);
            for arr in arr_iter {
                for elem in arr {
                    if !BUCKET_ALLOW_CHAR.contains(elem) {
                        return true;
                    }
                }
            }

            return false
        }

        true
    }

    pub(super) fn get_segment_length_from_deal(deal_hash: &Hash) -> u32 {
        if let Ok(deal) = <DealMap<T>>::try_get(deal_hash) {
            return deal.segment_list.len() as u32;
        }

        return 0;
    }    
}