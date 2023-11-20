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

mod example {
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

mod v2 {
	use super::*;
	use cp_cess_common::Hash;
	use crate::{FillerInfo, FillerMap as NewFillerMap};

	#[derive(Decode, Encode)]
	struct OldFillerInfo<T: crate::Config> {
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
		scan_size: u32,
		miner_address: AccountOf<T>,
		filler_hash: Hash,
	}

	generate_storage_alias!(
		FileBank,
		FillerMap<T: Config> => DoubleMap<
            (Blake2_128Concat, AccountOf<T>),
            (Blake2_128Concat, BoundedVec<u8, T::StringLimit>),
            OldFillerInfo<T>
        >
	);

	// generate_storage_alias!(
	// 	FileBank,
	// 	FillerMap<T: Config> => DoubleMap<
  //           (Blake2_128Concat, T::AccountId),
  //           (Blake2_128Concat, Hash),
  //           NewFillerInfo<T>
  //       >
	// );

	// #[derive(Decode, Encode)]
	// struct OldSliceInfo<T: Config> {
	// 	miner_id: u64,
	// 	shard_size: u64,
	// 	block_num: u32,
	// 	shard_id: BoundedVec<u8, T::StringLimit>,
	// 	miner_ip: BoundedVec<u8, T::StringLimit>,
	// 	miner_acc: AccountOf<T>,
	// }
	//
	// #[derive(Decode, Encode)]
	// struct NewSliceInfo<T: Config> {
	// 	miner_id: u64,
	// 	shard_size: u64,
	// 	block_num: u32,
	// 	shard_id: [u8; 72],
	// 	miner_ip: BoundedVec<u8, T::StringLimit>,
	// 	miner_acc: AccountOf<T>,
	// }
	//
	// generate_storage_alias!(
	// 	FileBank,
	// 	File
	// );
	//
	// struct OldPackageDetails<T: Config> {
	// 	pub(super) space: u128,
	// 	pub(super) used_space: u128,
	// 	pub(super) remaining_space: u128,
	// 	pub(super) tenancy: u32,
	// 	pub(super) package_type: u8,
	// 	pub(super) start: BlockNumberOf<T>,
	// 	pub(super) deadline: BlockNumberOf<T>,
	// 	pub(super) state: BoundedVec<u8, T::StringLimit>,
	// }
	//
	// struct NewPackageDetails<T: Config> {
	// 	pub(super) space: u128,
	// 	pub(super) used_space: u128,
	// 	pub(super) remaining_space: u128,
	// 	pub(super) tenancy: u32,
	// 	pub(super) package_type: PackageType,
	// 	pub(super) start: BlockNumberOf<T>,
	// 	pub(super) deadline: BlockNumberOf<T>,
	// 	pub(super) state: BoundedVec<u8, T::StringLimit>,
	// }

	pub fn migrate<T: Config>() -> Weight {
		let mut weight: Weight = 0;
		log::info!("-----------------------------test migrations start-----------------------------------");
		for (miner_acc, filler_id, old) in <FillerMap<T>>::iter() {
			log::info!("-----------------------------migrations value filler_id:{:?}, len: {}", filler_id.clone(), filler_id.as_slice().len());
			log::info!("old value filler_size: {}, index: {}, block_num: {}", old.filler_size, old.index, old.block_num);
			weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
			let filler_hash = Hash::slice_to_array_64(&filler_id).expect("error!");
			// {
			// 	Ok(slice) => slice,
			// 	Err(e) => {
			// 		log::info!("convert err: {:?}", e);
			// 		continue;
			// 	},
			// };
			log::info!("convert success!");
			let filler_hash = Hash(filler_hash);
			let new_value = FillerInfo::<T>{
				filler_size: old.filler_size,
				index: old.index,
				block_num: old.block_num,
				segment_size: old.segment_size,
				scan_size: old.scan_size,
				miner_address: old.miner_address.clone(),
				filler_hash: filler_hash.clone(),
			};
			log::info!("start insert");
			<NewFillerMap<T>>::insert(miner_acc, filler_hash, new_value);
			log::info!("end insert");
		}
		log::info!("migrations end!");
		weight
	}
}
