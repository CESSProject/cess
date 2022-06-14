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
use pallet_sminer::{Pallet as Sminer, Config as SminerConfig};
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
	crate::Config + pallet_cess_staking::Config + pallet_file_map::Config + pallet_sminer::Config
{
}
type SminerBalanceOf<T> = <<T as pallet_sminer::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

const SEED: u32 = 2190502;
const MAX_SPANS: u32 = 100;

pub fn add_file<T: Config>() -> Result<BoundedString<T>, &'static str> {
    let caller = account("user1", 100, SEED);
    let address = "user1".as_bytes().to_vec();
    let filename = "file1".as_bytes().to_vec();
    let fileid = "testfileid1".as_bytes().to_vec();
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
    FileBank::<T>::upload(RawOrigin::Signed(caller).into(), address, filename, fileid.clone(), filehash, public, backup, filesize, downloadfee)?;
    Ok(fileid.try_into().map_err(|_| "fileid convert failed")?)
}

pub fn add_filler<T: Config>(len: u32) -> Result<u32, &'static str> {
    let controller = testing_utils::create_funded_user::<T>("controller2", SEED, 100);
    let stash = testing_utils::create_funded_user::<T>("stash2", SEED, 100);
    let controller_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(controller.clone());
    let reward_destination = RewardDestination::Staked;
	let amount = <T as pallet_cess_staking::Config>::Currency::minimum_balance() * 10u32.into();
	whitelist_account!(stash);
    Staking::<T>::bond(RawOrigin::Signed(stash.clone()).into(), controller_lookup, amount, reward_destination)?;
    whitelist_account!(controller);

     FileMap::<T>::registration_scheduler(RawOrigin::Signed(controller.clone()).into(), stash, "127.0.0.1:8080".as_bytes().to_vec())?;
        
    let mut filler_list: Vec<FillerInfo<T>> = Vec::new();
    for i in 0 .. len {
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


    Ok(1)
}

pub fn add_miner<T: Config>() -> Result<T::AccountId, &'static str> {
    let miner: T::AccountId = account("miner1", 100, SEED);
    let ip = "1270008080".as_bytes().to_vec();
    let public_key = "thisisatestpublickey".as_bytes().to_vec();
    <T as pallet_sminer::Config>::Currency::make_free_balance_be(&miner, SminerBalanceOf::<T>::max_value());
    whitelist_account!(miner);
    Sminer::<T>::regnstk(RawOrigin::Signed(miner.clone()).into(), miner.clone(), ip, 0u32.into(), public_key)?;
    Ok(miner.clone())
}

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
        for i in 0 .. v {
            let filler_id: BoundedString<T> = i.to_string().as_bytes().to_vec().try_into().map_err(|_| "uint convert to BoundedVec Error")?;
            assert!(FillerMap::<T>::contains_key(&1, &filler_id));
        }
    }

    update_dupl {
        log::info!("test point start");
        let caller: T::AccountId = account("user1", 100, SEED);
        pallet_sminer::Pallet::<T>::init_benchmark();
        log::info!("point 1");
        let file_id = add_file::<T>()?;
        log::info!("point 2");
        let _ = add_miner::<T>()?;
        log::info!("point 3");
        let miner_id = add_filler::<T>(5)?;
        log::info!("point 4");
        let mut dupl_list: Vec<FileDuplicateInfo<T>> = Vec::new();
        let dupl_info = FileDuplicateInfo::<T>{
            miner_id: miner_id.clone() as u64,
            block_num: 0,
            segment_size: 123,
            acc: caller.clone(),
            miner_ip: Default::default(),
            dupl_id: "dupl1".as_bytes().to_vec().try_into().map_err(|_| "duplid convert err")?,
            rand_key: Default::default(),
            block_info: Default::default(),
        };
        dupl_list.push(dupl_info);
        log::info!("point 5");
    }: _(RawOrigin::Signed(caller), file_id.to_vec(), dupl_list)
    verify {
        log::info!("point 6");
        let file = FileBank::<T>::file(&file_id).unwrap();
        let dupl_id: BoundedString<T> = "dupl1".as_bytes().to_vec().try_into().map_err(|_| "duplid convert err")?;
        assert_eq!(file.file_dupl[0].dupl_id, dupl_id);
    }

    // delete_file {
    //     let caller: T::AccountId = account("user1", 100, SEED);
    //     let file_id = add_file::<T>()?;
    // }: _(RawOrigin::Signed(caller), file_id)
    // verify {
    //     assert!(File::<T>::contains_key(&file_id.try_into().map_err(|_| "fileid convert err")?));
    // }
}