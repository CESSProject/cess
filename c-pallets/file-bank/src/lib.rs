//! # File Bank Module
//!
//! Contain operations related info of files on multi-direction.
//!
//! ### Terminology
//!
//! * **Is Public:** Public or private.
//! * **Backups:** Number of duplicate.
//! * **Deadline:** Expiration time.
//!
//!
//! ### Interface
//!
//! ### Dispatchable Functions
//!
//! * `upload` - Upload info of stored file.
//! * `update` - Update info of uploaded file.
//! * `buyfile` - Buy file with download fee.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::traits::{
	FindAuthor, Randomness,
	StorageVersion,
	schedule::{Anon as ScheduleAnon, DispatchTime, Named as ScheduleNamed}, 
};

pub use pallet::*;
#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod weights;
// pub mod migrations;

mod types;
pub use types::*;

mod functions;

use codec::{Decode, Encode};
use frame_support::{
	// bounded_vec, 
	transactional, 
	PalletId, 
	dispatch::{Dispatchable, DispatchResult}, 
	pallet_prelude::*,
	weights::Weight,
	traits::schedule,
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use cp_cess_common::*;
use pallet_storage_handler::StorageHandle;
use cp_scheduler_credit::SchedulerCreditCounter;
use sp_runtime::{
	traits::{
		BlockNumberProvider, CheckedAdd,
	},
	RuntimeDebug, SaturatedConversion,
};
use sp_std::{
	convert::TryInto, 
	prelude::*, 
	str, 
	collections::btree_map::BTreeMap
};
use pallet_sminer::MinerControl;
use pallet_tee_worker::ScheduleFind;
use pallet_oss::OssFindAuthor;

pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{ensure, traits::Get};

	//pub use crate::weights::WeightInfo;
	use frame_system::ensure_signed;

	pub const FILE_PENDING: &str = "pending";
	pub const FILE_ACTIVE: &str = "active";

	#[pallet::config]
	pub trait Config: frame_system::Config + sp_std::fmt::Debug {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type WeightInfo: WeightInfo;

		type RuntimeCall: From<Call<Self>>;

		type FScheduler: ScheduleNamed<Self::BlockNumber, Self::SProposal, Self::SPalletsOrigin>;

		type AScheduler: ScheduleAnon<Self::BlockNumber, Self::SProposal, Self::SPalletsOrigin>;
		/// Overarching type of all pallets origins.
		type SPalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;
		/// The SProposal.
		type SProposal: Parameter + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin> + From<Call<Self>>;
		//Find the consensus of the current block
		type FindAuthor: FindAuthor<Self::AccountId>;
		//Used to find out whether the schedule exists
		type Scheduler: ScheduleFind<Self::AccountId>;
		//It is used to control the computing power and space of miners
		type MinerControl: MinerControl<Self::AccountId>;
		//Interface that can generate random seeds
		type MyRandomness: Randomness<Option<Self::Hash>, Self::BlockNumber>;

		type StorageHandle: StorageHandle<Self::AccountId>;
		/// pallet address.
		#[pallet::constant]
		type FilbakPalletId: Get<PalletId>;

		#[pallet::constant]
		type StringLimit: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type OneDay: Get<BlockNumberOf<Self>>;

		#[pallet::constant]
		type UploadFillerLimit: Get<u8> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type RecoverLimit: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type InvalidLimit: Get<u32> + Clone + Eq + PartialEq;
		// User defined name length limit
		#[pallet::constant]
		type NameStrLimit: Get<u32> + Clone + Eq + PartialEq;
		// In order to enable users to store unlimited number of files,
		// a large number is set as the boundary of BoundedVec.
		#[pallet::constant]
		type FileListLimit: Get<u32> + Clone + Eq + PartialEq;
		// Maximum number of containers that users can create.
		#[pallet::constant]
		type BucketLimit: Get<u32> + Clone + Eq + PartialEq;
		// Minimum length of bucket name.
		#[pallet::constant]
		type NameMinLength: Get<u32> + Clone + Eq + PartialEq;
		// Maximum number of segments.
		#[pallet::constant]
		type SegmentCount: Get<u32> + Clone + Eq + PartialEq;
		// Set number of fragment redundancy.
		#[pallet::constant]
		type FragmentCount: Get<u32> + Clone + Eq + PartialEq;
		// Maximum number of holders of a file
		#[pallet::constant]
		type OwnerLimit: Get<u32> + Clone + Eq + PartialEq;

		type CreditCounter: SchedulerCreditCounter<Self::AccountId>;
		//Used to confirm whether the origin is authorized
		type OssFindAuthor: OssFindAuthor<Self::AccountId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		//file upload declaration
		UploadDeclaration { operator: AccountOf<T>, owner: AccountOf<T>, deal_hash: Hash },
		//file uploaded.
		TransferReport { acc: AccountOf<T>, failed_list: Vec<Hash> },
		//File deletion event
		DeleteFile { operator:AccountOf<T>, owner: AccountOf<T>, file_hash: Hash },

		ReplaceFiller { acc: AccountOf<T>, filler_list: Vec<Hash> },

		CalculateEnd{ file_hash: Hash },
		//Filler chain success event
		FillerUpload { acc: AccountOf<T>, file_size: u64 },
		//File recovery
		RecoverFile { acc: AccountOf<T>, file_hash: [u8; 68] },
		//The miner cleaned up an invalid file event
		ClearInvalidFile { acc: AccountOf<T>, file_hash: Hash },
		//Event to successfully create a bucket
		CreateBucket { operator: AccountOf<T>, owner: AccountOf<T>, bucket_name: Vec<u8>},
		//Successfully delete the bucket event
		DeleteBucket { operator: AccountOf<T>, owner: AccountOf<T>, bucket_name: Vec<u8>},
	}
	#[pallet::error]
	pub enum Error<T> {
		Existed,

		FileExistent,
		//file doesn't exist.
		FileNonExistent,
		//overflow.
		Overflow,

		NotOwner,

		NotQualified,
		//It is not an error message for scheduling operation
		ScheduleNonExistent,
		//Error reporting when boundedvec is converted to VEC
		BoundedVecError,
		//Error that the storage has reached the upper limit.
		StorageLimitReached,
		//The miner's calculation power is insufficient, resulting in an error that cannot be
		// replaced
		MinerPowerInsufficient,

		IsZero,
		//Multi consensus query restriction of off chain workers
		Locked,

		LengthExceedsLimit,

		Declarated,

		BugInvalid,

		ConvertHashError,
		//No operation permission
		NoPermission,
		//user had same name bucket
		SameBucketName,
		//Bucket, file, and scheduling errors do not exist
		NonExistent,
		//Unexpected error
		Unexpected,
		//Less than minimum length
		LessMinLength,
		//The file is in an unprepared state
		Unprepared,
		//Transfer target acc already have this file
		IsOwned,
		//The file does not meet the specification
		SpecError,

		NodesInsufficient,
		// This is a bug that is reported only when the most undesirable 
		// situation occurs during a transaction execution process.
		PanicOverflow,

		InsufficientAvailableSpace,
		// The file is in a calculated tag state and cannot be deleted
		Calculate,

		MinerStateError,
	}

	
	#[pallet::storage]
	#[pallet::getter(fn deal_map)]
	pub(super) type DealMap<T: Config> = StorageMap<_, Blake2_128Concat, Hash, DealInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn file)]
	pub(super) type File<T: Config> =
		StorageMap<_, Blake2_128Concat, Hash, FileInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn user_hold_file_list)]
	pub(super) type UserHoldFileList<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<UserFileSliceInfo, T::StringLimit>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn filler_map)]
	pub(super) type FillerMap<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountOf<T>,
		Blake2_128Concat,
		Hash,
		FillerInfo<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn pending_replacements)]
	pub(super) type PendingReplacements<T: Config> = StorageMap<_, Blake2_128Concat, AccountOf<T>, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn invalid_file)]
	pub(super) type InvalidFile<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, BoundedVec<Hash, T::InvalidLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn miner_lock)]
	pub(super) type MinerLock<T: Config> = 
		StorageMap<_, Blake2_128Concat, AccountOf<T>, BlockNumberOf<T>>;

	// Stores the hash of the entire network segment and the sharing of each segment.
	// Used to determine whether segments can be shared.
	#[pallet::storage]
	#[pallet::getter(fn segment_map)]
	pub(super) type SegmentMap<T: Config> = StorageMap<_, Blake2_128Concat, Hash, (SegmentInfo<T>, u32)>;

	#[pallet::storage]
	#[pallet::getter(fn bucket)]
	pub(super) type Bucket<T: Config> =
		StorageDoubleMap<
			_,
			Blake2_128Concat,
			AccountOf<T>,
			Blake2_128Concat,
			BoundedVec<u8, T::NameStrLimit>,
			BucketInfo<T>,
		>;

	#[pallet::storage]
	#[pallet::getter(fn user_bucket_list)]
	pub(super) type UserBucketList<T: Config> = 
		StorageMap<
			_,
			Blake2_128Concat,
			AccountOf<T>,
			BoundedVec<BoundedVec<u8, T::NameStrLimit>, T::BucketLimit>,
			ValueQuery,
		>;
	
	#[pallet::storage]
	#[pallet::getter(fn restoral_target)]
	pub(super) type RestoralTarget<T: Config> = 
		StorageMap< _, Blake2_128Concat, AccountOf<T>, RestoralInfo<BlockNumberOf<T>>>;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberOf<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberOf<T>) -> Weight {
			let (mut weight, acc_list) = T::StorageHandle::frozen_task();

			for acc in acc_list.iter() {
				if let Ok(file_info_list) = <UserHoldFileList<T>>::try_get(&acc) {
					for file_info in file_info_list.iter() {
						if let Ok(file) = <File<T>>::try_get(&file_info.file_hash) {
							weight = weight.saturating_add(T::DbWeight::get().reads(1));
							if file.owner.len() > 1 {
								if let Ok(()) = Self::remove_file_owner(&file_info.file_hash, &acc, false) {
									weight = weight.saturating_add(T::DbWeight::get().reads_writes(2, 2));
								}
							 } else {
								if let Ok(temp_weight) = Self::remove_file_last_owner(&file_info.file_hash, &acc, false) {
									weight = weight.saturating_add(temp_weight);
								}
							}
						}
					}
					<UserHoldFileList<T>>::remove(&acc);
					// todo! clear all
					let _ = <Bucket<T>>::clear_prefix(&acc, 100000, None);
				}
			}
			
			weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Users need to make a declaration before uploading files.
		///
		/// This method is used to declare the file to be uploaded.
		/// If the file already exists on the chain,
		/// the user directly becomes one of the holders of the file
		/// If the file does not exist, after declaring the file,
		/// wait for the dispatcher to upload the meta information of the file
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// Parameters:
		/// - `file_hash`: Hash of the file to be uploaded.
		/// - `file_name`: User defined file name.
		#[pallet::call_index(0)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upload_declaration())]
		pub fn upload_declaration(
			origin: OriginFor<T>,
			file_hash: Hash,
			deal_info: BoundedVec<SegmentList<T>, T::SegmentCount>,
			user_brief: UserBrief<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			// Check if you have operation permissions.
			ensure!(Self::check_permission(sender.clone(), user_brief.user.clone()), Error::<T>::NoPermission);
			// Check file specifications.
			ensure!(Self::check_file_spec(&deal_info), Error::<T>::SpecError);
			// Check whether the user-defined name meets the rules.
			
			let minimum = T::NameMinLength::get();
			ensure!(user_brief.file_name.len() as u32 >= minimum, Error::<T>::SpecError);
			ensure!(user_brief.bucket_name.len() as u32 >= minimum, Error::<T>::SpecError);

			let needed_space = deal_info.len() as u128 * (SEGMENT_SIZE * 15 / 10);
			ensure!(T::StorageHandle::get_user_avail_space(&user_brief.user)? > needed_space, Error::<T>::InsufficientAvailableSpace);		

			if <File<T>>::contains_key(&file_hash) {
				T::StorageHandle::update_user_space(&user_brief.user, 1, needed_space)?;

				if <Bucket<T>>::contains_key(&user_brief.user, &user_brief.bucket_name) {
						Self::add_file_to_bucket(&user_brief.user, &user_brief.bucket_name, &file_hash)?;
					} else {
						Self::create_bucket_helper(&user_brief.user, &user_brief.bucket_name, Some(file_hash))?;
					}

				Self::add_user_hold_fileslice(&user_brief.user, file_hash, needed_space)?;

				<File<T>>::try_mutate(&file_hash, |file_opt| -> DispatchResult {
					let file = file_opt.as_mut().ok_or(Error::<T>::FileNonExistent)?;
					file.owner.try_push(user_brief.clone()).map_err(|_e| Error::<T>::BoundedVecError)?;
					Ok(())
				})?;
			} else {
				// Check whether the user's storage space is sufficient, 
				// if sufficient lock user's storage space.
				// Perform space calculations based on 1.5 times redundancy.
				let mut needed_list: BoundedVec<SegmentList<T>, T::SegmentCount> = Default::default();
				let mut share_info: Vec<SegmentInfo<T>> = Default::default();
				// Check whether there are segments that can be shared.
				for segment_list in &deal_info {
					if <SegmentMap<T>>::contains_key(segment_list.hash) {
						let segment_info = <SegmentMap<T>>::try_get(&segment_list.hash).map_err(|_| Error::<T>::BugInvalid)?.0;
						share_info.push(segment_info);
					} else {
						needed_list.try_push(segment_list.clone()).map_err(|_e| Error::<T>::BoundedVecError)?;
					}
				}

				if share_info.len() == deal_info.len() {
					T::StorageHandle::update_user_space(&user_brief.user, 1, needed_space)?;

					if <Bucket<T>>::contains_key(&user_brief.user, &user_brief.bucket_name) {
						Self::add_file_to_bucket(&user_brief.user, &user_brief.bucket_name, &file_hash)?;
					} else {
						Self::create_bucket_helper(&user_brief.user, &user_brief.bucket_name, Some(file_hash))?;
					}

					Self::add_user_hold_fileslice(&user_brief.user, file_hash, needed_space)?;

					Self::generate_file(&file_hash, deal_info, Default::default(), share_info, user_brief.clone(), FileState::Active)?;

				} else {
					T::StorageHandle::lock_user_space(&user_brief.user, needed_space)?;
					// TODO! Replace the file_hash param
					Self::generate_deal(file_hash.clone(), needed_list, deal_info, user_brief.clone(), share_info)?;
				}

			}

			Self::deposit_event(Event::<T>::UploadDeclaration { operator: sender, owner: user_brief.user, deal_hash: file_hash });

			Ok(())
		}
		
		#[pallet::call_index(1)]
		#[transactional]
		#[pallet::weight(1_000_000_000)]
		pub fn deal_reassign_miner(
			origin: OriginFor<T>,
			deal_hash: Hash,
			count: u8,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			if count < 5 {
				<DealMap<T>>::try_mutate(&deal_hash, |opt| -> DispatchResult {
					let deal_info = opt.as_mut().ok_or(Error::<T>::NonExistent)?;
					let miner_task_list = Self::random_assign_miner(&deal_info.needed_list)?;
					deal_info.assigned_miner = miner_task_list;
					deal_info.complete_list = Default::default();
					Self::start_first_task(deal_hash.0.to_vec(), deal_hash, count + 1)?;
					// unlock mienr space
					for miner_task in &deal_info.assigned_miner {
						let count = miner_task.fragment_list.len() as u128;
						T::MinerControl::unlock_space(&miner_task.miner, FRAGMENT_SIZE * count)?;
					}
					Ok(())
				})?;
			} else {
				let deal_info = <DealMap<T>>::try_get(&deal_hash).map_err(|_| Error::<T>::NonExistent)?;
				let needed_space = Self::cal_file_size(deal_info.segment_list.len() as u128);
				T::StorageHandle::unlock_user_space(&deal_info.user.user, needed_space)?;
				// unlock mienr space
				for miner_task in deal_info.assigned_miner {
					let count = miner_task.fragment_list.len() as u128;
					T::MinerControl::unlock_space(&miner_task.miner, FRAGMENT_SIZE * count)?;
				}
				
				<DealMap<T>>::remove(&deal_hash);
			}

			Ok(())
		}
		//TODO!
		// Transfer needs to be restricted, such as target consent
		/// Document ownership transfer function.
		///
		/// You can replace Alice, the holder of the file, with Bob. At the same time,
		/// Alice will lose the ownership of the file and release the corresponding use space.
		/// Bob will get the ownership of the file and increase the corresponding use space
		///
		/// Premise:
		/// - Alice has ownership of the file
		/// - Bob has enough space and corresponding bucket
		///
		/// Parameters:
		/// - `owner_bucket_name`: Origin stores the bucket name corresponding to the file
		/// - `target_brief`: Information about the transfer object
		/// - `file_hash`: File hash, which is also the unique identifier of the file
		#[pallet::call_index(2)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::ownership_transfer())]
		pub fn ownership_transfer(
			origin: OriginFor<T>,
			owner_bucket_name: BoundedVec<u8, T::NameStrLimit>,
			target_brief: UserBrief<T>,
			file_hash: Hash,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let file = <File<T>>::try_get(&file_hash).map_err(|_| Error::<T>::FileNonExistent)?;
			//If the file does not exist, false will also be returned
			ensure!(Self::check_is_file_owner(&sender, &file_hash), Error::<T>::NotOwner);
			ensure!(!Self::check_is_file_owner(&target_brief.user, &file_hash), Error::<T>::IsOwned);

			ensure!(file.stat == FileState::Active, Error::<T>::Unprepared);
			ensure!(<Bucket<T>>::contains_key(&target_brief.user, &target_brief.bucket_name), Error::<T>::NonExistent);
			//Modify the space usage of target acc,
			//and determine whether the space is enough to support transfer
			let file_size = Self::cal_file_size(file.segment_list.len() as u128);
			T::StorageHandle::update_user_space(&target_brief.user, 1, file_size)?;
			//Increase the ownership of the file for target acc
			<File<T>>::try_mutate(&file_hash, |file_opt| -> DispatchResult {
				let file = file_opt.as_mut().ok_or(Error::<T>::FileNonExistent)?;
				file.owner.try_push(target_brief.clone()).map_err(|_| Error::<T>::BoundedVecError)?;
				Ok(())
			})?;
			//Add files to the bucket of target acc
			<Bucket<T>>::try_mutate(
				&target_brief.user,
				&target_brief.bucket_name,
				|bucket_info_opt| -> DispatchResult {
					let bucket_info = bucket_info_opt.as_mut().ok_or(Error::<T>::NonExistent)?;
					bucket_info.object_list.try_push(file_hash.clone()).map_err(|_| Error::<T>::LengthExceedsLimit)?;
					Ok(())
			})?;
			//Increase the corresponding space usage for target acc
			Self::add_user_hold_fileslice(
				&target_brief.user,
				file_hash.clone(),
				file_size,
			)?;
			//Clean up the file holding information of the original user
			let file = <File<T>>::try_get(&file_hash).map_err(|_| Error::<T>::NonExistent)?;

			let _ = Self::delete_user_file(&file_hash, &sender, &file)?;

			Self::bucket_remove_file(&file_hash, &sender, &file)?;

			Self::remove_user_hold_file_list(&file_hash, &sender)?;
			// let _ = Self::clear_user_file(file_hash.clone(), &sender, true)?;

			Ok(())
		}
		/// Upload info of stored file.
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// The same file will only upload meta information once,
		/// which will be uploaded by consensus.
		///
		/// Parameters:
		/// - `file_hash`: The beneficiary related to signer account.
		/// - `file_size`: File size calculated by consensus.
		/// - `slice_info`: List of file slice information.
		#[pallet::call_index(3)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upload(2))]
		pub fn transfer_report(
			origin: OriginFor<T>,
			deal_hash: Vec<Hash>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(deal_hash.len() < 5, Error::<T>::LengthExceedsLimit);
			let mut failed_list: Vec<Hash> = Default::default();
			for hash in deal_hash {
				if !<DealMap<T>>::contains_key(&hash) {
					failed_list.push(hash);
					continue;
				} else {
					<DealMap<T>>::try_mutate(&hash, |deal_info_opt| -> DispatchResult {
						// can use unwrap because there was a judgment above
						let deal_info = deal_info_opt.as_mut().unwrap();
						let mut task_miner_list: Vec<AccountOf<T>> = Default::default();
						for miner_task in &deal_info.assigned_miner {
							task_miner_list.push(miner_task.miner.clone());
						}
						if task_miner_list.contains(&sender) {
							if !deal_info.complete_list.contains(&sender) {
								deal_info.complete_list.try_push(sender.clone()).map_err(|_| Error::<T>::BoundedVecError)?;
							}
							// If it is the last submitter of the order.
							if deal_info.complete_list.len() == deal_info.assigned_miner.len() {
								deal_info.stage = 2;
								Self::generate_file(
									&hash,
									deal_info.segment_list.clone(),
									deal_info.assigned_miner.clone(),
									deal_info.share_info.to_vec(),
									deal_info.user.clone(),
									FileState::Calculate,
								)?;

								for miner_task in deal_info.assigned_miner.iter() {
									let count = miner_task.fragment_list.len() as u32;
									// Miners need to report the replaced documents themselves. 
									// If a challenge is triggered before the report is completed temporarily, 
									// these documents to be replaced also need to be verified
									<PendingReplacements<T>>::try_mutate(miner_task.miner.clone(), |pending_count| -> DispatchResult {
										let pending_count_temp = pending_count.checked_add(count).ok_or(Error::<T>::Overflow)?;
										*pending_count = pending_count_temp;
										Ok(())
									})?;
								}	

								let needed_space = Self::cal_file_size(deal_info.segment_list.len() as u128);
								T::StorageHandle::unlock_and_used_user_space(&deal_info.user.user, needed_space)?;
								T::FScheduler::cancel_named(hash.0.to_vec()).map_err(|_| Error::<T>::Unexpected)?;
								Self::start_second_task(hash.0.to_vec(), hash, 5)?;
								if <Bucket<T>>::contains_key(&deal_info.user.user, &deal_info.user.bucket_name) {
									Self::add_file_to_bucket(&deal_info.user.user, &deal_info.user.bucket_name, &hash)?;
								} else {
									Self::create_bucket_helper(&deal_info.user.user, &deal_info.user.bucket_name, Some(hash))?;
								}
							}
						} else {
							failed_list.push(hash);
						}

						Ok(())
					})?;
				}
			}

			Self::deposit_event(Event::<T>::TransferReport{acc: sender, failed_list});
			
			Ok(())
		}

		#[pallet::call_index(4)]
		#[transactional]
		#[pallet::weight(1_000_000_000)]
		pub fn calculate_end(
			origin: OriginFor<T>,
			deal_hash: Hash,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			let deal_info = <DealMap<T>>::try_get(&deal_hash).map_err(|_| Error::<T>::NonExistent)?;
			for miner_task in deal_info.assigned_miner {
				let count = miner_task.fragment_list.len() as u32;
				// Accumulate the number of fragments stored by each miner
				T::MinerControl::unlock_space_to_service(&miner_task.miner, FRAGMENT_SIZE * count as u128)?;
			}

			<File<T>>::try_mutate(&deal_hash, |file_opt| -> DispatchResult {
				let file = file_opt.as_mut().ok_or(Error::<T>::BugInvalid)?;
				file.stat = FileState::Active;
				Ok(())
			})?;

			<DealMap<T>>::remove(&deal_hash);

			Self::deposit_event(Event::<T>::CalculateEnd{ file_hash: deal_hash });

			Ok(())
		}

		#[pallet::call_index(5)]
		#[transactional]
		#[pallet::weight(1_000_000_000)]
		pub fn replace_file_report(
			origin: OriginFor<T>,
			filler: Vec<Hash>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(filler.len() < 30, Error::<T>::LengthExceedsLimit);
			let pending_count = <PendingReplacements<T>>::get(&sender);
			ensure!(filler.len() as u32 <= pending_count, Error::<T>::LengthExceedsLimit);

			let mut count: u32 = 0;
			for filler_hash in filler.iter() {
				if <FillerMap<T>>::contains_key(&sender, filler_hash) {
					count += 1;
					<FillerMap<T>>::remove(&sender, filler_hash);
				} else {
					log::info!("filler nonexist!");
				}
			}

			<PendingReplacements<T>>::mutate(&sender, |pending_count| -> DispatchResult {
				let pending_count_temp = pending_count.checked_sub(count).ok_or(Error::<T>::Overflow)?;
				*pending_count = pending_count_temp;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::ReplaceFiller{ acc: sender, filler_list: filler });

			Ok(())
		}

		#[pallet::call_index(6)]
		#[transactional]
		#[pallet::weight(1_000_000_000)]
		pub fn delete_file(origin: OriginFor<T>, owner: AccountOf<T>, file_hash: Hash) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			// Check if you have operation permissions.
			ensure!(Self::check_permission(sender.clone(), owner.clone()), Error::<T>::NoPermission);

			let file = <File<T>>::try_get(&file_hash).map_err(|_| Error::<T>::NonExistent)?;

			let _ = Self::delete_user_file(&file_hash, &owner, &file)?;

			Self::bucket_remove_file(&file_hash, &owner, &file)?;

			Self::remove_user_hold_file_list(&file_hash, &owner)?;

			Self::deposit_event(Event::<T>::DeleteFile{ operator: sender, owner, file_hash });

			Ok(())
		}
		/// Upload idle files for miners.
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// Upload up to ten idle files for one transaction.
		/// Currently, the size of each idle file is fixed at 8MiB.
		///
		/// Parameters:
		/// - `miner`: For which miner, miner's wallet address.
		/// - `filler_list`: Meta information list of idle files.
		#[pallet::call_index(8)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upload_filler(filler_list.len() as u32))]
		pub fn upload_filler(
			origin: OriginFor<T>,
			miner: AccountOf<T>,
			filler_list: Vec<FillerInfo<T>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let limit = T::UploadFillerLimit::get();
			if filler_list.len() > limit as usize {
				Err(Error::<T>::LengthExceedsLimit)?;
			}
			if !T::Scheduler::contains_scheduler(sender.clone()) {
				Err(Error::<T>::ScheduleNonExistent)?;
			}
			let is_positive = T::MinerControl::is_positive(&miner)?;
			ensure!(is_positive, Error::<T>::NotQualified);

			for i in filler_list.iter() {
				if <FillerMap<T>>::contains_key(&miner, i.filler_hash.clone()) {
					Err(Error::<T>::FileExistent)?;
				}
				<FillerMap<T>>::insert(miner.clone(), i.filler_hash.clone(), i);
			}

			let power = M_BYTE
				.checked_mul(8)
				.ok_or(Error::<T>::Overflow)?
				.checked_mul(filler_list.len() as u128)
				.ok_or(Error::<T>::Overflow)?;
			T::MinerControl::add_miner_idle_space(&miner, power)?;
			T::StorageHandle::add_total_idle_space(power)?;
			Self::record_uploaded_fillers_size(&sender, &filler_list)?;

			Self::deposit_event(Event::<T>::FillerUpload { acc: sender, file_size: power as u64 });
			Ok(())
		}
		/// Clean up invalid idle files
		///
		/// When the idle documents are replaced or proved to be invalid,
		/// they will become invalid documents and need to be cleared by the miners
		///
		/// Parameters:
		/// - `file_hash`: Invalid file hash
		#[pallet::call_index(9)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::clear_invalid_file())]
		pub fn clear_invalid_file(origin: OriginFor<T>, file_hash: Hash) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			<InvalidFile<T>>::try_mutate(&sender, |o| -> DispatchResult {
				o.retain(|x| *x != file_hash);
				Ok(())
			})?;
			Self::deposit_event(Event::<T>::ClearInvalidFile { acc: sender, file_hash });
			Ok(())
		}


		#[pallet::call_index(11)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::create_bucket())]
		pub fn create_bucket(
			origin: OriginFor<T>,
			owner: AccountOf<T>,
			name: BoundedVec<u8, T::NameStrLimit>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::check_permission(sender.clone(), owner.clone()), Error::<T>::NoPermission);
			ensure!(!<Bucket<T>>::contains_key(&sender, &name), Error::<T>::SameBucketName);
			ensure!(name.len() >= T::NameMinLength::get() as usize, Error::<T>::LessMinLength);
			let bucket = BucketInfo::<T>{
				object_list: Default::default(),
				authority: vec![owner.clone()].try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
			};

			<Bucket<T>>::insert(&owner, &name, bucket);
			<UserBucketList<T>>::try_mutate(&owner, |bucket_list| -> DispatchResult{
				bucket_list.try_push(name.clone()).map_err(|_e| Error::<T>::LengthExceedsLimit)?;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::CreateBucket {
				operator: sender,
				owner,
				bucket_name: name.to_vec(),
			});

			Ok(())
		}

		#[pallet::call_index(12)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::delete_bucket())]
		pub fn delete_bucket(
			origin: OriginFor<T>,
			owner: AccountOf<T>,
			name: BoundedVec<u8, T::NameStrLimit>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::check_permission(sender.clone(), owner.clone()), Error::<T>::NoPermission);
			ensure!(<Bucket<T>>::contains_key(&owner, &name), Error::<T>::NonExistent);
			let bucket = <Bucket<T>>::try_get(&owner, &name).map_err(|_| Error::<T>::Unexpected)?;
			for file_hash in bucket.object_list.iter() {
				let _file = <File<T>>::try_get(file_hash).map_err(|_| Error::<T>::Unexpected)?;
			}
			<Bucket<T>>::remove(&owner, &name);
			<UserBucketList<T>>::try_mutate(&owner, |bucket_list| -> DispatchResult {
				let mut index = 0;
				for name_tmp in bucket_list.iter() {
					if *name_tmp == name {
						break;
					}
					index = index.checked_add(&1).ok_or(Error::<T>::Overflow)?;
				}
				bucket_list.remove(index);
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::DeleteBucket {
				operator: sender,
				owner,
				bucket_name: name.to_vec(),
			});
			Ok(())
		}

		#[pallet::call_index(13)]
		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn miner_exit_prep(
			origin: OriginFor<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let lock_time = <MinerLock<T>>::try_get(&sender).map_err(|_| Error::<T>::MinerStateError)?;
			let now = <frame_system::Pallet<T>>::block_number();
			ensure!(now < lock_time, Error::<T>::MinerStateError);

			let result = T::MinerControl::is_positive(&sender)?;
			ensure!(result, Error::<T>::MinerStateError);
			T::MinerControl::update_miner_state(&sender, "lock")?;

			let now = <frame_system::Pallet<T>>::block_number();
			let lock_time = T::OneDay::get().checked_add(&now).ok_or(Error::<T>::Overflow)?;
			<MinerLock<T>>::insert(&sender, lock_time);

			let task_id: Vec<u8> = sender.encode();
			T::FScheduler::schedule_named(
                task_id,
                DispatchTime::At(lock_time),
                Option::None,
                schedule::HARD_DEADLINE,
                frame_system::RawOrigin::Root.into(),
                Call::miner_exit{miner: sender}.into(), 
        	).map_err(|_| Error::<T>::Unexpected)?;

			Ok(())
		}

		#[pallet::call_index(14)]
		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn miner_exit(
			origin: OriginFor<T>,
			miner: AccountOf<T>,
		) -> DispatchResult {
			let _ = ensure_root(origin);

			// judge lock state.
			let result = T::MinerControl::is_lock(&miner)?;
			ensure!(result, Error::<T>::MinerStateError);
			// sub network total idle space.
			Self::clear_filler(&miner, None);
			let (idle_space, service_space) = T::MinerControl::get_power(&miner)?;
			T::StorageHandle::sub_total_idle_space(idle_space)?;

			T::MinerControl::execute_exit(&miner)?;

			Self::create_restoral_target(&miner, service_space)?;

			Ok(())
		}

		#[pallet::call_index(15)]
		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn miner_withdaw(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let restoral_info = <RestoralTarget<T>>::try_get(&sender).map_err(|_| Error::<T>::MinerStateError)?;
			let now = <frame_system::Pallet<T>>::block_number();

			ensure!(now >= restoral_info.cooling_block, Error::<T>::MinerStateError);

			T::MinerControl::withdraw(&sender)?;

			Ok(())
		}
	}
}

pub trait RandomFileList<AccountId> {
	//Get random challenge data
	fn get_random_challenge_data(
	) -> Result<Vec<(AccountId, Hash, [u8; 68], Vec<u32>, u64, DataType)>, DispatchError>;
	//Delete filler file
	fn delete_filler(miner_acc: AccountId, filler_hash: Hash) -> DispatchResult;
	//Delete all filler according to miner_acc
	fn delete_miner_all_filler(miner_acc: AccountId) -> Result<Weight, DispatchError>;
	//Delete file backup
	fn clear_file(_file_hash: Hash) -> Result<Weight, DispatchError>;

	fn force_miner_exit(miner: &AccountId) -> DispatchResult;
}

impl<T: Config> RandomFileList<<T as frame_system::Config>::AccountId> for Pallet<T> {
	fn get_random_challenge_data(
	) -> Result<Vec<(AccountOf<T>, Hash, [u8; 68], Vec<u32>, u64, DataType)>, DispatchError> {
		Ok(Default::default())
	}

	fn delete_filler(miner_acc: AccountOf<T>, filler_hash: Hash) -> DispatchResult {
		Pallet::<T>::delete_filler(miner_acc, filler_hash)?;
		Ok(())
	}
	fn delete_miner_all_filler(miner_acc: AccountOf<T>) -> Result<Weight, DispatchError> {
		let mut weight: Weight = Weight::from_ref_time(0);
		for (_, _value) in FillerMap::<T>::iter_prefix(&miner_acc) {
			weight = weight.saturating_add(T::DbWeight::get().writes(1 as u64));
		}
		#[allow(deprecated)]
		let _ = FillerMap::<T>::remove_prefix(&miner_acc, Option::None);
		weight = weight.saturating_add(T::DbWeight::get().writes(1 as u64));
		Ok(weight)
	}

	fn clear_file(_file_hash: Hash) -> Result<Weight, DispatchError> {
		let weight: Weight = Weight::from_ref_time(0);
		Ok(weight)
	}

	fn force_miner_exit(miner: &AccountOf<T>) -> DispatchResult {
		Self::force_miner_exit(miner)
	}
}

impl<T: Config> BlockNumberProvider for Pallet<T> {
	type BlockNumber = T::BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		<frame_system::Pallet<T>>::block_number()
	}
}
