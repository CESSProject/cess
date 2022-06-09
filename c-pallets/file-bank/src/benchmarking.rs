use super::*;
use crate::{Pallet as FileBank, *};
pub use frame_benchmarking::{
	account, benchmarks, impl_benchmark_test_suite, whitelist_account, whitelisted_caller,
};
use sp_runtime::{
	traits::{Bounded, One, StaticLookup, TrailingZeroInput, Zero},
	Perbill, Percent,
};
use pallet_cess_staking::{Pallet as Staking, Config as StakingConfig, testing_utils,
    RewardDestination
};
use pallet_file_map::{Pallet as FileMap, Config as FileMapConfig};
use codec::{Decode,alloc::string::ToString};
use frame_support::{
	dispatch::UnfilteredDispatchable,
	pallet_prelude::*,
	traits::{Currency, CurrencyToVote, Get, Imbalance},
};
use sp_std::prelude::*;

use frame_system::RawOrigin;

pub struct Pallet<T: Config>(FileBank<T>);
pub trait Config:
	crate::Config + pallet_cess_staking::Config + pallet_file_map::Config
{
}
const SEED: u32 = 2190502;
const MAX_SPANS: u32 = 100;

benchmarks! {
    upload {
        let caller = account("user1", 100, SEED);
        let address = "user1".as_bytes().to_vec();
        let filename = "file1".as_bytes().to_vec();
        let fileid = "1".as_bytes().to_vec();
        let filehash = "filehash".as_bytes().to_vec();
        let public = false;
        let backup: u8 = 3;
        let filesize: u64 = 123_456;
        let downloadfee: BalanceOf<T> = BalanceOf::<T>::try_from(1u128)
        .map_err(|_| "balance expected to be a u128")
        .unwrap();
        UserHoldSpaceDetails::<T>::insert(
            &caller, 
            StorageSpace {
                purchased_space: 1_000_000_000,
                remaining_space: 1_000_000_000,
                used_space: 0,
            }
        );
    }: _(RawOrigin::Signed(caller), address, filename, fileid, filehash, public, backup, filesize, downloadfee)
    verify {
        let bounded_fileid: BoundedString<T> = BoundedVec::try_from("1".as_bytes().to_vec())
        .map_err(|_| "vec convert to boundedvec error")?;
        assert!(File::<T>::contains_key(&bounded_fileid));
    }

    upload_filler {
        let v in 0 .. MAX_SPANS;

        let controller = testing_utils::create_funded_user::<T>("controller", SEED, 100);
        let stash = testing_utils::create_funded_user::<T>("stash", SEED, 100);
        let controller_lookup: <T::Lookup as StaticLookup>::Source
			= T::Lookup::unlookup(controller.clone());
		let reward_destination = RewardDestination::Staked;
		let amount = <T as pallet_cess_staking::Config>::Currency::minimum_balance() * 10u32.into();
		whitelist_account!(stash);
        Staking::<T>::bond(RawOrigin::Signed(stash.clone()).into(), controller_lookup, amount, reward_destination)?;
        whitelist_account!(controller);

        FileMap::<T>::registration_scheduler(RawOrigin::Signed(controller.clone()).into(), stash, "127.0.0.1:8080".as_bytes().to_vec())?;
        
        let mut filler_list: Vec<FillerInfo<T>> = Vec::new();
        for i in 0 .. v {
            let miner = account("miner1", 100, SEED);
            let new_filler = FillerInfo::<T> {
                miner_id: 1,
                filler_size: 1024 * 1024 * 8,
                block_num: 8,
                segment_size: 1024 * 1024,
                miner_address: miner,
                filler_block: Default::default(),
                filler_id: i.to_string().as_bytes().to_vec().try_into().map_err(|_| "uint convert to BoundedVec Error")?,
                filler_hash: "fillerhashhashhashhash".as_bytes().to_vec().try_into().map_err(|_| "Vec convert to BoundedVec Error")?,
            };
            filler_list.push(new_filler);
        }
    }: _(RawOrigin::Signed(controller), 1, filler_list)
    verify {
        assert_eq!(1, 1);
    }
}