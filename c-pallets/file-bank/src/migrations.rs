use crate::{BalanceOf, CodeHash, Config, Pallet, TrieId, Weight};
use codec::{Decode, Encode};
use frame_support::{
	codec, generate_storage_alias,
	pallet_prelude::*,
	storage::migration,
	traits::{Get, PalletInfoAccess},
	Identity, Twox64Concat,
};
use sp_std::{marker::PhantomData, prelude::*};

pub fn migrate<T: Config>() -> Weight {
	use frame_support::traits::StorageVersion;

	let version = StorageVersion::get::<Pallet<T>>();
	let mut weight: Weight = 0;

	if version < 2 {
        weight = weight.saturated_add(v2::migrate::<T>());
        StorageVersion::new(2).put::<Pallet<T>>();
    }

	weight
}

mod v2 {
    struct OldFillerInfo<T: Config> {
        filler_size: u64,
        index: u32,
        block_num: u32,
        segment_size: u32,
        scan_size: u32,
        miner_address: AccountOf<T>,
        filler_id: BoundedVec<u8, T::StringLimit>,
        filler_hash: BoundedVec<u8, T::StringLimit>,
    }

    struct NewFillerInfo<T: Config> {
        filler_size: u64,
        index: u32,
        block_num: u32,
        segment_size: u32,
        miner_address: AccountOf<T>,
        filler_id: BoundedVec<u8, T::StringLimit>,
        filler_hash: BoundedVec<u8, T::StringLimit>,
        is_delete: bool,
    }

    generate_storage_alias!(
		FileBank,
		FillerMap<T: Config> => DoubleMap< 
            (Twox64Concat, T::AccountId), 
            (Twox64Concat, BoundedVec<u8, T::StringLimit>),
            NewFillerInfo<T>
        >
	);

    pub fn migrate<T: Config>() -> Weight {
        let mut weight: Weight = 0;

        FillerMap::translate(|_key1, _key2, old: OldFillerInfo| {
            Some(NewFillerInfo::<T>{
                filler_size: old.filler_size,
                index: old.index,
                block_num: old.block_num,
                segment_size: old.segment_size,
                miner_address: old.miner_address,
                filler_id: old.filler_id,
                filler_hash: old.filler_hash,
                is_delete: false,
            })
        })
    }
}