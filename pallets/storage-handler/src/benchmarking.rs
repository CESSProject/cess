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
    buy_space {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
    }: _(RawOrigin::Signed(caller.clone()), 10)
    verify {
        assert!(<UserOwnedSpace<T>>::contains_key(&caller))
    }

    expansion_space {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
        StorageHandler::<T>::buy_space(RawOrigin::Signed(caller.clone()).into(), 10)?;
    }: _(RawOrigin::Signed(caller.clone()), 10)
    verify {
        assert!(<UserOwnedSpace<T>>::contains_key(&caller));
        let user_owned_space_info = <UserOwnedSpace<T>>::try_get(&caller).unwrap();
        assert_eq!(user_owned_space_info.total_space, 20 * G_BYTE)
    }

    renewal_space {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
        StorageHandler::<T>::buy_space(RawOrigin::Signed(caller.clone()).into(), 10)?;
    }: _(RawOrigin::Signed(caller.clone()), 10)
    verify {
        assert!(<UserOwnedSpace<T>>::contains_key(&caller));
        let user_owned_space_info = <UserOwnedSpace<T>>::try_get(&caller).unwrap();
        assert_eq!(user_owned_space_info.deadline, (40u32 * 14400u32).saturated_into())
    }

    create_order {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
    }: _(RawOrigin::Signed(caller.clone()), caller.clone(), OrderType::Buy, 1, 1, 1) 
    verify {
        let event = frame_system::pallet::Pallet::<T>::events().pop().unwrap();
        log::info!("event: {:?}", event.event);
    }

    exec_order {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
        StorageHandler::<T>::create_order(RawOrigin::Signed(caller.clone()).into(), caller.clone(), OrderType::Buy, 1, 1, 1)?;
        let mut order_id: BoundedVec<u8, ConstU32<32>> = Default::default();
        for (key, value) in <PayOrder<T>>::iter() {
            order_id = key
        }
    }: _(RawOrigin::Signed(caller.clone()), order_id)
    verify {
        assert!(<UserOwnedSpace<T>>::contains_key(&caller));
    }
}

