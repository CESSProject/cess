use crate::{AccountOf, Config, Pallet, Weight, BoundedString};
use codec::{Decode, Encode};
use frame_support::{
	codec, generate_storage_alias,
	pallet_prelude::*,
	traits::{Get},
};
use frame_support::traits::OnRuntimeUpgrade;

/// A struct that does not migration, but only checks that the counter prefix exists and is correct.
pub struct TestMigrationFileBank<T: crate::Config>(sp_std::marker::PhantomData<T>);
impl<T: crate::Config> OnRuntimeUpgrade for TestMigrationFileBank<T> {
	fn on_runtime_upgrade() -> Weight {
		migrate::<T>()
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		log::info!("🙋🏽‍file-bank check access");
		return Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		let weights = migrate::<T>();
		return Ok(())
	}
}

pub fn migrate<T: Config>() -> Weight {
	use frame_support::traits::StorageVersion;

	let version = StorageVersion::get::<Pallet<T>>();
	let mut weight: Weight = 0;

	if version < 2 {
        weight = weight.saturating_add(v2::migrate::<T>());
        StorageVersion::new(2).put::<Pallet<T>>();
	}

	weight
}

mod v2 {
    use super::*;

    #[derive(Decode, Encode)]
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

    #[derive(Decode, Encode)]
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
            (Blake2_128Concat, T::AccountId),
            (Blake2_128Concat, BoundedString<T>),
            NewFillerInfo<T>
        >
	);

    pub fn migrate<T: Config>() -> Weight {
        let mut weight: Weight = 0;

        <FillerMap<T>>::translate(|_key1, _key2, old: OldFillerInfo<T>| {
            weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
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
        });

        weight
    }
}
