
use super::{*, DealMap as NewDealMap, File as NewFile};
use crate::WeightInfo;
use frame_support::{
	storage_alias, DebugNoBound, DefaultNoBound,
};
use sp_std::collections::btree_map::BTreeMap;
use sp_runtime::{TryRuntimeError, Saturating};


#[derive(Encode, Decode, MaxEncodedLen, DefaultNoBound)]
pub struct Migration<T: Config> {
	last_key: Option<(Option<Hash>, Option<Hash>)>,
	_phantom: PhantomData<T>,
}

impl<T: Config> MigrationStep for Migration<T> {
	const VERSION: u16 = 3;

	fn max_step_weight() -> Weight {
		T::WeightInfo::v03_migration_step()
	}

	fn step(
		&mut self, 
		meter: &mut WeightMeter
	) -> IsFinished {
		let (file_key_opt, deal_key_opt) = if let Some((file_key_opt, deal_key_opt)) = self.last_key.take() {
			(
				file_step_migration::<T>(true, file_key_opt),
				dealmap_step_migration::<T>(true, deal_key_opt),
			)
		} else {
			(
				file_step_migration::<T>(false, None),
				dealmap_step_migration::<T>(false, None),
			)
		};

		if file_key_opt.is_none() && deal_key_opt.is_none() {
			meter.consume(T::WeightInfo::v03_migration_step());
			return IsFinished::Yes;
		} else {
			meter.consume(T::WeightInfo::v03_migration_step());
			self.last_key = Some((file_key_opt, deal_key_opt));
			return IsFinished::No;
		}
	}
}

#[storage_alias]
pub type File<T: Config> = StorageMap<Pallet<T>, Blake2_128Concat, Hash, OldFileInfo<T>>;

#[storage_alias]
pub type DealMap<T: Config> = StorageMap<Pallet<T>, Blake2_128Concat, Hash, OldDealInfo<T>>;

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct OldUserBrief<T: Config> {
	pub user: AccountOf<T>,
	pub file_name: BoundedVec<u8, T::NameStrLimit>,
	pub bucket_name: BoundedVec<u8, T::NameStrLimit>,
	pub territory_name: TerrName,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct OldFileInfo<T: Config> {
	pub segment_list: BoundedVec<SegmentInfo<T>, T::SegmentCount>,
	pub owner: BoundedVec<OldUserBrief<T>, T::OwnerLimit>,
	pub file_size: u128,
	pub completion: BlockNumberFor<T>,
	pub stat: FileState,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct OldDealInfo<T: Config> {
	pub file_size: u128,
	pub segment_list: BoundedVec<SegmentList<T>, T::SegmentCount>,
	pub user: OldUserBrief<T>,
	pub complete_list: BoundedVec<CompleteInfo<T>, T::FragmentCount>,
}

pub fn dealmap_step_migration<T: Config>(done_flag: bool, mut cursor: Option<Hash>) -> Option<Hash> {
	let mut iter = if let Some(last_key) = cursor {
		DealMap::<T>::iter_from(DealMap::<T>::hashed_key_for(last_key))
	} else {
		if done_flag {
			return None;
		}
		DealMap::<T>::iter()
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
		File::<T>::iter_from(File::<T>::hashed_key_for(last_key))
	} else {
		if done_flag {
			return None;
		}
		File::<T>::iter()
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

