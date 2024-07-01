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
        let territory_info = <Territory<T>>::try_get(&caller, &terr_name).unwrap();
        assert_eq!(territory_info.total_space, 20 * G_BYTE);
        assert!(<TerritoryKey<T>>::contains_key(&territory_info.token));
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
        let territory_info = <Territory<T>>::try_get(&caller, &terr_name).unwrap();
        assert_eq!(territory_info.deadline, (40u32 * 14400u32).saturated_into());
    }

    treeitory_consignment {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let terr_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
        StorageHandler::<T>::mint_territory(RawOrigin::Signed(caller.clone()).into(), 10, terr_name.clone())?;
        let price: BalanceOf<T> = 100_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
    }: _(RawOrigin::Signed(caller.clone()), terr_name.clone(), price)
    verify {
        assert!(<Territory<T>>::contains_key(&caller, &terr_name));
        let territory_info = <Territory<T>>::try_get(&caller, &terr_name).unwrap();
        assert!(<Consignment<T>>::contains_key(&territory_info.token));
        assert_eq!(territory_info.state, TerritoryState::OnConsignment);
    }

    buy_consignment {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let terr_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
        StorageHandler::<T>::mint_territory(RawOrigin::Signed(caller.clone()).into(), 10, terr_name.clone())?;
        let price: BalanceOf<T> = 100_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        StorageHandler::<T>::treeitory_consignment(RawOrigin::Signed(caller.clone()).into(), terr_name.clone(), price)?;
        let buyer: AccountOf<T> = account("user2", 100, SEED);
        T::Currency::make_free_balance_be(&buyer, free);
        let territory_info = <Territory<T>>::try_get(&caller, &terr_name).unwrap();
    }: _(RawOrigin::Signed(buyer.clone()), territory_info.token.clone(), terr_name.clone())
    verify {
        assert!(<Consignment<T>>::contains_key(&territory_info.token));
        let consignment_info = <Consignment<T>>::try_get(&territory_info.token).unwrap();
        assert!(consignment_info.locked);
    }

    exec_consignment {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let terr_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
        StorageHandler::<T>::mint_territory(RawOrigin::Signed(caller.clone()).into(), 10, terr_name.clone())?;
        let price: BalanceOf<T> = 100_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        StorageHandler::<T>::treeitory_consignment(RawOrigin::Signed(caller.clone()).into(), terr_name.clone(), price)?;
        let buyer: AccountOf<T> = account("user2", 100, SEED);
        T::Currency::make_free_balance_be(&buyer, free);
        let territory_info = <Territory<T>>::try_get(&caller, &terr_name).unwrap();
        StorageHandler::<T>::buy_consignment(RawOrigin::Signed(buyer.clone()).into(), territory_info.token.clone(), terr_name.clone())?;
    }: _(RawOrigin::Root, territory_info.token.clone(), terr_name.clone())
    verify {
        assert!(<Territory<T>>::contains_key(&buyer, &terr_name))
    }

    cancel_consignment {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let terr_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
        StorageHandler::<T>::mint_territory(RawOrigin::Signed(caller.clone()).into(), 10, terr_name.clone())?;
        let price: BalanceOf<T> = 100_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        StorageHandler::<T>::treeitory_consignment(RawOrigin::Signed(caller.clone()).into(), terr_name.clone(), price)?;
        let territory_info = <Territory<T>>::try_get(&caller, &terr_name).unwrap();
    }: _(RawOrigin::Signed(caller.clone()), terr_name.clone())
    verify {
        assert!(!<Consignment<T>>::contains_key(&territory_info.token))
    }

    cancel_purchase_action {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let terr_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
        StorageHandler::<T>::mint_territory(RawOrigin::Signed(caller.clone()).into(), 10, terr_name.clone())?;
        let price: BalanceOf<T> = 100_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        StorageHandler::<T>::treeitory_consignment(RawOrigin::Signed(caller.clone()).into(), terr_name.clone(), price)?;
        let buyer: AccountOf<T> = account("user2", 100, SEED);
        T::Currency::make_free_balance_be(&buyer, free);
        let territory_info = <Territory<T>>::try_get(&caller, &terr_name).unwrap();
        StorageHandler::<T>::buy_consignment(RawOrigin::Signed(buyer.clone()).into(), territory_info.token.clone(), terr_name.clone())?;
        let consignment_info = <Consignment<T>>::try_get(&territory_info.token).unwrap();
        assert!(consignment_info.locked);
    }: _(RawOrigin::Signed(buyer.clone()), territory_info.token.clone())
    verify {
        let consignment_info = <Consignment<T>>::try_get(&territory_info.token).unwrap();
        assert!(!consignment_info.locked);
    }

    territory_grants {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let terr_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
        StorageHandler::<T>::mint_territory(RawOrigin::Signed(caller.clone()).into(), 10, terr_name.clone())?;
        let receiver: AccountOf<T> = account("user2", 100, SEED);
        assert!(<Territory<T>>::contains_key(&caller, &terr_name));
        let territory_info = <Territory<T>>::try_get(&caller, &terr_name).unwrap();
    }: _(RawOrigin::Signed(caller.clone()), terr_name.clone(), receiver.clone())
    verify {
        let tname: TerrName = territory_info.token.0.to_vec().try_into().unwrap();
        assert!(!<Territory<T>>::contains_key(&caller, &terr_name));
        assert!(<Territory<T>>::contains_key(&receiver, &tname));
    }

    territory_rename {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let terr_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
        StorageHandler::<T>::mint_territory(RawOrigin::Signed(caller.clone()).into(), 10, terr_name.clone())?;
        let new_name: TerrName = "t2".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
        assert!(<Territory<T>>::contains_key(&caller, &terr_name));
    }: _(RawOrigin::Signed(caller.clone()), terr_name.clone(), new_name.clone())
    verify {
        assert!(!<Territory<T>>::contains_key(&caller, &terr_name));
        assert!(<Territory<T>>::contains_key(&caller, &new_name));
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

    reactivate_territory {
        let caller: AccountOf<T> = account("user1", 100, SEED);
        let terr_name: TerrName = "t1".as_bytes().to_vec().try_into().map_err(|_| "boundedvec error")?;
        let free: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        T::Currency::make_free_balance_be(&caller, free);
        increase_idle_space::<T>(100 * G_BYTE);
        StorageHandler::<T>::mint_territory(RawOrigin::Signed(caller.clone()).into(), 10, terr_name.clone())?;
        <Territory<T>>::try_mutate(&caller, &terr_name, |territory_info_opt| -> DispatchResult {
            let territory_info = territory_info_opt.as_mut().unwrap();
            territory_info.state = TerritoryState::Expired;
            <TerritoryFrozen<T>>::remove(territory_info.deadline, territory_info.token);
            Ok(())
        }).map_err(|_| "reactivate_territory: try mutate erro")?;
    }: _(RawOrigin::Signed(caller.clone()), terr_name.clone(), 10)
    verify {
        let territory_info = <Territory<T>>::try_get(&caller, &terr_name).unwrap();
        assert_eq!(territory_info.state, TerritoryState::Active);
    }

    
}

