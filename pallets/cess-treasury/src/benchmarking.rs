#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::{Pallet as CessTreasury};

pub fn initialize_reward<T: Config>() {
    let era_reward: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
    let currency_reward: BalanceOf<T> = 8_000_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
    let reserve_reward: BalanceOf<T> = 8_000_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
    let reward_acc = T::MinerRewardId::get().into_account_truncating();
    let reserve_acc = T::ReserveRewardId::get().into_account_truncating();

    <T as crate::Config>::Currency::make_free_balance_be(
        &reward_acc,
        currency_reward,
    );
    <T as crate::Config>::Currency::make_free_balance_be(
        &reserve_acc,
        reserve_reward,
    );

    EraReward::<T>::put(era_reward);
    CurrencyReward::<T>::put(currency_reward);
    ReserveReward::<T>::put(reserve_reward);
}
