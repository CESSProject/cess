#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::{Pallet as Sminer};
// use codec::{alloc::string::ToString, Decode};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_system::RawOrigin;
// use frame_support::{
// 	traits::{Currency, Get},
// };
// use pallet_tee_worker::benchmarking::{Config as TeeWorkerConfig};
// use sp_runtime::{
// 	traits::{Bounded, CheckedMul},
// };
const BASE_UNIT: u128 = 1_000_000_000_000_000_000;
const SEED: u32 = 2190502;
pub struct Pallet<T: Config>(Sminer<T>);
pub trait Config:
	crate::Config + pallet_tee_worker::Config + pallet_cess_staking::Config + pallet_cess_treasury::Config
{
}

pub fn register_miner<T: Config>(account: AccountOf<T>) -> Result<(), &'static str> {
    <T as crate::Config>::Currency::make_free_balance_be(
        &account,
        160_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!"),
    );
    let peer_id: PeerId = [5u8; 38];
    let staking_val: BalanceOf<T> = 4_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
    let tib_count = 1;
    Sminer::<T>::regnstk(RawOrigin::Signed(account.clone()).into(), account.clone(), peer_id, staking_val, tib_count).map_err(|_| "miner register prep failed")?;
    Ok(())
}

pub fn register_positive_miner<T: Config>(account: AccountOf<T>) -> Result<(), &'static str> {
    register_miner::<T>(account.clone())?;
    let pois_key = PoISKey {
        g: [5u8; 256],
        n: [6u8; 256],
    };
    let space_proof_info = SpaceProofInfo::<AccountOf<T>> {
        miner: account.clone(),
        front: u64::MIN,
        rear: u64::MIN,
        pois_key: pois_key.clone(),
        accumulator: pois_key.g,
    };
    let encoding = space_proof_info.encode();
    let tee_puk = pallet_tee_worker::benchmarking::get_pubkey::<T>();
    let tee_puk_encode = tee_puk.encode();
    let mut original = Vec::new();
    original.extend_from_slice(&encoding);
    original.extend_from_slice(&tee_puk_encode);
    let original_text = sp_io::hashing::sha2_256(&original);
    let sig = pallet_tee_worker::benchmarking::sign_message::<T>(&original_text);
    let sig: BoundedVec<u8, ConstU32<64>> = sig.0.to_vec().try_into().map_err(|_| "bounded convert error")?;

    Sminer::<T>::register_pois_key(RawOrigin::Signed(account.clone()).into(), pois_key, sig.clone(), sig, tee_puk).map_err(|_| "register positive miner failed")?;
    Ok(())
}

benchmarks! {
    regnstk {
        log::info!("regnstk start");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
        let caller = account("user1", 100, SEED);
        let peer_id: PeerId = [5u8; 38];
        <T as crate::Config>::Currency::make_free_balance_be(
            &caller,
            160_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!"),
        );

        let staking_val: BalanceOf<T> = 4_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        let tib_count = 1;
    }: _(RawOrigin::Signed(caller.clone()), caller.clone(), peer_id, staking_val, tib_count)
    verify {
        assert!(<MinerItems<T>>::contains_key(&caller))
    }

    increase_collateral {
        log::info!("increase_collateral start");
        let caller: AccountOf<T> = account("user1", 100, SEED);
        register_miner::<T>(caller.clone())?;
        let staking_val: BalanceOf<T> = 4_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
    }: _(RawOrigin::Signed(caller.clone()), caller.clone(), staking_val)
    verify {
        assert!(<MinerItems<T>>::contains_key(&caller));
        let miner_info = <MinerItems<T>>::get(caller).unwrap();
        assert_eq!(miner_info.collaterals, 8_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!"))
    }

    register_pois_key {
        log::info!("register pois key start");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
        let caller: AccountOf<T> = account("user1", 100, SEED);
        register_miner::<T>(caller.clone())?;
        let pois_key = PoISKey {
            g: [5u8; 256],
            n: [6u8; 256],
        };
        let space_proof_info = SpaceProofInfo::<AccountOf<T>> {
            miner: caller.clone(),
            front: u64::MIN,
            rear: u64::MIN,
            pois_key: pois_key.clone(),
            accumulator: pois_key.g,
        };
        let encoding = space_proof_info.encode();
        let tee_puk = pallet_tee_worker::benchmarking::get_pubkey::<T>();
		let tee_puk_encode = tee_puk.encode();
		let mut original = Vec::new();
		original.extend_from_slice(&encoding);
		original.extend_from_slice(&tee_puk_encode);
		let original_text = sp_io::hashing::sha2_256(&original);
        let sig = pallet_tee_worker::benchmarking::sign_message::<T>(&original_text);
        let sig: BoundedVec<u8, ConstU32<64>> = sig.0.to_vec().try_into().map_err(|_| "bounded convert error")?;
    }: _(RawOrigin::Signed(caller.clone()), pois_key, sig.clone(), sig, tee_puk)
    verify {
        assert!(<MinerItems<T>>::contains_key(&caller));
        let miner_info = <MinerItems<T>>::get(caller).unwrap();
        assert_eq!(miner_info.state, Sminer::<T>::str_to_bound(STATE_POSITIVE)?)
    }

    update_beneficiary {
        log::info!("update beneficiary start");
        let caller: AccountOf<T> = account("user1", 100, SEED);
        register_miner::<T>(caller.clone())?;
        let beneficiary: AccountOf<T> = account("user2", 100, SEED);
    }: _(RawOrigin::Signed(caller.clone()), beneficiary.clone())
    verify {
        assert!(<MinerItems<T>>::contains_key(&caller));
        let miner_info = <MinerItems<T>>::get(caller).unwrap();
        assert_eq!(miner_info.beneficiary, beneficiary)
    }

    update_peer_id {
        log::info!("update peer id start");
        let caller: AccountOf<T> = account("user1", 100, SEED);
        register_miner::<T>(caller.clone())?;
        let new_peer_id: PeerId = [5u8; 38];
    }: _(RawOrigin::Signed(caller.clone()), new_peer_id)
    verify {
        assert!(<MinerItems<T>>::contains_key(&caller));
        let miner_info = <MinerItems<T>>::get(caller).unwrap();
        assert_eq!(miner_info.peer_id, new_peer_id)
    }

    receive_reward {
        let v in 1 .. 80;
        log::info!("receive_reward start");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
        pallet_cess_treasury::benchmarking::initialize_reward::<T>();
        let caller: AccountOf<T> = account("user1", 100, SEED);
        register_positive_miner::<T>(caller.clone())?;
        let mut total_reward: BalanceOf<T> = BalanceOf::<T>::zero();
        let mut order_list: BoundedVec<RewardOrder<BalanceOf<T>, BlockNumberFor<T>>, ConstU32<90>> = Default::default();
        let order_reward: BalanceOf<T> = 1_800_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        let each_amount: BalanceOf<T> = 10_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        for i in 0 .. v {
            let reward_order = RewardOrder::<BalanceOf<T>, BlockNumberFor<T>> {
                receive_count: 0,
                max_count: 90,
                atonce: false,
                order_reward: order_reward,
                each_amount: each_amount,
                last_receive_block: 1u32.saturated_into(),
            };
            total_reward = total_reward + order_reward;
            order_list.try_push(reward_order).unwrap();
        }

        let reward = Reward::<T> {
            total_reward: total_reward,
            reward_issued: BalanceOf::<T>::zero(),
            order_list: order_list,
        };

        <RewardMap<T>>::insert(&caller, reward);
        frame_system::Pallet::<T>::set_block_number(28805u32.into());
    }: _(RawOrigin::Signed(caller.clone()))
    verify {
        let reward_info = <RewardMap<T>>::get(&caller).unwrap();
        let vreward: BalanceOf<T> = (((900 * BASE_UNIT) + (20 * BASE_UNIT)) * v as u128).try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        assert_eq!(reward_info.reward_issued, vreward);
    }

    miner_exit_prep {
        log::info!("miner exit prep start");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
        pallet_cess_treasury::benchmarking::initialize_reward::<T>();
        let caller: AccountOf<T> = account("user1", 100, SEED);
        register_positive_miner::<T>(caller.clone())?;
        frame_system::Pallet::<T>::set_block_number(28805000u32.into());
    }: _(RawOrigin::Signed(caller.clone()), caller.clone()) 
    verify {
        assert!(<MinerItems<T>>::contains_key(&caller));
        let miner_info = <MinerItems<T>>::get(caller).unwrap();
        assert_eq!(miner_info.state, Sminer::<T>::str_to_bound(STATE_LOCK)?)
    }

    miner_exit {
        log::info!("miner exit start");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
        pallet_cess_treasury::benchmarking::initialize_reward::<T>();
        let caller: AccountOf<T> = account("user1", 100, SEED);
        register_positive_miner::<T>(caller.clone())?;
        frame_system::Pallet::<T>::set_block_number(28805000u32.into());
        Sminer::<T>::miner_exit_prep(RawOrigin::Signed(caller.clone()).into(), caller.clone())?;
    }: _(RawOrigin::Root, caller.clone())
    verify {
        assert!(<RestoralTarget<T>>::contains_key(&caller));
    }

    faucet_top_up {
        let caller: T::AccountId = whitelisted_caller();
        let existential_deposit = <T as crate::Config>::Currency::minimum_balance();
        let faucet_reward: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        <T as crate::Config>::Currency::make_free_balance_be(
	 		&caller,
	 		faucet_reward,
	 	);
         let fa: BalanceOf<T> =  existential_deposit.checked_mul(&10u32.saturated_into()).ok_or("over flow")?;
     }: _(RawOrigin::Signed(caller), fa)
     verify {
        let pallet_acc = <T as crate::Config>::FaucetId::get().into_account_truncating();
        let free = <T as crate::Config>::Currency::free_balance(&pallet_acc);
        assert_eq!(free, fa)
     }

    faucet {
        let caller: T::AccountId = whitelisted_caller();
        let faucet_acc: AccountOf<T> = T::FaucetId::get().into_account_truncating();
        let existential_deposit = <T as crate::Config>::Currency::minimum_balance();
        let faucet_reward: BalanceOf<T> = 365_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        <T as crate::Config>::Currency::make_free_balance_be(
            &caller,
            existential_deposit,
        );
        <T as crate::Config>::Currency::make_free_balance_be(
            &faucet_acc,
            faucet_reward,
        );
    }: _(RawOrigin::Signed(caller.clone()), caller.clone())
    verify {
        assert!(<FaucetRecordMap<T>>::contains_key(&caller))
    }

    regnstk_assign_staking {
        log::info!("regnstk_assign_staking start");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
        let caller = account("user1", 100, SEED);
        let staking_account = account("user2", 100, SEED);
        let peer_id: PeerId = [5u8; 38];
        <T as crate::Config>::Currency::make_free_balance_be(
            &caller,
            160_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!"),
        );

        let staking_val: BalanceOf<T> = 4_000_000_000_000_000_000_000u128.try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        let tib_count = 1;
    }: _(RawOrigin::Signed(caller.clone()), caller.clone(), peer_id, staking_account, tib_count)
    verify {
        assert!(<MinerItems<T>>::contains_key(&caller))
    }

    increase_declaration_space {
        log::info!("increase_declaration_space start");
        pallet_tee_worker::benchmarking::generate_workers::<T>();
        pallet_cess_treasury::benchmarking::initialize_reward::<T>();
        let caller: AccountOf<T> = account("user1", 100, SEED);
        register_positive_miner::<T>(caller.clone())?;
        let tib_count = 5;
    }: _(RawOrigin::Signed(caller.clone()), tib_count)
    verify {
        assert!(<MinerItems<T>>::contains_key(&caller));
        let miner_info = <MinerItems<T>>::get(caller).unwrap();
        assert_eq!(miner_info.state, Sminer::<T>::str_to_bound(STATE_FROZEN)?)
    }
}