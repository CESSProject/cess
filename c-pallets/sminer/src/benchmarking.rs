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

    let sig: TeeRsaSignature = [227, 166, 170, 215, 215, 31, 118, 201, 102, 86, 232, 166, 55, 135, 18, 12, 93, 9, 227, 82, 180, 58, 46, 33, 28, 75, 235, 78, 60, 173, 142, 55, 42, 193, 18, 184, 61, 188, 149, 251, 147, 136, 182, 80, 138, 174, 84, 140, 97, 63, 120, 208, 185, 3, 2, 161, 185, 157, 147, 107, 32, 11, 72, 127, 195, 125, 127, 81, 188, 170, 142, 116, 146, 84, 236, 210, 224, 67, 46, 82, 101, 157, 210, 215, 211, 191, 143, 204, 171, 173, 102, 208, 140, 64, 32, 94, 111, 79, 179, 0, 109, 36, 237, 111, 242, 155, 20, 126, 177, 201, 2, 27, 216, 97, 246, 39, 6, 6, 91, 98, 87, 133, 181, 45, 207, 57, 112, 206, 148, 200, 141, 178, 79, 160, 173, 158, 32, 225, 90, 170, 230, 109, 164, 121, 235, 16, 180, 171, 48, 219, 128, 63, 110, 249, 17, 17, 102, 60, 164, 160, 119, 187, 199, 232, 83, 84, 0, 2, 228, 34, 114, 113, 158, 104, 147, 218, 133, 251, 80, 115, 46, 72, 196, 93, 72, 170, 228, 234, 25, 68, 72, 145, 255, 46, 201, 14, 63, 37, 78, 162, 144, 120, 231, 143, 241, 157, 193, 233, 181, 27, 141, 192, 82, 134, 99, 140, 104, 189, 102, 170, 163, 49, 31, 223, 74, 255, 138, 161, 112, 161, 180, 237, 27, 55, 136, 97, 190, 45, 249, 162, 97, 130, 45, 223, 215, 222, 109, 181, 154, 61, 188, 66, 120, 19, 65, 200];

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
        let sig: TeeRsaSignature = [227, 166, 170, 215, 215, 31, 118, 201, 102, 86, 232, 166, 55, 135, 18, 12, 93, 9, 227, 82, 180, 58, 46, 33, 28, 75, 235, 78, 60, 173, 142, 55, 42, 193, 18, 184, 61, 188, 149, 251, 147, 136, 182, 80, 138, 174, 84, 140, 97, 63, 120, 208, 185, 3, 2, 161, 185, 157, 147, 107, 32, 11, 72, 127, 195, 125, 127, 81, 188, 170, 142, 116, 146, 84, 236, 210, 224, 67, 46, 82, 101, 157, 210, 215, 211, 191, 143, 204, 171, 173, 102, 208, 140, 64, 32, 94, 111, 79, 179, 0, 109, 36, 237, 111, 242, 155, 20, 126, 177, 201, 2, 27, 216, 97, 246, 39, 6, 6, 91, 98, 87, 133, 181, 45, 207, 57, 112, 206, 148, 200, 141, 178, 79, 160, 173, 158, 32, 225, 90, 170, 230, 109, 164, 121, 235, 16, 180, 171, 48, 219, 128, 63, 110, 249, 17, 17, 102, 60, 164, 160, 119, 187, 199, 232, 83, 84, 0, 2, 228, 34, 114, 113, 158, 104, 147, 218, 133, 251, 80, 115, 46, 72, 196, 93, 72, 170, 228, 234, 25, 68, 72, 145, 255, 46, 201, 14, 63, 37, 78, 162, 144, 120, 231, 143, 241, 157, 193, 233, 181, 27, 141, 192, 82, 134, 99, 140, 104, 189, 102, 170, 163, 49, 31, 223, 74, 255, 138, 161, 112, 161, 180, 237, 27, 55, 136, 97, 190, 45, 249, 162, 97, 130, 45, 223, 215, 222, 109, 181, 154, 61, 188, 66, 120, 19, 65, 200];
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
        log::info!("point 1");
        let caller: T::AccountId = whitelisted_caller();
        let existential_deposit = <T as crate::Config>::Currency::minimum_balance();
        <T as crate::Config>::Currency::make_free_balance_be(
	 		&caller,
	 		BalanceOf::<T>::max_value(),
	 	);
         let fa: BalanceOf<T> =  existential_deposit.checked_mul(&10u32.saturated_into()).ok_or("over flow")?;
         log::info!("point 2");
     }: _(RawOrigin::Signed(caller), fa)
     verify {
        let pallet_acc = <T as crate::Config>::PalletId::get().into_account_truncating();
        let free = <T as crate::Config>::Currency::free_balance(&pallet_acc);
        assert_eq!(free, fa)
     }
}

// }
