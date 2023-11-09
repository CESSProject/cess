use crate::*;

pub struct Receptionist<T: Config>(PhantomData<T>);

impl<T: Config> Receptionist<T> {
    pub fn fly_upload_file(file_hash: Hash, user_brief: UserBrief<T>, used_space: u128) -> DispatchResult {
        T::StorageHandle::update_user_space(&user_brief.user, 1, used_space)?;

        if <Bucket<T>>::contains_key(&user_brief.user, &user_brief.bucket_name) {
            Pallet::<T>::add_file_to_bucket(&user_brief.user, &user_brief.bucket_name, &file_hash)?;
        } else {
            Pallet::<T>::create_bucket_helper(&user_brief.user, &user_brief.bucket_name, Some(file_hash))?;
        }

        Pallet::<T>::add_user_hold_fileslice(&user_brief.user, file_hash, used_space)?;

        <File<T>>::try_mutate(&file_hash, |file_opt| -> DispatchResult {
            let file = file_opt.as_mut().ok_or(Error::<T>::FileNonExistent)?;
            file.owner.try_push(user_brief.clone()).map_err(|_e| Error::<T>::BoundedVecError)?;
            Ok(())
        })?;

        Ok(())
    }

    pub fn generate_deal(
        file_hash: Hash,
        deal_info: BoundedVec<SegmentList<T>, T::SegmentCount>,
        user_brief: UserBrief<T>,
        needed_space: u128,
        file_size: u128,
    ) -> DispatchResult {
        T::StorageHandle::lock_user_space(&user_brief.user, needed_space)?;
        // TODO! Replace the file_hash param
        Pallet::<T>::generate_deal(file_hash.clone(), deal_info, user_brief.clone(), file_size)?;
        
        Ok(())
    }

    pub fn qualification_report_processing(sender: AccountOf<T>, deal_hash: Hash, deal_info: &mut DealInfo<T>, index: u8) -> DispatchResult {
        deal_info.complete_part(sender.clone(), index)?;

        // If it is the last submitter of the order.
        if deal_info.complete_list.len() == FRAGMENT_COUNT as usize {
            deal_info.completed_all()?;
            Pallet::<T>::generate_file(
                &deal_hash,
                deal_info.segment_list.clone(),
                deal_info.complete_list.clone(),
                deal_info.user.clone(),
                FileState::Calculate,
                deal_info.file_size,
            )?;

            let segment_count = deal_info.segment_list.len();
            let needed_space = Pallet::<T>::cal_file_size(segment_count as u128);
            T::StorageHandle::unlock_and_used_user_space(&deal_info.user.user, needed_space)?;
            T::StorageHandle::sub_total_idle_space(needed_space)?;
            T::StorageHandle::add_total_service_space(needed_space)?;
            let result = T::FScheduler::cancel_named(deal_hash.0.to_vec()).map_err(|_| Error::<T>::Unexpected);
            if let Err(_) = result {
                log::info!("transfer report cancel schedule failed: {:?}", deal_hash.clone());
            }
            // Calculate the maximum time required for storage nodes to tag files
            let max_needed_cal_space = (segment_count as u32).checked_mul(FRAGMENT_SIZE as u32).ok_or(Error::<T>::Overflow)?;
            let mut life: u32 = (max_needed_cal_space / TRANSFER_RATE as u32).checked_add(11).ok_or(Error::<T>::Overflow)?;
            life = (max_needed_cal_space / CALCULATE_RATE as u32)
                .checked_add(30).ok_or(Error::<T>::Overflow)?
                .checked_add(life).ok_or(Error::<T>::Overflow)?;
                Pallet::<T>::start_second_task(deal_hash.0.to_vec(), deal_hash, life)?;
            if <Bucket<T>>::contains_key(&deal_info.user.user, &deal_info.user.bucket_name) {
                Pallet::<T>::add_file_to_bucket(&deal_info.user.user, &deal_info.user.bucket_name, &deal_hash)?;
            } else {
                Pallet::<T>::create_bucket_helper(&deal_info.user.user, &deal_info.user.bucket_name, Some(deal_hash))?;
            }
            Pallet::<T>::add_user_hold_fileslice(&deal_info.user.user, deal_hash.clone(), needed_space)?;
            Pallet::<T>::deposit_event(Event::<T>::StorageCompleted{ file_hash: deal_hash });
        }

        Ok(())
    }
}