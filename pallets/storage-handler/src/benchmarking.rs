use crate::{Pallet as StorageHandler, *};
use frame_system::RawOrigin;
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use sp_runtime::traits::Bounded;

const SEED: u32 = 2190502;

pub fn increase_idle_space<T: Config>(space: u128) {
    <TotalIdleSpace<T>>::mutate(|total_space| {
        *total_space = *total_space + space;
    });
}

benchmarks! {
    mint_territory {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
        let terr_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
    }: _(RawOrigin::Signed(caller.clone()), 10, terr_name.clone())
    verify {
        assert!(<Territory<T>>::contains_key(&caller, &terr_name))
    }

    expanding_territory {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
        let terr_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
        StorageHandler::<T>::mint_territory(RawOrigin::Signed(caller.clone()).into(), 10, terr_name.clone())?;
    }: _(RawOrigin::Signed(caller.clone()), terr_name.clone(), 10)
    verify {
        assert!(<Territory<T>>::contains_key(&caller, &terr_name));
        let user_owned_space_info = <Territory<T>>::try_get(&caller, &terr_name).unwrap();
        assert_eq!(user_owned_space_info.total_space, 20 * G_BYTE);
        assert!(<TerritoryKey<T>>::contains_key(&user_owned_space_info.token));
    }

    renewal_territory {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let terr_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
        StorageHandler::<T>::mint_territory(RawOrigin::Signed(caller.clone()).into(), 10, terr_name.clone())?;
    }: _(RawOrigin::Signed(caller.clone()), terr_name.clone(), 10)
    verify {
        assert!(<Territory<T>>::contains_key(&caller, &terr_name));
        let user_owned_space_info = <Territory<T>>::try_get(&caller, &terr_name).unwrap();
        assert_eq!(user_owned_space_info.deadline, (40u32 * 14400u32).saturated_into());
    }

    create_order {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let terr_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
    }: _(RawOrigin::Signed(caller.clone()), caller.clone(), terr_name.clone(),  OrderType::Buy, 1, 1, 1) 
    verify {
        let event = frame_system::pallet::Pallet::<T>::events().pop().unwrap();
        log::info!("event: {:?}", event.event);
    }

    exec_order {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let terr_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
        StorageHandler::<T>::create_order(RawOrigin::Signed(caller.clone()).into(), caller.clone(), terr_name.clone(), OrderType::Buy, 1, 1, 1)?;
        let mut order_id: BoundedVec<u8, ConstU32<32>> = Default::default();
        for (key, value) in <PayOrder<T>>::iter() {
            order_id = key
        }
    }: _(RawOrigin::Signed(caller.clone()), order_id)
    verify {
        assert!(<Territory<T>>::contains_key(&caller, &terr_name));
    }
}

