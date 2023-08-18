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
        log::info!("register hashing: {:?}", hashing);

        let sig: TeeRsaSignature = [27, 227, 6, 81, 243, 202, 129, 247, 142, 47, 147, 80, 173, 149, 139, 98, 89, 161, 88, 135, 198, 87, 212, 162, 210, 161, 33, 2, 239, 67, 206, 191, 180, 207, 235, 88, 16, 164, 232, 152, 182, 57, 3, 196, 1, 139, 194, 146, 241, 51, 102, 165, 50, 11, 240, 234, 75, 211, 130, 105, 42, 204, 17, 108, 254, 137, 58, 70, 2, 46, 93, 109, 216, 145, 120, 151, 29, 214, 182, 132, 96, 144, 63, 127, 124, 182, 255, 231, 211, 40, 220, 202, 53, 207, 214, 183, 214, 226, 242, 170, 27, 169, 218, 24, 171, 199, 192, 240, 15, 155, 46, 64, 205, 101, 212, 62, 212, 92, 65, 170, 174, 76, 50, 119, 125, 239, 134, 23, 11, 126, 41, 7, 29, 39, 216, 122, 103, 167, 97, 29, 196, 196, 181, 122, 202, 252, 98, 169, 151, 29, 188, 21, 55, 15, 238, 211, 250, 195, 51, 205, 82, 205, 110, 159, 126, 215, 107, 159, 9, 201, 110, 224, 156, 125, 250, 187, 144, 23, 22, 129, 217, 98, 91, 122, 8, 45, 251, 19, 152, 161, 94, 140, 17, 250, 63, 34, 85, 113, 205, 179, 55, 79, 39, 147, 200, 187, 125, 25, 153, 187, 184, 194, 216, 217, 130, 160, 99, 164, 118, 53, 146, 215, 239, 53, 81, 128, 35, 161, 73, 241, 250, 101, 212, 89, 224, 15, 220, 35, 191, 61, 114, 172, 238, 244, 184, 116, 185, 40, 249, 8, 99, 24, 98, 48, 126, 133];
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
