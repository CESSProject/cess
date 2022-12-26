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
	schedule::{Anon as ScheduleAnon, DispatchTime, Named as ScheduleNamed},
	Currency, ExistenceRequirement::AllowDeath, FindAuthor, Randomness, ReservableCurrency,
	StorageVersion,
};

pub use pallet::*;
#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod weights;
// pub mod migrations;

mod types;
pub use types::*;

mod constants;
pub use constants::*;

mod impls;
pub use impls::*;

use sp_std::str::FromStr;

use codec::{Decode, Encode};
use frame_support::{
	ensure,
	dispatch::{DispatchResult, Dispatchable},
	pallet_prelude::*, 
	transactional, 
	weights::Weight, 
	PalletId,
	traits::schedule,
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use cp_cess_common::*;
use cp_scheduler_credit::SchedulerCreditCounter;
use sp_runtime::{
	traits::{
		AccountIdConversion, BlockNumberProvider, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub,
		SaturatedConversion,
	},
	RuntimeDebug,
};
use sp_std::{convert::TryInto, prelude::*, str};
use sp_application_crypto::{
	ecdsa::{Signature, Public}, 
	
};
use sp_io::hashing::sha2_256;
use serde_json::Value;
pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::traits::Get;
	use pallet_file_map::ScheduleFind;
	use pallet_sminer::MinerControl;
	use pallet_oss::OssFindAuthor;
	//pub use crate::weights::WeightInfo;
	use frame_system::ensure_signed;
	use cp_cess_common::Hash;

	#[pallet::config]
	pub trait Config: frame_system::Config + sp_std::fmt::Debug {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		type WeightInfo: WeightInfo;

		type SScheduler: ScheduleNamed<Self::BlockNumber, Self::SProposal, Self::SPalletsOrigin>;

		type AScheduler: ScheduleAnon<Self::BlockNumber, Self::SProposal, Self::SPalletsOrigin>;
		/// Overarching type of all pallets origins.
		type SPalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;
		/// The SProposal.
		type SProposal: Parameter + Dispatchable<Origin = Self::Origin> + From<Call<Self>>;

		type Call: From<Call<Self>>;
		//Find the consensus of the current block
		type FindAuthor: FindAuthor<Self::AccountId>;
		//Used to find out whether the schedule exists
		type Scheduler: ScheduleFind<Self::AccountId>;
		//It is used to control the computing power and space of miners
		type MinerControl: MinerControl<Self::AccountId, <<Self as pallet::Config>::Currency as Currency<Self::AccountId>>::Balance>;
		//Interface that can generate random seeds
		type MyRandomness: Randomness<Option<Self::Hash>, Self::BlockNumber>;
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
		//User defined name length limit
		#[pallet::constant]
		type NameStrLimit: Get<u32> + Clone + Eq + PartialEq;
		//In order to enable users to store unlimited number of files,
		//a large number is set as the boundary of BoundedVec.
		#[pallet::constant]
		type FileListLimit: Get<u32> + Clone + Eq + PartialEq;
		//Maximum number of containers that users can create
		#[pallet::constant]
		type BucketLimit: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type FrozenDays: Get<BlockNumberOf<Self>> + Clone + Eq + PartialEq;
		//Minimum length of bucket name
		#[pallet::constant]
		type MinLength: Get<u32> + Clone + Eq + PartialEq;

		type CreditCounter: SchedulerCreditCounter<Self::AccountId>;
		//Used to confirm whether the origin is authorized
		type OssFindAuthor: OssFindAuthor<Self::AccountId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		//file uploaded.
		FileUpload { acc: AccountOf<T>, file_hash: Hash },
		// User buy package event
		BuySpace { acc: AccountOf<T>, storage_capacity: u128, spend: BalanceOf<T> },
		// Expansion Space
		ExpansionSpace { acc: AccountOf<T>, expansion_space: u128, fee: BalanceOf<T> },
		// Package upgrade
		RenewalSpace { acc: AccountOf<T>, renewal_days: u32, fee: BalanceOf<T> },
		// File deletion event
		DeleteFile { operator:AccountOf<T>, owner: AccountOf<T>, file_hash: Hash },
		// Filler chain success event
		FillerUpload { acc: AccountOf<T>, file_size: u64 },
		// File recovery
		RecoverFile { acc: AccountOf<T>, file_hash: [u8; 68] },
		// The miner cleaned up an invalid file event
		ClearInvalidFile { acc: AccountOf<T>, file_hash: Hash },
		// Event to successfully create a bucket
		CreateBucket { operator: AccountOf<T>, owner: AccountOf<T>, bucket_name: Vec<u8>},
		// Successfully delete the bucket event
		DeleteBucket { operator: AccountOf<T>, owner: AccountOf<T>, bucket_name: Vec<u8>},
		// Deal declaration success event
		UploadDeal { user: AccountOf<T>, assigned: AccountOf<T>, file_hash: Hash },
		// Event of file upload failure
		UploadDealFailed {user: AccountOf<T>, file_hash: Hash },
		// Reassign events responsible for consensus for orders
		ReassignedDeal { user: AccountOf<T>, assigned: AccountOf<T>, file_hash: Hash },
		// Miner Upload Autonomous File
		UploadAutonomyFile { user: AccountOf<T>, file_hash: Hash, file_size: u64 },
		// Miner Delete Autonomous File
		DeleteAutonomyFile { user: AccountOf<T>, file_hash: Hash },
		// Fly upload file event
		FlyUpload { operator: AccountOf<T>, owner: AccountOf<T>, file_hash: Hash },
		// Miner exit
		MinerExit { acc: AccountOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		FileExistent,
		//file doesn't exist.
		FileNonExistent,
		//overflow.
		Overflow,
		//When the user uploads a file, the purchased space is not enough
		InsufficientStorage,
		//Internal developer usage error
		WrongOperation,
		//haven't bought space at all
		NotPurchasedSpace,

		PurchasedSpace,
		//Expired storage space
		LeaseExpired,

		LeaseFreeze,
		//Exceeded the maximum amount expected by the user
		ExceedExpectations,

		ConversionError,

		InsufficientBalance,

		AlreadyRepair,

		NotOwner,

		AlreadyReceive,

		AlreadyExist,

		NotQualified,

		UserNotDeclared,
		//Signature error of offline working machine
		NoLocalAcctForSigning,
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
		//When filling and sealing the order, the rule is not met.
		SubStandard,
		//verify error
		VerifyFailed,
		//Conversion binary error
		BinaryError,
		//Insufficient space left
		InsufficientLeft,
		//Analyzing the Errors in the Original Signature of Miners
		ParseError,
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
	#[pallet::getter(fn autonomy_file)]
	pub(super) type AutonomyFile<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountOf<T>,
		Blake2_128Concat,
		Hash,
		AutonomyFileInfo<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn invalid_file)]
	pub(super) type InvalidFile<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, BoundedVec<Hash, T::InvalidLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn user_owned_space)]
	pub(super) type UserOwnedSpace<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, OwnedSpaceDetails<T>>;

	#[pallet::storage]
	#[pallet::getter(fn unit_price)]
	pub(super) type UnitPrice<T: Config> = StorageValue<_, BalanceOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn unique_hash_map)]
	pub(super) type UniqueHashMap<T: Config> = StorageMap<_, Blake2_128Concat, Hash, bool>;

	#[pallet::storage]
	#[pallet::getter(fn deal_map)]
	pub(super) type DealMap<T: Config> = StorageMap<_, Blake2_128Concat, Hash, DealInfo<T>>;

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

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		// price / gib / 30days
		pub price: BalanceOf<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				price: 30u32.saturated_into(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			UnitPrice::<T>::put(self.price);
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberOf<T>> for Pallet<T> {
		//Used to calculate whether it is implied to submit spatiotemporal proof
		//Cycle every 7.2 hours
		//When there is an uncommitted space-time certificate, the corresponding miner will be
		// punished and the corresponding data segment will be removed
		fn on_initialize(now: BlockNumberOf<T>) -> Weight {
			let number: u128 = now.saturated_into();
			let block_oneday: BlockNumberOf<T> = <T as pallet::Config>::OneDay::get();
			let oneday: u32 = block_oneday.saturated_into();
			let mut weight: Weight = Default::default();
			if number % oneday as u128 == 0 {
				log::info!("Start lease expiration check");
				for (acc, info) in <UserOwnedSpace<T>>::iter() {
					weight = weight.saturating_add(T::DbWeight::get().reads(1 as Weight));
					if now > info.deadline {
						let frozen_day: BlockNumberOf<T> = <T as pallet::Config>::FrozenDays::get();
						if now > info.deadline + frozen_day {
							log::info!("clear user:#{}'s files", number);
							let result = T::MinerControl::sub_purchased_space(info.total_space);
							match result {
								Ok(()) => log::info!("sub purchased space success"),
								Err(e) => log::error!("failed sub purchased space: {:?}", e),
							};
							weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
							let result = Self::clear_expired_file(&acc);
							let weight_temp = match result {
								Ok(weight) => {
									log::info!("clear expired file success");
									weight
								},
								Err(e) => {
									log::error!("failed clear expired file: {:?}", e);
									0
						 		},
							};
							weight = weight.saturating_add(weight_temp);
						} else {
							if info.state != SpaceState::Frozen {
								let result = <UserOwnedSpace<T>>::try_mutate(
									&acc,
									|s_opt| -> DispatchResult {
										let s = s_opt
											.as_mut()
											.ok_or(Error::<T>::NotPurchasedSpace)?;
										s.state = SpaceState::Frozen;

										Ok(())
									},
								);
								match result {
									Ok(()) => log::info!("user space frozen: #{}", number),
									Err(e) => log::error!("frozen failed: {:?}", e),
								}
								weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
							}
						}
					}
				}
				log::info!("End lease expiration check");
			}
			weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn exit_miner(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let _ = Self::clear_miner_idle_file(&sender);
			let _ = Self::clear_miner_autonomy_file(&sender);
			let _ = T::MinerControl::miner_exit(sender.clone());
		
			Self::deposit_event(Event::<T>::MinerExit { acc: sender });
			
			Ok(())
		}

		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn upload_deal(
			origin: OriginFor<T>,
			file_hash: Hash,
			file_size: u64,
			slices: Vec<Hash>,
			user_details: Details<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			// Check whether the signature source has permission.
			ensure!(Self::check_permission(sender.clone(), user_details.user.clone()), Error::<T>::NoPermission);
			//Check whether the bucket name complies with the rules
			ensure!(user_details.bucket_name.len() >= 3 && user_details.bucket_name.len() <= 63, Error::<T>::LessMinLength);
			//Check whether the file size is a multiple of 512mib
			ensure!(file_size % SLICE_DEFAULT_BYTE as u64 == 0, Error::<T>::SubStandard);
			// Check whether the whole network is unique, 
			// and insert the data if it is unique.
			Self::insert_unique_hash(&file_hash)?;
			for slice_hash in &slices {
				Self::insert_unique_hash(slice_hash)?;
			}
			// Distributed random scheduling
			let seed: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();
			let index = Self::generate_random_number(seed)?;
			let scheduler = T::Scheduler::get_random_scheduler(index as usize)?;
			// Create Timed Tasks
			let survival_block = seed.checked_add(1200).ok_or(Error::<T>::Overflow)?;
			let scheduler_id: Vec<u8> = file_hash.0.to_vec();
			T::SScheduler::schedule_named(
					scheduler_id,
					DispatchTime::At(survival_block.saturated_into()),
					Option::None,
					schedule::HARD_DEADLINE,
					frame_system::RawOrigin::Root.into(),
					Call::reassign_deal{file_hash: file_hash.clone()}.into(),
			).map_err(|_| Error::<T>::Unexpected)?;
			// Judge whether the space is enough and lock the space.
			Self::lock_space(&user_details.user, BACKUP_COUNT as u128 * slices.len() as u128 * SLICE_DEFAULT_BYTE, (survival_block * DEAL_EXCUTIVE_COUNT_MAX as u32).into())?;
			// create new deal.
			let scheduler_id: [u8; 64] = Hash::slice_to_array_64(&file_hash.0).map_err(|_| Error::<T>::ConversionError)?;
			let deal = DealInfo::<T> {
				file_size: file_size,
				user_details: user_details.clone(),
				slices: slices.try_into().map_err(|_| Error::<T>::BoundedVecError)?,
				backups: Default::default(),
				scheduler: scheduler.clone(),
				requester: user_details.user.clone(),
				state: OrderState::Assigned,
				survival_block: survival_block.saturated_into(),
				time_task: scheduler_id,
				executive_counts: 1,
			};

			DealMap::<T>::insert(&file_hash, deal);
			Self::deposit_event(Event::<T>::UploadDeal { user: user_details.user.clone(), assigned: scheduler, file_hash });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn reassign_deal(origin: OriginFor<T>, file_hash: Hash) -> DispatchResult {
			let _ = ensure_root(origin)?;

			let mut deal = DealMap::<T>::try_get(&file_hash).map_err(|_| Error::<T>::Unexpected)?;

			if deal.executive_counts == DEAL_EXCUTIVE_COUNT_MAX {
				Self::unlock_space_free(&deal.user_details.user, BACKUP_COUNT as u128 * deal.slices.len() as u128 * SLICE_DEFAULT_BYTE)?;
				//clear deal
				DealMap::<T>::remove(&file_hash);
				//clear slice hash and file hash from unique hash map
				UniqueHashMap::<T>::remove(&file_hash);
				for slice_hash in deal.slices {
					UniqueHashMap::<T>::remove(slice_hash);
				}

				Self::deposit_event(Event::<T>::UploadDealFailed {user: deal.user_details.user, file_hash });
			} else {
				deal.executive_counts = deal.executive_counts.checked_add(1).ok_or(Error::<T>::Overflow)?;
				// Reassign Schedule
				let seed: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();
				let index = Self::generate_random_number(seed)?;
				let scheduler = T::Scheduler::get_random_scheduler(index as usize)?;
				deal.scheduler = scheduler.clone();
				// Initialize deal.
				deal.backups = Default::default();
				// Reassign timing task
				let scheduler_id: Vec<u8> = file_hash.0.to_vec();
				T::SScheduler::schedule_named(
					scheduler_id,
					DispatchTime::At(deal.survival_block),
					Option::None,
					schedule::HARD_DEADLINE,
					frame_system::RawOrigin::Root.into(),
					Call::reassign_deal{file_hash: file_hash.clone()}.into(),
				).map_err(|_| Error::<T>::Unexpected)?;
				// re-insert deal
				DealMap::<T>::insert(&file_hash, deal.clone());
				Self::deposit_event(Event::<T>::ReassignedDeal { user: deal.user_details.user, assigned: scheduler, file_hash });
			}

			Ok(())
		}

		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn pack_deal(
			origin: OriginFor<T>,
			file_hash: Hash,
			slice_summary: [Vec<SliceSummary<T>>; 3],
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			//Judge whether the order exists, 
			//and get the order if it exists
			let deal = DealMap::<T>::try_get(&file_hash).map_err(|_| Error::<T>::NonExistent)?;
			//Judge whether it is responsible consensus
			ensure!(deal.scheduler == sender, Error::<T>::NoPermission);
			//Judge whether the length of sealing information meets
			ensure!(slice_summary[0].len() == deal.slices.len(), Error::<T>::SubStandard);
			ensure!(slice_summary[1].len() == deal.slices.len(), Error::<T>::SubStandard);
			ensure!(slice_summary[2].len() == deal.slices.len(), Error::<T>::SubStandard);
			let mut index = 0;
			let _ = index.checked_add(&deal.slices.len()).ok_or(Error::<T>::Overflow)?;
			//Verify all slice backups and sgx signatures
			for hash in deal.slices {
				let message1: Value = serde_json::from_slice(&slice_summary[0][index].message).map_err(|_| Error::<T>::SubStandard)?;
				let message2: Value = serde_json::from_slice(&slice_summary[1][index].message).map_err(|_| Error::<T>::SubStandard)?;
				let message3: Value = serde_json::from_slice(&slice_summary[2][index].message).map_err(|_| Error::<T>::SubStandard)?;

				if let Value::String(slice_hash1) = &message1["sliceHash"] {
					let slice_hash1 = Hash::new(slice_hash1.as_bytes()).map_err(|_| Error::<T>::ConversionError)?;
					ensure!(slice_hash1 == hash, Error::<T>::SubStandard);
				} else {
					Err(Error::<T>::SubStandard)?;
				}

				if let Value::String(slice_hash2) = &message2["sliceHash"] {
					let slice_hash2 = Hash::new(slice_hash2.as_bytes()).map_err(|_| Error::<T>::ConversionError)?;
					ensure!(slice_hash2 == hash, Error::<T>::SubStandard);
				} else {
					Err(Error::<T>::SubStandard)?;
				}

				if let Value::String(slice_hash3) = &message3["sliceHash"] {
					let slice_hash3 = Hash::new(slice_hash3.as_bytes()).map_err(|_| Error::<T>::ConversionError)?;
					ensure!(slice_hash3 == hash, Error::<T>::SubStandard);
				} else {
					Err(Error::<T>::SubStandard)?;
				}
		
				//verify signature
				let pk = T::MinerControl::get_public(&slice_summary[0][index].miner)?;
				let result = sp_io::crypto::ecdsa_verify_prehashed(&slice_summary[0][index].signature, &sha2_256(&&slice_summary[0][index].message), &pk);
				ensure!(result, Error::<T>::VerifyFailed);

				let pk = T::MinerControl::get_public(&slice_summary[1][index].miner)?;
				let result = sp_io::crypto::ecdsa_verify_prehashed(&slice_summary[1][index].signature, &sha2_256(&slice_summary[1][index].message), &pk);
				ensure!(result, Error::<T>::VerifyFailed);

				let pk = T::MinerControl::get_public(&slice_summary[2][index].miner)?;
				let result = sp_io::crypto::ecdsa_verify_prehashed(&slice_summary[2][index].signature, &sha2_256(&slice_summary[2][index].message), &pk);
				ensure!(result, Error::<T>::VerifyFailed);

				index = index + 1;
			}
			//Start to replace miners' files and extract backup meta information.
			let mut backups: [Backup<T>; 3] = Default::default();
			backups[0].index = 1;
			for slice in slice_summary[0].clone() {
				let slice_info = SliceInfo::<T>::serde_json_parse(slice.miner.clone(), slice.message.to_vec())?;
				Self::replace_file(slice_info.miner_acc.clone(), slice_info.slice_hash.clone())?;
				backups[0].slices.try_push(slice_info).map_err(|_| Error::<T>::Overflow)?;
			}

			backups[1].index = 2;
			for slice in slice_summary[1].clone() {
				let slice_info = SliceInfo::<T>::serde_json_parse(slice.miner.clone(), slice.message.to_vec())?;
				Self::replace_file(slice_info.miner_acc.clone(), slice_info.slice_hash.clone())?;
				backups[1].slices.try_push(slice_info).map_err(|_| Error::<T>::Overflow)?;
			}

			backups[2].index = 3;
			for slice in slice_summary[2].clone() {
				let slice_info = SliceInfo::<T>::serde_json_parse(slice.miner.clone(), slice.message.to_vec())?;
				Self::replace_file(slice_info.miner_acc.clone(), slice_info.slice_hash.clone())?;
				backups[2].slices.try_push(slice_info).map_err(|_| Error::<T>::Overflow)?;
			}
			// Create file meta information
			let mut user_details_list: BoundedVec<Details<T>, T::StringLimit> = Default::default();
			user_details_list.try_push(deal.user_details.clone()).map_err(|_| Error::<T>::BoundedVecError)?;
			let file = FileInfo::<T> {
				file_size: deal.file_size,
				file_state: FILE_ACTIVE.as_bytes().to_vec().try_into().map_err(|_| Error::<T>::BoundedVecError)?,
				user_details_list: user_details_list,
				backups: backups,
			};
			// Judge whether the bucket exists. If not, create a bucket
			Self::file_into_bucket(&deal.user_details.user, deal.user_details.bucket_name.clone(), file_hash.clone())?;
			//insert file
			<File<T>>::insert(&file_hash, file);
			
			let file_info = UserFileSliceInfo { file_hash: file_hash.clone(), file_size: deal.file_size };
			<UserHoldFileList<T>>::try_mutate(&deal.user_details.user, |v| -> DispatchResult {
				v.try_push(file_info).map_err(|_| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;
			// unlocked space, add used space
			Self::unlock_space_used(&deal.user_details.user, (BACKUP_COUNT as u64 * deal.file_size).into())?;
			// scheduler increase credit points
			Self::record_uploaded_files_size(&sender, deal.file_size)?;
			// Cancle timed task
			let _ = T::SScheduler::cancel_named(deal.time_task.to_vec()).map_err(|_| Error::<T>::Unexpected)?;

			Self::deposit_event(Event::<T>::FileUpload { acc: deal.user_details.user, file_hash });

			Ok(())
		}

		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn fly_upload(
			origin: OriginFor<T>,
			file_hash: Hash,
			user_details: Details<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(Self::check_permission(sender.clone(), user_details.user.clone()), Error::<T>::NoPermission);
			ensure!(user_details.bucket_name.len() >= 3 && user_details.bucket_name.len() <= 63, Error::<T>::LessMinLength);

			ensure!(File::<T>::contains_key(&file_hash), Error::<T>::NonExistent);
			//Check whether it is already the holder.
			let mut file_list = <UserHoldFileList<T>>::get(&user_details.user);
			for file in file_list.iter() {
				if file.file_hash == file_hash {
					Err(Error::<T>::AlreadyExist)?;
				}
			}

			let file_size = <File<T>>::try_mutate(&file_hash, |opt_file| -> Result<u64, DispatchError> {
				let file = opt_file.as_mut().ok_or(Error::<T>::Unexpected)?;
				
				file.user_details_list.try_push(user_details.clone()).map_err(|_| Error::<T>::BoundedVecError)?;

				Ok(file.file_size)
			})?;

			let file_info = UserFileSliceInfo { file_hash, file_size };

			file_list.try_push(file_info).map_err(|_| Error::<T>::StorageLimitReached)?;

			Self::update_user_space(user_details.user.clone(), 1, file_size.into())?;
			
			Self::file_into_bucket(&user_details.user, user_details.bucket_name, file_hash.clone())?;

			Self::deposit_event(Event::<T>::FlyUpload { operator: sender, owner: user_details.user, file_hash: file_hash });

			Ok(())
		}


		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn update_price(origin: OriginFor<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;
			let default_price: BalanceOf<T> = 30u32.saturated_into();
			UnitPrice::<T>::put(default_price);

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
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::ownership_transfer())]
		pub fn ownership_transfer(
			origin: OriginFor<T>,
			owner_bucket_name: BoundedVec<u8, T::NameStrLimit>,
			target_brief: Details<T>,
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
			Self::update_user_space(target_brief.user.clone(), 1, file.file_size.into())?;
			//Increase the ownership of the file for target acc
			<File<T>>::try_mutate(&file_hash, |file_opt| -> DispatchResult {
				let file = file_opt.as_mut().ok_or(Error::<T>::FileNonExistent)?;
				file.user_details_list.try_push(target_brief.clone()).map_err(|_| Error::<T>::BoundedVecError)?;
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

			for filler in filler_list.iter() {
				ensure!(filler.filler_size as u128 == SLICE_DEFAULT_BYTE, Error::<T>::SubStandard);
				if <FillerMap<T>>::contains_key(&miner, filler.filler_hash.clone()) {
					Err(Error::<T>::FileExistent)?;
				}
				Self::insert_unique_hash(&filler.filler_hash)?;
				<FillerMap<T>>::insert(miner.clone(), filler.filler_hash.clone(), filler);
				let binary = filler.filler_hash.binary().map_err(|_| Error::<T>::BinaryError)?;
				T::MinerControl::insert_idle_bloom(&miner, binary)?;
			}

			let power = M_BYTE
				.checked_mul(8)
				.ok_or(Error::<T>::Overflow)?
				.checked_mul(filler_list.len() as u128)
				.ok_or(Error::<T>::Overflow)?;

			T::MinerControl::add_idle_space(miner, power)?;
			

			Self::record_uploaded_fillers_size(&sender, &filler_list)?;

			Self::deposit_event(Event::<T>::FillerUpload { acc: sender, file_size: power as u64 });
			Ok(())
		}

		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn upload_autonomy_file (
			origin: OriginFor<T>,
			file_hash: Hash,
			file_size: u64,
			slices: Vec<Hash>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::insert_unique_hash(&file_hash)?;

			ensure!(file_size % SLICE_DEFAULT_BYTE as u64 == 0, Error::<T>::SubStandard);
			
			for slice_hash in slices.iter() {
				Self::insert_unique_hash(&slice_hash)?;

				let binary = slice_hash.binary().map_err(|_| Error::<T>::BinaryError)?;

				T::MinerControl::insert_autonomy_bloom(&sender, binary)?;
			}

			T::MinerControl::add_autonomy_space(sender.clone(), slices.len() as u128 * SLICE_DEFAULT_BYTE)?;

			let file_info = AutonomyFileInfo::<T> {
				file_hash: file_hash.clone(),
				file_size: file_size,
				slices: slices.try_into().map_err(|_| Error::<T>::BoundedVecError)?,
				miner_acc: sender.clone(),
			};

			AutonomyFile::<T>::insert(&sender, &file_hash, file_info);

			Self::deposit_event(Event::<T>::UploadAutonomyFile { user: sender, file_hash, file_size});

			Ok(())
		}

		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn delete_autonomy_file(
			origin: OriginFor<T>,
			file_hash: Hash,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			Self::challenge_clear_autonomy(sender.clone(), file_hash.clone())?;

			Self::deposit_event(Event::<T>::DeleteAutonomyFile { user: sender, file_hash: file_hash });

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
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::delete_file())]
		pub fn delete_file(origin: OriginFor<T>, owner: AccountOf<T>, fileid: Hash) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<File<T>>::contains_key(fileid.clone()), Error::<T>::FileNonExistent);
			ensure!(Self::check_permission(sender.clone(), owner.clone()), Error::<T>::NoPermission);

			let file = <File<T>>::try_get(&fileid).map_err(|_| Error::<T>::NonExistent)?; //read 1

			let mut result: bool = false;
			for user_details_temp in file.user_details_list.iter() {
				if user_details_temp.user == owner.clone() {
					//The above has been judged. Unwrap will be performed only if the key exists
					let _ = Self::clear_bucket_file(&fileid, &owner, &user_details_temp.bucket_name)?;
					let _ = Self::clear_user_file(fileid, &owner, file.user_details_list.len() > 1)?;

					result = true;
					break;
				}
			}
			ensure!(result, Error::<T>::NotOwner);
			Self::deposit_event(Event::<T>::DeleteFile { operator: sender, owner, file_hash: fileid });
			Ok(())
		}

		//**********************************************************************************************************************************************
		//************************************************************Storage space lease***********
		//************************************************************Storage **********************
		//************************************************************Storage **********************
		//************************************************************Storage ********
		//**********************************************************************************************************************************************
		/// Transaction of user purchasing space.
		///
		/// The dispatch origin of this call must be Signed.
		///
		/// Parameters:
		/// - `gib_count`: Quantity of several gibs purchased.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::buy_space())]
		pub fn buy_space(origin: OriginFor<T>, gib_count: u32) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!<UserOwnedSpace<T>>::contains_key(&sender), Error::<T>::PurchasedSpace);
			let space= G_BYTE.checked_mul(gib_count as u128).ok_or(Error::<T>::Overflow)?;
			let unit_price = <UnitPrice<T>>::try_get()
				.map_err(|_e| Error::<T>::BugInvalid)?;

			Self::new_puchased_space(sender.clone(), space, 30)?;
			T::MinerControl::add_purchased_space(space)?;
			let price: BalanceOf<T> = unit_price
				.checked_mul(&gib_count.saturated_into())
				.ok_or(Error::<T>::Overflow)?;
			ensure!(
				<T as pallet::Config>::Currency::can_slash(&sender, price.clone()),
				Error::<T>::InsufficientBalance
			);
			let acc = T::FilbakPalletId::get().into_account();
			<T as pallet::Config>::Currency::transfer(&sender, &acc, price.clone(), AllowDeath)?;

			Self::deposit_event(Event::<T>::BuySpace { acc: sender, storage_capacity: space, spend: price });
			Ok(())
		}
		/// Upgrade package (expansion of storage space)
		///
		/// It can only be called when the package has been purchased,
		/// And the upgrade target needs to be higher than the current package.
		///
		/// Parameters:
		/// - `gib_count`: Additional purchase quantity of several gibs.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::expansion_space())]
		pub fn expansion_space(origin: OriginFor<T>, gib_count: u32) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let cur_owned_space = <UserOwnedSpace<T>>::try_get(&sender)
				.map_err(|_e| Error::<T>::NotPurchasedSpace)?;
			let now = <frame_system::Pallet<T>>::block_number();
			ensure!(now < cur_owned_space.deadline, Error::<T>::LeaseExpired);
			ensure!(
				cur_owned_space.state != SpaceState::Frozen,
				Error::<T>::LeaseFreeze
			);
			// The unit price recorded in UnitPrice is the unit price of one month.
			// Here, the daily unit price is calculated.
			let day_unit_price = <UnitPrice<T>>::try_get()
				.map_err(|_e| Error::<T>::BugInvalid)?
				.checked_div(&30u32.saturated_into()).ok_or(Error::<T>::Overflow)?;
			let space = G_BYTE.checked_mul(gib_count as u128).ok_or(Error::<T>::Overflow)?;
			//Calculate remaining days.
			let block_oneday: BlockNumberOf<T> = <T as pallet::Config>::OneDay::get();
			let diff_block = cur_owned_space.deadline.checked_sub(&now).ok_or(Error::<T>::Overflow)?;
			let mut remain_day: u32 = diff_block
				.checked_div(&block_oneday)
				.ok_or(Error::<T>::Overflow)?
				.saturated_into();
			if diff_block % block_oneday != 0u32.saturated_into() {
				remain_day = remain_day
					.checked_add(1)
					.ok_or(Error::<T>::Overflow)?
					.saturated_into();
			}
			//Calculate the final price difference to be made up.
			let price: BalanceOf<T> = day_unit_price
				.checked_mul(&gib_count.saturated_into())
				.ok_or(Error::<T>::Overflow)?
				.checked_mul(&remain_day.saturated_into())
				.ok_or(Error::<T>::Overflow)?
				.try_into()
				.map_err(|_e| Error::<T>::Overflow)?;
			//Judge whether the balance is sufficient
			ensure!(
				<T as pallet::Config>::Currency::can_slash(&sender, price.clone()),
				Error::<T>::InsufficientBalance
			);

			let acc: AccountOf<T> = T::FilbakPalletId::get().into_account();
			T::MinerControl::add_purchased_space(
				space,
			)?;

			Self::expension_puchased_package(sender.clone(), space)?;

			<T as pallet::Config>::Currency::transfer(&sender, &acc, price.clone(), AllowDeath)?;

			Self::deposit_event(Event::<T>::ExpansionSpace {
				acc: sender,
				expansion_space: space,
				fee: price,
			});
			Ok(())
		}
		/// Package renewal
		///
		/// Currently, lease renewal only supports single month renewal
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::renewal_space())]
		pub fn renewal_space(origin: OriginFor<T>, days: u32) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let cur_owned_space = <UserOwnedSpace<T>>::try_get(&sender)
				.map_err(|_e| Error::<T>::NotPurchasedSpace)?;

			let days_unit_price = <UnitPrice<T>>::try_get()
				.map_err(|_e| Error::<T>::BugInvalid)?
				.checked_div(&30u32.saturated_into())
				.ok_or(Error::<T>::Overflow)?;
			let gib_count = cur_owned_space.total_space.checked_div(G_BYTE).ok_or(Error::<T>::Overflow)?;
			let price: BalanceOf<T> = days_unit_price
				.checked_mul(&gib_count.saturated_into())
				.ok_or(Error::<T>::Overflow)?
				.checked_mul(&days.saturated_into())
				.ok_or(Error::<T>::Overflow)?
				.try_into()
				.map_err(|_e| Error::<T>::Overflow)?;
			ensure!(
				<T as pallet::Config>::Currency::can_slash(&sender, price.clone()),
				Error::<T>::InsufficientBalance
			);
			let acc = T::FilbakPalletId::get().into_account();
			<T as pallet::Config>::Currency::transfer(&sender, &acc, price.clone(), AllowDeath)?;
			Self::update_puchased_package(sender.clone(), days)?;
			Self::deposit_event(Event::<T>::RenewalSpace {
				acc: sender,
				renewal_days: days,
				fee: price,
			});
			Ok(())
		}
		/// Clean up invalid idle files
		///
		/// When the idle documents are replaced or proved to be invalid,
		/// they will become invalid documents and need to be cleared by the miners
		///
		/// Parameters:
		/// - `file_hash`: Invalid file hash
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
		// #[transactional]
		// #[pallet::weight(<T as pallet::Config>::WeightInfo::recover_file())]
		// pub fn recover_file(
		// 	origin: OriginFor<T>,
		// 	shard_id: [u8; 68],
		// 	slice_info: SliceInfo<T>,
		// 	avail: bool,
		// ) -> DispatchResult {
		// 	//Vec to BoundedVec
		// 	let sender = ensure_signed(origin)?;
		// 	//Get fileid from shardid,
		// 	let file_hash = Hash::from_shard_id(&shard_id).map_err(|_| Error::<T>::ConvertHashError)?;
		// 	//Delete the corresponding recovery slice request pool
		// 	<FileRecovery<T>>::try_mutate(&sender, |o| -> DispatchResult {
		// 		o.retain(|x| *x != shard_id);
		// 		Ok(())
		// 	})?;
		// 	if !<File<T>>::contains_key(&file_hash) {
		// 		Err(Error::<T>::FileNonExistent)?;
		// 	}
		// 	if avail {
		// 		<File<T>>::try_mutate(&file_hash, |opt| -> DispatchResult {
		// 			let o = opt.as_mut().unwrap();
		// 			o.slice_info
		// 				.try_push(slice_info)
		// 				.map_err(|_| Error::<T>::StorageLimitReached)?;
		// 			o.file_state = FILE_ACTIVE
		// 				.as_bytes()
		// 				.to_vec()
		// 				.try_into()
		// 				.map_err(|_e| Error::<T>::BoundedVecError)?;
		// 			Ok(())
		// 		})?;
		// 	} else {
		// 		let _weight = Self::clear_file(file_hash)?;
		// 	}
		// 	Self::deposit_event(Event::<T>::RecoverFile { acc: sender, file_hash: shard_id });
		// 	Ok(())
		// }

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
			ensure!(name.len() >= T::MinLength::get() as usize, Error::<T>::LessMinLength);
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
				Self::clear_user_file(*file_hash, &owner, file.user_details_list.len() > 1)?;
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
		fn insert_unique_hash(file_hash: &Hash) -> DispatchResult {
			ensure!(Self::check_is_unique(file_hash), Error::<T>::AlreadyExist);
			<UniqueHashMap<T>>::insert(file_hash, true);
			Ok(())
		}

		fn remove_unique_hash(file_hash: &Hash) -> DispatchResult {
			ensure!(!Self::check_is_unique(file_hash), Error::<T>::NonExistent);
			<UniqueHashMap<T>>::remove(file_hash);
			Ok(())
		}

		// Exist return false, NonExistent return true
		fn check_is_unique(file_hash: &Hash) -> bool {
			!<UniqueHashMap<T>>::contains_key(file_hash)
		}

		fn lock_space(acc: &AccountOf<T>, file_size: u128, survival_block: BlockNumberOf<T>) -> DispatchResult {
			<UserOwnedSpace<T>>::try_mutate(acc, |opt_space| -> DispatchResult {
				let space = opt_space.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;
				//File upload is prohibited in the frozen space
				if space.state == SpaceState::Frozen {
					Err(Error::<T>::LeaseFreeze)?;
				}
				//Space remaining expiration time must be greater than survival block
				if space.deadline > survival_block {
					Err(Error::<T>::InsufficientLeft)?;
				}

				ensure!(space.free_space > file_size, Error::<T>::InsufficientStorage);

				space.free_space = space.free_space.checked_sub(file_size).ok_or(Error::<T>::Overflow)?;
				space.locked_space = space.locked_space.checked_add(file_size).ok_or(Error::<T>::Overflow)?;

				Ok(())
			})?;

			Ok(())
		}

		fn unlock_space_free(acc: &AccountOf<T>, file_size: u128) -> DispatchResult {
			<UserOwnedSpace<T>>::try_mutate(acc, |opt_space| -> DispatchResult {
				let space = opt_space.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;

				space.free_space = space.free_space.checked_add(file_size).ok_or(Error::<T>::Overflow)?;
				space.locked_space = space.locked_space.checked_sub(file_size).ok_or(Error::<T>::Overflow)?;

				Ok(())
			})?;

			Ok(())
		}

		fn unlock_space_used(acc: &AccountOf<T>, file_size: u128) -> DispatchResult {
			<UserOwnedSpace<T>>::try_mutate(&acc, |opt_space| -> DispatchResult {
				let space = opt_space.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;
				space.used_space = space.used_space.checked_add(file_size).ok_or(Error::<T>::Overflow)?;
				space.locked_space = space.locked_space.checked_sub(file_size).ok_or(Error::<T>::Overflow)?;
				Ok(())
			})?;

			Ok(())
		}

		//Before using, you need to judge whether the name conforms to the rules
		fn file_into_bucket(user: &AccountOf<T>, bucket_name: BoundedVec<u8, T::NameStrLimit>, file_hash: Hash) -> DispatchResult {
			if !<Bucket<T>>::contains_key(user, &bucket_name) {
				let mut object_list: BoundedVec<Hash, T::FileListLimit> = Default::default();
				object_list.try_push(file_hash.clone()).map_err(|_| Error::<T>::BoundedVecError)?;
				let bucket = BucketInfo::<T> {
					total_capacity: 0,
					available_capacity: 0,
					object_num: 0,
					object_list: object_list,
					authority: Default::default(),
				};

				<Bucket<T>>::insert(user, &bucket_name, bucket);

				<UserBucketList<T>>::try_mutate(&user, |bucket_list| -> DispatchResult {
					bucket_list.try_push(bucket_name).map_err(|_| Error::<T>::BoundedVecError)?;
					Ok(())
				})?;
			} else {
				<Bucket<T>>::try_mutate(&user, &bucket_name, |opt_bucket| -> DispatchResult {
					let bucket = opt_bucket.as_mut().ok_or(Error::<T>::Unexpected)?;
					bucket.object_list.try_push(file_hash.clone()).map_err(|_| Error::<T>::BoundedVecError)?;
					Ok(())
				})?;
			}

			Ok(())
		}
		/// helper: update_puchased_package.
		///
		/// How to update the corresponding data after renewing the package.
		/// Currently, it only supports and defaults to one month.
		///
		/// Parameters:
		/// - `acc`: Account
		/// - `days`: Days of renewal
		fn update_puchased_package(acc: AccountOf<T>, days: u32) -> DispatchResult {
			<UserOwnedSpace<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
				let s = s_opt.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;
				let one_day = <T as pallet::Config>::OneDay::get();
				let now = <frame_system::Pallet<T>>::block_number();
				let sur_block: BlockNumberOf<T> =
					one_day.checked_mul(&days.saturated_into()).ok_or(Error::<T>::Overflow)?;
				if now > s.deadline {
					s.start = now;
					s.deadline = now.checked_add(&sur_block).ok_or(Error::<T>::Overflow)?;
				} else {
					s.deadline = s.deadline.checked_add(&sur_block).ok_or(Error::<T>::Overflow)?;
				}
				Ok(())
			})?;
			Ok(())
		}
		/// helper: Expand storage space.
		///
		/// Relevant data of users after updating the expansion package.
		///
		/// Parameters:
		/// - `space`: Size after expansion.
		/// - `package_type`: New package type.
		fn expension_puchased_package(
			acc: AccountOf<T>,
			space: u128,
		) -> DispatchResult {
			<UserOwnedSpace<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
				let s = s_opt.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;
				s.free_space = s.free_space.checked_add(space).ok_or(Error::<T>::Overflow)?;
				s.total_space = s.total_space.checked_add(space).ok_or(Error::<T>::Overflow)?;
				Ok(())
			})?;

			Ok(())
		}
		/// helper: Initial purchase space initialization method.
		///
		/// Purchase a package and create data corresponding to the user.
		/// UserOwnedSpace Storage.
		///
		/// Parameters:
		/// - `space`: Buy storage space size.
		/// - `month`: Month of purchase of package, It is 1 at present.
		/// - `package_type`: Package type.
		fn new_puchased_space(
			acc: AccountOf<T>,
			space: u128,
			days: u32,
		) -> DispatchResult {
			let now = <frame_system::Pallet<T>>::block_number();
			let one_day = <T as pallet::Config>::OneDay::get();
			let sur_block: BlockNumberOf<T> = one_day
				.checked_mul(&days.saturated_into())
				.ok_or(Error::<T>::Overflow)?;
			let deadline = now.checked_add(&sur_block).ok_or(Error::<T>::Overflow)?;
			let info = OwnedSpaceDetails::<T> {
				total_space: space,
				used_space: u128::MIN,
				locked_space: u128::MIN,
				free_space: space,
				start: now,
				deadline,
				state: SpaceState::Nomal,
			};
			<UserOwnedSpace<T>>::insert(&acc, info);
			Ok(())
		}

		/// helper: update user storage space.
		///
		/// Modify the corresponding data after the user uploads the file or deletes the file
		/// Modify used_space, remaining_space
		/// operation = 1, add used_space
		/// operation = 2, sub used_space
		///
		/// Parameters:
		/// - `operation`: operation type 1 or 2.
		/// - `size`: file size.
		fn update_user_space(acc: AccountOf<T>, operation: u8, size: u128) -> DispatchResult {
			match operation {
				1 => {
					<UserOwnedSpace<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
						let s = s_opt.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;
						if s.state == SpaceState::Frozen {
							Err(Error::<T>::LeaseFreeze)?;
						}
						if size > s.free_space {
							Err(Error::<T>::InsufficientStorage)?;
						}
						s.used_space =
							s.used_space.checked_add(size).ok_or(Error::<T>::Overflow)?;
						s.free_space =
							s.free_space.checked_sub(size).ok_or(Error::<T>::Overflow)?;
						Ok(())
					})?;
				},
				2 => <UserOwnedSpace<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
					let s = s_opt.as_mut().unwrap();
					s.used_space = s.used_space.checked_sub(size).ok_or(Error::<T>::Overflow)?;
					s.free_space =
						s.total_space.checked_add(size).ok_or(Error::<T>::Overflow)?;
					Ok(())
				})?,
				_ => Err(Error::<T>::WrongOperation)?,
			}
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
			let mut weight: Weight = 0;
			let file =
					<File<T>>::try_get(&file_hash).map_err(|_| Error::<T>::FileNonExistent)?; //read 1
			  weight = weight.saturating_add(T::DbWeight::get().reads(1 as Weight));
			for user_details in file.user_details_list.iter() {
				Self::update_user_space(user_details.user.clone(), 2, file.file_size.into())?; //read 1 write 1 * n
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
			}

			for backup in file.backups.iter() {
				for slice in backup.slices.iter() {
					let hash_temp = Hash::from_shard_id(&slice.shard_id).map_err(|_| Error::<T>::ConvertHashError)?;
					Self::add_invalid_file(slice.miner_acc.clone(), hash_temp)?; //read 1 write 1 * n
					weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
					T::MinerControl::sub_service_space(slice.miner_acc.clone(), slice.shard_size.into())?; //read 3 write 2
					weight = weight.saturating_add(T::DbWeight::get().reads_writes(3, 2));
				}
			}

			<File<T>>::remove(file_hash);
			Ok(weight)
		}

		pub fn clear_bucket_file(
			file_hash: &Hash,
			owner: &AccountOf<T>,
			bucket_name: &BoundedVec<u8, T::NameStrLimit>,
		) -> Result<Weight, DispatchError> {
			let mut weight: Weight = 0;
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
			let mut weight: Weight = 0;
			//If the file still has an owner, only the corresponding owner will be cleared.
			//If the owner is unique, the file meta information will be cleared.
			if is_multi {
				//read 1 write 1
				let mut file_size: u64 = 0;
				<File<T>>::try_mutate(&file_hash, |s_opt| -> DispatchResult {
					let s = s_opt.as_mut().unwrap();
					file_size = s.file_size;
					let mut index: usize = 0;
					for user_details in s.user_details_list.iter() {
						if user_details.user == user.clone() {
							break
						}
						index = index.checked_add(1).ok_or(Error::<T>::Overflow)?;
					}
					weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
					s.user_details_list.remove(index);
					Ok(())
				})?;
				Self::update_user_space(user.clone(), 2, file_size.into())?; //read 1 write 1
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
		fn replace_file(miner_acc: AccountOf<T>, slice_hash: Hash) -> DispatchResult {
			//add space
			T::MinerControl::add_service_space(miner_acc.clone(), SLICE_DEFAULT_BYTE)?;
			T::MinerControl::sub_idle_space(miner_acc.clone(), SLICE_DEFAULT_BYTE)?;
			//Modify the corresponding Blum filter.
			let (filler_hash, _) = FillerMap::<T>::iter_prefix(&miner_acc).next().ok_or(Error::<T>::Unexpected)?;
			let filler_binary = filler_hash.binary().map_err(|_| Error::<T>::BinaryError)?;
			let slice_binary = slice_hash.binary().map_err(|_| Error::<T>::BinaryError)?;

			T::MinerControl::insert_slice_update_bloom(&miner_acc, slice_binary, filler_binary)?;

			<FillerMap<T>>::remove(miner_acc.clone(), filler_hash.clone()); 

			<InvalidFile<T>>::try_mutate(&miner_acc, |o| -> DispatchResult {
				o.try_push(filler_hash).map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;
			
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

		pub fn challenge_clear_idle(acc: AccountOf<T>, file_hash: Hash) -> DispatchResult {
			ensure!(FillerMap::<T>::contains_key(&acc, &file_hash), Error::<T>::NonExistent);
			// Clean up idle files.
			FillerMap::<T>::remove(&acc, &file_hash);
			// Clear the unique tag of the whole network.
			UniqueHashMap::<T>::remove(&file_hash);
			// Reduce corresponding space.
			T::MinerControl::sub_idle_space(acc.clone(), SLICE_DEFAULT_BYTE)?;
			// Modify the Bloom Filter
			let binary = file_hash.binary().map_err(|_| Error::<T>::BinaryError)?;
			T::MinerControl::remove_idle_bloom(&acc, binary)?;

			Ok(())
		}

		pub fn challenge_clear_service(acc: AccountOf<T>, shard_id: [u8; 68]) -> DispatchResult {
			let file_hash = Hash::from_shard_id(&shard_id).map_err(|_| Error::<T>::ConvertHashError)?;

			<File<T>>::try_mutate(&file_hash, |opt_file| -> DispatchResult {
				let file = opt_file.as_mut().ok_or(Error::<T>::NonExistent)?;
				// parse backup index from shard_id
				let temp = &[shard_id[66]];
				let temp = str::from_utf8(temp).map_err(|_| Error::<T>::ConversionError)?;
				let backup_index = u8::from_str(temp).map_err(|_| Error::<T>::ConversionError)?;
				
				let mut index = 0;
				for slice in file.backups[backup_index as usize].slices.iter() {
					if slice.shard_id == shard_id {
						// Modify the Bloom Filter
						let binary = slice.slice_hash.binary().map_err(|_| Error::<T>::BinaryError)?;
						T::MinerControl::remove_service_bloom(&acc, binary)?;

						file.backups[backup_index as usize].slices.remove(index);
						break;
					}
					index = index + 1;
				}

				Ok(())
			})?;
			// Reduce corresponding space.
			T::MinerControl::sub_service_space(acc.clone(), SLICE_DEFAULT_BYTE)?;	
			
			Ok(())
		}

		pub fn challenge_clear_autonomy(acc: AccountOf<T>, file_hash: Hash) -> DispatchResult {
			let autonomy_file = AutonomyFile::<T>::try_get(&acc, &file_hash).map_err(|_| Error::<T>::NonExistent)?;

			for slice_hash in autonomy_file.slices.iter() {
				UniqueHashMap::<T>::remove(&slice_hash);
				// Modify the Bloom Filter
				let binary = slice_hash.binary().map_err(|_| Error::<T>::BinaryError)?;
				T::MinerControl::remove_autonomy_bloom(&acc, binary)?;
			}
			// Clean up idle files.
			AutonomyFile::<T>::remove(&acc, &file_hash);
			// Clear the unique tag of the whole network.
			UniqueHashMap::<T>::remove(&file_hash);
			// Reduce corresponding space.
			T::MinerControl::sub_autonomy_space(acc.clone(), autonomy_file.slices.len() as u128 * SLICE_DEFAULT_BYTE)?;

			Ok(())
		}

		// Called when the miner exits normally or is forcibly kicked out
		pub fn clear_miner_autonomy_file(acc: &AccountOf<T>) -> Weight {
			let mut weight: Weight = 0;
			for (file_hash, file) in <AutonomyFile<T>>::iter_prefix(acc) {
				for slice_hash in file.slices.iter() {
					<UniqueHashMap<T>>::remove(slice_hash);
					weight = weight.saturating_add(T::DbWeight::get().writes(1 as Weight));
				}
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
				<AutonomyFile<T>>::remove(acc, &file_hash);
			}
			weight
		}
		// Called when the miner exits normally or is forcibly kicked out
		pub fn clear_miner_idle_file(acc: &AccountOf<T>) -> Weight {
			let mut weight: Weight = 0;

			for (file_hash, _) in <FillerMap<T>>::iter_prefix(acc) {
				<UniqueHashMap<T>>::remove(&file_hash);
				<FillerMap<T>>::remove(&acc, &file_hash);
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 2));
			}

			weight 
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
				for user_details in file.user_details_list.iter() {
					if &user_details.user == acc {
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
			let mut weight: Weight = 0;
			let file_list =
				<UserHoldFileList<T>>::try_get(&acc).map_err(|_| Error::<T>::Overflow)?;
			for v in file_list.iter() {
				let file = <File<T>>::try_get(&v.file_hash).map_err(|_| Error::<T>::Overflow)?;
				let weight_temp = Self::clear_user_file(v.file_hash.clone(), acc, file.user_details_list.len() > 1)?;
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

	fn add_invalid_file(miner_acc: AccountId, file_hash: Hash) -> DispatchResult;

	fn challenge_clear_idle(acc: AccountId, file_hash: Hash) -> DispatchResult;

	fn challenge_clear_service(acc: AccountId, shard_id: [u8; 68]) -> DispatchResult;

	fn challenge_clear_autonomy(acc: AccountId, file_hash: Hash) -> DispatchResult;

	fn clear_miner_file(acc: AccountId) -> Weight;
}

impl<T: Config> RandomFileList<<T as frame_system::Config>::AccountId> for Pallet<T> {

	fn add_invalid_file(miner_acc: AccountOf<T>, file_hash: Hash) -> DispatchResult {
		Pallet::<T>::add_invalid_file(miner_acc, file_hash)?;
		Ok(())
	}

	fn challenge_clear_idle(acc: AccountOf<T>, file_hash: Hash) -> DispatchResult {
		Pallet::<T>::challenge_clear_idle(acc, file_hash)
	}

	fn challenge_clear_service(acc: AccountOf<T>, shard_id: [u8; 68]) -> DispatchResult {
		Pallet::<T>::challenge_clear_service(acc, shard_id)
	}

	fn challenge_clear_autonomy(acc: AccountOf<T>, file_hash: Hash) -> DispatchResult {
		Pallet::<T>::challenge_clear_autonomy(acc, file_hash)
	}

	fn clear_miner_file(acc: AccountOf<T>) -> Weight {
		let mut weight: Weight = 0;
		let weight_temp = Pallet::<T>::clear_miner_autonomy_file(&acc);
		weight = weight.saturating_add(weight_temp);
		let weight_temp = Pallet::<T>::clear_miner_idle_file(&acc);	
		weight = weight.saturating_add(weight_temp);

		weight
	}
}

impl<T: Config> BlockNumberProvider for Pallet<T> {
	type BlockNumber = T::BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		<frame_system::Pallet<T>>::block_number()
	}
}
