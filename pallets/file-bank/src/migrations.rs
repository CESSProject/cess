use super::*;
use frame_support::traits::OnRuntimeUpgrade;
use sp_runtime::Saturating;

/// A struct that does not migration, but only checks that the counter prefix exists and is correct.
pub struct MigrationFileBank<T: crate::Config>(sp_std::marker::PhantomData<T>);
impl<T: crate::Config> OnRuntimeUpgrade for MigrationFileBank<T> {
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
	let mut weight: Weight = Weight::zero();

	if version < 3 {
		weight = weight.saturating_add(v3::migrate::<T>());
        StorageVersion::new(3).put::<Pallet<T>>();
	}

	weight
}

mod v3 {
	use super::*;

	#[derive(Decode)]
	struct OldUserBrief<T: Config> {
		pub user: AccountOf<T>,
		pub file_name: BoundedVec<u8, T::NameStrLimit>,
		pub bucket_name: BoundedVec<u8, T::NameStrLimit>,
		pub territory_name: TerrName,
	}

	#[derive(Decode)]
	struct OldFileInfo<T: Config> {
		pub(super) segment_list: BoundedVec<SegmentInfo<T>, T::SegmentCount>,
		pub(super) owner: BoundedVec<OldUserBrief<T>, T::OwnerLimit>,
		pub(super) file_size: u128,
		pub(super) completion: BlockNumberFor<T>,
		pub(super) stat: FileState,
	}

	#[derive(Decode)]
	struct OldDealInfo<T: Config> {
		pub(super) file_size: u128,
		pub(super) segment_list: BoundedVec<SegmentList<T>, T::SegmentCount>,
		pub(super) user: OldUserBrief<T>,
		pub(super) complete_list: BoundedVec<CompleteInfo<T>, T::FragmentCount>,
	}

	pub fn migrate<T: Config>() -> Weight {
		let mut weight: Weight = Weight::zero();
		let mut translated = 0u64;

		log::info!("File-Bank migrate start");

		File::<T>::translate::<OldFileInfo<T>, _>(|_key, old_value| {
			translated.saturating_inc();

			let mut owner_list: BoundedVec<UserBrief<T>, T::OwnerLimit> = Default::default();
			for owner in old_value.owner {
				let new_owner = UserBrief::<T>{
					user: owner.user,
					file_name: owner.file_name,
					territory_name: owner.territory_name,
				};

				owner_list.try_push(new_owner).unwrap();
			}

			Some(FileInfo::<T>{
				segment_list: old_value.segment_list,
				owner: owner_list,
				file_size: old_value.file_size,
				completion: old_value.completion,
				stat: old_value.stat,
			})
		});

		DealMap::<T>::translate::<OldDealInfo<T>, _>(|_key, old_value| {
			translated.saturating_inc();

			Some(DealInfo::<T>{
				file_size: old_value.file_size,
				segment_list: old_value.segment_list,
				user: UserBrief::<T>{
					user: old_value.user.user,
					file_name: old_value.user.file_name,
					territory_name: old_value.user.territory_name,
				},
				complete_list: old_value.complete_list,
			})
		});

		log::info!(
			"Upgraded {} pools, storage to version {}",
			translated,
			3,
		);

		T::DbWeight::get().reads_writes(translated + 1, translated + 1)
	}
}
