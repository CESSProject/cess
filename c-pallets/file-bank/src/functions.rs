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
        miner_task_list: BoundedVec<MinerTaskList<T>, T::StringLimit>,
        share_info: Vec<SegmentInfo<T>>,
        user_brief: UserBrief<T>,
        stat: FileState,
    ) -> DispatchResult {
        let mut segment_info_list: BoundedVec<SegmentInfo<T>, T::SegmentCount> = Default::default();
        for segment in deal_info.iter() {
            let mut segment_info = SegmentInfo::<T> {
                hash: segment.hash,
                fragment_list: Default::default(),
            };

            let mut flag = true;
            for share_segment_info in &share_info {
                if segment.hash == share_segment_info.hash {
                    segment_info.fragment_list = share_segment_info.fragment_list.clone();
                    flag = false;
                    break;
                }
            };

            if flag {
                for frag_hash in segment.fragment_list.iter() {
                    for miner_task in &miner_task_list {
                        if miner_task.fragment_list.contains(frag_hash) {
                            let frag_info = FragmentInfo::<T> {
                                hash:  *frag_hash,
                                avail: true,
                                miner: miner_task.miner.clone(),
                            };
                            segment_info.fragment_list.try_push(frag_info).map_err(|_e| Error::<T>::BoundedVecError)?;
                        }
                    }
                }
            }
            
            segment_info_list.try_push(segment_info).map_err(|_e| Error::<T>::BoundedVecError)?;
        }

        for segment_info in &segment_info_list {
            if <SegmentMap<T>>::contains_key(segment_info.hash) {
                <SegmentMap<T>>::try_mutate(segment_info.hash, |segment_opt| -> DispatchResult {
                    let segment_tuple = segment_opt.as_mut().ok_or(Error::<T>::BugInvalid)?;
                    segment_tuple.1 = segment_tuple.1.checked_add(1).ok_or(Error::<T>::Overflow)?;
                    Ok(())
                })?;
            } else {
                <SegmentMap<T>>::insert(segment_info.hash, (segment_info, 0));
                T::StorageHandle::add_total_service_space(Self::cal_file_size(1))?;
                T::StorageHandle::sub_total_idle_space(Self::cal_file_size(1))?;
            }
        }

        let cur_block = <frame_system::Pallet<T>>::block_number();

        let file_info = FileInfo::<T> {
            completion: cur_block,
            stat: stat,
            segment_list: segment_info_list,
            owner: vec![user_brief].try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
        };

        <File<T>>::insert(file_hash, file_info);

        Ok(())
    }

    pub fn create_bucket_helper(
        user: &AccountOf<T>, 
        bucket_name: &BoundedVec<u8, T::NameStrLimit>, 
        file_hash: Option<Hash>,
    ) -> DispatchResult {
        // TODO! len() & ?
        ensure!(bucket_name.len() >= 3, Error::<T>::LessMinLength);
        ensure!(!<Bucket<T>>::contains_key(user, bucket_name), Error::<T>::Existed);

        let mut bucket = BucketInfo::<T> {
            object_list: Default::default(),
            authority: vec![user.clone()].try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
        };

        if let Some(hash) = file_hash {
            bucket.object_list.try_push(hash).map_err(|_e| Error::<T>::BoundedVecError)?;
        }

        <Bucket<T>>::insert(user, bucket_name, bucket);

        Ok(())
    }

    pub fn add_file_to_bucket(
        user: &AccountOf<T>, 
        bucket_name: &BoundedVec<u8, T::NameStrLimit>, 
        file_hash: &Hash,
    ) -> DispatchResult {
        <Bucket<T>>::try_mutate(user, bucket_name, |bucket_opt| -> DispatchResult {
            let bucket = bucket_opt.as_mut().ok_or(Error::<T>::NonExistent)?;
            bucket.object_list.try_push(*file_hash).map_err(|_e| Error::<T>::BoundedVecError)?;

            Ok(())
        })
    }

    pub(super) fn generate_deal(
        file_hash: Hash, 
        needed_list: BoundedVec<SegmentList<T>, T::SegmentCount>, 
        file_info: BoundedVec<SegmentList<T>, T::SegmentCount>, 
        user_brief: UserBrief<T>,
        share_info: Vec<SegmentInfo<T>>,
    ) -> DispatchResult {
        let miner_task_list = Self::random_assign_miner(&needed_list)?;

        Self::start_first_task(file_hash.0.to_vec(), file_hash, 1)?;

        let deal = DealInfo::<T> {
            stage: 1,
            segment_list: file_info,
            needed_list: needed_list,
            user: user_brief,
            assigned_miner: miner_task_list,
            share_info: share_info.try_into().map_err(|_| Error::<T>::BoundedVecError)?,
            complete_list: Default::default(),
        };

        DealMap::insert(&file_hash, deal);

        Ok(())
    }

    pub(super) fn start_first_task(task_id: Vec<u8>, deal_hash: Hash, count: u8) -> DispatchResult {
        let start: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();
        let survival_block = start.checked_add(600 * (count as u32)).ok_or(Error::<T>::Overflow)?;

        T::FScheduler::schedule_named(
                task_id, // TODO!
                DispatchTime::At(survival_block.saturated_into()),
                Option::None,
                schedule::HARD_DEADLINE,
                frame_system::RawOrigin::Root.into(),
                Call::deal_reassign_miner{deal_hash: deal_hash, count: count}.into(), // TODO!
        ).map_err(|_| Error::<T>::Unexpected)?;

        Ok(())
    }

    pub(super) fn start_second_task(task_id: Vec<u8>, deal_hash: Hash, count: u8) -> DispatchResult {
        let start: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();
        let survival_block = start.checked_add(1 * (count as u32)).ok_or(Error::<T>::Overflow)?;

        T::FScheduler::schedule_named(
                task_id, // TODO!
                DispatchTime::At(survival_block.saturated_into()),
                Option::None,
                schedule::HARD_DEADLINE,
                frame_system::RawOrigin::Root.into(),
                Call::calculate_end{deal_hash: deal_hash}.into(), // TODO!
        ).map_err(|_| Error::<T>::Unexpected)?;

        Ok(())
    }

    pub(super) fn random_assign_miner(
        needed_list: &BoundedVec<SegmentList<T>, T::SegmentCount>
    ) -> Result<BoundedVec<MinerTaskList<T>, T::StringLimit>, DispatchError> {
        let mut index_list: Vec<u32> = Default::default();
        let mut miner_task_list: BoundedVec<MinerTaskList<T>, T::StringLimit> = Default::default();
        let mut miner_idle_space_list: Vec<u128> = Default::default();
        // The optimal number of miners required for storage.
        // segment_size * 1.5 / fragment_size.
        let miner_count: u32 = (SEGMENT_SIZE * 15 / 10 / FRAGMENT_SIZE) as u32;
        let mut seed = <frame_system::Pallet<T>>::block_number().saturated_into();

        let all_miner = T::MinerControl::get_all_miner()?;
        let total = all_miner.len() as u32;

        // ensure!(total > miner_count, Error::<T>::NodesInsufficient);
        // Maximum number of cycles set to prevent dead cycles TODO!
        let max_count = miner_count * 5;
        let mut cur_count = 0;
        let mut total_idle_space = 0;
        // start random choose miner
        loop {
            // Get a random subscript.
            let index = Self::generate_random_number(seed)? as u32 % total;
            // seed + 1
            seed = seed.checked_add(1).ok_or(Error::<T>::Overflow)?;
            // Number of cycles plus 1
            cur_count += 1;
            // When the number of cycles reaches the upper limit, the cycle ends.
            if cur_count == max_count {
                break;
            }
            // End the cycle after all storage nodes have been traversed.
            if total == index_list.len() as u32 {
                break;
            }
            // Continue to the next cycle when the current random result already exists.
            if index_list.contains(&index) {
                continue;
            }
            
            // Judge whether the idle space of the miners is sufficient.
            let miner = all_miner[index as usize].clone();
            let result = T::MinerControl::is_positive(&miner)?;
            if !result {
                continue;
            }
            // Record current cycle results.
            index_list.push(index);
            let cur_space: u128 = T::MinerControl::get_miner_idle_space(&miner)?;
            // If sufficient, the miner is selected.
            if cur_space > needed_list.len() as u128 * FRAGMENT_SIZE {
                // Accumulate all idle space of currently selected miners
                total_idle_space = total_idle_space.checked_add(&cur_space).ok_or(Error::<T>::Overflow)?;
                let miner_task = MinerTaskList::<T>{
                    miner: miner,
                    fragment_list: Default::default(),
                };
                miner_task_list.try_push(miner_task).map_err(|_e| Error::<T>::BoundedVecError)?;
                miner_idle_space_list.push(cur_space);
            }
            // If the selected number of miners has reached the optimal number, the cycle ends.
            if miner_task_list.len() as u32 == miner_count {
                break;
            }
        }
        
        ensure!(miner_task_list.len() != 0, Error::<T>::BugInvalid);
        ensure!(total_idle_space > SEGMENT_SIZE * 15 / 10, Error::<T>::NodesInsufficient);

        // According to the selected miner.
        // Assign responsible documents to miners.
        for segment_list in needed_list {
            let mut index = 0;
            for hash in &segment_list.fragment_list {
                // To prevent the number of miners from not meeting the fragment number.
                // It may occur that one miner stores multiple fragments
                loop {
                    // Obtain the account of the storage node through the subscript.
                    // To prevent exceeding the boundary, use '%'.
                    let temp_index = index % miner_task_list.len();
                    let cur_space = miner_idle_space_list[temp_index];
                    // To prevent a miner from storing multiple fragments,
                    // the idle space is insufficient
                    if cur_space > (miner_task_list[temp_index].fragment_list.len() as u128 + 1) * FRAGMENT_SIZE {
                        miner_task_list[temp_index].fragment_list.try_push(*hash).map_err(|_e| Error::<T>::BoundedVecError)?;
                        break;
                    }
                    index = index.checked_add(1).ok_or(Error::<T>::PanicOverflow)?;
                }
                index = index.checked_add(1).ok_or(Error::<T>::PanicOverflow)?;
            }
        }
        // lock miner space
        for miner_task in miner_task_list.iter() {
            T::MinerControl::lock_space(&miner_task.miner, miner_task.fragment_list.len() as u128 * FRAGMENT_SIZE)?;
        }
         
        
        Ok(miner_task_list)
    }

    pub(super) fn cal_file_size(len: u128) -> u128 {
        len * (SEGMENT_SIZE * 15 / 10)
    }

    // The status of the file must be confirmed before use.
    pub(super) fn remove_file_owner(file_hash: &Hash, acc: &AccountOf<T>) -> DispatchResult {
        <File<T>>::try_mutate(file_hash, |file_opt| -> DispatchResult {
            let file = file_opt.as_mut().ok_or(Error::<T>::Overflow)?;
            for (index, user_brief) in file.owner.iter().enumerate() {
                if acc == &user_brief.user {
                    let file_size = Self::cal_file_size(file.segment_list.len() as u128);
                    T::StorageHandle::update_user_space(acc, 2, file_size)?;
                    file.owner.remove(index);
                    break;
                }
            }
            Ok(())
        })?;

        Ok(())
    }

    // The status of the file must be confirmed before use.
    pub(super) fn remove_file_last_owner(file_hash: &Hash, acc: &AccountOf<T>) -> DispatchResult {
        let file = <File<T>>::try_get(file_hash).map_err(|_| Error::<T>::NonExistent)?;
        // Record the total number of fragments that need to be deleted.
        let mut total_fragment_dec = 0;
        // Used to record and store the amount of service space that miners need to reduce, 
        // and read changes once through counting
        let mut miner_list: BTreeMap<AccountOf<T>, u32> = Default::default();
        // Traverse every segment
        for segment_info in file.segment_list.iter() {
            let flag = <SegmentMap<T>>::try_mutate(segment_info.hash, |segment_opt| -> Result<bool, DispatchError> {
                let (segment_info, count) = segment_opt.as_mut().ok_or(Error::<T>::BugInvalid)?;
                // Determine whether the segment is shared
                if *count >= 1 {
                    *count = count.checked_sub(1).ok_or(Error::<T>::Overflow)?;
                } else {
                    for fragment_info in segment_info.fragment_list.iter() {
                        // The total number of fragments in a file should never exceed u32
                        total_fragment_dec += 1;
                        if miner_list.contains_key(&fragment_info.miner) {
                            let temp_count = miner_list.get_mut(&fragment_info.miner).ok_or(Error::<T>::BugInvalid)?;
                            // The total number of fragments in a file should never exceed u32
                            *temp_count += 1;
                        } else {
                            miner_list.insert(fragment_info.miner.clone(), 1);
                        }
                    }
                    return Ok(true);
                }
                Ok(false)
            })?;

            if flag {
                <SegmentMap<T>>::remove(segment_info.hash);
            }
        }

        for (miner, count) in miner_list.iter() {
            T::MinerControl::sub_miner_service_space(miner, FRAGMENT_SIZE * *count as u128)?;
        }

        let file_size = Self::cal_file_size(file.segment_list.len() as u128);
        T::StorageHandle::update_user_space(acc, 2, file_size)?;
        T::StorageHandle::sub_total_service_space(total_fragment_dec as u128 * FRAGMENT_SIZE)?;

        <File<T>>::remove(file_hash);

        Ok(())
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
                None => Default::default(),
            };
            let random_number = <u32>::decode(&mut random_seed.as_ref()).unwrap_or(0);
            if random_number != 0 {
                return Ok(random_number)
            }
            counter = counter.checked_add(1).ok_or(Error::<T>::Overflow)?;
        }
    }
    /// helper: delete filler.
    ///
    /// delete filler.
    ///
    /// Parameters:
    /// - `miner_acc`: miner AccountId.
    /// - `filler_hash`: filler hash.
    /// Result:
    /// - DispatchResult
    pub fn delete_filler(miner_acc: AccountOf<T>, filler_hash: Hash) -> DispatchResult {
        if !<FillerMap<T>>::contains_key(&miner_acc, filler_hash.clone()) {
            Err(Error::<T>::FileNonExistent)?;
        }
        let _value = <FillerMap<T>>::try_get(&miner_acc, filler_hash.clone()) //read 1
            .map_err(|_e| Error::<T>::FileNonExistent)?;
        <FillerMap<T>>::remove(miner_acc, filler_hash.clone()); //write 1

        Ok(())
    }

    pub fn clear_bucket_file(
        file_hash: &Hash,
        owner: &AccountOf<T>,
        bucket_name: &BoundedVec<u8, T::NameStrLimit>,
    ) -> Result<Weight, DispatchError> {
        let mut weight: Weight = Weight::from_ref_time(0);
        ensure!(<Bucket<T>>::contains_key(owner, bucket_name), Error::<T>::NonExistent);

        <Bucket<T>>::try_mutate(owner, bucket_name, |bucket_opt| -> DispatchResult {
            let bucket = bucket_opt.as_mut().ok_or(Error::<T>::Unexpected)?;
            ensure!(bucket.object_list.contains(file_hash), Error::<T>::NonExistent);
            let mut index: usize = 0;
            for object in bucket.object_list.iter() {
                if object == file_hash {
                    break;
                }
                index = index.checked_add(1).ok_or(Error::<T>::Overflow)?;
            }
            bucket.object_list.remove(index);
            Ok(())
        })?;
        weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

        Ok(weight)
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
    ) -> DispatchResult {
        let file_info =
            UserFileSliceInfo { file_hash: file_hash, file_size };
        <UserHoldFileList<T>>::try_mutate(user, |v| -> DispatchResult {
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

    pub(super) fn _record_uploaded_files_size(scheduler_id: &T::AccountId, file_size: u64) -> DispatchResult {
        T::CreditCounter::record_proceed_block_size(scheduler_id, file_size)?;
        Ok(())
    }

    pub(super) fn record_uploaded_fillers_size(scheduler_id: &T::AccountId, fillers: &Vec<FillerInfo<T>>) -> DispatchResult {
        for filler in fillers {
            T::CreditCounter::record_proceed_block_size(scheduler_id, filler.filler_size)?;
        }
        Ok(())
    }

    pub(super) fn clear_filler(miner: &AccountOf<T>, maybe_cursor: Option<&[u8]>) {
        let result = <FillerMap<T>>::clear_prefix(miner, 100000, maybe_cursor);
        if let Some(cursor) = result.maybe_cursor {
            Self::clear_filler(miner, Some(&cursor));
        }
    }
}