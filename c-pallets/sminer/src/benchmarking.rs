use super::*;
use crate::{Pallet as Sminer};
use codec::{alloc::string::ToString, Decode};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use frame_support::{
	traits::{Currency, Get},
};
use pallet_tee_worker::benchmarking::{Config as TeeWorkerConfig};
use sp_runtime::{
	traits::{Bounded, CheckedMul},
};
use frame_system::RawOrigin;

const SEED: u32 = 2190502;
pub struct Pallet<T: Config>(Sminer<T>);
pub trait Config:
	crate::Config + pallet_tee_worker::benchmarking::Config + pallet_cess_staking::Config
{
}
// const MAX_SPANS: u32 = 100;

pub fn add_miner<T: Config>(name: &'static str) -> Result<T::AccountId, &'static str> {
    // let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;

    let caller = account(name, 100, SEED);
    let peer_id: PeerId = [5u8; 38];
    let staking_val: BalanceOf<T> = 2000u32.saturated_into();
    <T as crate::Config>::Currency::make_free_balance_be(
        &caller,
        BalanceOf::<T>::max_value(),
    );

    if <MinerItems<T>>::contains_key(&caller) {
        Err("create miner failed, existed")?;
    }

	<T as crate::Config>::Currency::reserve(&caller, staking_val)?;

    let pois_key = PoISKey {
        g: [2u8; 256],
        n: [3u8; 256],
    };

    let space_proof_info = SpaceProofInfo::<AccountOf<T>> {
        miner: caller.clone(),
        front: u64::MIN,
        rear: u64::MIN,
        pois_key: pois_key.clone(),
        accumulator: pois_key.g,
    };

    let encoding = space_proof_info.encode();
	let original_text = sp_io::hashing::sha2_256(&encoding);
    MinerPublicKey::<T>::insert(&original_text, caller.clone());

    <MinerItems<T>>::insert(
        &caller,
        MinerInfo::<T> {
            beneficiary: caller.clone(),
            peer_id: peer_id,
            collaterals: staking_val,
            debt: BalanceOf::<T>::zero(),
            state: STATE_POSITIVE.as_bytes().to_vec().try_into().unwrap(),
            idle_space: u128::MIN,
            service_space: u128::MIN,
            lock_space: u128::MIN,
            space_proof_info,			
            service_bloom_filter: Default::default(),
            tee_signature: [8u8; 256],
        },
    );

    AllMiner::<T>::try_mutate(|all_miner| -> DispatchResult {
        all_miner
            .try_push(caller.clone())
            .map_err(|_e| Error::<T>::StorageLimitReached)?;
        Ok(())
    })?;

    RewardMap::<T>::insert(
        &caller,
        Reward::<T>{
            total_reward: 0u32.saturated_into(),
            reward_issued: 0u32.saturated_into(),
            currently_available_reward: 0u32.saturated_into(),
            order_list: Default::default()
        },
    );

    Ok(caller)
}

pub fn freeze_miner<T: Config>(acc: AccountOf<T>) -> Result<(), &'static str> {
    <MinerItems<T>>::try_mutate(&acc, |miner_opt| -> Result<(), &'static str> {
        let miner = miner_opt.as_mut().expect("Not Miner");

        miner.state = STATE_FROZEN.as_bytes().to_vec().try_into().unwrap();

        Ok(())
    })?;

    Ok(())
}

benchmarks! {
    regnstk {
        let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
        let caller = account("user1", 100, SEED);
        let peer_id: PeerId = [5u8; 38];
        <T as crate::Config>::Currency::make_free_balance_be(
            &caller,
            BalanceOf::<T>::max_value(),
        );
        let pois_key = PoISKey {
            g: [2u8; 256],
            n: [3u8; 256],
        };

        let space_proof_info = SpaceProofInfo::<AccountOf<T>> {
            miner: caller.clone(),
            front: u64::MIN,
            rear: u64::MIN,
            pois_key: pois_key.clone(),
            accumulator: pois_key.g,
        };

        let sig: TeeRsaSignature = [174, 153, 189, 101, 87, 124, 131, 98, 21, 63, 176, 60, 107, 132, 223, 121, 123, 246, 47, 248, 53, 146, 91, 55, 65, 241, 172, 104, 85, 226, 237, 223, 22, 135, 254, 63, 229, 61, 124, 6, 150, 100, 222, 231, 115, 27, 172, 148, 36, 208, 213, 124, 253, 71, 207, 68, 34, 77, 68, 101, 131, 182, 222, 185, 130, 227, 54, 34, 81, 56, 231, 95, 19, 70, 140, 150, 36, 155, 135, 208, 245, 141, 155, 129, 157, 41, 20, 234, 241, 105, 2, 238, 1, 154, 145, 149, 132, 201, 146, 47, 132, 154, 96, 167, 225, 19, 117, 224, 158, 195, 206, 34, 81, 75, 247, 181, 71, 6, 26, 27, 30, 229, 198, 133, 248, 154, 141, 105, 13, 55, 205, 183, 78, 124, 121, 77, 121, 188, 41, 255, 86, 254, 180, 41, 24, 89, 41, 91, 233, 135, 60, 242, 174, 109, 46, 242, 186, 116, 88, 187, 250, 173, 194, 7, 52, 116, 233, 43, 121, 132, 12, 146, 164, 93, 46, 32, 103, 148, 242, 228, 42, 167, 197, 242, 83, 26, 43, 110, 40, 86, 1, 153, 151, 19, 192, 18, 249, 208, 17, 220, 15, 158, 253, 123, 153, 13, 56, 226, 95, 234, 189, 126, 126, 101, 151, 12, 149, 15, 175, 172, 6, 228, 253, 68, 226, 30, 20, 33, 66, 235, 103, 190, 91, 71, 217, 104, 68, 68, 9, 42, 95, 1, 253, 66, 6, 41, 105, 35, 102, 94, 254, 48, 238, 15, 136, 86];
    }: _(RawOrigin::Signed(caller.clone()), caller.clone(), peer_id, 2_000u32.into(), pois_key, sig)
    verify {
        assert!(<MinerItems<T>>::contains_key(&caller))
    }

    increase_collateral {
        let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
        let miner = add_miner::<T>("user1")?;
    }: _(RawOrigin::Signed(miner.clone()), 2_000u32.into())
    verify {
        let miner_info = <MinerItems<T>>::get(&miner).unwrap();
        assert_eq!(4000u32, miner_info.collaterals.saturated_into::<u32>());
    }

    update_beneficiary {
        let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
        let miner = add_miner::<T>("user1")?;
        let caller: AccountOf<T> = account("user2", 100, SEED);
    }: _(RawOrigin::Signed(miner.clone()), caller.clone())
    verify {
        let miner_info = <MinerItems<T>>::get(&miner).unwrap();
        assert_eq!(caller, miner_info.beneficiary);
    }

    update_peer_id {
        let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
        let miner = add_miner::<T>("user1")?;
        let peer_id = [9u8; 38];
    }: _(RawOrigin::Signed(miner.clone()), peer_id.clone())
    verify {
        let miner_info = <MinerItems<T>>::get(&miner).unwrap();
        assert_eq!(peer_id, miner_info.peer_id);
    }

    faucet_top_up {
        let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;
        let caller: T::AccountId = whitelisted_caller();
        let existential_deposit = <T as crate::Config>::Currency::minimum_balance();
        <T as crate::Config>::Currency::make_free_balance_be(
	 		&caller,
	 		BalanceOf::<T>::max_value(),
	 	);
         let fa: BalanceOf<T> =  existential_deposit.checked_mul(&10u32.saturated_into()).ok_or("over flow")?;
     }: _(RawOrigin::Signed(caller), fa)
     verify {
        let pallet_acc = <T as crate::Config>::PalletId::get().into_account_truncating();
        let free = <T as crate::Config>::Currency::free_balance(&pallet_acc);
        assert_eq!(free, fa)
     }
}

// }
