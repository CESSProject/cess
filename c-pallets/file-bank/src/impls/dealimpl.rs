use crate::*;

impl<T: Config> DealInfo<T> {
    pub fn complete_part(&mut self, miner: AccountOf<T>, index: u8) -> DispatchResult {
        for task_info in &mut self.miner_task_list {
            if task_info.index == index {
                match task_info.miner {
                    Some(_) => Err(Error::<T>::Existed)?,
                    None => {
                        task_info.miner = Some(miner.clone());
                        T::MinerControl::lock_space(&miner, task_info.fragment_list.len() as u128 * FRAGMENT_SIZE)?;
                        self.complete_list.try_push(index).map_err(|_| Error::<T>::BoundedVecError)?;
                    },
                };
            } else {
                if let Some(other_miner) = &task_info.miner {
                    if other_miner == &miner {
                        Err(Error::<T>::Existed)?;
                    }
                };
            }
        }

        Ok(())
    }

    pub fn completed_all(&mut self) -> DispatchResult {
        self.stage = 2;

        for miner_task in self.miner_task_list.iter() {
            // Miners need to report the replaced documents themselves. 
            // If a challenge is triggered before the report is completed temporarily, 
            // these documents to be replaced also need to be verified
            if let Some(miner) = &miner_task.miner {
                <PendingReplacements<T>>::try_mutate(miner, |pending_space| -> DispatchResult {
                    let replace_space = FRAGMENT_SIZE
                        .checked_mul(miner_task.fragment_list.len() as u128).ok_or(Error::<T>::Overflow)?;
                    let pending_space_temp = pending_space.checked_add(replace_space).ok_or(Error::<T>::Overflow)?;
                    *pending_space = pending_space_temp;
                    Ok(())
                })?;
            }
        }

        Ok(())
    }
}