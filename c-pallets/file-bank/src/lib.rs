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
	Currency, ExistenceRequirement::AllowDeath, FindAuthor, Randomness, ReservableCurrency,
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
use cp_cess_common::{DataType, Hash as H68, M_BYTE, T_BYTE, G_BYTE, FRAGMENT_SIZE, SEGMENT_SIZE};
use pallet_storage_handler::StorageHandle;
use cp_scheduler_credit::SchedulerCreditCounter;
use sp_runtime::{
	traits::{
		AccountIdConversion, BlockNumberProvider, CheckedAdd,
	},
	RuntimeDebug, SaturatedConversion,
};
use sp_std::{
	convert::TryInto, 
	prelude::*, 
	str, 
	collections::btree_map::BTreeMap
};

pub use weights::WeightInfo;

type Hash = H68;
type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{ensure, traits::Get};
	use pallet_tee_worker::ScheduleFind;
	use pallet_sminer::MinerControl;
	use pallet_oss::OssFindAuthor;
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
		//file updated.
		FileUpdate { acc: AccountOf<T>, fileid: Vec<u8> },

		FileChangeState { acc: AccountOf<T>, fileid: Vec<u8> },
		//Storage information of scheduling storage file slice
		InsertFileSlice { fileid: Vec<u8> },
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

		NodesInsuffcient,
		// This is a bug that is reported only when the most undesirable 
		// situation occurs during a transaction execution process.
		PanicOverflow,

		InsufficientAvailableSpace,
		// The file is in a calculated tag state and cannot be deleted
		Calculate,
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
	#[pallet::getter(fn file_recovery)]
	pub(super) type FileRecovery<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, BoundedVec<[u8; 68], T::RecoverLimit>, ValueQuery>;

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

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

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
			deal_info: SegmentList<T>,
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
				let mut needed_list: SegmentList<T> = Default::default();
				let mut share_info: Vec<SegmentInfo<T>> = Default::default();
				// Check whether there are segments that can be shared.
				for (hash, list) in &deal_info {
					if <SegmentMap<T>>::contains_key(hash) {
						let segment_info = <SegmentMap<T>>::try_get(&hash).map_err(|_| Error::<T>::BugInvalid)?.0;
						share_info.push(segment_info);
					} else {
						needed_list.try_push((*hash, list.clone())).map_err(|_e| Error::<T>::BoundedVecError)?;
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
					let deal_info = Self::generate_deal(file_hash.clone(), needed_list, deal_info, user_brief.clone(), share_info)?;
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
					for (miner, task_list) in &deal_info.assigned_miner {
						let count = task_list.len() as u128;
						T::MinerControl::unlock_space(miner, FRAGMENT_SIZE * count)?;
					}
					Ok(())
				})?;
			} else {
				let deal_info = <DealMap<T>>::try_get(&deal_hash).map_err(|_| Error::<T>::NonExistent)?;
				let needed_space = Self::cal_file_size(deal_info.segment_list.len() as u128);
				T::StorageHandle::unlock_user_space(&deal_info.user.user, needed_space)?;
				// unlock mienr space
				for (miner, task_list) in deal_info.assigned_miner {
					let count = task_list.len() as u128;
					T::MinerControl::unlock_space(&miner, FRAGMENT_SIZE * count)?;
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
			let _ = Self::clear_bucket_file(&file_hash, &sender, &owner_bucket_name)?;
			// TODO! delete
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
						for (miner, _) in &deal_info.assigned_miner {
							task_miner_list.push(miner.clone());
						}
						if task_miner_list.contains(&sender) {
							if !deal_info.complete_list.contains(&sender) {
								deal_info.complete_list.try_push(sender.clone()).map_err(|_| Error::<T>::BoundedVecError)?;
							}
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
			let mut idle_count: u128 = 0;
			for (miner, task_list) in deal_info.assigned_miner {
				let mut count = task_list.len() as u32;
				// Accumulate the number of fragments stored by each miner
				idle_count += count as u128;
				T::MinerControl::unlock_space_to_service(&miner, FRAGMENT_SIZE * count as u128)?;
				// Miners need to report the replaced documents themselves. 
				// If a challenge is triggered before the report is completed temporarily, 
				// these documents to be replaced also need to be verified
				<PendingReplacements<T>>::try_mutate(&miner, |pending_count| -> DispatchResult {
					let pending_count_temp = pending_count.checked_add(count).ok_or(Error::<T>::Overflow)?;
					*pending_count = pending_count_temp;
					Ok(())
				})?;
			}

			let needed_space = Self::cal_file_size(deal_info.segment_list.len() as u128);
			T::StorageHandle::sub_total_idle_space(idle_count * FRAGMENT_SIZE)?;

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
			ensure!(file.stat != FileState::Calculate, Error::<T>::Calculate);

			let mut acc_list: Vec<AccountOf<T>> = Default::default();
			for user_brief in file.owner.into_iter() {
				acc_list.push(user_brief.user);
			}
			ensure!(acc_list.contains(&owner), Error::<T>::NotOwner);

			if acc_list.len() > 1 {
				Self::remove_file_owner(&file_hash, &owner)?;
			} else {
				Self::remove_file_last_owner(&file_hash, &owner)?;
			}

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
			let miner_state = T::MinerControl::get_miner_state(miner.clone())?;
			if !(miner_state == "positive".as_bytes().to_vec()) {
				Err(Error::<T>::NotQualified)?;
			}
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
				let file = <File<T>>::try_get(file_hash).map_err(|_| Error::<T>::Unexpected)?;
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
	}

	impl<T: Config> Pallet<T> {
		pub fn check_file_spec(seg_list: &SegmentList<T>) -> bool {
			let spec_len = T::FragmentCount::get();

			for (_hash, frag_list) in seg_list {
				if frag_list.len() as u32 != spec_len {
					return false
				}
			}

			true 
		}

		pub fn generate_file(
			file_hash: &Hash,
			deal_info: SegmentList<T>,
			miner_task_list: MinerTaskList<T>,
			share_info: Vec<SegmentInfo<T>>,
			user_brief: UserBrief<T>,
			stat: FileState,
		) -> DispatchResult {
			let mut segment_info_list: BoundedVec<SegmentInfo<T>, T::SegmentCount> = Default::default();
			for (hash, frag_list) in deal_info.iter() {
				let mut segment_info = SegmentInfo::<T> {
					hash: *hash,
					fragment_list: Default::default(),
				};

				for share_segment_info in &share_info {
					if hash == &share_segment_info.hash {
						segment_info.fragment_list = share_segment_info.fragment_list.clone().try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
						break;
					}
				}

				for frag_hash in frag_list.iter() {
					for (miner, task_list) in &miner_task_list {
						if task_list.contains(frag_hash) {
							let frag_info = FragmentInfo::<T> {
								hash:  *frag_hash,
								avail: true,
								miner: miner.clone(),
							};
							segment_info.fragment_list.try_push(frag_info).map_err(|_e| Error::<T>::BoundedVecError)?;
						}
					}
				}

				segment_info_list.try_push(segment_info).map_err(|_e| Error::<T>::BoundedVecError)?;
			}

			for segment_info in &segment_info_list {
				if <SegmentMap<T>>::contains_key(segment_info.hash) {
					<SegmentMap<T>>::try_mutate(segment_info.hash, |segment_opt| -> DispatchResult {
						let segment_tuple = segment_opt.as_mut().ok_or(Error::<T>::BugInvalid)?;
						segment_tuple.1 = segment_tuple.1.checked_add(1).ok_or(Error::<T>::Overflow)?;
						Ok(())
					})?;
				} else {
					<SegmentMap<T>>::insert(segment_info.hash, (segment_info, 1));
					T::StorageHandle::add_total_service_space(Self::cal_file_size(1));
				}
			}

			let cur_block = <frame_system::Pallet<T>>::block_number();

			let file_info = FileInfo::<T> {
				completion: cur_block,
				stat: stat,
				segment_list: segment_info_list,
				owner: vec![user_brief].try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
			};

			<File<T>>::insert(file_hash, file_info);

			Ok(())
		}

		pub fn create_bucket_helper(
			user: &AccountOf<T>, 
			bucket_name: &BoundedVec<u8, T::NameStrLimit>, 
			file_hash: Option<Hash>,
		) -> DispatchResult {
			// TODO! len() & ?
			ensure!(bucket_name.len() >= 3, Error::<T>::LessMinLength);
			ensure!(!<Bucket<T>>::contains_key(user, bucket_name), Error::<T>::Existed);

			let mut bucket = BucketInfo::<T> {
				object_list: Default::default(),
				authority: vec![user.clone()].try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
			};

			if let Some(hash) = file_hash {
				bucket.object_list.try_push(hash).map_err(|_e| Error::<T>::BoundedVecError)?;
			}

			<Bucket<T>>::insert(user, bucket_name, bucket);

			Ok(())
		}

		pub fn add_file_to_bucket(
			user: &AccountOf<T>, 
			bucket_name: &BoundedVec<u8, T::NameStrLimit>, 
			file_hash: &Hash,
		) -> DispatchResult {
			<Bucket<T>>::try_mutate(user, bucket_name, |bucket_opt| -> DispatchResult {
				let bucket = bucket_opt.as_mut().ok_or(Error::<T>::NonExistent)?;
				bucket.object_list.try_push(*file_hash).map_err(|_e| Error::<T>::BoundedVecError)?;

				Ok(())
			})
		}

		pub fn generate_deal(
			file_hash: Hash, 
			needed_list: SegmentList<T>, 
			file_info: SegmentList<T>, 
			user_brief: UserBrief<T>,
			share_info: Vec<SegmentInfo<T>>,
		) -> DispatchResult {
			let miner_task_list = Self::random_assign_miner(&needed_list)?;

			Self::start_first_task(file_hash.0.to_vec(), file_hash, 1)?;

			let deal = DealInfo::<T> {
				stage: 1,
				segment_list: file_info,
				needed_list: needed_list,
				user: user_brief,
				assigned_miner: miner_task_list,
				share_info: share_info.try_into().map_err(|_| Error::<T>::BoundedVecError)?,
				complete_list: Default::default(),
			};

			DealMap::insert(&file_hash, deal);

			Ok(())
		}

		pub fn start_first_task(task_id: Vec<u8>, deal_hash: Hash, count: u8) -> DispatchResult {
			let start: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();
			let survival_block = start.checked_add(600 * (count as u32)).ok_or(Error::<T>::Overflow)?;

			T::FScheduler::schedule_named(
					task_id, // TODO!
					DispatchTime::At(survival_block.saturated_into()),
					Option::None,
					schedule::HARD_DEADLINE,
					frame_system::RawOrigin::Root.into(),
					Call::deal_reassign_miner{deal_hash: deal_hash, count: count}.into(), // TODO!
			).map_err(|_| Error::<T>::Unexpected)?;

			Ok(())
		}

		pub fn start_second_task(task_id: Vec<u8>, deal_hash: Hash, count: u8) -> DispatchResult {
			let start: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();
			let survival_block = start.checked_add(600 * (count as u32)).ok_or(Error::<T>::Overflow)?;

			T::FScheduler::schedule_named(
					task_id, // TODO!
					DispatchTime::At(survival_block.saturated_into()),
					Option::None,
					schedule::HARD_DEADLINE,
					frame_system::RawOrigin::Root.into(),
					Call::calculate_end{deal_hash: deal_hash}.into(), // TODO!
			).map_err(|_| Error::<T>::Unexpected)?;

			Ok(())
		}

		pub fn random_assign_miner(needed_list: &SegmentList<T>) -> Result<MinerTaskList<T>, DispatchError> {
			let mut index_list: Vec<u32> = Default::default();
			let mut miner_task_list: MinerTaskList<T> = Default::default();
			let mut miner_idle_space_list: Vec<u128> = Default::default();
			// The optimal number of miners required for storage.
			// segment_size * 1.5 / fragment_size.
			let miner_count: u32 = (SEGMENT_SIZE * 15 / 10 / FRAGMENT_SIZE) as u32;
			let mut seed = <frame_system::Pallet<T>>::block_number().saturated_into();

			let all_miner = T::MinerControl::get_all_miner()?;
			let total = all_miner.len() as u32;

			ensure!(total > miner_count, Error::<T>::NodesInsuffcient);
			// Maximum number of cycles set to prevent dead cycles TODO!
			let max_count = miner_count * 5;
			let mut cur_count = 0;
			let mut total_idle_space = 0;
			// start random choose miner
			loop {
				// Get a random subscript.
				let index = Self::generate_random_number(seed)? as u32 % total;
				// seed + 1
				seed = seed.checked_add(1).ok_or(Error::<T>::Overflow)?;
				// Number of cycles plus 1
				cur_count += 1;
				// When the number of cycles reaches the upper limit, the cycle ends.
				if cur_count == max_count {
					break;
				}
				// End the cycle after all storage nodes have been traversed.
				if total == index_list.len() as u32 {
					break;
				}
				// Continue to the next cycle when the current random result already exists.
				if index_list.contains(&index) {
					continue;
				}
				// Record current cycle results.
				index_list.push(index);
				// Judge whether the idle space of the miners is sufficient.
				let miner = all_miner[index as usize].clone();
				let cur_space: u128 = T::MinerControl::get_miner_idle_space(&miner)?;
				// If sufficient, the miner is selected.
				if cur_space > needed_list.len() as u128 * FRAGMENT_SIZE {
					// Accumulate all idle space of currently selected miners
					total_idle_space = total_idle_space.checked_add(&cur_space).ok_or(Error::<T>::Overflow)?;
					miner_task_list.try_push((miner, Default::default())).map_err(|_e| Error::<T>::BoundedVecError)?;
					miner_idle_space_list.push(cur_space);
				}
				// If the selected number of miners has reached the optimal number, the cycle ends.
				if miner_task_list.len() as u32 == miner_count {
					break;
				}
			}
			
			ensure!(miner_task_list.len() != 0, Error::<T>::BugInvalid);
			ensure!(total_idle_space > SEGMENT_SIZE * 15 / 10, Error::<T>::NodesInsuffcient);

			// According to the selected miner.
			// Assign responsible documents to miners.
			for (_hash, frag_list) in needed_list {
				let mut index = 0;
				for hash in frag_list {
					// To prevent the number of miners from not meeting the fragment number.
					// It may occur that one miner stores multiple fragments
					loop {
						// Obtain the account of the storage node through the subscript.
						// To prevent exceeding the boundary, use '%'.
						let temp_index = index % miner_task_list.len();
						let cur_space = miner_idle_space_list[temp_index];
						// To prevent a miner from storing multiple fragments, 
						// the idle space is insufficient
						if cur_space > (miner_task_list[temp_index].1.len() as u128 + 1) * FRAGMENT_SIZE {
							miner_task_list[temp_index].1.try_push(*hash).map_err(|_e| Error::<T>::BoundedVecError)?;
							break;
						}
						index = index.checked_add(1).ok_or(Error::<T>::PanicOverflow)?;
					}
					index = index.checked_add(1).ok_or(Error::<T>::PanicOverflow)?;
				}
			}
			// lock miner space
			for (acc, task_hash_list) in miner_task_list.iter() {
				T::MinerControl::lock_space(acc, task_hash_list.len() as u128 * FRAGMENT_SIZE)?;
			}
			 
			
			Ok(miner_task_list)
		}

		fn cal_file_size(len: u128) -> u128 {
			len * (SEGMENT_SIZE * 15 / 10)
		}

		// The status of the file must be confirmed before use.
		fn remove_file_owner(file_hash: &Hash, acc: &AccountOf<T>) -> DispatchResult {
			<File<T>>::try_mutate(file_hash, |file_opt| -> DispatchResult {
				let file = file_opt.as_mut().ok_or(Error::<T>::Overflow)?;
				for (index, user_brief) in file.owner.iter().enumerate() {
					if acc == &user_brief.user {
						let file_size = Self::cal_file_size(file.segment_list.len() as u128);
						T::StorageHandle::update_user_space(acc, 2, file_size)?;
						file.owner.remove(index);
						break;
					}
				}
				Ok(())
			})?;

			Ok(())
		}

		// The status of the file must be confirmed before use.
		fn remove_file_last_owner(file_hash: &Hash, acc: &AccountOf<T>) -> DispatchResult {
			let file = <File<T>>::try_get(file_hash).map_err(|_| Error::<T>::NonExistent)?;
			// Record the total number of fragments that need to be deleted.
			let mut total_fragment_dec = 0;
			// Used to record and store the amount of service space that miners need to reduce, 
			// and read changes once through counting
			let mut miner_list: BTreeMap<AccountOf<T>, u32> = Default::default();
			// Traverse every segment
			for segment_info in file.segment_list.iter() {
				let flag = <SegmentMap<T>>::try_mutate(segment_info.hash, |segment_opt| -> Result<bool, DispatchError> {
					let (segment_info, count) = segment_opt.as_mut().ok_or(Error::<T>::BugInvalid)?;
					// Determine whether the segment is shared
					if *count > 1 {
						*count = count.checked_sub(1).ok_or(Error::<T>::Overflow)?;
					} else {
						for fragment_info in segment_info.fragment_list.iter() {
							// The total number of fragments in a file should never exceed u32
							total_fragment_dec += 1;
							if miner_list.contains_key(&fragment_info.miner) {
								let temp_count = miner_list.get_mut(&fragment_info.miner).ok_or(Error::<T>::BugInvalid)?;
								// The total number of fragments in a file should never exceed u32
								*temp_count += 1;
							} else {
								miner_list.insert(fragment_info.miner.clone(), 1);
							}
						}
						return Ok(true);
					}
					Ok(false)
				})?;

				if flag {
					<SegmentMap<T>>::remove(segment_info.hash);
				}
			}

			for (miner, count) in miner_list.iter() {
				T::MinerControl::sub_miner_service_space(miner, FRAGMENT_SIZE * *count as u128)?;
			}

			let file_size = Self::cal_file_size(file.segment_list.len() as u128);
			T::StorageHandle::update_user_space(acc, 2, file_size);
			T::StorageHandle::sub_total_service_space(total_fragment_dec as u128 * FRAGMENT_SIZE)?;

			<File<T>>::remove(file_hash);

			Ok(())
		}
		/// helper: generate random number.
		///
		/// Get a random number.
		///
		/// Parameters:
		/// - `seed`: random seed.
		/// Result:
		/// - `u32`: random number.
		pub fn generate_random_number(seed: u32) -> Result<u32, DispatchError> {
			let mut counter = 0;
			loop {
				let (random_seed, _) =
					T::MyRandomness::random(&(T::FilbakPalletId::get(), seed + counter).encode());
				let random_seed = match random_seed {
					Some(v) => v,
					None => Default::default(),
				};
				let random_number = <u32>::decode(&mut random_seed.as_ref()).unwrap_or(0);
				if random_number != 0 {
					return Ok(random_number)
				}
				counter = counter.checked_add(1).ok_or(Error::<T>::Overflow)?;
			}
		}
		/// helper: delete filler.
		///
		/// delete filler.
		///
		/// Parameters:
		/// - `miner_acc`: miner AccountId.
		/// - `filler_hash`: filler hash.
		/// Result:
		/// - DispatchResult
		pub fn delete_filler(miner_acc: AccountOf<T>, filler_hash: Hash) -> DispatchResult {
			if !<FillerMap<T>>::contains_key(&miner_acc, filler_hash.clone()) {
				Err(Error::<T>::FileNonExistent)?;
			}
			let value = <FillerMap<T>>::try_get(&miner_acc, filler_hash.clone()) //read 1
				.map_err(|_e| Error::<T>::FileNonExistent)?;
			<FillerMap<T>>::remove(miner_acc, filler_hash.clone()); //write 1

			Ok(())
		}

		pub fn clear_bucket_file(
			file_hash: &Hash,
			owner: &AccountOf<T>,
			bucket_name: &BoundedVec<u8, T::NameStrLimit>,
		) -> Result<Weight, DispatchError> {
			let mut weight: Weight = Weight::from_ref_time(0);
			ensure!(<Bucket<T>>::contains_key(owner, bucket_name), Error::<T>::NonExistent);

			<Bucket<T>>::try_mutate(owner, bucket_name, |bucket_opt| -> DispatchResult {
				let bucket = bucket_opt.as_mut().ok_or(Error::<T>::Unexpected)?;
				ensure!(bucket.object_list.contains(file_hash), Error::<T>::NonExistent);
				let mut index: usize = 0;
				for object in bucket.object_list.iter() {
					if object == file_hash {
						break;
					}
					index = index.checked_add(1).ok_or(Error::<T>::Overflow)?;
				}
				bucket.object_list.remove(index);
				Ok(())
			})?;
			weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

			Ok(weight)
		}
		/// helper: add user hold fileslice.
		///
		/// Add files held by users.
		///
		/// Parameters:
		/// - `user`: AccountId.
		/// - `file_hash_bound`: file hash.
		/// - `file_size`: file size.
		///
		/// Result:
		/// - DispatchResult
		fn add_user_hold_fileslice(
			user: &AccountOf<T>,
			file_hash: Hash,
			file_size: u128,
		) -> DispatchResult {
			let file_info =
				UserFileSliceInfo { file_hash: file_hash, file_size };
			<UserHoldFileList<T>>::try_mutate(user, |v| -> DispatchResult {
				v.try_push(file_info).map_err(|_| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;

			Ok(())
		}
		/// helper: get current scheduler.
		///
		/// Get the current block consensus.
		///
		/// Parameters:
		///
		/// Result:
		/// - AccountOf: consensus
		fn get_current_scheduler() -> Result<AccountOf<T>, DispatchError> {
			let digest = <frame_system::Pallet<T>>::digest();
			let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
			let acc = T::FindAuthor::find_author(pre_runtime_digests).map(|a| a);
			let acc = match acc {
				Some(e) => T::Scheduler::get_controller_acc(e),
				None => T::Scheduler::get_first_controller()?,
			};
			Ok(acc)
		}
		/// helper: check_is_file_owner.
		///
		/// Check whether the user is the owner of the file.
		///
		/// Parameters:
		///
		/// Result:
		/// - acc: Inspected user.
		/// - file_hash: File hash, the unique identifier of the file.
		pub fn check_is_file_owner(acc: &AccountOf<T>, file_hash: &Hash) -> bool {
			if let Some(file) = <File<T>>::get(file_hash) {
				for user_brief in file.owner.iter() {
					if &user_brief.user == acc {
						return true;
					}
				}
			}
			false
		}
		/// helper: Permission check method.
		/// Check whether the origin has the owner's authorization
		/// or whether the origin is the owner
		///
		/// Parameters:
		/// - `acc`: AccountId.
		///
		/// Result:
		/// - bool: True means there is permission, false means there is no permission.
		fn check_permission(operator: AccountOf<T>, owner: AccountOf<T>) -> bool {
			if owner == operator || T::OssFindAuthor::is_authorized(owner, operator) {
				return true;
			}
			false
		}

		fn record_uploaded_files_size(scheduler_id: &T::AccountId, file_size: u64) -> DispatchResult {
			T::CreditCounter::record_proceed_block_size(scheduler_id, file_size)?;
			Ok(())
		}

		fn record_uploaded_fillers_size(scheduler_id: &T::AccountId, fillers: &Vec<FillerInfo<T>>) -> DispatchResult {
			for filler in fillers {
				T::CreditCounter::record_proceed_block_size(scheduler_id, filler.filler_size)?;
			}
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
	fn clear_file(file_hash: Hash) -> Result<Weight, DispatchError>;
	//The function executed when the challenge fails, allowing the miner to delete invalid files
	fn add_recovery_file(file_id: [u8; 68]) -> DispatchResult;
	//The function executed when the challenge fails to let the consensus schedule recover the file
	fn add_invalid_file(miner_acc: AccountId, file_hash: Hash) -> DispatchResult;
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
		for (_, value) in FillerMap::<T>::iter_prefix(&miner_acc) {
			weight = weight.saturating_add(T::DbWeight::get().writes(1 as u64));
		}
		#[allow(deprecated)]
		let _ = FillerMap::<T>::remove_prefix(&miner_acc, Option::None);
		weight = weight.saturating_add(T::DbWeight::get().writes(1 as u64));
		Ok(weight)
	}

	fn clear_file(file_hash: Hash) -> Result<Weight, DispatchError> {
		let weight: Weight = Weight::from_ref_time(0);
		Ok(weight)
	}

	fn add_recovery_file(file_id: [u8; 68]) -> DispatchResult {
		Pallet::<T>::add_recovery_file(file_id)?;
		Ok(())
	}

	fn add_invalid_file(miner_acc: AccountOf<T>, file_hash: Hash) -> DispatchResult {
		Pallet::<T>::add_invalid_file(miner_acc, file_hash)?;
		Ok(())
	}
}

impl<T: Config> BlockNumberProvider for Pallet<T> {
	type BlockNumber = T::BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		<frame_system::Pallet<T>>::block_number()
	}
}
