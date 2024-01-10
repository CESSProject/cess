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

    pub fn completed_all(&self) -> DispatchResult {
        for complete_info in self.complete_list.iter() {
            let replace_space = FRAGMENT_SIZE  
                    .checked_mul(self.segment_list.len() as u128)
                    .ok_or(Error::<T>::Overflow)?;
            T::MinerControl::increase_replace_space(&complete_info.miner, replace_space)?;
        }

        Ok(())
    }
}