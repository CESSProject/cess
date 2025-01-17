use crate::*;

pub struct Receptionist<T: Config>(PhantomData<T>);

impl<T: Config> Receptionist<T> {
    pub fn fly_upload_file(file_hash: Hash, user_brief: UserBrief<T>) -> DispatchResult {
        <File<T>>::try_mutate(&file_hash, |file_opt| -> DispatchResult {
            let file = file_opt.as_mut().ok_or(Error::<T>::FileNonExistent)?;
            let needed_space = SEGMENT_SIZE
                .checked_mul(15).ok_or(Error::<T>::Overflow)?
                .checked_div(10).ok_or(Error::<T>::Overflow)?
                .checked_mul(file.segment_list.len() as u128).ok_or(Error::<T>::Overflow)?;
            ensure!(T::StorageHandle::get_user_avail_space(&user_brief.user, &user_brief.territory_name)? > needed_space, Error::<T>::InsufficientAvailableSpace);
            T::StorageHandle::add_territory_used_space(&user_brief.user, &user_brief.territory_name, needed_space)?;

            Pallet::<T>::add_user_hold_fileslice(&user_brief.user, file_hash, needed_space, user_brief.territory_name.clone())?;
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
        T::StorageHandle::lock_user_space(&user_brief.user, &user_brief.territory_name, needed_space)?;
        // TODO! Replace the file_hash param
        Pallet::<T>::generate_deal(file_hash.clone(), deal_info, user_brief.clone(), file_size)?;
        
        Ok(())
    }

    pub fn qualification_report_processing(sender: AccountOf<T>, deal_hash: Hash, deal_info: &mut DealInfo<T>, index: u8) -> DispatchResult {
        deal_info.complete_part(sender.clone(), index)?;
        <DealMap<T>>::insert(&deal_hash, deal_info.clone());
        // If it is the last submitter of the order.
        if deal_info.complete_list.len() == FRAGMENT_COUNT as usize {
            deal_info.completed_all()?;
            Pallet::<T>::generate_file(
                &deal_hash,
                deal_info.segment_list.clone(),
                deal_info.complete_list.clone(),
                deal_info.user.clone(),
                FileState::Active,
                deal_info.file_size,
            )?;

            let segment_count = deal_info.segment_list.len();
            let needed_space = Pallet::<T>::cal_file_size(segment_count as u128);
            T::StorageHandle::unlock_and_used_user_space(&deal_info.user.user, &deal_info.user.territory_name, needed_space)?;
            T::StorageHandle::sub_total_idle_space(needed_space)?;
            T::StorageHandle::add_total_service_space(needed_space)?;

            Pallet::<T>::add_user_hold_fileslice(&deal_info.user.user, deal_hash.clone(), needed_space, deal_info.user.territory_name.clone())?;
            <DealMap<T>>::remove(deal_hash);
            Pallet::<T>::deposit_event(Event::<T>::StorageCompleted{ file_hash: deal_hash });
        }

        Ok(())
    }
}