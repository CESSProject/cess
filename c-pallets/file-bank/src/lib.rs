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
// use sc_network::Multiaddr;

pub use pallet::*;
#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod weights;
// pub mod migrations;

mod types;
pub use types::*;

mod functions;

mod constants;
use constants::*;

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
use pallet_tee_worker::TeeWorkerHandler;
use pallet_oss::OssFindAuthor;
use cp_enclave_verify::verify_rsa;

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
		type TeeWorkerHandler: TeeWorkerHandler<Self::AccountId>;
		//It is used to control the computing power and space of miners
		type MinerControl: MinerControl<Self::AccountId, Self::BlockNumber>;
		//Interface that can generate random seeds
		type MyRandomness: Randomness<Option<Self::Hash>, Self::BlockNumber>;

		type StorageHandle: StorageHandle<Self::AccountId>;
		/// pallet address.
		#[pallet::constant]
		type FilbakPalletId: Get<PalletId>;

		#[pallet::constant]
		type UserFileLimit: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type OneDay: Get<BlockNumberOf<Self>>;

		// User defined name length limit
		#[pallet::constant]
		type NameStrLimit: Get<u32> + Clone + Eq + PartialEq;
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

		#[pallet::constant]
		type RestoralOrderLife: Get<u32> + Clone + Eq + PartialEq;

		type CreditCounter: SchedulerCreditCounter<Self::AccountId>;
		//Used to confirm whether the origin is authorized
		type OssFindAuthor: OssFindAuthor<Self::AccountId>;

		#[pallet::constant]
		type MissionCount: Get<u32> + Clone + Eq + PartialEq;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		//file upload declaration
		UploadDeclaration { operator: AccountOf<T>, owner: AccountOf<T>, deal_hash: Hash },
		//file uploaded.
		TransferReport { acc: AccountOf<T>, failed_list: Vec<Hash> },
		//File deletion event
		DeleteFile { operator:AccountOf<T>, owner: AccountOf<T>, file_hash_list: Vec<Hash> },

		ReplaceFiller { acc: AccountOf<T>, filler_list: Vec<Hash> },

		CalculateEnd{ file_hash: Hash },

		IdleSpaceCert { acc: AccountOf<T>, space: u128 },

		ReplaceIdleSpace { acc: AccountOf<T>, space: u128 },
		//Event to successfully create a bucket
		CreateBucket { operator: AccountOf<T>, owner: AccountOf<T>, bucket_name: Vec<u8>},
		//Successfully delete the bucket event
		DeleteBucket { operator: AccountOf<T>, owner: AccountOf<T>, bucket_name: Vec<u8>},

		GenerateRestoralOrder { miner: AccountOf<T>, fragment_hash: Hash },

		ClaimRestoralOrder { miner: AccountOf<T>, order_id: Hash },

		RecoveryCompleted { miner: AccountOf<T>, order_id: Hash },

		StorageCompleted { file_hash: Hash },
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

		Expired,

		VerifyTeeSigFailed,

		InsufficientReplaceable,
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
		BoundedVec<UserFileSliceInfo, T::UserFileLimit>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn pending_replacements)]
	pub(super) type PendingReplacements<T: Config> = StorageMap<_, Blake2_128Concat, AccountOf<T>, u128, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn miner_lock)]
	pub(super) type MinerLock<T: Config> = 
		StorageMap<_, Blake2_128Concat, AccountOf<T>, BlockNumberOf<T>>;

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
	#[pallet::getter(fn restoral_order)]
	pub(super) type RestoralOrder<T: Config> = 
		StorageMap<_, Blake2_128Concat, Hash, RestoralOrderInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn clear_user_list)]
	pub(super) type ClearUserList<T: Config> = 
		StorageValue<_, BoundedVec<AccountOf<T>, ConstU32<5000>>, ValueQuery>;
	
	#[pallet::storage]
	#[pallet::getter(fn task_failed_count)]
	pub(super) type TaskFailedCount<T: Config> = 
		StorageMap<_, Blake2_128Concat, AccountOf<T>, u8, ValueQuery>;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberOf<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberOf<T>) -> Weight {
			let days = T::OneDay::get();
			let mut weight: Weight = Weight::from_ref_time(0);
			if now % days == 0u32.saturated_into() {
				let (temp_weight, acc_list) = T::StorageHandle::frozen_task();
				weight = weight.saturating_add(temp_weight);
				let temp_acc_list: BoundedVec<AccountOf<T>, ConstU32<5000>> = 
					acc_list.try_into().unwrap_or_default();
				ClearUserList::<T>::put(temp_acc_list);
				weight = weight.saturating_add(T::DbWeight::get().writes(1));
			}

			let mut count: u32 = 0;
			let acc_list = ClearUserList::<T>::get();
			weight = weight.saturating_add(T::DbWeight::get().reads(1));
			for acc in acc_list.iter() {
				// todo! Delete in blocks, and delete a part of each block
				if let Ok(mut file_info_list) = <UserHoldFileList<T>>::try_get(&acc) {
					weight = weight.saturating_add(T::DbWeight::get().reads(1));
					while let Some(file_info) = file_info_list.pop() {
						count = count.checked_add(1).unwrap_or(300);
						if count == 300 {
							<UserHoldFileList<T>>::insert(&acc, file_info_list);
							return weight;
						}
						if let Ok(file) = <File<T>>::try_get(&file_info.file_hash) {
							weight = weight.saturating_add(T::DbWeight::get().reads(1));
							if file.owner.len() > 1 {
								match Self::remove_file_owner(&file_info.file_hash, &acc, false) {
									Ok(()) => weight = weight.saturating_add(T::DbWeight::get(). reads_writes(2, 2)),
									Err(e) => log::info!("delete file {:?} failed. error is: {:?}", e, file_info.file_hash),
								};
							 } else {
								match Self::remove_file_last_owner(&file_info.file_hash, &acc, false) {
									Ok(temp_weight) => weight = weight.saturating_add(temp_weight),
									Err(e) => log::info!("delete file {:?} failed. error is: {:?}", e, file_info.file_hash),
								};
								if let Ok(temp_weight) = Self::remove_file_last_owner(&file_info.file_hash, &acc, false) {
									weight = weight.saturating_add(temp_weight);
								}
							}
						} else {
							log::error!("space lease, delete file bug!");
							log::error!("acc: {:?}, file_hash: {:?}", &acc, &file_info.file_hash);
						}
					}

					match T::StorageHandle::delete_user_space_storage(&acc) {
						Ok(temp_weight) => weight = weight.saturating_add(temp_weight),
						Err(e) => log::info!("delete user sapce error: {:?}, \n failed user: {:?}", e, acc),
					}

					ClearUserList::<T>::mutate(|target_list| {
						target_list.retain(|temp_acc| temp_acc != acc);
					});

					<UserHoldFileList<T>>::remove(&acc);
					// todo! clear all
					let _ = <Bucket<T>>::clear_prefix(&acc, 100000, None);
					<UserBucketList<T>>::remove(&acc);
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
			file_size: u128,
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

			let needed_space = SEGMENT_SIZE
				.checked_mul(15).ok_or(Error::<T>::Overflow)?
				.checked_div(10).ok_or(Error::<T>::Overflow)?
				.checked_mul(deal_info.len() as u128).ok_or(Error::<T>::Overflow)?;
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
				T::StorageHandle::lock_user_space(&user_brief.user, needed_space)?;
				// TODO! Replace the file_hash param
				Self::generate_deal(file_hash.clone(), deal_info, user_brief.clone(), file_size)?;
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
			life: u32,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			if count < 20 {
				if let Err(e) = <DealMap<T>>::try_mutate(&deal_hash, |opt| -> DispatchResult {
					log::info!("point 1");
					let deal_info = opt.as_mut().ok_or(Error::<T>::NonExistent)?;
					// unlock mienr space
					let mut needed_list: BoundedVec<MinerTaskList<T>, T::FragmentCount> = Default::default();
					let mut selected_miner: BoundedVec<AccountOf<T>, T::FragmentCount> = Default::default();
					for miner_task in &deal_info.assigned_miner.clone() {
						if !deal_info.complete_list.contains(&miner_task.miner) {
							needed_list.try_push(miner_task.clone()).map_err(|_| Error::<T>::Overflow)?;
						}
						selected_miner.try_push(miner_task.miner.clone()).map_err(|_| Error::<T>::Overflow)?;
					}
					log::info!("point 2");
					let mut miner_task_list = Self::reassign_miner(needed_list.clone(), selected_miner.clone())?.to_vec();
					log::info!("point 3");
					for miner_task in &deal_info.assigned_miner.clone() {
						if !deal_info.complete_list.contains(&miner_task.miner) {
							deal_info.assigned_miner.retain(|temp_info| temp_info.miner != miner_task.miner);
							let task_count = miner_task.fragment_list.len() as u128;
							let unlock_space = FRAGMENT_SIZE.checked_mul(task_count).ok_or(Error::<T>::Overflow)?;
							T::MinerControl::unlock_space(&miner_task.miner, unlock_space)?;
							Self::add_task_failed_count(&miner_task.miner)?;
						}
					}
					log::info!("point 4");
					deal_info.assigned_miner.try_append(&mut miner_task_list).map_err(|_| Error::<T>::Overflow)?;
					deal_info.count = count;
					// count <= 20
					log::info!("point 5");
					Self::start_first_task(deal_hash.0.to_vec(), deal_hash, count + 1, life)?;
					Ok(())
				}) {
					log::info!("point Err: {:?}", e);
					Self::remove_deal(&deal_hash)?;
				}
			} else {
				Self::remove_deal(&deal_hash)?;
			}

			Ok(())
		}
		/// Transfer needs to be restricted, such as target consent
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
									deal_info.file_size,
								)?;

								let mut max_task_count = 0;
								for miner_task in deal_info.assigned_miner.iter() {
									let count = miner_task.fragment_list.len() as u128;
									if count > max_task_count {
										max_task_count = count;
									}
									// Miners need to report the replaced documents themselves. 
									// If a challenge is triggered before the report is completed temporarily, 
									// these documents to be replaced also need to be verified
									<PendingReplacements<T>>::try_mutate(miner_task.miner.clone(), |pending_space| -> DispatchResult {
										let replace_space = FRAGMENT_SIZE.checked_mul(count).ok_or(Error::<T>::Overflow)?;
										let pending_space_temp = pending_space.checked_add(replace_space).ok_or(Error::<T>::Overflow)?;
										*pending_space = pending_space_temp;
										Ok(())
									})?;

									Self::zero_task_failed_count(&miner_task.miner)?;
								}

								let needed_space = Self::cal_file_size(deal_info.segment_list.len() as u128);

								T::StorageHandle::unlock_and_used_user_space(&deal_info.user.user, needed_space)?;
								T::StorageHandle::sub_total_idle_space(needed_space)?;
								T::StorageHandle::add_total_service_space(needed_space)?;

								let result = T::FScheduler::cancel_named(hash.0.to_vec()).map_err(|_| Error::<T>::Unexpected);
								if let Err(_) = result {
									log::info!("transfer report cancel schedule failed: {:?}", hash.clone());
								}
								// Calculate the maximum time required for storage nodes to tag files
								let max_needed_cal_space = (max_task_count as u32).checked_mul(FRAGMENT_SIZE as u32).ok_or(Error::<T>::Overflow)?;
								let mut life: u32 = (max_needed_cal_space / TRANSFER_RATE as u32).checked_add(11).ok_or(Error::<T>::Overflow)?;
								life = (max_needed_cal_space / CALCULATE_RATE as u32)
									.checked_add(10).ok_or(Error::<T>::Overflow)?
									.checked_add(life).ok_or(Error::<T>::Overflow)?;

								Self::start_second_task(hash.0.to_vec(), hash, life)?;
								if <Bucket<T>>::contains_key(&deal_info.user.user, &deal_info.user.bucket_name) {
									Self::add_file_to_bucket(&deal_info.user.user, &deal_info.user.bucket_name, &hash)?;
								} else {
									Self::create_bucket_helper(&deal_info.user.user, &deal_info.user.bucket_name, Some(hash))?;
								}

								Self::add_user_hold_fileslice(&deal_info.user.user, hash.clone(), needed_space)?;

								Self::deposit_event(Event::<T>::StorageCompleted{ file_hash: hash });
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
				let mut hash_list: Vec<Box<[u8; 256]>> = Default::default(); 
				for fragment_hash in miner_task.fragment_list {
					let hash_temp = fragment_hash.binary().map_err(|_| Error::<T>::BugInvalid)?;
					hash_list.push(hash_temp);
				}
				// Accumulate the number of fragments stored by each miner
				let unlock_space = FRAGMENT_SIZE.checked_mul(count as u128).ok_or(Error::<T>::Overflow)?;
				T::MinerControl::unlock_space_to_service(&miner_task.miner, unlock_space)?;
				T::MinerControl::insert_service_bloom(&miner_task.miner, hash_list)?;
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
		pub fn replace_idle_space(
			origin: OriginFor<T>,
			idle_sig_info: SpaceProofInfo<AccountOf<T>>,
			tee_sig: TeeRsaSignature,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let original = idle_sig_info.encode();
			let original = sp_io::hashing::sha2_256(&original);
			// FOR TEST
			// log::info!("sha2_256 original: {:?}", original);

			let tee_puk = T::TeeWorkerHandler::get_tee_publickey()?;

			ensure!(verify_rsa(&tee_puk, &original, &tee_sig), Error::<T>::VerifyTeeSigFailed);

			let count = T::MinerControl::delete_idle_update_accu(
				&sender, 
				idle_sig_info.accumulator,
				idle_sig_info.front,
				tee_sig,
			)?;

			<PendingReplacements<T>>::try_mutate(&sender, |pending_space| -> DispatchResult {
				if *pending_space / IDLE_SEG_SIZE == 0 {
					Err(Error::<T>::InsufficientReplaceable)?;
				}
				let replace_space = IDLE_SEG_SIZE.checked_mul(count.into()).ok_or(Error::<T>::Overflow)?;
				log::info!("pending_space: {:?}, replace_space: {:?}, count: {:?}", pending_space, replace_space, count);
				let pending_space_temp = pending_space.checked_sub(replace_space).ok_or(Error::<T>::Overflow)?;
				*pending_space = pending_space_temp;

				Self::deposit_event(Event::<T>::ReplaceIdleSpace { acc: sender.clone(), space: replace_space });

				Ok(())
			})?;

			

			Ok(())
		}

		#[pallet::call_index(6)]
		#[transactional]
		#[pallet::weight(1_000_000_000)]
		pub fn delete_file(origin: OriginFor<T>, owner: AccountOf<T>, file_hash_list: Vec<Hash>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			// Check if you have operation permissions.
			ensure!(Self::check_permission(sender.clone(), owner.clone()), Error::<T>::NoPermission);
			ensure!(file_hash_list.len() < 10, Error::<T>::LengthExceedsLimit);

			for file_hash in file_hash_list.iter() {
				let file = <File<T>>::try_get(&file_hash).map_err(|_| Error::<T>::NonExistent)?;
				let _ = Self::delete_user_file(&file_hash, &owner, &file)?;
				Self::bucket_remove_file(&file_hash, &owner, &file)?;
				Self::remove_user_hold_file_list(&file_hash, &owner)?;
			}

			Self::deposit_event(Event::<T>::DeleteFile{ operator: sender, owner, file_hash_list });

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
		#[pallet::weight(100_000_000)]
		pub fn cert_idle_space(
			origin: OriginFor<T>,
			idle_sig_info: SpaceProofInfo<AccountOf<T>>,
			tee_sig: TeeRsaSignature,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let original = idle_sig_info.encode();
			let original = sp_io::hashing::sha2_256(&original);
			// FOR TEST
			log::info!("sha2_256 original: {:?}", original);

			let tee_puk = T::TeeWorkerHandler::get_tee_publickey()?;

			ensure!(verify_rsa(&tee_puk, &original, &tee_sig), Error::<T>::VerifyTeeSigFailed);

			let idle_space = T::MinerControl::add_miner_idle_space(
				&sender, 
				idle_sig_info.accumulator, 
				idle_sig_info.rear,
				tee_sig,
			)?;

			T::StorageHandle::add_total_idle_space(idle_space)?;

			Self::deposit_event(Event::<T>::IdleSpaceCert{ acc: sender, space: idle_space });

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
			
			Self::create_bucket_helper(&owner, &name, None)?;

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
				if file.owner.len() > 1 {
					Self::remove_file_owner(file_hash, &owner, true)?;
				 } else {
					Self::remove_file_last_owner(file_hash, &owner, true)?;
				}

				Self::remove_user_hold_file_list(file_hash, &owner)?;
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

		// restoral file
		#[pallet::call_index(13)]
		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn generate_restoral_order(
			origin: OriginFor<T>,
			file_hash: Hash,
			restoral_fragment: Hash,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(
				!RestoralOrder::<T>::contains_key(&restoral_fragment),
				Error::<T>::Existed,
			);

			<File<T>>::try_mutate(&file_hash, |file_opt| -> DispatchResult {
				let file = file_opt.as_mut().ok_or(Error::<T>::NonExistent)?;
				for segment in &mut file.segment_list {
					for fragment in &mut segment.fragment_list {
						if &fragment.hash == &restoral_fragment {
							if &fragment.miner == &sender {
								let restoral_order = RestoralOrderInfo::<T> {
									count: u32::MIN,
									miner: sender.clone(),
									origin_miner: sender.clone(),
									file_hash: file_hash,
									fragment_hash: restoral_fragment.clone(),
									gen_block: <frame_system::Pallet<T>>::block_number(),
									deadline: Default::default(),
								};

								fragment.avail = false;

								<RestoralOrder<T>>::insert(&restoral_fragment, restoral_order);

								Self::deposit_event(Event::<T>::GenerateRestoralOrder{ miner: sender, fragment_hash: restoral_fragment});

								return Ok(())
							}
						}
					}
				}

				Err(Error::<T>::SpecError)?
			})
		}

		#[pallet::call_index(14)]
		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn claim_restoral_order(
			origin: OriginFor<T>,
			restoral_fragment: Hash,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let is_positive = T::MinerControl::is_positive(&sender)?;
			ensure!(is_positive, Error::<T>::MinerStateError);

			let now = <frame_system::Pallet<T>>::block_number();
			<RestoralOrder<T>>::try_mutate(&restoral_fragment, |order_opt| -> DispatchResult {
				let order = order_opt.as_mut().ok_or(Error::<T>::NonExistent)?;
				
				ensure!(now > order.deadline, Error::<T>::SpecError);

				let life = T::RestoralOrderLife::get();
				order.count = order.count.checked_add(1).ok_or(Error::<T>::Overflow)?;
				order.deadline = now.checked_add(&life.saturated_into()).ok_or(Error::<T>::Overflow)?;
				order.miner = sender.clone();

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::ClaimRestoralOrder{ miner: sender, order_id: restoral_fragment});

			Ok(())
		}

		#[pallet::call_index(15)]
		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn claim_restoral_noexist_order(
			origin: OriginFor<T>,
			miner: AccountOf<T>,
			file_hash: Hash,
			restoral_fragment: Hash,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let is_positive = T::MinerControl::is_positive(&sender)?;
			ensure!(is_positive, Error::<T>::MinerStateError);

			ensure!(
				!RestoralOrder::<T>::contains_key(&restoral_fragment),
				Error::<T>::Existed,
			);

			ensure!(
				T::MinerControl::restoral_target_is_exist(&miner),
				Error::<T>::NonExistent,
			);

			<File<T>>::try_mutate(&file_hash, |file_opt| -> DispatchResult {
				let file = file_opt.as_mut().ok_or(Error::<T>::NonExistent)?;
				for segment in &mut file.segment_list {
					for fragment in &mut segment.fragment_list {
						if &fragment.hash == &restoral_fragment {
							if fragment.miner == miner {
								let now = <frame_system::Pallet<T>>::block_number();
								let life = T::RestoralOrderLife::get();
								let deadline = now.checked_add(&life.saturated_into()).ok_or(Error::<T>::Overflow)?;
								let restoral_order = RestoralOrderInfo::<T> {
									count: u32::MIN,
									miner: sender.clone(),
									origin_miner: fragment.miner.clone(),
									file_hash: file_hash,
									fragment_hash: restoral_fragment.clone(),
									gen_block: <frame_system::Pallet<T>>::block_number(),
									deadline,
								};

								fragment.avail = false;

								<RestoralOrder<T>>::insert(&restoral_fragment, restoral_order);

								return Ok(())
							}
						}
					}
				}
	
				Err(Error::<T>::SpecError)?
			})?;

			Self::deposit_event(Event::<T>::ClaimRestoralOrder{ miner: sender, order_id: restoral_fragment});

			Ok(())
		}

		#[pallet::call_index(16)]
		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn restoral_order_complete(
			origin: OriginFor<T>,
			fragment_hash: Hash,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let is_positive = T::MinerControl::is_positive(&sender)?;
			ensure!(is_positive, Error::<T>::MinerStateError);

			let order = <RestoralOrder<T>>::try_get(&fragment_hash).map_err(|_| Error::<T>::NonExistent)?;
			ensure!(&order.miner == &sender, Error::<T>::SpecError);

			let now = <frame_system::Pallet<T>>::block_number();
			ensure!(now < order.deadline, Error::<T>::Expired);

			if !<File<T>>::contains_key(&order.file_hash) {
				<RestoralOrder<T>>::remove(fragment_hash);
				return Ok(());
			} else {
				<File<T>>::try_mutate(&order.file_hash, |file_opt| -> DispatchResult {
					let file = file_opt.as_mut().ok_or(Error::<T>::BugInvalid)?;

					for segment in &mut file.segment_list {
						for fragment in &mut segment.fragment_list {
							if &fragment.hash == &fragment_hash {
								if &fragment.miner == &order.origin_miner {
									let binary = fragment.hash.binary().map_err(|_| Error::<T>::BugInvalid)?;
									T::MinerControl::delete_service_bloom(&fragment.miner, vec![binary.clone()])?;
									T::MinerControl::insert_service_bloom(&sender, vec![binary])?;
									T::MinerControl::sub_miner_service_space(&fragment.miner, FRAGMENT_SIZE)?;
									T::MinerControl::add_miner_service_space(&sender, FRAGMENT_SIZE)?;

									// TODO!
									T::MinerControl::delete_idle_update_space(&sender, FRAGMENT_SIZE)?;
									<PendingReplacements<T>>::try_mutate(&sender, |pending_space| -> DispatchResult {
										let pending_space_temp = pending_space.checked_add(FRAGMENT_SIZE).ok_or(Error::<T>::Overflow)?;
										*pending_space = pending_space_temp;
										Ok(())
									})?;

									if T::MinerControl::restoral_target_is_exist(&fragment.miner) {
										T::MinerControl::update_restoral_target(&fragment.miner, FRAGMENT_SIZE)?;
									}

									fragment.avail = true;
									fragment.miner = sender.clone();
									return Ok(());
								}
							}
						}
					}

					Ok(())
				})?;
			}

			<RestoralOrder<T>>::remove(fragment_hash);

			Self::deposit_event(Event::<T>::RecoveryCompleted{ miner: sender, order_id: fragment_hash});
		
			Ok(())
		}
		
		#[pallet::call_index(20)]
		#[transactional]
		#[pallet::weight(10_000_000_000)]
		pub fn root_clear_failed_count(origin: OriginFor<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;

			for (miner, _) in <TaskFailedCount<T>>::iter() {
				<TaskFailedCount<T>>::remove(&miner);
			}

			Ok(())
		}

		#[pallet::call_index(21)]
		#[transactional]
		#[pallet::weight(10_000_000_000)]
		pub fn miner_clear_failed_count(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			<TaskFailedCount<T>>::remove(&sender);

			Ok(())
		}
	}
}

pub trait RandomFileList<AccountId> {
	//Get random challenge data
	fn get_random_challenge_data(
	) -> Result<Vec<(AccountId, Hash, [u8; 68], Vec<u32>, u64, DataType)>, DispatchError>;
	//Delete file backup
	fn clear_file(_file_hash: Hash) -> Result<Weight, DispatchError>;
}

impl<T: Config> RandomFileList<<T as frame_system::Config>::AccountId> for Pallet<T> {
	fn get_random_challenge_data(
	) -> Result<Vec<(AccountOf<T>, Hash, [u8; 68], Vec<u32>, u64, DataType)>, DispatchError> {
		Ok(Default::default())
	}

	fn clear_file(_file_hash: Hash) -> Result<Weight, DispatchError> {
		let weight: Weight = Weight::from_ref_time(0);
		Ok(weight)
	}
}

impl<T: Config> BlockNumberProvider for Pallet<T> {
	type BlockNumber = T::BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		<frame_system::Pallet<T>>::block_number()
	}
}
