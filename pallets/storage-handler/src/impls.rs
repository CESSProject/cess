use super::*;

pub trait StorageHandle<AccountId> {
    fn check_territry_owner(acc: &AccountId, name: &TerrName) -> DispatchResult;
    fn add_territory_used_space(acc: &AccountId, name: &TerrName, size: u128) -> DispatchResult;
    fn sub_territory_used_space(acc: &AccountId, name: &TerrName, size: u128) -> DispatchResult;
    fn add_total_idle_space(increment: u128) -> DispatchResult;
	fn sub_total_idle_space(decrement: u128) -> DispatchResult;
	fn add_total_service_space(increment: u128) -> DispatchResult;
	fn sub_total_service_space(decrement: u128) -> DispatchResult;
    fn get_total_idle_space() -> u128;
    fn get_total_service_space() -> u128;
    fn get_avail_space() -> Result<u128, DispatchError>;
    fn lock_user_space(acc: &AccountId, name: &TerrName, needed_space: u128) -> DispatchResult;
    fn unlock_user_space(acc: &AccountId, name: &TerrName, needed_space: u128) -> DispatchResult;
    fn unlock_and_used_user_space(acc: &AccountId, name: &TerrName, needed_space: u128) -> DispatchResult;
    fn get_user_avail_space(acc: &AccountId, name: &TerrName) -> Result<u128, DispatchError>;
    fn frozen_task() -> (Weight, Vec<(AccountId, TerrName)>);
}

impl<T: Config> StorageHandle<T::AccountId> for Pallet<T> {
    fn check_territry_owner(acc: &T::AccountId, name: &TerrName) -> DispatchResult {
        ensure!(<Territory<T>>::contains_key(acc, name), Error::<T>::NotHaveTerritory);

        Ok(())
    }
    // fn update_user_space(acc: &T::AccountId, opeartion: u8, size: u128) -> DispatchResult {
    //     Pallet::<T>::update_user_space(acc, opeartion, size)
    // }
    fn add_territory_used_space(acc: &T::AccountId, name: &TerrName, size: u128) -> DispatchResult {
        <Territory<T>>::try_mutate(acc, name, |t_opt| -> DispatchResult {
            let t = t_opt.as_mut().ok_or(Error::<T>::NotHaveTerritory)?;
            ensure!(t.state == TerritoryState::Active, Error::<T>::NotActive);
            ensure!(size <= t.remaining_space, Error::<T>::InsufficientStorage);
            t.used_space =
                t.used_space.checked_add(size).ok_or(Error::<T>::Overflow)?;
            t.remaining_space =
                t.remaining_space.checked_sub(size).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }

    fn sub_territory_used_space(acc: &T::AccountId, name: &TerrName, size: u128) -> DispatchResult {
        <Territory<T>>::try_mutate(acc, name, |t_opt| -> DispatchResult {
            let t = t_opt.as_mut().ok_or(Error::<T>::NotHaveTerritory)?;
            t.used_space = t.used_space.checked_sub(size).ok_or(Error::<T>::Overflow)?;
            t.remaining_space =
                t.remaining_space.checked_add(size).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }

    fn add_total_idle_space(increment: u128) -> DispatchResult {
        TotalIdleSpace::<T>::try_mutate(|total_power| -> DispatchResult {
            *total_power = total_power.checked_add(increment).ok_or(Error::<T>::Overflow)?;
            Ok(())
        }) //read 1 write 1
    }

	fn sub_total_idle_space(decrement: u128) -> DispatchResult {
        TotalIdleSpace::<T>::try_mutate(|total_power| -> DispatchResult {
            *total_power = total_power.checked_sub(decrement).ok_or(Error::<T>::Overflow)?;
            Ok(())
        }) //read 1 write 1
    }

	fn add_total_service_space(increment: u128) -> DispatchResult {
        TotalServiceSpace::<T>::try_mutate(|total_space| -> DispatchResult {
            *total_space = total_space.checked_add(increment).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }

	fn sub_total_service_space(decrement: u128) -> DispatchResult {
        TotalServiceSpace::<T>::try_mutate(|total_space| -> DispatchResult {
            *total_space = total_space.checked_sub(decrement).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }

    fn get_avail_space() -> Result<u128, DispatchError> {
        let purchased_space = <PurchasedSpace<T>>::get();
        let total_space = <TotalIdleSpace<T>>::get().checked_add(<TotalServiceSpace<T>>::get()).ok_or(Error::<T>::Overflow)?;
        //If the total space on the current chain is less than the purchased space, 0 will be
        // returned.
        if total_space < purchased_space {
            return Ok(0);
        }
        //Calculate available space.
        let value = total_space.checked_sub(purchased_space).ok_or(Error::<T>::Overflow)?;

        Ok(value)
    }

    // fn get_total_space() -> Result<u128, DispatchError> {
	// 	Pallet::<T>::get_total_space()
	// }

    fn lock_user_space(acc: &T::AccountId, name: &TerrName, needed_space: u128) -> DispatchResult {
        <Territory<T>>::try_mutate(acc, name, |storage_space_opt| -> DispatchResult {
            let storage_space = storage_space_opt.as_mut().ok_or(Error::<T>::NotHaveTerritory)?;
            ensure!(storage_space.state == TerritoryState::Active, Error::<T>::NotActive);
            ensure!(storage_space.remaining_space >= needed_space, Error::<T>::InsufficientStorage);
            storage_space.locked_space = storage_space.locked_space.checked_add(needed_space).ok_or(Error::<T>::Overflow)?;
            storage_space.remaining_space = storage_space.remaining_space.checked_sub(needed_space).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }

    fn unlock_user_space(acc: &T::AccountId, name: &TerrName, needed_space: u128) -> DispatchResult {
        <Territory<T>>::try_mutate(acc, name, |storage_space_opt| -> DispatchResult {
            let storage_space = storage_space_opt.as_mut().ok_or(Error::<T>::NotHaveTerritory)?;
            storage_space.locked_space = storage_space.locked_space.checked_sub(needed_space).ok_or(Error::<T>::Overflow)?;
            storage_space.remaining_space = storage_space.remaining_space.checked_add(needed_space).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }

    fn unlock_and_used_user_space(acc: &T::AccountId, name: &TerrName, needed_space: u128) -> DispatchResult {
        <Territory<T>>::try_mutate(acc, name, |storage_space_opt| -> DispatchResult {
            let storage_space = storage_space_opt.as_mut().ok_or(Error::<T>::NotHaveTerritory)?;
            storage_space.locked_space = storage_space.locked_space.checked_sub(needed_space).ok_or(Error::<T>::Overflow)?;
            storage_space.used_space = storage_space.used_space.checked_add(needed_space).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }

    fn get_user_avail_space(acc: &T::AccountId, name: &TerrName) -> Result<u128, DispatchError> {
        let info = <Territory<T>>::try_get(acc, name).map_err(|_e| Error::<T>::NotHaveTerritory)?;
        Ok(info.remaining_space)
    }

    fn frozen_task() -> (Weight, Vec<(AccountOf<T>, TerrName)>) {
        Self::frozen_task()
    }

    fn get_total_idle_space() -> u128 {
        <TotalIdleSpace<T>>::get()
    }

    fn get_total_service_space() -> u128 {
        <TotalServiceSpace<T>>::get()
    }
}