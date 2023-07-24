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
    let _ = pallet_tee_worker::benchmarking::tee_register::<T>()?;

    let caller = account(name, 100, SEED);
    let peer_id: PeerId = [5u8; 38];
    <T as crate::Config>::Currency::make_free_balance_be(
        &caller,
        BalanceOf::<T>::max_value(),
    );

    let pois_key = PoISKey {
        g: [2u8; 256],
        n: [3u8; 256],
    };

    let space_proof_info = SpaceProofInfo::<AccountOf<T>, BlockNumberOf<T>> {
        last_operation_block: u32::MIN.saturated_into(),
        miner: caller.clone(),
        front: u64::MIN,
        rear: u64::MIN,
        pois_key: pois_key.clone(),
        accumulator: pois_key.g,
    };

    let sig: TeeRsaSignature = [36, 8, 36, 142, 71, 88, 69, 251, 61, 33, 93, 146, 158, 182, 54, 55, 119, 253, 106, 235, 229, 232, 38, 37, 106, 190, 185, 148, 185, 140, 167, 113, 201, 143, 221, 239, 250, 34, 9, 51, 113, 82, 99, 16, 214, 90, 246, 143, 104, 131, 246, 60, 157, 170, 241, 72, 165, 170, 115, 46, 47, 142, 215, 145, 34, 252, 212, 32, 230, 1, 46, 197, 13, 57, 104, 230, 55, 147, 185, 170, 94, 84, 127, 142, 247, 150, 204, 205, 123, 189, 100, 207, 154, 176, 232, 111, 11, 149, 75, 128, 170, 115, 114, 0, 111, 36, 6, 40, 39, 176, 52, 102, 219, 120, 173, 143, 172, 106, 109, 124, 225, 217, 52, 164, 84, 57, 86, 251, 31, 133, 84, 29, 205, 113, 245, 216, 203, 221, 200, 195, 26, 7, 241, 53, 71, 232, 229, 215, 227, 47, 99, 145, 19, 149, 220, 23, 47, 212, 214, 89, 223, 104, 6, 64, 117, 243, 186, 213, 137, 62, 98, 166, 142, 234, 20, 32, 190, 207, 221, 115, 97, 238, 248, 107, 88, 109, 35, 184, 57, 134, 51, 59, 0, 96, 27, 100, 163, 50, 12, 195, 77, 230, 178, 244, 160, 189, 187, 183, 90, 244, 220, 121, 66, 176, 149, 253, 26, 40, 60, 116, 112, 253, 14, 9, 40, 216, 73, 48, 155, 46, 2, 23, 59, 173, 189, 186, 16, 45, 94, 3, 34, 194, 245, 215, 245, 118, 21, 224, 105, 100, 64, 195, 46, 122, 176, 50];

    Sminer::<T>::regnstk(
        RawOrigin::Signed(caller.clone()).into(), 
        caller.clone(), 
        peer_id, 
        2_000u32.into(), 
        pois_key, 
        sig,
    ).map_err(|_| "create miner failed")?;
    Ok(caller)
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

        let space_proof_info = SpaceProofInfo::<AccountOf<T>, BlockNumberOf<T>> {
            last_operation_block: u32::MIN.saturated_into(),
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
        let miner = add_miner::<T>("user1")?;
    }: _(RawOrigin::Signed(miner.clone()), 2_000u32.into())
    verify {
        let miner_info = <MinerItems<T>>::get(&miner).unwrap();
        assert_eq!(4000u32, miner_info.collaterals.saturated_into::<u32>());
    }

    update_beneficiary {
        let miner = add_miner::<T>("user1")?;
        let caller: AccountOf<T> = account("user2", 100, SEED);
    }: _(RawOrigin::Signed(miner.clone()), caller.clone())
    verify {
        let miner_info = <MinerItems<T>>::get(&miner).unwrap();
        assert_eq!(caller, miner_info.beneficiary);
    }

    update_peer_id {
        let miner = add_miner::<T>("user1")?;
        let peer_id = [9u8; 38];
    }: _(RawOrigin::Signed(miner.clone()), peer_id.clone())
    verify {
        let miner_info = <MinerItems<T>>::get(&miner).unwrap();
        assert_eq!(peer_id, miner_info.peer_id);
    }

    faucet_top_up {
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
