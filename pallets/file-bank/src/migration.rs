use super::*;
use sp_std::collections::btree_map::BTreeMap;
use sp_runtime::{TryRuntimeError};
use frame_support::{
	storage_alias, weights::WeightMeter,
	migrations::{MigrationId, SteppedMigration, SteppedMigrationError},
};

#[cfg(feature = "try-runtime")]
use sp_runtime::Vec;

pub const PALLET_MIGRATIONS_ID: &[u8; 26] = b"pallet-file-bank-migration";

pub struct SteppedFileBank<T: Config, W: weights::WeightInfo>(PhantomData<(T, W)>);
impl<T: Config, W: weights::WeightInfo> SteppedMigration for SteppedFileBank<T, W> {
	type Cursor = (Option<Hash>, Option<Hash>);

	type Identifier = MigrationId<26>;

	fn id() -> Self::Identifier {
		MigrationId { pallet_id: *PALLET_MIGRATIONS_ID, version_from: 2, version_to: 3 }
	}

	fn step(
		mut cursor: Option<Self::Cursor>, 
		meter: &mut WeightMeter
	) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
		let required = W::migration_step();

		if meter.remaining().any_lt(required) {
			return Err(SteppedMigrationError::InsufficientWeight { required });
		}

		loop {
			if meter.try_consume(required).is_err() {
				break;
			}

			let (file_key_opt, deal_key_opt) = if let Some((file_key_opt, deal_key_opt)) = cursor {
				(
					v3::file_step_migration::<T>(true, file_key_opt),
					v3::dealmap_step_migration::<T>(true, deal_key_opt),
				)
			} else {
				(
					v3::file_step_migration::<T>(false, None),
					v3::dealmap_step_migration::<T>(false, None),
				)
			};

			if file_key_opt.is_none() && deal_key_opt.is_none() {
				cursor = None;
				break
			} else {
				cursor = Some((file_key_opt, deal_key_opt));
			}
		}

		Ok(cursor)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		use codec::Encode;
		
		let file_records: BTreeMap<_, _> = v3::File::<T>::iter()
			.take(10)
			.collect();
		let deal_records: BTreeMap<_, _> = v3::DealMap::<T>::iter()
			.take(10)
			.collect();
		
		let res = (file_records.encode(), deal_records.encode());
		Ok(res.encode())
	}
	#[cfg(feature = "try-runtime")]
	fn post_upgrade(prev_state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let res = <(Vec<u8>, Vec<u8>)>::decode(&mut &prev_state[..])
			.expect("Failed to decode the previous storage state");
		
		let (file_state, deal_state) = (
			<BTreeMap<Hash, v3::OldFileInfo<T>>>::decode(&mut &res.0[..])
				.expect("Failed to decode the previous file state"),
			<BTreeMap<Hash, v3::OldDealInfo<T>>>::decode(&mut &res.1[..])
				.expect("Failed to decode the previous deal state"),
		);

		for (key, value) in file_state {
			let file = File::<T>::get(key)
				.expect("Migrated file should exist");
			assert!(file.file_size == value.file_size, "File size mismatch");
		}

		for (key, value) in deal_state {
			let deal = DealMap::<T>::get(key)
				.expect("Migrated deal should exist");
			assert!(deal.file_size == value.file_size, "Deal size mismatch");
		}

		log::info!(
			"file-bank check access success: verified first 10 records of File and DealMap"
		);
		Ok(())
	}
}

pub mod v3 {
	use super::{*, DealMap as NewDealMap, File as NewFile};

	#[storage_alias]
	pub type File<T: Config> = StorageMap<Pallet<T>, Blake2_128Concat, Hash, OldFileInfo<T>>;

	#[storage_alias]
	pub type DealMap<T: Config> = StorageMap<Pallet<T>, Blake2_128Concat, Hash, OldDealInfo<T>>;

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct OldUserBrief<T: Config> {
		pub user: AccountOf<T>,
		pub file_name: BoundedVec<u8, T::NameStrLimit>,
		pub bucket_name: BoundedVec<u8, T::NameStrLimit>,
		pub territory_name: TerrName,
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct OldFileInfo<T: Config> {
		pub segment_list: BoundedVec<SegmentInfo<T>, T::SegmentCount>,
		pub owner: BoundedVec<OldUserBrief<T>, T::OwnerLimit>,
		pub file_size: u128,
		pub completion: BlockNumberFor<T>,
		pub stat: FileState,
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct OldDealInfo<T: Config> {
		pub file_size: u128,
		pub segment_list: BoundedVec<SegmentList<T>, T::SegmentCount>,
		pub user: OldUserBrief<T>,
		pub complete_list: BoundedVec<CompleteInfo<T>, T::FragmentCount>,
	}

	pub fn dealmap_step_migration<T: Config>(done_flag: bool, mut cursor: Option<Hash>) -> Option<Hash> {
		let mut iter = if let Some(last_key) = cursor {
			v3::DealMap::<T>::iter_from(v3::DealMap::<T>::hashed_key_for(last_key))
		} else {
			if done_flag {
				return None;
			}
			v3::DealMap::<T>::iter()
		};

		if let Some((last_key, value)) = iter.next() {
			let user_brief = UserBrief::<T>{
				user: value.user.user,
				file_name: value.user.file_name,
				territory_name: value.user.territory_name,
			};

			let new_info = DealInfo::<T>{
				file_size: value.file_size,
				segment_list: value.segment_list,
				user: user_brief, 
				complete_list: value.complete_list,
			};

			NewDealMap::<T>::insert(last_key, new_info);

			cursor = Some(last_key);
		} else {
			cursor = None;
		}

		cursor
	}
	
	pub fn file_step_migration<T: Config>(done_flag: bool, mut cursor: Option<Hash>) -> Option<Hash> {
		let mut iter = if let Some(last_key) = cursor {
			v3::File::<T>::iter_from(v3::File::<T>::hashed_key_for(last_key))
		} else {
			if done_flag {
				return None;
			}
			v3::File::<T>::iter()
		};

		if let Some((last_key, value)) = iter.next() {
			let mut new_owner: BoundedVec<UserBrief<T>, T::OwnerLimit> = Default::default();
			for owner in value.owner {
				new_owner.try_push(UserBrief::<T>{
					user: owner.user,
					file_name: owner.file_name,
					territory_name: owner.territory_name,
				}).unwrap();
			}

			let file = FileInfo::<T>{
				segment_list: value.segment_list,
				owner: new_owner,
				file_size: value.file_size,
				completion: value.completion,
				stat: value.stat,
			};

			NewFile::<T>::insert(last_key, file);

			cursor = Some(last_key);
		} else {
			cursor = None;
		}

		cursor
	}
}