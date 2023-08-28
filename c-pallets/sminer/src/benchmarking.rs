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

pub fn bench_miner_exit<T: Config>(miner: AccountOf<T>) -> Result<(), &'static str> {
    Sminer::<T>::miner_exit_prep(RawOrigin::Signed(miner.clone()).into()).map_err(|_| "miner exit prep failed")?;
    Sminer::<T>::miner_exit(RawOrigin::Root.into(), miner.clone()).map_err(|_| "miner exit failed")?;

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

        let encoding = space_proof_info.encode();
        let hashing = sp_io::hashing::sha2_256(&encoding);

        let sig: TeeRsaSignature = [151, 66, 19, 21, 197, 133, 110, 137, 144, 124, 158, 44, 178, 8, 38, 111, 87, 41, 21, 178, 205, 70, 94, 130, 231, 130, 118, 228, 46, 78, 165, 31, 64, 57, 105, 18, 58, 250, 170, 246, 57, 207, 14, 180, 87, 160, 68, 157, 214, 37, 103, 187, 77, 6, 41, 167, 35, 93, 134, 89, 78, 140, 196, 174, 132, 177, 75, 212, 135, 115, 79, 251, 125, 150, 226, 163, 129, 104, 74, 39, 231, 117, 87, 162, 10, 61, 189, 165, 160, 190, 111, 66, 178, 31, 11, 201, 67, 119, 99, 125, 84, 181, 25, 141, 38, 241, 180, 95, 104, 94, 46, 30, 233, 99, 73, 97, 77, 101, 226, 70, 74, 83, 233, 102, 136, 208, 142, 109, 220, 122, 95, 9, 212, 34, 227, 248, 58, 138, 24, 159, 56, 201, 46, 106, 240, 49, 172, 193, 211, 25, 108, 238, 250, 202, 79, 44, 149, 27, 231, 106, 148, 135, 245, 157, 3, 69, 196, 11, 151, 129, 136, 216, 248, 146, 119, 83, 209, 107, 196, 252, 140, 18, 123, 198, 173, 25, 18, 234, 47, 151, 133, 179, 242, 179, 10, 111, 185, 72, 247, 96, 152, 120, 19, 213, 77, 52, 149, 120, 90, 34, 22, 8, 36, 63, 74, 114, 128, 119, 254, 19, 71, 163, 21, 41, 115, 226, 52, 107, 156, 75, 6, 229, 85, 26, 84, 0, 186, 234, 94, 229, 240, 14, 3, 49, 219, 130, 160, 238, 133, 197, 17, 13, 108, 31, 135, 107];
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
        let pallet_acc = <T as crate::Config>::FaucetId::get().into_account_truncating();
        let free = <T as crate::Config>::Currency::free_balance(&pallet_acc);
        assert_eq!(free, fa)
     }

    miner_exit_prep {
        let miner: AccountOf<T> = add_miner::<T>("miner1")?;
    }: _(RawOrigin::Signed(miner.clone()))
    verify {
        let miner_info = <MinerItems<T>>::try_get(&miner).unwrap();
        assert_eq!(miner_info.state.to_vec(), STATE_LOCK.as_bytes().to_vec())
    }

    miner_exit {
        let miner: AccountOf<T> = add_miner::<T>("miner1")?;
        Sminer::<T>::miner_exit_prep(RawOrigin::Signed(miner.clone()).into())?;
        
    }: _(RawOrigin::Root, miner.clone())
    verify {
        let miner_info = <MinerItems<T>>::try_get(&miner).unwrap();
        assert_eq!(miner_info.state.to_vec(), STATE_EXIT.as_bytes().to_vec())
    }

    miner_withdraw {
        let miner: AccountOf<T> = add_miner::<T>("miner1")?;
        Sminer::<T>::miner_exit_prep(RawOrigin::Signed(miner.clone()).into())?;
        Sminer::<T>::miner_exit(RawOrigin::Root.into(), miner.clone())?;
    }: _(RawOrigin::Signed(miner.clone()))
    verify {
        assert!(!<MinerItems<T>>::contains_key(&miner));
    }
}

// }
