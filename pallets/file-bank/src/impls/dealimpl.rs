use crate::*;

impl<T: Config> DealInfo<T> {
    pub fn complete_part(&mut self, miner: AccountOf<T>, index: u8) -> DispatchResult {
        for complete_info in &self.complete_list {
            ensure!(index != complete_info.index, Error::<T>::Existed);
            ensure!(miner != complete_info.miner, Error::<T>::Existed);
        }
        
        T::MinerControl::lock_space(&miner, FRAGMENT_SIZE * self.segment_list.len() as u128)?;

        let complete_info = CompleteInfo::<T> {
            index,
            miner,
        };

        self.complete_list.try_push(complete_info).map_err(|_| Error::<T>::BoundedVecError)?;

        Ok(())
    }

    pub fn completed_all(&mut self) -> DispatchResult {
        for complete_info in self.complete_list.iter() {
            <PendingReplacements<T>>::try_mutate(&complete_info.miner, |pending_space| -> DispatchResult {
                let replace_space = FRAGMENT_SIZE  
                    .checked_mul(self.segment_list.len() as u128)
                    .ok_or(Error::<T>::Overflow)?;

                *pending_space = pending_space
                    .checked_add(replace_space).ok_or(Error::<T>::Overflow)?;

                Ok(())
            })?;
        }

        Ok(())
    }
}