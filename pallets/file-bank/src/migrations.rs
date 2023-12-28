use crate::{AccountOf, Config, Pallet, Weight};
use codec::{Decode, Encode};
use frame_support::{
	storage_alias,
	pallet_prelude::*,
	traits::{Get},
};
use sp_std::vec::Vec;
use frame_support::traits::OnRuntimeUpgrade;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;
/// A struct that does not migration, but only checks that the counter prefix exists and is correct.
pub struct TestMigrationFileBank<T: crate::Config>(sp_std::marker::PhantomData<T>);
impl<T: crate::Config> OnRuntimeUpgrade for TestMigrationFileBank<T> {
	fn on_runtime_upgrade() -> Weight {
		migrate::<T>()
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		log::info!("🙋🏽file-bank check access");
		return Ok(Default::default())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let weights = migrate::<T>();
		return Ok(())
	}
}

pub fn migrate<T: Config>() -> Weight {

	let version = StorageVersion::get::<Pallet<T>>();
	let mut weight: Weight = Weight::zero();

	if version < 3 {
		weight = weight.saturating_add(v3::migrate::<T>());
        StorageVersion::new(3).put::<Pallet<T>>();
	}

	weight
}

// mod v2 {
// 	use super::*;
// 	use cp_cess_common::Hash;
// 	use crate::{FillerInfo, FillerMap as NewFillerMap};

// 	#[derive(Decode, Encode)]
// 	struct OldFillerInfo<T: crate::Config> {
// 		filler_size: u64,
// 		index: u32,
// 		block_num: u32,
// 		segment_size: u32,
// 		scan_size: u32,
// 		miner_address: AccountOf<T>,
// 		filler_id: BoundedVec<u8, T::StringLimit>,
// 		filler_hash: BoundedVec<u8, T::StringLimit>,
// 	}

// 	#[derive(Decode, Encode)]
// 	struct NewFillerInfo<T: Config> {
// 		filler_size: u64,
// 		index: u32,
// 		block_num: u32,
// 		segment_size: u32,
// 		scan_size: u32,
// 		miner_address: AccountOf<T>,
// 		filler_hash: Hash,
// 	}

// 	generate_storage_alias!(
// 		FileBank,
// 		FillerMap<T: Config> => DoubleMap<
//             (Blake2_128Concat, AccountOf<T>),
//             (Blake2_128Concat, BoundedVec<u8, T::StringLimit>),
//             OldFillerInfo<T>
//         >
// 	);

// 	pub fn migrate<T: Config>() -> Weight {
// 		let mut weight: Weight = 0;
// 		log::info!("-----------------------------test migrations start-----------------------------------");
// 		for (miner_acc, filler_id, old) in <FillerMap<T>>::iter() {
// 			log::info!("-----------------------------migrations value filler_id:{:?}, len: {}", filler_id.clone(), filler_id.as_slice().len());
// 			log::info!("old value filler_size: {}, index: {}, block_num: {}", old.filler_size, old.index, old.block_num);
// 			weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
// 			let filler_hash = Hash::slice_to_array_64(&filler_id).expect("error!");
// 			// {
// 			// 	Ok(slice) => slice,
// 			// 	Err(e) => {
// 			// 		log::info!("convert err: {:?}", e);
// 			// 		continue;
// 			// 	},
// 			// };
// 			log::info!("convert success!");
// 			let filler_hash = Hash(filler_hash);
// 			let new_value = FillerInfo::<T>{
// 				filler_size: old.filler_size,
// 				index: old.index,
// 				block_num: old.block_num,
// 				segment_size: old.segment_size,
// 				scan_size: old.scan_size,
// 				miner_address: old.miner_address.clone(),
// 				filler_hash: filler_hash.clone(),
// 			};
// 			log::info!("start insert");
// 			<NewFillerMap<T>>::insert(miner_acc, filler_hash, new_value);
// 			log::info!("end insert");
// 		}
// 		log::info!("migrations end!");
// 		weight
// 	}
// }

mod v3 {
	use super::*;
	use cp_cess_common::Hash;
	use crate::{*, File as NewFileMap};

	#[derive(Decode, Encode)]
	pub struct OldFileInfo<T: Config> {
		pub(super) segment_list: BoundedVec<OldSegmentInfo<T>, T::SegmentCount>,
		pub(super) owner: BoundedVec<UserBrief<T>, T::OwnerLimit>,
		pub(super) file_size: u128,
		pub(super) completion: BlockNumberFor<T>,
		pub(super) stat: FileState,
	}

	#[derive(Decode, Encode)]
	pub struct OldSegmentInfo<T: Config> {
		pub(super) hash: Hash,
		pub(super) fragment_list: BoundedVec<OldFragmentInfo<T>, T::FragmentCount>,
	}

	#[derive(Decode, Encode)]
	pub struct OldFragmentInfo<T: Config> {
		pub(super) hash: Hash,
		pub(super) avail: bool,
		pub(super) miner: AccountOf<T>,
	}
	
	#[storage_alias]
	type File<T: Config> = StorageMap<FileBank, Blake2_128Concat, Hash, OldFileInfo<T>>;

	pub fn migrate<T: Config>() -> Weight {
		let mut weight: Weight = Weight::zero();
		log::info!("-----------------------------test migrations start-----------------------------------");
		for (file_hash, old_info) in <File<T>>::iter() {
			let mut segment_list: BoundedVec<SegmentInfo<T>, T::SegmentCount> = Default::default();
			weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
			for segment in old_info.segment_list {
				let mut fragment_list: BoundedVec<FragmentInfo<T>, T::FragmentCount> = Default::default();
				for fragment in segment.fragment_list {
					if old_info.stat == FileState::Calculate {
						let new_fragment_info = FragmentInfo {
							hash: fragment.hash,
							avail: true,
							tag: None,
							miner: fragment.miner,
						};

						fragment_list.try_push(new_fragment_info).expect("error");
					} else {
						let now = <frame_system::Pallet<T>>::block_number();
						let new_fragment_info = FragmentInfo {
							hash: fragment.hash,
							avail: true,
							tag: Some(now),
							miner: fragment.miner,
						};

						fragment_list.try_push(new_fragment_info).expect("error");
					}
				}
				let segment_info = SegmentInfo::<T> {
					hash: segment.hash,
					fragment_list: fragment_list,
				};
				segment_list.try_push(segment_info).expect("error");
			}

			let new_file_info = FileInfo {
				segment_list: segment_list,
				owner: old_info.owner,
				file_size: old_info.file_size,
				completion: old_info.completion,
				stat: FileState::Active,
			};

			<NewFileMap<T>>::insert(file_hash, new_file_info);
			log::info!("migrate file_hash: {:?} success", file_hash);
		}

		weight
	}
}
