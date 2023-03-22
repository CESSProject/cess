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
	schedule::{DispatchTime, Named as ScheduleNamed},
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
	dispatch::DispatchResult, pallet_prelude::*, transactional, weights::Weight, PalletId,
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
use sp_std::{convert::TryInto, prelude::*, str};

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

		type SScheduler: ScheduleNamed<Self::BlockNumber, Self::SProposal, Self::SPalletsOrigin>;
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
		UploadDeclaration { operator: AccountOf<T>, owner: AccountOf<T>, file_hash: Hash, file_name: Vec<u8> },
		//file uploaded.
		FileUpload { acc: AccountOf<T> },
		//file updated.
		FileUpdate { acc: AccountOf<T>, fileid: Vec<u8> },

		FileChangeState { acc: AccountOf<T>, fileid: Vec<u8> },
		//Storage information of scheduling storage file slice
		InsertFileSlice { fileid: Vec<u8> },
		//File deletion event
		DeleteFile { operator:AccountOf<T>, owner: AccountOf<T>, file_hash_list: Vec<Hash>, failed_list: Vec<Hash> },
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
	}

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
	#[pallet::getter(fn invalid_file)]
	pub(super) type InvalidFile<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, BoundedVec<Hash, T::InvalidLimit>, ValueQuery>;

	// Stores the hash of the entire network segment and the sharing of each segment.
	// Used to determine whether segments can be shared.
	#[pallet::storage]
	#[pallet::getter(fn segment_map)]
	pub(super) type SegmentMap<T: Config> = StorageMap<_, Blake2_128Concat, Hash, u32>;

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
		#[pallet::call_index(0)]
		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn clear_all_filler(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let mut x = 0;
			for (_, value) in FillerMap::<T>::iter_prefix(&sender) {
				<FillerKeysMap<T>>::remove(value.index);
				x += 1;
			}

			#[warn(deprecated)]
			let _ = FillerMap::<T>::remove_prefix(&sender, Option::None);

			let _ = T::MinerControl::sub_miner_idle_space(&sender, x * 8 * 1024 * 1024)?;
			T::StorageHandle::sub_total_idle_space(x * 8 * 1024 * 1024)?;
			Ok(())
		}
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
		#[pallet::call_index(1)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upload_declaration())]
		pub fn upload_declaration(
			origin: OriginFor<T>,
			file_hash: Hash,
			file_info: SegmentList<T>,
			user_brief: UserBrief<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			// Check if you have operation permissions.
			ensure!(Self::check_permission(sender.clone(), user_brief.user.clone()), Error::<T>::NoPermission);
			// Check file specifications.
			ensure!(Self::check_file_spec(&file_info), Error::<T>::SpecError);
			// Check whether the user-defined name meets the rules.
			let minimum = T::NameMinLength::get();
			ensure!(user_brief.file_name >= minimum, Error::<T>::SpecError);
			ensure!(user_brief.bucket_name >= minimum, Error::<T>::SpecError);
			
			if <File<T>>::contains_key(&file_hash) {

			} else {
				// Check whether the user's storage space is sufficient, 
				// if sufficient lock user's storage space.
				// Perform space calculations based on 1.5 times redundancy.
				T::StorageHandle::lock_user_space(&user_brief.user, file_info.len() as u128 * (SEGMENT_SIZE * 10 / 5))?;

				let mut needed_list: SegmentList<T> = Default::default();
				let mut counter = 0;
				// Check whether there are segments that can be shared.
				for (hash, list) in &file_info {
					if <SegmentMap<T>>::contains_key(hash) {
						counter += 1;
						<SegmentMap<T>>::try_mutate(hash, |share_count_opt| -> DispatchResult {
							let share_count = share_count_opt.ok_or(Error::<T>::BugInvalid)?;
							share_count.checked_add(1).ok_or(Error::<T>::Overflow)?;
							Ok(())
						})?;
					} else {
						needed_list.push(*list);
					}
				}

				if counter == file_info.len() {

				} else {
					Self::generate_deal()
				}

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

			ensure!(file.file_state.to_vec() == FILE_ACTIVE.as_bytes().to_vec(), Error::<T>::Unprepared);
			ensure!(<Bucket<T>>::contains_key(&target_brief.user, &target_brief.bucket_name), Error::<T>::NonExistent);
			//Modify the space usage of target acc,
			//and determine whether the space is enough to support transfer
			T::StorageHandle::update_user_space(&target_brief.user, 1, file.file_size.into())?;
			//Increase the ownership of the file for target acc
			<File<T>>::try_mutate(&file_hash, |file_opt| -> DispatchResult {
				let file = file_opt.as_mut().ok_or(Error::<T>::FileNonExistent)?;
				file.user_brief_list.try_push(target_brief.clone()).map_err(|_| Error::<T>::BoundedVecError)?;
				Ok(())
			})?;
			//Add files to the bucket of target acc
			<Bucket<T>>::try_mutate(
				&target_brief.user,
				&target_brief.bucket_name,
				|bucket_info_opt| -> DispatchResult {
					let bucket_info = bucket_info_opt.as_mut().ok_or(Error::<T>::NonExistent)?;
					bucket_info.object_num = bucket_info.object_num.checked_add(1).ok_or(Error::<T>::Overflow)?;
					bucket_info.object_list.try_push(file_hash.clone()).map_err(|_| Error::<T>::LengthExceedsLimit)?;
					Ok(())
			})?;
			//Increase the corresponding space usage for target acc
			Self::add_user_hold_fileslice(
				target_brief.user.clone(),
				file_hash.clone(),
				file.file_size.into(),
			)?;
			//Clean up the file holding information of the original user
			let _ = Self::clear_bucket_file(&file_hash, &sender, &owner_bucket_name)?;
			let _ = Self::clear_user_file(file_hash.clone(), &sender, true)?;

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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upload(slice_info.len() as u32))]
		pub fn upload(
			origin: OriginFor<T>,
			file_hash: Hash,
			file_size: u64,
			slice_info: Vec<SliceInfo<T>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(
				T::Scheduler::contains_scheduler(sender.clone()),
				Error::<T>::ScheduleNonExistent
			);
			ensure!(<File<T>>::contains_key(&file_hash), Error::<T>::FileNonExistent);

			<File<T>>::try_mutate(&file_hash, |s_opt| -> DispatchResult {
				let s = s_opt.as_mut().ok_or(Error::<T>::FileNonExistent)?;
				for user_brief in s.user_brief_list.iter() {
					T::StorageHandle::update_user_space(&user_brief.user, 1, file_size.into())?;
					Self::add_user_hold_fileslice(
						user_brief.user.clone(),
						file_hash.clone(),
						file_size,
					)?;
				}
				if s.file_state.to_vec() == FILE_ACTIVE.as_bytes().to_vec() {
					Err(Error::<T>::FileExistent)?;
				}
				s.file_size = file_size;
				s.slice_info =
					slice_info.clone().try_into().map_err(|_| Error::<T>::BoundedVecError)?;
				s.file_state = FILE_ACTIVE
					.as_bytes()
					.to_vec()
					.try_into()
					.map_err(|_| Error::<T>::BoundedVecError)?;
				if !<FileKeysMap<T>>::contains_key(s.index) {
					<FileKeysMap<T>>::insert(s.index, (true, file_hash.clone()));
				}
				Ok(())
			})?;

			//To be tested
			for v in slice_info.iter() {
				Self::replace_file(v.miner_acc.clone(), v.shard_size)?;
			}

			Self::record_uploaded_files_size(&sender, file_size)?;

			Self::deposit_event(Event::<T>::FileUpload { acc: sender.clone() });
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
		#[pallet::call_index(4)]
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
			let mut count: u32 = <FillerIndexCount<T>>::get();
			for i in filler_list.iter() {
				count = count.checked_add(1).ok_or(Error::<T>::Overflow)?;
				if <FillerMap<T>>::contains_key(&miner, i.filler_hash.clone()) {
					Err(Error::<T>::FileExistent)?;
				}
				let mut value = i.clone();
				value.index = count;
				<FillerMap<T>>::insert(miner.clone(), i.filler_hash.clone(), value);
				<FillerKeysMap<T>>::insert(count, (miner.clone(), i.filler_hash.clone()));
			}
			<FillerIndexCount<T>>::put(count);

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
		/// User deletes uploaded files.
		///
		/// The dispatch origin of this call must be Signed.
		///
		/// If a file has multiple holders,
		/// only the user's ownership of the file will be deleted
		/// and the space purchased by the user will be released
		///
		/// Parameters:
		/// - `fileid`: For which miner, miner's wallet address.
		#[pallet::call_index(5)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::delete_file())]
		pub fn delete_file(origin: OriginFor<T>, owner: AccountOf<T>, file_hash_list: Vec<Hash>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let mut delete_failed_list: Vec<Hash> = Default::default();
			for file_hash in file_hash_list.iter() {
				if !<File<T>>::contains_key(file_hash) {
					delete_failed_list.push(*file_hash);
					continue;
				}
				if !Self::check_permission(sender.clone(), owner.clone()) {
					delete_failed_list.push(*file_hash);
					continue;
				}

				let file = match <File<T>>::try_get(file_hash) {
					Ok(file) => file,
					Err(_e) => {
						delete_failed_list.push(*file_hash);
						continue;
					},
				}; //read 1

				let mut result: bool = false;
				for user_brief_temp in file.user_brief_list.iter() {
					if user_brief_temp.user == owner.clone() {
						//The above has been judged. Unwrap will be performed only if the key exists
						let _ = match Self::clear_bucket_file(file_hash, &owner, &user_brief_temp.bucket_name) {
							Ok(weight) => weight,
							Err(_e) => {
								delete_failed_list.push(*file_hash);
								continue;
							},
						};

						let _ = match Self::clear_user_file(*file_hash, &owner, file.user_brief_list.len() > 1){
							Ok(weight) => weight,
							Err(_e) => {
								delete_failed_list.push(*file_hash);
								continue;
							},
						};

						result = true;
						break;
					}
				}
				if !result {
					delete_failed_list.push(*file_hash);
					continue;
				}
			}		
			Self::deposit_event(Event::<T>::DeleteFile { operator: sender, owner, file_hash_list: file_hash_list,  failed_list: delete_failed_list });
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
		/// Recover files.
		///
		/// The dispatch origin of this call must be Signed.
		///
		/// When one or more slices of the service file have problems,
		/// the file needs to be scheduled for recovery.
		/// When more than one third of the contents of the file are damaged,
		/// the file cannot be recovered.
		///
		/// Parameters:
		/// - `shard_id`: Corrupt file slice id.
		/// - `slice_info`: New slice information.
		/// - `avail`: Whether the file can be recovered normally.
		#[pallet::call_index(10)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::recover_file())]
		pub fn recover_file(
			origin: OriginFor<T>,
			shard_id: [u8; 68],
			slice_info: SliceInfo<T>,
			avail: bool,
		) -> DispatchResult {
			//Vec to BoundedVec
			let sender = ensure_signed(origin)?;
			//Get fileid from shardid,
			let file_hash = Hash::from_shard_id(&shard_id).map_err(|_| Error::<T>::ConvertHashError)?;
			//Delete the corresponding recovery slice request pool
			<FileRecovery<T>>::try_mutate(&sender, |o| -> DispatchResult {
				o.retain(|x| *x != shard_id);
				Ok(())
			})?;
			if !<File<T>>::contains_key(&file_hash) {
				Err(Error::<T>::FileNonExistent)?;
			}
			if avail {
				<File<T>>::try_mutate(&file_hash, |opt| -> DispatchResult {
					let o = opt.as_mut().unwrap();
					o.slice_info
						.try_push(slice_info)
						.map_err(|_| Error::<T>::StorageLimitReached)?;
					o.file_state = FILE_ACTIVE
						.as_bytes()
						.to_vec()
						.try_into()
						.map_err(|_e| Error::<T>::BoundedVecError)?;
					Ok(())
				})?;
			} else {
				let _weight = Self::clear_file(file_hash)?;
			}
			Self::deposit_event(Event::<T>::RecoverFile { acc: sender, file_hash: shard_id });
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
				total_capacity: 0,
				available_capacity: 0,
				object_num: 0,
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
				Self::clear_user_file(*file_hash, &owner, file.user_brief_list.len() > 1)?;
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

		pub fn generate_deal(
			file_hash: Hash, 
			needed_list: SegmentList<T>, 
			file_info: SegmentList<T>, 
			user_brief: UserBrief<T>,
		) -> Result<DealInfo<T>, DispatchError> {
			let task_id: Vec<u8> = file_hash.0.to_vec(); // TODO!
			Self::start_first_task(task_id.clone())?;

			let miner_task_list = Self::random_assign_miner(needed_list)?;

			DealInfo::<T> {
				stage: 1,
				count: 0,
				time_task: task_id,
				segment_list: file_info,
				user: user_brief,
				assigned_miner: miner_task_list,
			}
		}

		pub fn start_first_task(task_id: Vec<u8>) -> DispatchResult {
			let start: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();
			let survival_block = start.checked_add(600 * file_info.len()).ok_or(Error::<T>::Overflow)?;

			T::SScheduler::schedule_named(
					task_id, // TODO!
					DispatchTime::At(survival_block.saturated_into()),
					Option::None,
					schedule::HARD_DEADLINE,
					frame_system::RawOrigin::Root.into(),
					Call::reassign_deal{file_hash: file_hash.clone()}.into(), // TODO!
			).map_err(|_| Error::<T>::Unexpected)?;

			Ok(())
		}

		pub fn random_assign_miner(needed_list: Vec<Hash>) -> Result<MinerTaskList<T>, DispatchError> {
			let mut index_list: Vec<u32> = Default::default();
			let mut miner_task_list: MinerTaskList<T> = Default::default();
			// Number of miners required
			let miner_count = SEGMENT_SIZE * 15 / 10 / FRAGMENT_SIZE;
			let mut seed = 0;

			let all_miner = T::MinerControl::get_all_miner()?;

			loop {
				let index = Self::generate_random_number(seed)? % miner_count;
				seed = seed.checked_add(1).ok_or(Error::<T>::Overflow)?;
				if index_list::contains(index) {
					continue;
				}
				index_list.push(index);
				if index_list.len() == miner_count {
					for (m_index, hash) in all_miner.enumerate() {
						if index_list::contains(m_index) {

						}
					}
					break;
				}
			}

			miner_task_list.push((*hash, Default::default()));
	
			enuser!(miner_task_list.len() == miner_count, Error::<T>::BugInvalid);
			for frag_list in needed_list {
				let mut index = 0;
				for hash in frag_list {
					miner_task_list[index].1.try_push(*hash).map_err(|_e| Error::<T>::BoundedVecError)?;
					index += 1;
				}
			}
		
			// lock miner space
			let lock_space: u128 =  (needed_list.len() * FRAGMENT_SIZE).into();
			for 
			
			Ok(miner_task_list)
		}
		/// helper: Generate random challenge data.
		///
		/// Generate random challenge data, package and return.
		/// Extract 4.6% of idle documents,
		/// and then extract 4.6% of service documents
		///
		/// Parameters:
		///
		/// Result:
		/// - `Vec<(AccountOf<T>, Vec<u8>, Vec<u8>, u64, u8)>`: Returns a tuple.
		/// - `tuple.0`: miner AccountId.
		/// - `tuple.1`: filler id or shard id
		/// - `tuple.2`: List of extracted blocks.
		/// - `tuple.3`: file size or shard size.
		/// - `tuple.4`: file type 1 or 2, 1 is file slice, 2 is filler.
		pub fn get_random_challenge_data(
		) -> Result<Vec<(AccountOf<T>, Hash, [u8; 68], Vec<u32>, u64, DataType)>, DispatchError> {
			log::info!("get random filler...");
			let filler_list = Self::get_random_filler()?;
			log::info!("get random filler success");
			let mut data: Vec<(AccountOf<T>, Hash, [u8; 68], Vec<u32>, u64, DataType)> = Vec::new();
			log::info!("storage filler data...");
			for v in filler_list {
				let length = v.block_num;
				let number_list = Self::get_random_numberlist(length, 3, length)?;
				let miner_acc = v.miner_address.clone();
				let filler_hash = v.filler_hash.clone();
				let file_size = v.filler_size.clone();
				let mut block_list: Vec<u32> = Vec::new();
				for i in number_list.iter() {
					block_list.push(*i as u32);
				}
				data.push((miner_acc, filler_hash, [0u8; 68], block_list, file_size, DataType::Filler));
			}
			log::info!("storage filler success!");
			log::info!("get file data...");
			let file_list = Self::get_random_file()?;
			log::info!("get file data success!");
			log::info!("storage filler data...");
			for (_, file) in file_list {
				let slice_number_list = Self::get_random_numberlist(
					file.slice_info.len() as u32,
					3,
					file.slice_info.len() as u32,
				)?;
				for slice_index in slice_number_list.iter() {
					let index_temp = *slice_index - 1;
					let miner_id = T::MinerControl::get_miner_id(
						file.slice_info[index_temp as usize].miner_acc.clone(),
					)?;
					if file.slice_info[index_temp as usize].miner_id != miner_id {
						continue
					}
					let mut block_list: Vec<u32> = Vec::new();
					let length = file.slice_info[index_temp as usize].block_num;
					let number_list = Self::get_random_numberlist(length, 3, length)?;
					let file_hash = Hash::from_shard_id(&file.slice_info[index_temp as usize].shard_id).map_err(|_| Error::<T>::ConvertHashError)?;
					let miner_acc = file.slice_info[index_temp as usize].miner_acc.clone();
					let slice_size = file.slice_info[index_temp as usize].shard_size;
					for i in number_list.iter() {
						block_list.push(*i as u32);
					}
					data.push((miner_acc, file_hash, file.slice_info[index_temp as usize].shard_id, block_list, slice_size, DataType::File));
				}
				log::info!("storage filler success!");
			}

			Ok(data)
		}
		/// helper: get random filler.
		///
		/// Extract 4.6% of idle documents,
		///
		/// Parameters:
		///
		/// Result:
		/// - `Vec<FillerInfo<T>>`: Fill file information list.
		fn get_random_filler() -> Result<Vec<FillerInfo<T>>, DispatchError> {
			let length = Self::get_fillermap_length()?;
			log::info!("filler length: {}", length);
			let limit = <FillerIndexCount<T>>::get();
			log::info!("filler limit: {}", limit);
			log::info!("get filler random data...");
			let number_list = Self::get_random_numberlist(length, 1, limit)?;
			log::info!("get filler random data success");
			let mut filler_list: Vec<FillerInfo<T>> = Vec::new();
			for i in number_list.iter() {
				let result = <FillerKeysMap<T>>::get(i);
				let (acc, filler_hash) = match result {
					Some(x) => x,
					None => Err(Error::<T>::BugInvalid)?,
				};
				let filler =
					<FillerMap<T>>::try_get(acc, filler_hash).map_err(|_e| Error::<T>::BugInvalid)?;
				filler_list.push(filler);
			}
			Ok(filler_list)
		}
		/// helper: get random file.
		///
		/// Extract 4.6% of service documents,
		///
		/// Parameters:
		///
		/// Result:
		/// - `Vec<(BoundedString<T>, FileInfo<T>)>`: Fill file information list.
		fn get_random_file() -> Result<Vec<(Hash, FileInfo<T>)>, DispatchError> {
			let length = Self::get_tee_worker_length()?;
			log::info!("file length: {}", length);
			let limit = <FileIndexCount<T>>::get();
			log::info!("file limit: {}", limit);
			log::info!("file random number start generate...");
			let number_list = Self::get_random_numberlist(length, 2, limit)?;
			log::info!("file random number generate success!");
			let mut file_list: Vec<(Hash, FileInfo<T>)> = Vec::new();
			for i in number_list.iter() {
				let file_id =
					<FileKeysMap<T>>::try_get(i).map_err(|_e| Error::<T>::FileNonExistent)?.1;
				let value =
					<File<T>>::try_get(&file_id).map_err(|_e| Error::<T>::FileNonExistent)?;
				if value.file_state.to_vec() == FILE_ACTIVE.as_bytes().to_vec() {
					file_list.push((file_id.clone(), value));
				}
			}
			Ok(file_list)
		}
		/// helper: get random number list.
		///
		/// Method to get random subscript array.
		///
		/// Parameters:
		/// - `length`: Total number of randomly extracted files.
		/// - `random_type`: Random type.
		/// - `limit`: What is the upper limit of the acquisition range.
		/// Result:
		/// - `Vec<u32>`: random number list.
		fn get_random_numberlist(
			length: u32,
			random_type: u8,
			limit: u32,
		) -> Result<Vec<u32>, DispatchError> {
			log::info!("random number generate start...");
			let mut seed: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();
			if length == 0 {
				return Ok(Vec::new())
			}
			let num = match random_type {
				1 => length
					.checked_mul(46)
					.ok_or(Error::<T>::Overflow)?
					.checked_div(1000)
					.ok_or(Error::<T>::Overflow)?
					.checked_add(1)
					.ok_or(Error::<T>::Overflow)?,
				2 => length
					.checked_mul(46)
					.ok_or(Error::<T>::Overflow)?
					.checked_div(1000)
					.ok_or(Error::<T>::Overflow)?
					.checked_add(1)
					.ok_or(Error::<T>::Overflow)?,
				_ => length
					.checked_mul(46)
					.ok_or(Error::<T>::Overflow)?
					.checked_div(1000)
					.ok_or(Error::<T>::Overflow)?
					.checked_add(1)
					.ok_or(Error::<T>::Overflow)?,
			};
			log::info!("num is: {}", num);
			let mut number_list: Vec<u32> = Vec::new();
			log::info!("goto in loop...");
			loop {
				seed = seed.checked_add(1).ok_or(Error::<T>::Overflow)?;
				log::info!("seed is: {}, list len: {}, num is: {}, length: {}, limit: {}", seed, number_list.len(), num, length, limit);
				if number_list.len() >= num as usize {
					number_list.sort();
					number_list.dedup();
					if number_list.len() >= num as usize {
						break
					}
				}

				let random = Self::generate_random_number(seed)? % limit + 1;
				let result: bool = match random_type {
					1 => Self::judge_filler_exist(random),
					2 => Self::judge_file_exist(random),
					_ => true,
				};
				log::info!("random is: {}, is exist: {}", random, result);
				if !result {
					//Start the next cycle if the file does not exist
					continue
				}
				number_list.push(random);
			}
			Ok(number_list)
		}
		/// helper: judge_file_exist.
		///
		/// Help method for generating random files.
		/// Used to determine whether the file of the subscript exists.
		///
		/// Parameters:
		/// - `index`: file index.
		/// Result:
		/// - `bool`: True represents existence, False represents unexistence.
		fn judge_file_exist(index: u32) -> bool {
			let result = <FileKeysMap<T>>::get(index);
			let result = match result {
				Some(x) => x.0,
				None => false,
			};
			result
		}
		/// helper: judge filler exist.
		///
		/// Help method for generating random fillers.
		/// Used to determine whether the filler of the subscript exists.
		///
		/// Parameters:
		/// - `index`: filler index.
		/// Result:
		/// - `bool`: True represents existence, False represents unexistence.
		fn judge_filler_exist(index: u32) -> bool {
			let result = <FillerKeysMap<T>>::get(index);
			let result = match result {
				Some(_x) => true,
				None => false,
			};
			result
		}
		/// helper: get fillermap length.
		///
		/// Gets the length of the padding filler.
		///
		/// Parameters:
		/// - `index`: filler index.
		/// Result:
		/// - `u32`: Length of the whole network filling file.
		fn get_fillermap_length() -> Result<u32, DispatchError> {
			let count = <FillerKeysMap<T>>::count();
			Ok(count)
		}
		/// helper: get TeeWorker length.
		///
		/// Gets the length of the padding file.
		///
		/// Parameters:
		/// - `index`: filler index.
		/// Result:
		/// - `bool`: Length of the whole network service file.
		fn get_tee_worker_length() -> Result<u32, DispatchError> {
			Ok(<FileKeysMap<T>>::count())
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
			<FillerKeysMap<T>>::remove(value.index); //write 1
			<FillerMap<T>>::remove(miner_acc, filler_hash.clone()); //write 1

			Ok(())
		}
		/// helper: clear file.
		///
		/// delete file.
		///
		/// Parameters:
		/// - `file_hash`: file hash.
		///
		/// Result:
		/// - DispatchResult
		pub fn clear_file(file_hash: Hash) -> Result<Weight, DispatchError> {
			let mut weight: Weight = Weight::from_ref_time(0);
			let file =
					<File<T>>::try_get(&file_hash).map_err(|_| Error::<T>::FileNonExistent)?; //read 1
			  weight = weight.saturating_add(T::DbWeight::get().reads(1 as u64));
			for user_brief in file.user_brief_list.iter() {
				T::StorageHandle::update_user_space(&user_brief.user, 2, file.file_size.into())?; //read 1 write 1 * n
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
			}
			for slice in file.slice_info.iter() {
				let hash = Hash::from_shard_id(&slice.shard_id).map_err(|_| Error::<T>::ConvertHashError)?;
				Self::add_invalid_file(slice.miner_acc.clone(), hash)?; //read 1 write 1 * n
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
				T::MinerControl::sub_miner_service_space(&slice.miner_acc, slice.shard_size.into())?; //read 3 write 2
				T::StorageHandle::sub_total_service_space(slice.shard_size.into())?;
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(3, 2));
			}
			<File<T>>::remove(file_hash);
			<FileKeysMap<T>>::remove(file.index);
			Ok(weight)
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
				bucket.object_num = bucket.object_num.checked_sub(1).ok_or(Error::<T>::Overflow)?;
				Ok(())
			})?;
			weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

			Ok(weight)
		}
		/// helper: clear user file.
		///
		/// Remove a holder from the file.
		/// If a file has no holder, it will be cleaned up
		///
		/// Parameters:
		/// - `file_hash`: file hash.
		/// - `user`: hloder AccountId
		///
		/// Result:
		/// - DispatchResult
		pub fn clear_user_file(
			file_hash: Hash,
			user: &AccountOf<T>,
			is_multi: bool,
		) -> Result<Weight, DispatchError> {
			let mut weight: Weight = Weight::from_ref_time(0);
			//If the file still has an owner, only the corresponding owner will be cleared.
			//If the owner is unique, the file meta information will be cleared.
			if is_multi {
				//read 1 write 1
				let mut file_size: u64 = 0;
				<File<T>>::try_mutate(&file_hash, |s_opt| -> DispatchResult {
					let s = s_opt.as_mut().unwrap();
					file_size = s.file_size;
					let mut index: usize = 0;
					for user_brief in s.user_brief_list.iter() {
						if user_brief.user == user.clone() {
							break
						}
						index = index.checked_add(1).ok_or(Error::<T>::Overflow)?;
					}
					weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
					s.user_brief_list.remove(index);
					Ok(())
				})?;
				T::StorageHandle::update_user_space(&user, 2, file_size.into())?; //read 1 write 1
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
			} else {
				let weight_temp = Self::clear_file(file_hash.clone())?;
				weight = weight.saturating_add(weight_temp);
			}
			//read 1 write 1
			<UserHoldFileList<T>>::try_mutate(&user, |s| -> DispatchResult {
				s.retain(|x| x.file_hash != file_hash.clone());
				Ok(())
			})?;
			weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
			Ok(weight)
		}
		/// helper: add recovery file.
		///
		/// Create a recovery task,
		/// and the executor is the consensus of the current block
		///
		/// Parameters:
		/// - `shard id`: shard id.
		///
		/// Result:
		/// - DispatchResult
		pub fn add_recovery_file(shard_id: [u8; 68]) -> DispatchResult {
			let acc = Self::get_current_scheduler()?;
			let file_id = Hash::from_shard_id(&shard_id).map_err(|_| Error::<T>::ConvertHashError)?;
			if !<File<T>>::contains_key(&file_id) {
				Err(Error::<T>::FileNonExistent)?;
			}
			<File<T>>::try_mutate(&file_id, |opt| -> DispatchResult {
				let o = opt.as_mut().unwrap();
				o.slice_info.retain(|x| x.shard_id.to_vec() != shard_id);
				Ok(())
			})?; //read 1 write 1
			<FileRecovery<T>>::try_mutate(&acc, |o| -> DispatchResult {
				o.try_push(shard_id.try_into().map_err(|_e| Error::<T>::BoundedVecError)?)
					.map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?; //read 1 write 1

			Ok(())
		}
		/// helper: replace file.
		///
		/// service file replace fill file.
		/// Determine which filler files to replace according to size.
		///
		/// Parameters:
		/// - `miner_acc`: miner AccountId.
		/// - `file_size`: service file size
		///
		/// Result:
		/// - DispatchResult
		fn replace_file(miner_acc: AccountOf<T>, file_size: u64) -> DispatchResult {
			let (power, space) = T::MinerControl::get_power_and_space(miner_acc.clone())?;
			//Judge whether the current miner's remaining is enough to store files
			if power > space {
				if power - space < file_size.into() {
					Err(Error::<T>::MinerPowerInsufficient)?;
				}
			} else {
				Err(Error::<T>::Overflow)?;
			}
			//How many files to replace, round up
			let replace_num = (file_size as u128)
				.checked_div(8)
				.ok_or(Error::<T>::Overflow)?
				.checked_div(M_BYTE)
				.ok_or(Error::<T>::Overflow)?
				.checked_add(1)
				.ok_or(Error::<T>::Overflow)?;
			let mut counter = 0;
			let mut filler_hash_list: BoundedVec<Hash, T::StringLimit> = Default::default();
			for (filler_hash, _) in <FillerMap<T>>::iter_prefix(miner_acc.clone()) {
				if counter == replace_num {
					break
				}
				filler_hash_list
					.try_push(filler_hash.clone())
					.map_err(|_| Error::<T>::StorageLimitReached)?;
				counter = counter.checked_add(1).ok_or(Error::<T>::Overflow)?;
				//Clear information on the chain
				Self::delete_filler(miner_acc.clone(), filler_hash)?;
			}
			//Notify the miner to clear the corresponding data segment
			<InvalidFile<T>>::try_mutate(&miner_acc, |o| -> DispatchResult {
				for file_hash in filler_hash_list {
					o.try_push(file_hash).map_err(|_e| Error::<T>::StorageLimitReached)?;
				}
				Ok(())
			})?;
			//add space
			T::MinerControl::add_miner_service_space(&miner_acc, file_size.into())?;
			T::StorageHandle::add_total_service_space(file_size.into())?;
			T::MinerControl::sub_miner_idle_space(&miner_acc, replace_num * M_BYTE * 8)?;
			T::StorageHandle::sub_total_idle_space(replace_num * M_BYTE * 8)?;
			Ok(())
		}
		/// helper: add invalid file.
		///
		/// Clear the files that failed
		/// the challenge or have been replaced
		///
		/// Parameters:
		/// - `miner_acc`: miner AccountId.
		/// - `file_hash`: file hash.
		///
		/// Result:
		/// - DispatchResult
		pub fn add_invalid_file(miner_acc: AccountOf<T>, file_hash: Hash) -> DispatchResult {
			<InvalidFile<T>>::try_mutate(&miner_acc, |o| -> DispatchResult {
				o.try_push(file_hash.try_into().map_err(|_e| Error::<T>::BoundedVecError)?)
					.map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?; //read 1 write 1

			Ok(())
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
			user: AccountOf<T>,
			file_hash: Hash,
			file_size: u64,
		) -> DispatchResult {
			let file_info =
				UserFileSliceInfo { file_hash: file_hash.clone(), file_size };
			<UserHoldFileList<T>>::try_mutate(&user, |v| -> DispatchResult {
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
				for user_brief in file.user_brief_list.iter() {
					if &user_brief.user == acc {
						return true;
					}
				}
			}
			false
		}
		/// helper: clear expired file.
		///
		/// It is used to perform the final step
		/// when the user's space expires
		/// and no renewal is performed within the specified time
		///
		/// Parameters:
		/// - `acc`: AccountId.
		///
		/// Result:
		/// - DispatchResult
		fn clear_expired_file(acc: &AccountOf<T>) -> Result<Weight, DispatchError> {
			let mut weight: Weight = Weight::from_ref_time(0);
			let file_list =
				<UserHoldFileList<T>>::try_get(&acc).map_err(|_| Error::<T>::Overflow)?;
			for v in file_list.iter() {
				let file = <File<T>>::try_get(&v.file_hash).map_err(|_| Error::<T>::Overflow)?;
				let weight_temp = Self::clear_user_file(v.file_hash.clone(), acc, file.user_brief_list.len() > 1)?;
				weight = weight.saturating_add(weight_temp);
			}

			Ok(weight)
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
		let result = Pallet::<T>::get_random_challenge_data()?;
		Ok(result)
	}

	fn delete_filler(miner_acc: AccountOf<T>, filler_hash: Hash) -> DispatchResult {
		Pallet::<T>::delete_filler(miner_acc, filler_hash)?;
		Ok(())
	}
	fn delete_miner_all_filler(miner_acc: AccountOf<T>) -> Result<Weight, DispatchError> {
		let mut weight: Weight = Weight::from_ref_time(0);
		for (_, value) in FillerMap::<T>::iter_prefix(&miner_acc) {
			<FillerKeysMap<T>>::remove(value.index);
			weight = weight.saturating_add(T::DbWeight::get().writes(1 as u64));
		}
		#[allow(deprecated)]
		let _ = FillerMap::<T>::remove_prefix(&miner_acc, Option::None);
		weight = weight.saturating_add(T::DbWeight::get().writes(1 as u64));
		Ok(weight)
	}

	fn clear_file(file_hash: Hash) -> Result<Weight, DispatchError> {
		let weight = Pallet::<T>::clear_file(file_hash)?;
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
