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

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

use frame_support::traits::{
	FindAuthor, Randomness,
	StorageVersion,
	schedule::{Anon as ScheduleAnon, Named as ScheduleNamed}, 
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

mod impls;
use impls::receptionist::Receptionist;

use codec::{Decode, Encode};
use frame_support::{
	// bounded_vec, 
	transactional, 
	PalletId, 
	dispatch::DispatchResult, 
	pallet_prelude::*,
	weights::Weight,
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use cp_cess_common::*;
use pallet_storage_handler::StorageHandle;
use cp_scheduler_credit::SchedulerCreditCounter;
use sp_runtime::{
	traits::{
		BlockNumberProvider, CheckedAdd, Dispatchable
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

		type FScheduler: ScheduleNamed<BlockNumberFor<Self>, Self::SProposal, Self::SPalletsOrigin>;

		type AScheduler: ScheduleAnon<BlockNumberFor<Self>, Self::SProposal, Self::SPalletsOrigin>;
		/// Overarching type of all pallets origins.
		type SPalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;
		/// The SProposal.
		type SProposal: Parameter + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin> + From<Call<Self>>;
		// Find the consensus of the current block
		type FindAuthor: FindAuthor<Self::AccountId>;
		// Used to find out whether the schedule exists
		type TeeWorkerHandler: TeeWorkerHandler<Self::AccountId>;
		// It is used to control the computing power and space of miners
		type MinerControl: MinerControl<Self::AccountId, BlockNumberFor<Self>>;
		// Interface that can generate random seeds	
		type MyRandomness: Randomness<Option<Self::Hash>, BlockNumberFor<Self>>;

		type StorageHandle: StorageHandle<Self::AccountId>;
		/// pallet address.
		#[pallet::constant]
		type FilbakPalletId: Get<PalletId>;

		#[pallet::constant]
		type UserFileLimit: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type OneDay: Get<BlockNumberFor<Self>>;

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
		TransferReport { acc: AccountOf<T>, deal_hash: Hash },
		//File deletion event
		DeleteFile { operator:AccountOf<T>, owner: AccountOf<T>, file_hash: Hash },

		ReplaceFiller { acc: AccountOf<T>, filler_list: Vec<Hash> },

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

		CalculateReport { miner: AccountOf<T>, file_hash: Hash },
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

		NotEmpty,
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

		TeeNoPermission,
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
	#[pallet::getter(fn miner_lock)]
	pub(super) type MinerLock<T: Config> = 
		StorageMap<_, Blake2_128Concat, AccountOf<T>, BlockNumberFor<T>>;

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
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			let days = T::OneDay::get();
			let mut weight: Weight = Weight::zero();
			// FOR TESTING
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
						count = count.checked_add(1).unwrap_or(ONCE_MAX_CLEAR_FILE);
						if count == ONCE_MAX_CLEAR_FILE {
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
		/// Upload Declaration of Data Storage
		///
		/// This function allows a user to upload a declaration for data storage, specifying the file's metadata,
		/// deal information, and ownership details. It is used to initiate the storage process of a file.
		///
		/// Parameters:
		/// - `origin`: The origin of the transaction.
		/// - `file_hash`: The unique hash identifier of the file.
		/// - `deal_info`: A list of segment details for data storage.
		/// - `user_brief`: A brief description of the user and the file's ownership.
		/// - `file_size`: The size of the file in bytes.
		#[pallet::call_index(0)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upload_declaration(deal_info.len() as u32))]
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

			if <File<T>>::contains_key(&file_hash) {
				Receptionist::<T>::fly_upload_file(file_hash, user_brief.clone())?;
			} else {
				let needed_space = SEGMENT_SIZE
					.checked_mul(15).ok_or(Error::<T>::Overflow)?
					.checked_div(10).ok_or(Error::<T>::Overflow)?
					.checked_mul(deal_info.len() as u128).ok_or(Error::<T>::Overflow)?;
            	ensure!(T::StorageHandle::get_user_avail_space(&user_brief.user)? > needed_space, Error::<T>::InsufficientAvailableSpace);
				Receptionist::<T>::generate_deal(file_hash, deal_info, user_brief.clone(), needed_space, file_size)?;
			}

			Self::deposit_event(Event::<T>::UploadDeclaration { operator: sender, owner: user_brief.user, deal_hash: file_hash });

			Ok(())
		}
		
		/// Transfer Ownership of a File
		///
		/// This function allows the owner of a file to transfer ownership to another account. The file is identified by its unique
		/// `file_hash`, and the ownership is transferred to the target user specified in the `target_brief`. The sender of the
		/// transaction must be the current owner of the file.
		///
		/// Parameters:
		/// - `origin`: The origin of the transaction, representing the current owner of the file.
		/// - `target_brief`: User brief information of the target user to whom ownership is being transferred.
		/// - `file_hash`: The unique hash identifier of the file to be transferred
		#[pallet::call_index(2)]
		#[transactional]
		/// FIX ME
		#[pallet::weight(Weight::zero())]
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

		/// Transfer Report for a Storage Deal
		///
		/// This function allows miners participating in a storage deal to report that they have successfully stored the data segments.
		/// A storage deal is identified by its unique `deal_hash`. Miners who are part of the deal and have successfully stored their assigned
		/// data segments can call this function to report completion.
		///
		/// Parameters:
		/// - `origin`: The origin of the transaction, representing the reporting miner.
		/// - `deal_hash`: The unique hash identifier of the storage deal being reported.
		#[pallet::call_index(3)]
		// #[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::transfer_report(Pallet::<T>::get_segment_length_from_deal(&deal_hash)))]
		pub fn transfer_report(
			origin: OriginFor<T>,
			index: u8,
			deal_hash: Hash,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(index as u32 <= FRAGMENT_COUNT, Error::<T>::SpecError);
			ensure!(index > 0, Error::<T>::SpecError);
			let is_positive = T::MinerControl::is_positive(&sender)?;
			ensure!(is_positive, Error::<T>::MinerStateError);

			let mut deal_info = <DealMap<T>>::try_get(&deal_hash).map_err(|_| Error::<T>::NonExistent)?;
			Receptionist::<T>::qualification_report_processing(sender.clone(), deal_hash, &mut deal_info, index)?;

			Self::deposit_event(Event::<T>::TransferReport{acc: sender, deal_hash: deal_hash});

			Ok(())
		}

		#[pallet::call_index(1)]
		// FIX ME
		#[pallet::weight(Weight::zero())]
		pub fn calculate_report(
			origin: OriginFor<T>,
			tee_sig: TeeRsaSignature,
			tag_sig_info: TagSigInfo<AccountOf<T>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(
				T::TeeWorkerHandler::can_tag(&tag_sig_info.tee_acc),
				Error::<T>::TeeNoPermission
			);
			let idle_sig_info_encode = tag_sig_info.encode();
			let original = sp_io::hashing::sha2_256(&idle_sig_info_encode);
			let tee_puk = T::TeeWorkerHandler::get_tee_publickey()?;

			ensure!(verify_rsa(&tee_puk, &original, &tee_sig), Error::<T>::VerifyTeeSigFailed);
			ensure!(tag_sig_info.miner == sender, Error::<T>::VerifyTeeSigFailed);

			<File<T>>::try_mutate(&tag_sig_info.file_hash, |file_info_opt| -> DispatchResult {
				let file_info = file_info_opt.as_mut().ok_or(Error::<T>::NonExistent)?;
				let now = <frame_system::Pallet<T>>::block_number();
				let mut count: u128 = 0;
				let mut hash_list: Vec<Box<[u8; 256]>> = Default::default();
				let mut flag = false;
				for segment in file_info.segment_list.iter_mut() {
					for fragment in segment.fragment_list.iter_mut() {
						if fragment.miner == sender {
							if fragment.tag.is_none() {
								fragment.tag = Some(now);
								count = count + 1;
								let hash_temp = fragment.hash.binary().map_err(|_| Error::<T>::BugInvalid)?;
								hash_list.push(hash_temp);
								flag = true;
							}
						}
					}
				}

				ensure!(flag, Error::<T>::NonExistent);

				let unlock_space = FRAGMENT_SIZE.checked_mul(count as u128).ok_or(Error::<T>::Overflow)?;
				T::MinerControl::unlock_space_to_service(&sender, unlock_space)?;
				T::MinerControl::insert_service_bloom(&sender, hash_list)?;

				if T::TeeWorkerHandler::is_bonded(&tag_sig_info.tee_acc) {
					let bond_stash = T::TeeWorkerHandler::get_stash(&tag_sig_info.tee_acc)?;
					T::CreditCounter::increase_point_for_tag(&bond_stash, unlock_space)?;
				}

				Self::deposit_event(Event::<T>::CalculateReport{ miner: sender, file_hash: tag_sig_info.file_hash});

				Ok(())
			})?;

			Ok(())
		}

		/// Replace Idle Space
		///
		/// This function allows a signed user to replace their idle storage space.
		///
		/// Parameters:
		/// - `origin`: The origin of the transaction.
		/// - `idle_sig_info`: Space proof information associated with the idle space to be replaced.
		/// - `tee_sig`: A signature from the TEE (Trusted Execution Environment) used to verify the idle space replacement.
		///
		/// The function ensures that the caller has enough pending space to perform the replacement.
		#[pallet::call_index(5)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::replace_idle_space())]
		pub fn replace_idle_space(
			origin: OriginFor<T>,
			idle_sig_info: SpaceProofInfo<AccountOf<T>>,
			tee_sig_need_verify: TeeRsaSignature,
			tee_sig: TeeRsaSignature,
			tee_acc: AccountOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(
				T::TeeWorkerHandler::can_cert(&tee_acc),
				Error::<T>::TeeNoPermission
			);
			let idle_sig_info_encode = idle_sig_info.encode();
			let tee_acc_encode = tee_acc.encode();
			let mut original = Vec::new();
			original.extend_from_slice(&idle_sig_info_encode);
			original.extend_from_slice(&tee_acc_encode);
			let original = sp_io::hashing::sha2_256(&original);
			let tee_puk = T::TeeWorkerHandler::get_tee_publickey()?;

			ensure!(verify_rsa(&tee_puk, &original, &tee_sig_need_verify), Error::<T>::VerifyTeeSigFailed);

			let count = T::MinerControl::delete_idle_update_accu(
				&sender, 
				idle_sig_info.accumulator,
				idle_sig_info.front,
				idle_sig_info.rear,
				tee_sig,
			)?;
			let replace_space = IDLE_SEG_SIZE.checked_mul(count.into()).ok_or(Error::<T>::Overflow)?;
			T::MinerControl::decrease_replace_space(&sender, replace_space)?;

			if T::TeeWorkerHandler::is_bonded(&tee_acc) {
				let bond_stash = T::TeeWorkerHandler::get_stash(&tee_acc)?;
				T::CreditCounter::increase_point_for_replace(&bond_stash, replace_space)?;
			}
			
			Self::deposit_event(Event::<T>::ReplaceIdleSpace { acc: sender.clone(), space: replace_space });

			Ok(())
		}

		/// Delete File
		///
		/// This function allows an authorized user to delete a file associated with a specific storage hash.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization.
		/// - `owner`: The owner of the file, representing the user who has permission to delete the file.
		/// - `file_hash`: The unique hash identifier of the file to be deleted.
		#[pallet::call_index(6)]
		#[transactional]
		#[pallet::weight({
			let v = Pallet::<T>::get_segment_length_from_file(&file_hash);
			<T as pallet::Config>::WeightInfo::delete_file(v)
		})]
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

		/// Certify Idle Space
		///
		/// This function allows a user to certify their idle storage space by providing 
		/// a proof of their idle space's integrity and availability.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization.
		/// - `idle_sig_info`: Information about the idle space, including the accumulator and rear values.
		/// - `tee_sig`: A cryptographic signature provided by a Trusted Execution Environment (TEE) to verify the proof.
		#[pallet::call_index(8)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::cert_idle_space())]
		pub fn cert_idle_space(
			origin: OriginFor<T>,
			idle_sig_info: SpaceProofInfo<AccountOf<T>>,
			tee_sig_need_verify: TeeRsaSignature,
			tee_sig: TeeRsaSignature,
			tee_acc: AccountOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(
				T::TeeWorkerHandler::can_cert(&tee_acc),
				Error::<T>::TeeNoPermission
			);
			let idle_sig_info_encode = idle_sig_info.encode();
			let tee_acc_encode = tee_acc.encode();
			let mut original = Vec::new();
			original.extend_from_slice(&idle_sig_info_encode);
			original.extend_from_slice(&tee_acc_encode);
			let original = sp_io::hashing::sha2_256(&original);

			let tee_puk = T::TeeWorkerHandler::get_tee_publickey()?;

			ensure!(verify_rsa(&tee_puk, &original, &tee_sig_need_verify), Error::<T>::VerifyTeeSigFailed);

			let idle_space = T::MinerControl::add_miner_idle_space(
				&sender, 
				idle_sig_info.accumulator, 
				idle_sig_info.front,
				idle_sig_info.rear,
				tee_sig,
			)?;

			if T::TeeWorkerHandler::is_bonded(&tee_acc) {
				let bond_stash = T::TeeWorkerHandler::get_stash(&tee_acc)?;
				T::CreditCounter::increase_point_for_cert(&bond_stash, idle_space)?;
			}

			T::StorageHandle::add_total_idle_space(idle_space)?;

			Self::deposit_event(Event::<T>::IdleSpaceCert{ acc: sender, space: idle_space });

			Ok(())
		}

		/// Create a Data Storage Bucket
		///
		/// This function allows a user with appropriate permissions to create a new data storage bucket.
		///
		/// Parameters:
		/// - `origin`: The origin of the transaction.
		/// - `owner`: The account identifier of the bucket owner.
		/// - `name`: The name of the new bucket.
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

		/// Delete a Bucket
		///
		/// This function allows a user to delete an empty storage bucket that they own. 
		/// Deleting a bucket is only possible if it contains no files.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization.
		/// - `owner`: The owner's account for whom the bucket should be deleted.
		/// - `name`: The name of the bucket to be deleted.
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
			ensure!(bucket.object_list.len() == 0, Error::<T>::NotEmpty);

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

		/// Generate a Restoration Order
		///
		/// This function allows a user to generate a restoration order for a specific fragment of a file. 
		/// A restoration order is used to request the restoration of a lost or corrupted fragment from the network.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization.
		/// - `file_hash`: The hash of the file that the fragment belongs to.
		/// - `restoral_fragment`: The hash of the fragment to be restored.
		#[pallet::call_index(13)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::generate_restoral_order())]
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

		/// Claim a Restoration Order
		///
		/// This function allows a network miner to claim a restoration order for a specific fragment of a file. 
		/// A restoration order is generated when a user requests the restoration of a lost or corrupted fragment. 
		/// Miners can claim these orders to provide the requested fragments.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization. This function can only be called by authorized miners.
		/// - `restoral_fragment`: The hash of the fragment associated with the restoration order to be claimed.
		#[pallet::call_index(14)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::claim_restoral_order())]
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

		/// Claim a Non-Existent Restoration Order
		///
		/// This function allows an authorized network miner to claim a non-existent restoration order. 
		/// A non-existent restoration order is generated when a user requests the restoration of a lost or corrupted fragment, 
		/// but the corresponding restoration order does not exist.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization. This function can only be called by authorized miners.
		/// - `miner`: The miner's account that claims the non-existent restoration order.
		/// - `file_hash`: The hash of the file associated with the non-existent restoration order.
		/// - `restoral_fragment`: The hash of the fragment for which the non-existent restoration order is being claimed.
		#[pallet::call_index(15)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::claim_restoral_noexist_order())]
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

		/// Complete a Restoration Order
		///
		/// This function allows authorized network miners to complete a restoration order, 
		/// indicating that they have successfully restored a fragment to its original state. 
		/// Restoration orders are generated when users request the restoration of lost or corrupted fragments. 
		/// Miners claim these orders and provide the requested fragments for restoration.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization. This function can only be called by authorized miners.
		/// - `fragment_hash`: The hash of the fragment to be restored, for which the restoration order is being completed.
		#[pallet::call_index(16)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::restoral_order_complete())]
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
									T::MinerControl::insert_service_bloom(&sender, vec![binary.clone()])?;
									T::MinerControl::add_miner_service_space(&sender, FRAGMENT_SIZE)?;

									// TODO!
									T::MinerControl::delete_idle_update_space(&sender, FRAGMENT_SIZE)?;
									T::MinerControl::increase_replace_space(&sender, FRAGMENT_SIZE)?;

									if T::MinerControl::restoral_target_is_exist(&fragment.miner) {
										T::MinerControl::update_restoral_target(&fragment.miner, FRAGMENT_SIZE)?;
									} else {
										if fragment.tag.is_some() {
											T::MinerControl::delete_service_bloom(&fragment.miner, vec![binary])?;
											T::MinerControl::sub_miner_service_space(&fragment.miner, FRAGMENT_SIZE)?;
										} else {
											T::MinerControl::unlock_space_direct(&fragment.miner, FRAGMENT_SIZE)?;
										}
									}


									fragment.tag = Some(now);
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
		// FOR TEST
		#[pallet::call_index(20)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
		pub fn root_clear_failed_count(origin: OriginFor<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;

			for (miner, _) in <TaskFailedCount<T>>::iter() {
				<TaskFailedCount<T>>::remove(&miner);
			}

			Ok(())
		}

		#[pallet::call_index(21)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
		pub fn miner_clear_failed_count(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			<TaskFailedCount<T>>::remove(&sender);

			Ok(())
		}

		#[pallet::call_index(22)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
		pub fn root_clear_file(origin: OriginFor<T>, owner: AccountOf<T>, file_hash: Hash) -> DispatchResult {
			let sender = ensure_root(origin)?;

			let file = <File<T>>::try_get(&file_hash).map_err(|_| Error::<T>::NonExistent)?;
			Self::bucket_remove_file(&file_hash, &owner, &file)?;
			Self::remove_user_hold_file_list(&file_hash, &owner)?;
			<File<T>>::remove(file_hash);

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
		let weight: Weight = Weight::zero();
		Ok(weight)
	}
}

impl<T: Config> BlockNumberProvider for Pallet<T> {
	type BlockNumber = BlockNumberFor<T>;

	fn current_block_number() -> Self::BlockNumber {
		<frame_system::Pallet<T>>::block_number()
	}
}