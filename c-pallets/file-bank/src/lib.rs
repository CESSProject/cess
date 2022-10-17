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
use cp_cess_common::{PackageType, DataType, Hash as H68};
use cp_scheduler_credit::SchedulerCreditCounter;
use sp_runtime::{
	traits::{
		AccountIdConversion, BlockNumberProvider, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub,
		SaturatedConversion,
	},
	RuntimeDebug,
};
use sp_std::{convert::TryInto, prelude::*, str};

pub use weights::WeightInfo;

type Hash = H68;
type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
type BoundedString<T> = BoundedVec<u8, <T as Config>::StringLimit>;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{ensure, traits::Get};
	use pallet_file_map::ScheduleFind;
	use pallet_sminer::MinerControl;
	//pub use crate::weights::WeightInfo;
	use frame_system::ensure_signed;
	use cp_cess_common::Hash;

	pub const M_BYTE: u128 = 1_048_576;
	pub const G_BYTE: u128 = 1_048_576 * 1024;
	pub const T_BYTE: u128 = 1_048_576 * 1024 * 1024;

	pub const PACKAGE_1_SIZE: u128 = G_BYTE * 10;
	pub const PACKAGE_2_SIZE: u128 = G_BYTE * 500;
	pub const PACKAGE_3_SIZE: u128 = T_BYTE * 1;
	pub const PACKAGE_4_SIZE: u128 = T_BYTE * 5;

	pub const FILE_PENDING: &str = "pending";
	pub const FILE_ACTIVE: &str = "active";
	pub const PACKAGE_NORMAL: &str = "normal";
	pub const PACKAGE_FROZEN: &str = "frozen";

	#[pallet::config]
	pub trait Config: frame_system::Config + sp_std::fmt::Debug {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		type WeightInfo: WeightInfo;

		type Call: From<Call<Self>>;
		//Find the consensus of the current block
		type FindAuthor: FindAuthor<Self::AccountId>;
		//Used to find out whether the schedule exists
		type Scheduler: ScheduleFind<Self::AccountId>;
		//It is used to control the computing power and space of miners
		type MinerControl: MinerControl<Self::AccountId>;
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

		type CreditCounter: SchedulerCreditCounter<Self::AccountId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		//file upload declaration
		UploadDeclaration { acc: AccountOf<T>, file_hash: Hash, file_name: Vec<u8> },
		//file uploaded.
		FileUpload { acc: AccountOf<T> },
		//file updated.
		FileUpdate { acc: AccountOf<T>, fileid: Vec<u8> },

		FileChangeState { acc: AccountOf<T>, fileid: Vec<u8> },
		//Storage information of scheduling storage file slice
		InsertFileSlice { fileid: Vec<u8> },
		//User buy package event
		BuyPackage { acc: AccountOf<T>, size: u128, fee: BalanceOf<T> },
		//Package upgrade
		PackageUpgrade { acc: AccountOf<T>, old_type: PackageType, new_type: PackageType, fee: BalanceOf<T> },
		//Package upgrade
		PackageRenewal { acc: AccountOf<T>, package_type: PackageType, fee: BalanceOf<T> },
		//Expired storage space
		LeaseExpired { acc: AccountOf<T>, size: u128 },
		//Storage space expiring within 24 hours
		LeaseExpireIn24Hours { acc: AccountOf<T>, size: u128 },
		//File deletion event
		DeleteFile { acc: AccountOf<T>, fileid: Hash },
		//Filler chain success event
		FillerUpload { acc: AccountOf<T>, file_size: u64 },
		//File recovery
		RecoverFile { acc: AccountOf<T>, file_hash: [u8; 72] },
		//The miner cleaned up an invalid file event
		ClearInvalidFile { acc: AccountOf<T>, file_hash: Hash },
		//Users receive free space events
		ReceiveSpace { acc: AccountOf<T> },
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
		NotPurchasedPackage,

		PurchasedPackage,
		//Expired storage space
		LeaseExpired,

		LeaseFreeze,
		//Exceeded the maximum amount expected by the user
		ExceedExpectations,

		ConversionError,

		InsufficientAvailableSpace,

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
		StorageMap<_, Blake2_128Concat, AccountOf<T>, BoundedVec<[u8; 72], T::RecoverLimit>, ValueQuery>;

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

	#[pallet::storage]
	#[pallet::getter(fn purchase_package)]
	pub(super) type PurchasedPackage<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, PackageDetails<T>>;

	#[pallet::storage]
	#[pallet::getter(fn file_keys_map)]
	pub(super) type FileKeysMap<T: Config> =
		CountedStorageMap<_, Blake2_128Concat, u32, (bool, Hash)>;

	#[pallet::storage]
	#[pallet::getter(fn file_index_count)]
	pub(super) type FileIndexCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn filler_keys_map)]
	pub(super) type FillerKeysMap<T: Config> =
		CountedStorageMap<_, Blake2_128Concat, u32, (AccountOf<T>, Hash)>;

	#[pallet::storage]
	#[pallet::getter(fn filler_index_count)]
	pub(super) type FillerIndexCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

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
			let mut weight: Weight = 0;
			if number % oneday as u128 == 0 {
				log::info!("Start lease expiration check");
				for (acc, info) in <PurchasedPackage<T>>::iter() {
					weight = weight.saturating_add(T::DbWeight::get().reads(1 as Weight));
					if now > info.deadline {
						let frozen_day: BlockNumberOf<T> = match info.package_type {
							PackageType::Package1 => (0 * oneday).saturated_into(),
							PackageType::Package2 => (7 * oneday).saturated_into(),
							PackageType::Package3 => (14 * oneday).saturated_into(),
							PackageType::Package4 => (20 * oneday).saturated_into(),
							_ => (30 * oneday).saturated_into(),
						};
						if now > info.deadline + frozen_day {
							log::info!("clear user:#{}'s files", number);
							let result = T::MinerControl::sub_purchased_space(info.space);
							match result {
								Ok(()) => log::info!("sub purchased space success"),
								Err(e) => log::error!("failed sub purchased space: {:?}", e),
							};
							weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
							let result = Self::clear_expired_file(&acc);
							let weight1 = match result {
								Ok(weight) => {
									log::info!("clear expired file success");
									weight
								},
								Err(e) => {
									log::error!("failed clear expired file: {:?}", e);
									0
						 		},
							};
							weight = weight.saturating_add(weight1);
						} else {
							if info.state.to_vec() != PACKAGE_FROZEN.as_bytes().to_vec() {
								let result = <PurchasedPackage<T>>::try_mutate(
									&acc,
									|s_opt| -> DispatchResult {
										let s = s_opt
											.as_mut()
											.ok_or(Error::<T>::NotPurchasedPackage)?;
										s.state = PACKAGE_FROZEN
											.as_bytes()
											.to_vec()
											.try_into()
											.map_err(|_e| Error::<T>::BoundedVecError)?;
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
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upload_declaration())]
		pub fn upload_declaration(
			origin: OriginFor<T>,
			file_hash: Hash,
			file_name: Vec<u8>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let file_name_bound: BoundedString<T> = Self::vec_to_bound::<u8>(file_name.clone())?;
			if <File<T>>::contains_key(&file_hash) {
				<File<T>>::try_mutate(&file_hash, |s_opt| -> DispatchResult {
					let s = s_opt.as_mut().ok_or(Error::<T>::FileNonExistent)?;
					if s.user.contains(&sender) {
						Err(Error::<T>::Declarated)?;
					}
					Self::update_user_space(sender.clone(), 1, s.file_size.into())?;
					Self::add_user_hold_fileslice(
						sender.clone(),
						file_hash.clone(),
						s.file_size,
					)?;
					s.user.try_push(sender.clone()).map_err(|_| Error::<T>::StorageLimitReached)?;
					s.file_name
						.try_push(file_name_bound.clone())
						.map_err(|_| Error::<T>::StorageLimitReached)?;
					Ok(())
				})?;
			} else {
				let count =
					<FileIndexCount<T>>::get().checked_add(1).ok_or(Error::<T>::Overflow)?;
				<File<T>>::insert(
					&file_hash,
					FileInfo::<T> {
						file_size: 0,
						index: count,
						file_state: FILE_PENDING
							.as_bytes()
							.to_vec()
							.try_into()
							.map_err(|_| Error::<T>::BoundedVecError)?,
						user: vec![sender.clone()]
							.try_into()
							.map_err(|_| Error::<T>::BoundedVecError)?,
						file_name: vec![file_name_bound]
							.try_into()
							.map_err(|_| Error::<T>::BoundedVecError)?,
						slice_info: Default::default(),
					},
				);
				<FileIndexCount<T>>::put(count);
				<FileKeysMap<T>>::insert(count, (false, file_hash.clone()));
			}
			Self::deposit_event(Event::<T>::UploadDeclaration {
				acc: sender,
				file_hash,
				file_name,
			});
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
				for user in s.user.iter() {
					Self::update_user_space(user.clone(), 1, file_size.into())?;
					Self::add_user_hold_fileslice(
						user.clone(),
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
				<FileKeysMap<T>>::try_mutate(s.index, |v_opt| -> DispatchResult {
					let v = v_opt.as_mut().ok_or(Error::<T>::FileNonExistent)?;
					v.0 = true;
					Ok(())
				})?;
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
			T::MinerControl::add_power(&miner, power)?;

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
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::delete_file())]
		pub fn delete_file(origin: OriginFor<T>, fileid: Hash) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<File<T>>::contains_key(fileid.clone()), Error::<T>::FileNonExistent);
			//The above has been judged. Unwrap will be performed only if the key exists
			Self::clear_user_file(fileid, &sender)?;
			Self::deposit_event(Event::<T>::DeleteFile { acc: sender, fileid });
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
		/// - `package_type`: Package type (1 / 2 / 3 / 4 / 5).
		/// - `count`: When type = 5, it means that the user wants to buy count TIBS.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::buy_package())]
		pub fn buy_package(origin: OriginFor<T>, package_type: PackageType, count: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!<PurchasedPackage<T>>::contains_key(&sender), Error::<T>::PurchasedPackage);

			let (space, g_unit_price, month) = match package_type {
				PackageType::Package1 => (PACKAGE_1_SIZE, 0, 1),
				PackageType::Package2 => (PACKAGE_2_SIZE, Self::get_price(PACKAGE_2_SIZE)?, 1),
				PackageType::Package3 => (PACKAGE_3_SIZE, Self::get_price(PACKAGE_3_SIZE)?, 1),
				PackageType::Package4 => (PACKAGE_4_SIZE, Self::get_price(PACKAGE_4_SIZE)?, 1),
				PackageType::Package5 => {
					if count <= 5 {
						Err(Error::<T>::WrongOperation)?;
					}
					(count * T_BYTE, Self::get_price(count * T_BYTE)?, 1)
				},
			};

			Self::add_puchased_package(sender.clone(), space, month as u32, package_type)?;
			T::MinerControl::add_purchased_space(space)?;
			let count_gb = space.checked_div(G_BYTE).ok_or(Error::<T>::Overflow)?;
			let price: BalanceOf<T> = g_unit_price
				.checked_mul(count_gb)
				.ok_or(Error::<T>::Overflow)?
				.try_into()
				.map_err(|_e| Error::<T>::Overflow)?;
			ensure!(
				<T as pallet::Config>::Currency::can_slash(&sender, price.clone()),
				Error::<T>::InsufficientBalance
			);
			let acc = T::FilbakPalletId::get().into_account();
			<T as pallet::Config>::Currency::transfer(&sender, &acc, price.clone(), AllowDeath)?;

			Self::deposit_event(Event::<T>::BuyPackage { acc: sender, size: space, fee: price });
			Ok(())
		}
		/// Upgrade package (expansion of storage space)
		///
		/// It can only be called when the package has been purchased,
		/// And the upgrade target needs to be higher than the current package.
		///
		/// Parameters:
		/// - `package_type`: Package type (1 / 2 / 3 / 4 / 5).
		/// - `count`: When type = 5, it means that the user wants to buy count TIBS.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upgrade_package())]
		pub fn upgrade_package(
			origin: OriginFor<T>,
			package_type: PackageType,
			count: u128,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let cur_package = <PurchasedPackage<T>>::try_get(&sender)
				.map_err(|_e| Error::<T>::NotPurchasedPackage)?;

			let now = <frame_system::Pallet<T>>::block_number();
			ensure!(now < cur_package.deadline, Error::<T>::LeaseExpired);
			ensure!(
				cur_package.state.to_vec() != PACKAGE_FROZEN.as_bytes().to_vec(),
				Error::<T>::LeaseFreeze
			);

			let cur_gb_unit_price = match cur_package.package_type {
				PackageType::Package1 => 0,
				_ => Self::get_price(cur_package.space)?,
			};
			let cur_count_gb = cur_package.space.checked_div(G_BYTE).ok_or(Error::<T>::Overflow)?;
			let cur_price =
				cur_gb_unit_price.checked_mul(cur_count_gb).ok_or(Error::<T>::Overflow)?;

			let (space, up_gb_unit_price) = match package_type {
				PackageType::Package2 => (PACKAGE_2_SIZE, Self::get_price(PACKAGE_2_SIZE)?),
				PackageType::Package3 => (PACKAGE_3_SIZE, Self::get_price(PACKAGE_3_SIZE)?),
				PackageType::Package4 => (PACKAGE_4_SIZE, Self::get_price(PACKAGE_4_SIZE)?),
				PackageType::Package5 => {
					let cur_count =
						cur_package.space.checked_div(T_BYTE).ok_or(Error::<T>::Overflow)?;
					if count <= cur_count {
						Err(Error::<T>::WrongOperation)?;
					}
					(count * T_BYTE, Self::get_price(count * T_BYTE)?)
				},
				_ => Err(Error::<T>::WrongOperation)?,
			};

			ensure!(
				space > cur_package.space,
				Error::<T>::WrongOperation
			);
			//Calculate daily average price difference.
			let up_count_gb = space.checked_div(G_BYTE).ok_or(Error::<T>::Overflow)?;
			let up_price = up_gb_unit_price.checked_mul(up_count_gb).ok_or(Error::<T>::Overflow)?;
			let diff_day_price = up_price
				.checked_sub(cur_price)
				.ok_or(Error::<T>::Overflow)?
				.checked_div(30)
				.ok_or(Error::<T>::Overflow)?;
			//Calculate remaining days.
			let block_oneday: BlockNumberOf<T> = <T as pallet::Config>::OneDay::get();
			let diff_block = cur_package.deadline.checked_sub(&now).ok_or(Error::<T>::Overflow)?;
			let mut remain_day =
				diff_block.checked_div(&block_oneday).ok_or(Error::<T>::Overflow)?;
			if diff_block % block_oneday != 0u32.saturated_into() {
				remain_day =
					remain_day.checked_add(&1u32.saturated_into()).ok_or(Error::<T>::Overflow)?;
			}
			//Calculate the final price difference to be made up.
			let price: BalanceOf<T> = diff_day_price
				.checked_mul(remain_day.try_into().map_err(|_e| Error::<T>::Overflow)?)
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
				space.checked_sub(cur_package.space).ok_or(Error::<T>::Overflow)?,
			)?;
			Self::expension_puchased_package(sender.clone(), space, package_type.clone())?;
			<T as pallet::Config>::Currency::transfer(&sender, &acc, price.clone(), AllowDeath)?;

			Self::deposit_event(Event::<T>::PackageUpgrade {
				acc: sender,
				old_type: cur_package.package_type,
				new_type: package_type,
				fee: price,
			});
			Ok(())
		}
		/// Package renewal
		///
		/// Currently, lease renewal only supports single month renewal
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::renewal_package())]
		pub fn renewal_package(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let cur_package = <PurchasedPackage<T>>::try_get(&sender)
				.map_err(|_e| Error::<T>::NotPurchasedPackage)?;

			let g_unit_price = match cur_package.package_type {
				PackageType::Package1 => 0,
				_ => Self::get_price(cur_package.space)?,
			};
			let count_gb = cur_package.space.checked_div(G_BYTE).ok_or(Error::<T>::Overflow)?;
			let price: BalanceOf<T> = g_unit_price
				.checked_mul(count_gb)
				.ok_or(Error::<T>::Overflow)?
				.try_into()
				.map_err(|_e| Error::<T>::Overflow)?;
			ensure!(
				<T as pallet::Config>::Currency::can_slash(&sender, price.clone()),
				Error::<T>::InsufficientBalance
			);
			let acc = T::FilbakPalletId::get().into_account();
			<T as pallet::Config>::Currency::transfer(&sender, &acc, price.clone(), AllowDeath)?;
			Self::update_puchased_package(sender.clone())?;
			Self::deposit_event(Event::<T>::PackageRenewal {
				acc: sender,
				package_type: cur_package.package_type,
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
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::recover_file())]
		pub fn recover_file(
			origin: OriginFor<T>,
			shard_id: [u8; 72],
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
	}

	impl<T: Config> Pallet<T> {
		/// helper: update_puchased_package.
		///
		/// How to update the corresponding data after renewing the package.
		/// Currently, it only supports and defaults to one month.
		///
		/// Parameters:
		/// - `acc`: Account
		fn update_puchased_package(acc: AccountOf<T>) -> DispatchResult {
			<PurchasedPackage<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
				let s = s_opt.as_mut().ok_or(Error::<T>::NotPurchasedPackage)?;
				let one_day = <T as pallet::Config>::OneDay::get();
				let now = <frame_system::Pallet<T>>::block_number();
				let sur_block: BlockNumberOf<T> =
					one_day.checked_mul(&30u32.saturated_into()).ok_or(Error::<T>::Overflow)?;
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
			package_type: PackageType,
		) -> DispatchResult {
			<PurchasedPackage<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
				let s = s_opt.as_mut().ok_or(Error::<T>::NotPurchasedPackage)?;
				s.remaining_space = s
					.remaining_space
					.checked_add(space.checked_sub(s.space).ok_or(Error::<T>::Overflow)?)
					.ok_or(Error::<T>::Overflow)?;
				s.space = space;
				s.package_type = package_type;
				Ok(())
			})?;

			Ok(())
		}
		/// helper: Initial purchase space initialization method.
		///
		/// Purchase a package and create data corresponding to the user.
		/// PurchasedPackage Storage.
		///
		/// Parameters:
		/// - `space`: Buy storage space size.
		/// - `month`: Month of purchase of package, It is 1 at present.
		/// - `package_type`: Package type.
		fn add_puchased_package(
			acc: AccountOf<T>,
			space: u128,
			month: u32,
			package_type: PackageType,
		) -> DispatchResult {
			let now = <frame_system::Pallet<T>>::block_number();
			let one_day = <T as pallet::Config>::OneDay::get();
			let sur_block: BlockNumberOf<T> = month
				.checked_mul(30)
				.ok_or(Error::<T>::Overflow)?
				.checked_mul(one_day.saturated_into())
				.ok_or(Error::<T>::Overflow)?
				.saturated_into();
			let deadline = now.checked_add(&sur_block).ok_or(Error::<T>::Overflow)?;
			let info = PackageDetails::<T> {
				space,
				used_space: 0,
				remaining_space: space,
				tenancy: month,
				package_type,
				start: now,
				deadline,
				state: PACKAGE_NORMAL
					.as_bytes()
					.to_vec()
					.try_into()
					.map_err(|_e| Error::<T>::BoundedVecError)?,
			};
			<PurchasedPackage<T>>::insert(&acc, info);
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
					<PurchasedPackage<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
						let s = s_opt.as_mut().ok_or(Error::<T>::NotPurchasedPackage)?;
						if s.state.to_vec() == PACKAGE_FROZEN.as_bytes().to_vec() {
							Err(Error::<T>::LeaseFreeze)?;
						}
						if size > s.space - s.used_space {
							Err(Error::<T>::InsufficientStorage)?;
						}
						s.used_space =
							s.used_space.checked_add(size).ok_or(Error::<T>::Overflow)?;
						s.remaining_space =
							s.remaining_space.checked_sub(size).ok_or(Error::<T>::Overflow)?;
						Ok(())
					})?;
				},
				2 => <PurchasedPackage<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
					let s = s_opt.as_mut().unwrap();
					s.used_space = s.used_space.checked_sub(size).ok_or(Error::<T>::Overflow)?;
					s.remaining_space =
						s.space.checked_sub(s.used_space).ok_or(Error::<T>::Overflow)?;
					Ok(())
				})?,
				_ => Err(Error::<T>::WrongOperation)?,
			}
			Ok(())
		}
		/// helper: Get space unit price.
		///
		/// Calculate the unit price according to the size of the purchase space
		///
		/// Parameters:
		/// - `buy_space`: Purchase space size, in bytes.
		///
		/// Result:
		/// - `u128`: price.
		pub fn get_price(buy_space: u128) -> Result<u128, DispatchError> {
			//Get the available space on the current chain
			let total_space = T::MinerControl::get_space()?;
			//If it is not 0, the logic is executed normally
			if total_space == 0 {
				Err(Error::<T>::IsZero)?;
			}
			//Calculation rules
			//The price is based on 1024 / available space on the current chain
			//Multiply by the base value 1 tcess * 1_000 (1_000_000_000_000 * 1_000)
			let price: u128 = buy_space
				.checked_mul(1_000_000_000_000)
				.ok_or(Error::<T>::Overflow)?
				.checked_mul(10_000)
				.ok_or(Error::<T>::Overflow)?
				.checked_div(total_space)
				.ok_or(Error::<T>::Overflow)?
				.checked_add(1_000_000_000_000_000)
				.ok_or(Error::<T>::Overflow)?;

			return Ok(price)
		}

		fn vec_to_bound<P>(param: Vec<P>) -> Result<BoundedVec<P, T::StringLimit>, DispatchError> {
			let result: BoundedVec<P, T::StringLimit> =
				param.try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
			Ok(result)
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
		) -> Result<Vec<(AccountOf<T>, Hash, [u8; 72], Vec<u8>, u64, DataType)>, DispatchError> {
			let filler_list = Self::get_random_filler()?;
			let mut data: Vec<(AccountOf<T>, Hash, [u8; 72], Vec<u8>, u64, DataType)> = Vec::new();
			for v in filler_list {
				let length = v.block_num;
				let number_list = Self::get_random_numberlist(length, 3, length)?;
				let miner_acc = v.miner_address.clone();
				let filler_hash = v.filler_hash.clone();
				let file_size = v.filler_size.clone();
				let mut block_list: Vec<u8> = Vec::new();
				for i in number_list.iter() {
					block_list.push((*i as u8) + 1);
				}
				data.push((miner_acc, filler_hash, [0u8; 72], block_list, file_size, DataType::Filler));
			}

			let file_list = Self::get_random_file()?;
			for (_, file) in file_list {
				let slice_number_list = Self::get_random_numberlist(
					file.slice_info.len() as u32,
					3,
					file.slice_info.len() as u32,
				)?;
				for slice_index in slice_number_list.iter() {
					let miner_id = T::MinerControl::get_miner_id(
						file.slice_info[*slice_index as usize].miner_acc.clone(),
					)?;
					if file.slice_info[*slice_index as usize].miner_id != miner_id {
						continue
					}
					let mut block_list: Vec<u8> = Vec::new();
					let length = file.slice_info[*slice_index as usize].block_num;
					let number_list = Self::get_random_numberlist(length, 3, length)?;
					let file_hash = Hash::from_shard_id(&file.slice_info[*slice_index as usize].shard_id).map_err(|_| Error::<T>::ConvertHashError)?;
					let miner_acc = file.slice_info[*slice_index as usize].miner_acc.clone();
					let slice_size = file.slice_info[*slice_index as usize].shard_size;
					for i in number_list.iter() {
						block_list.push((*i as u8) + 1);
					}
					data.push((miner_acc, file_hash, file.slice_info[*slice_index as usize].shard_id, block_list, slice_size, DataType::File));
				}
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
			let limit = <FillerIndexCount<T>>::get();
			let number_list = Self::get_random_numberlist(length, 1, limit)?;
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
			let length = Self::get_file_map_length()?;
			let limit = <FileIndexCount<T>>::get();
			let number_list = Self::get_random_numberlist(length, 2, limit)?;
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
			let mut number_list: Vec<u32> = Vec::new();
			loop {
				seed = seed.checked_add(1).ok_or(Error::<T>::Overflow)?;
				if number_list.len() >= num as usize {
					number_list.sort();
					number_list.dedup();
					if number_list.len() >= num as usize {
						break
					}
				}
				let random = Self::generate_random_number(seed)? % limit;
				let result: bool = match random_type {
					1 => Self::judge_filler_exist(random),
					2 => Self::judge_file_exist(random),
					_ => true,
				};
				if !result {
					//Start the next cycle if the file does not exist
					continue
				}
				log::info!("List addition: {}", random);
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
		/// helper: get filemap length.
		///
		/// Gets the length of the padding file.
		///
		/// Parameters:
		/// - `index`: filler index.
		/// Result:
		/// - `bool`: Length of the whole network service file.
		fn get_file_map_length() -> Result<u32, DispatchError> {
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
			let mut weight: Weight = 0;
			let file =
					<File<T>>::try_get(&file_hash).map_err(|_| Error::<T>::FileNonExistent)?; //read 1
			  weight = weight.saturating_add(T::DbWeight::get().reads(1 as Weight));
			for user in file.user.iter() {
				Self::update_user_space(user.clone(), 2, file.file_size.into())?; //read 1 write 1 * n
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
			}
			for slice in file.slice_info.iter() {
				let hash = Hash::from_shard_id(&slice.shard_id).map_err(|_| Error::<T>::ConvertHashError)?;
				Self::add_invalid_file(slice.miner_acc.clone(), hash)?; //read 1 write 1 * n
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
				T::MinerControl::sub_space(&slice.miner_acc, slice.shard_size.into())?; //read 3 write 2
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(3, 2));
			}
			<File<T>>::remove(file_hash);
			<FileKeysMap<T>>::remove(file.index);
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
		) -> Result<Weight, DispatchError> {
			let mut weight: Weight = 0;
			let file = <File<T>>::get(&file_hash).unwrap(); //read 1
			weight = weight.saturating_add(T::DbWeight::get().reads(1 as Weight));
			ensure!(file.user.contains(user), Error::<T>::NotOwner);
			//If the file still has an owner, only the corresponding owner will be cleared.
			//If the owner is unique, the file meta information will be cleared.
			if file.user.len() > 1 {
				//read 1 write 1
				<File<T>>::try_mutate(&file_hash, |s_opt| -> DispatchResult {
					let s = s_opt.as_mut().unwrap();
					let mut index = 0;
					for acc in s.user.iter() {
						if *acc == user.clone() {
							break
						}
						index = index.checked_add(&1).ok_or(Error::<T>::Overflow)?;
					}
					weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
					s.user.remove(index);
					s.file_name.remove(index);
					Ok(())
				})?;
				Self::update_user_space(user.clone(), 2, file.file_size.clone().into())?; //read 1 write 1
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
			} else {
				let weight1 = Self::clear_file(file_hash.clone())?;
				weight = weight.saturating_add(weight1);
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
		pub fn add_recovery_file(shard_id: [u8; 72]) -> DispatchResult {
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
			T::MinerControl::add_space(&miner_acc, file_size.into())?;
			T::MinerControl::sub_power(miner_acc.clone(), replace_num * M_BYTE * 8)?;
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
				let weight1 = Self::clear_user_file(v.file_hash.clone(), acc)?;
				weight = weight.saturating_add(weight1);
			}

			Ok(weight)
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
	) -> Result<Vec<(AccountId, Hash, [u8; 72], Vec<u8>, u64, DataType)>, DispatchError>;
	//Delete filler file
	fn delete_filler(miner_acc: AccountId, filler_hash: Hash) -> DispatchResult;
	//Delete all filler according to miner_acc
	fn delete_miner_all_filler(miner_acc: AccountId) -> Result<Weight, DispatchError>;
	//Delete file backup
	fn clear_file(file_hash: Hash) -> Result<Weight, DispatchError>;
	//The function executed when the challenge fails, allowing the miner to delete invalid files
	fn add_recovery_file(file_id: [u8; 72]) -> DispatchResult;
	//The function executed when the challenge fails to let the consensus schedule recover the file
	fn add_invalid_file(miner_acc: AccountId, file_hash: Hash) -> DispatchResult;
}

impl<T: Config> RandomFileList<<T as frame_system::Config>::AccountId> for Pallet<T> {
	fn get_random_challenge_data(
	) -> Result<Vec<(AccountOf<T>, Hash, [u8; 72], Vec<u8>, u64, DataType)>, DispatchError> {
		let result = Pallet::<T>::get_random_challenge_data()?;
		Ok(result)
	}

	fn delete_filler(miner_acc: AccountOf<T>, filler_hash: Hash) -> DispatchResult {
		Pallet::<T>::delete_filler(miner_acc, filler_hash)?;
		Ok(())
	}
	fn delete_miner_all_filler(miner_acc: AccountOf<T>) -> Result<Weight, DispatchError> {
		let mut weight: Weight = 0;
		for (_, value) in FillerMap::<T>::iter_prefix(&miner_acc) {
			<FillerKeysMap<T>>::remove(value.index);
			weight = weight.saturating_add(T::DbWeight::get().writes(1 as Weight));
		}
		let _ = FillerMap::<T>::remove_prefix(&miner_acc, Option::None);
		weight = weight.saturating_add(T::DbWeight::get().writes(1 as Weight));
		Ok(weight)
	}

	fn clear_file(file_hash: Hash) -> Result<Weight, DispatchError> {
		let weight = Pallet::<T>::clear_file(file_hash)?;
		Ok(weight)
	}

	fn add_recovery_file(file_id: [u8; 72]) -> DispatchResult {
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
