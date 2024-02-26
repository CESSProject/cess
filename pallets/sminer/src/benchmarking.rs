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

// // const MAX_SPANS: u32 = 100;



// pub fn increase_reward<T: Config>(b_reward: u128) -> Result<(), &'static str> {
//     let acc = T::FaucetId::get().into_account_truncating();

//     <T as crate::Config>::Currency::make_free_balance_be(
//         &acc,
//         BalanceOf::<T>::max_value(),
//     );

//     let b_reward: BalanceOf<T> = b_reward.try_into().map_err(|_| "convert error")?;

//     <CurrencyReward<T>>::mutate(|reward| {
//         *reward = *reward + b_reward;
//     });

//     Ok(())
// }

// pub fn add_miner<T: Config>(name: &'static str) -> Result<T::AccountId, &'static str> {
//     // let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;

//     let caller = account(name, 100, SEED);
//     let peer_id: PeerId = [5u8; 38];
//     let staking_val: BalanceOf<T> = 2000u32.saturated_into();
//     <T as crate::Config>::Currency::make_free_balance_be(
//         &caller,
//         BalanceOf::<T>::max_value(),
//     );

//     if <MinerItems<T>>::contains_key(&caller) {
//         Err("create miner failed, existed")?;
//     }

// 	<T as crate::Config>::Currency::reserve(&caller, staking_val)?;

//     let pois_key = PoISKey {
//         g: [2u8; 256],
//         n: [3u8; 256],
//     };

//     let space_proof_info = SpaceProofInfo::<AccountOf<T>> {
//         miner: caller.clone(),
//         front: u64::MIN,
//         rear: u64::MIN,
//         pois_key: pois_key.clone(),
//         accumulator: pois_key.g,
//     };

//     let encoding = space_proof_info.encode();
// 	let original_text = sp_io::hashing::sha2_256(&encoding);
//     MinerPublicKey::<T>::insert(&original_text, caller.clone());

//     <MinerItems<T>>::insert(
//         &caller,
//         MinerInfo::<T> {
//             beneficiary: caller.clone(),
//             peer_id: peer_id,
//             collaterals: staking_val,
//             debt: BalanceOf::<T>::zero(),
//             state: STATE_POSITIVE.as_bytes().to_vec().try_into().unwrap(),
//             idle_space: u128::MIN,
//             service_space: u128::MIN,
//             lock_space: u128::MIN,
//             space_proof_info,			
//             service_bloom_filter: Default::default(),
//             tee_signature: [8u8; 256],
//         },
//     );

//     AllMiner::<T>::try_mutate(|all_miner| -> DispatchResult {
//         all_miner
//             .try_push(caller.clone())
//             .map_err(|_e| Error::<T>::StorageLimitReached)?;
//         Ok(())
//     })?;

//     RewardMap::<T>::insert(
//         &caller,
//         Reward::<T>{
//             total_reward: 0u32.saturated_into(),
//             reward_issued: 0u32.saturated_into(),
//             currently_available_reward: 0u32.saturated_into(),
//             order_list: Default::default()
//         },
//     );

//     Ok(caller)
// }

// pub fn freeze_miner<T: Config>(acc: AccountOf<T>) -> Result<(), &'static str> {
//     <MinerItems<T>>::try_mutate(&acc, |miner_opt| -> Result<(), &'static str> {
//         let miner = miner_opt.as_mut().expect("Not Miner");

//         miner.state = STATE_FROZEN.as_bytes().to_vec().try_into().unwrap();

//         Ok(())
//     })?;

//     Ok(())
// }

// pub fn bench_miner_exit<T: Config>(miner: AccountOf<T>) -> Result<(), &'static str> {
//     Sminer::<T>::miner_exit_prep(RawOrigin::Signed(miner.clone()).into()).map_err(|_| "miner exit prep failed")?;
//     Sminer::<T>::miner_exit(RawOrigin::Root.into(), miner.clone()).map_err(|_| "miner exit failed")?;

//     Ok(())
// }

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
        pallet_cess_treasury::benchmarking::initialize_reward::<T>();
        let caller: AccountOf<T> = account("user1", 100, SEED);
        register_miner::<T>(caller.clone())?;
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
                last_receive_block: 0u32.saturated_into(),
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
        frame_system::Pallet::<T>::set_block_number(28800u32.into());
    }: _(RawOrigin::Signed(caller.clone()))
    verify {
        let reward_info = <RewardMap<T>>::get(&caller).unwrap();
        let vreward: BalanceOf<T> = (((900 * BASE_UNIT) + (20 * BASE_UNIT)) * v as u128).try_into().map_err(|_| "tryinto error!").expect("tryinto error!");
        assert_eq!(reward_info.reward_issued, vreward);
    }


}

    
// }

// benchmarks! {
//     regnstk {
//         let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
//         let caller = account("user1", 100, SEED);
//         let peer_id: PeerId = [5u8; 38];
//         <T as crate::Config>::Currency::make_free_balance_be(
//             &caller,
//             BalanceOf::<T>::max_value(),
//         );
//         let pois_key = PoISKey {
//             g: [2u8; 256],
//             n: [3u8; 256],
//         };

//         let space_proof_info = SpaceProofInfo::<AccountOf<T>> {
//             miner: caller.clone(),
//             front: u64::MIN,
//             rear: u64::MIN,
//             pois_key: pois_key.clone(),
//             accumulator: pois_key.g,
//         };

//         let encoding = space_proof_info.encode();
//         let hashing = sp_io::hashing::sha2_256(&encoding);

//         let sig: TeeRsaSignature = [151, 66, 19, 21, 197, 133, 110, 137, 144, 124, 158, 44, 178, 8, 38, 111, 87, 41, 21, 178, 205, 70, 94, 130, 231, 130, 118, 228, 46, 78, 165, 31, 64, 57, 105, 18, 58, 250, 170, 246, 57, 207, 14, 180, 87, 160, 68, 157, 214, 37, 103, 187, 77, 6, 41, 167, 35, 93, 134, 89, 78, 140, 196, 174, 132, 177, 75, 212, 135, 115, 79, 251, 125, 150, 226, 163, 129, 104, 74, 39, 231, 117, 87, 162, 10, 61, 189, 165, 160, 190, 111, 66, 178, 31, 11, 201, 67, 119, 99, 125, 84, 181, 25, 141, 38, 241, 180, 95, 104, 94, 46, 30, 233, 99, 73, 97, 77, 101, 226, 70, 74, 83, 233, 102, 136, 208, 142, 109, 220, 122, 95, 9, 212, 34, 227, 248, 58, 138, 24, 159, 56, 201, 46, 106, 240, 49, 172, 193, 211, 25, 108, 238, 250, 202, 79, 44, 149, 27, 231, 106, 148, 135, 245, 157, 3, 69, 196, 11, 151, 129, 136, 216, 248, 146, 119, 83, 209, 107, 196, 252, 140, 18, 123, 198, 173, 25, 18, 234, 47, 151, 133, 179, 242, 179, 10, 111, 185, 72, 247, 96, 152, 120, 19, 213, 77, 52, 149, 120, 90, 34, 22, 8, 36, 63, 74, 114, 128, 119, 254, 19, 71, 163, 21, 41, 115, 226, 52, 107, 156, 75, 6, 229, 85, 26, 84, 0, 186, 234, 94, 229, 240, 14, 3, 49, 219, 130, 160, 238, 133, 197, 17, 13, 108, 31, 135, 107];
//     }: _(RawOrigin::Signed(caller.clone()), caller.clone(), peer_id, 2_000u32.into(), pois_key, sig)
//     verify {
//         assert!(<MinerItems<T>>::contains_key(&caller))
//     }

//     increase_collateral {
//         let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
//         let miner = add_miner::<T>("user1")?;
//     }: _(RawOrigin::Signed(miner.clone()), 2_000u32.into())
//     verify {
//         let miner_info = <MinerItems<T>>::get(&miner).unwrap();
//         assert_eq!(4000u32, miner_info.collaterals.saturated_into::<u32>());
//     }

//     update_beneficiary {
//         let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
//         let miner = add_miner::<T>("user1")?;
//         let caller: AccountOf<T> = account("user2", 100, SEED);
//     }: _(RawOrigin::Signed(miner.clone()), caller.clone())
//     verify {
//         let miner_info = <MinerItems<T>>::get(&miner).unwrap();
//         assert_eq!(caller, miner_info.beneficiary);
//     }

//     update_peer_id {
//         let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
//         let miner = add_miner::<T>("user1")?;
//         let peer_id = [9u8; 38];
//     }: _(RawOrigin::Signed(miner.clone()), peer_id.clone())
//     verify {
//         let miner_info = <MinerItems<T>>::get(&miner).unwrap();
//         assert_eq!(peer_id, miner_info.peer_id);
//     }

//     faucet_top_up {
//         let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
//         let caller: T::AccountId = whitelisted_caller();
//         let existential_deposit = <T as crate::Config>::Currency::minimum_balance();
//         <T as crate::Config>::Currency::make_free_balance_be(
// 	 		&caller,
// 	 		BalanceOf::<T>::max_value(),
// 	 	);
//          let fa: BalanceOf<T> =  existential_deposit.checked_mul(&10u32.saturated_into()).ok_or("over flow")?;
//      }: _(RawOrigin::Signed(caller), fa)
//      verify {
//         let pallet_acc = <T as crate::Config>::FaucetId::get().into_account_truncating();
//         let free = <T as crate::Config>::Currency::free_balance(&pallet_acc);
//         assert_eq!(free, fa)
//      }

//     miner_exit_prep {
//         let miner: AccountOf<T> = add_miner::<T>("miner1")?;
//     }: _(RawOrigin::Signed(miner.clone()))
//     verify {
//         let miner_info = <MinerItems<T>>::try_get(&miner).unwrap();
//         assert_eq!(miner_info.state.to_vec(), STATE_LOCK.as_bytes().to_vec())
//     }

//     miner_exit {
//         let miner: AccountOf<T> = add_miner::<T>("miner1")?;
//         Sminer::<T>::miner_exit_prep(RawOrigin::Signed(miner.clone()).into())?;
        
//     }: _(RawOrigin::Root, miner.clone())
//     verify {
//         let miner_info = <MinerItems<T>>::try_get(&miner).unwrap();
//         assert_eq!(miner_info.state.to_vec(), STATE_EXIT.as_bytes().to_vec())
//     }

//     miner_withdraw {
//         let miner: AccountOf<T> = add_miner::<T>("miner1")?;
//         Sminer::<T>::miner_exit_prep(RawOrigin::Signed(miner.clone()).into())?;
//         Sminer::<T>::miner_exit(RawOrigin::Root.into(), miner.clone())?;
//     }: _(RawOrigin::Signed(miner.clone()))
//     verify {
//         assert!(!<MinerItems<T>>::contains_key(&miner));
//     }
// }

// // }
