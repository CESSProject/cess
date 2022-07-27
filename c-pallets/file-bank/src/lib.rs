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
};
pub use pallet::*;
#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod weights;

mod types;
pub use types::*;

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::{
	offchain as rt_offchain,
	traits::{
		AccountIdConversion, BlockNumberProvider, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub,
		SaturatedConversion,
	},
	RuntimeDebug,
};
use sp_std::{convert::TryInto, prelude::*, str};

use frame_support::{dispatch::DispatchResult, pallet_prelude::*, PalletId};
use frame_system::offchain::{AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer};
use sp_core::crypto::KeyTypeId;
pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
type BoundedString<T> = BoundedVec<u8, <T as Config>::StringLimit>;
type BoundedList<T> =
	BoundedVec<BoundedVec<u8, <T as Config>::StringLimit>, <T as Config>::StringLimit>;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{ensure, traits::Get};
	use pallet_file_map::ScheduleFind;
	use pallet_sminer::MinerControl;
	//pub use crate::weights::WeightInfo;
	use frame_system::{ensure_signed, pallet_prelude::*};

	const HTTP_REQUEST_STR: &str = "https://arweave.net/price/1048576";
	// const HTTP_REQUEST_STR: &str = "https://api.coincap.io/v2/assets/polkadot";
	pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"cess");
	const FETCH_TIMEOUT_PERIOD: u64 = 60_000; // in milli-seconds
										  //1MB converted byte size
	const M_BYTE: u128 = 1_048_576;

	pub mod crypto {
		use super::KEY_TYPE;
		use sp_core::sr25519::Signature as Sr25519Signature;
		use sp_runtime::{
			app_crypto::{app_crypto, sr25519},
			traits::Verify,
			MultiSignature, MultiSigner,
		};

		app_crypto!(sr25519, KEY_TYPE);

		pub struct TestAuthId;
		// implemented for ocw-runtime
		impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::sr25519::Signature;
			type GenericPublic = sp_core::sr25519::Public;
		}

		// implemented for mock runtime in test
		impl
			frame_system::offchain::AppCrypto<
				<Sr25519Signature as Verify>::Signer,
				Sr25519Signature,
			> for TestAuthId
		{
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::sr25519::Signature;
			type GenericPublic = sp_core::sr25519::Public;
		}
	}

	#[pallet::config]
	pub trait Config:
		frame_system::Config
		+ pallet_sminer::Config
		+ sp_std::fmt::Debug
		+ CreateSignedTransaction<Call<Self>>
	{
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		type WeightInfo: WeightInfo;

		type Call: From<Call<Self>>;

		//Find the consensus of the current block
		type FindAuthor: FindAuthor<Self::AccountId>;

		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
		//Used to find out whether the schedule exists
		type Scheduler: ScheduleFind<Self::AccountId>;
		//It is used to control the computing power and space of miners
		type MinerControl: MinerControl<Self::AccountId>;
		//Interface that can generate random seeds
		type MyRandomness: Randomness<Self::Hash, Self::BlockNumber>;
		/// pallet address.
		#[pallet::constant]
		type FilbakPalletId: Get<PalletId>;

		#[pallet::constant]
		type StringLimit: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type OneDay: Get<BlockNumberOf<Self>>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		//file upload declaration
		UploadDeclaration { acc: AccountOf<T>, file_hash: Vec<u8>, file_name: Vec<u8> },
		//file uploaded.
		FileUpload { acc: AccountOf<T> },
		//file updated.
		FileUpdate { acc: AccountOf<T>, fileid: Vec<u8> },

		FileChangeState { acc: AccountOf<T>, fileid: Vec<u8> },
		//file bought.
		BuyFile { acc: AccountOf<T>, money: BalanceOf<T>, fileid: Vec<u8> },
		//file purchased before.
		Purchased { acc: AccountOf<T>, fileid: Vec<u8> },
		//Storage information of scheduling storage file slice
		InsertFileSlice { fileid: Vec<u8> },
		//User purchase space
		BuySpace { acc: AccountOf<T>, size: u128, fee: BalanceOf<T> },
		//Expired storage space
		LeaseExpired { acc: AccountOf<T>, size: u128 },
		//Storage space expiring within 24 hours
		LeaseExpireIn24Hours { acc: AccountOf<T>, size: u128 },
		//File deletion event
		DeleteFile { acc: AccountOf<T>, fileid: Vec<u8> },
		//Filler chain success event
		FillerUpload { acc: AccountOf<T>, file_size: u64 },
		//File recovery
		RecoverFile { acc: AccountOf<T>, file_hash: Vec<u8> },
		//The miner cleaned up an invalid file event
		ClearInvalidFile { acc: AccountOf<T>, file_hash: Vec<u8> },
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
		NotPurchasedSpace,
		//Expired storage space
		LeaseExpired,
		//Exceeded the maximum amount expected by the user
		ExceedExpectations,

		ConversionError,

		InsufficientAvailableSpace,

		AlreadyRepair,

		NotOwner,

		AlreadyReceive,

		AlreadyExist,

		NotQualified,

		UserNotDeclared,
		//HTTP interaction error of offline working machine
		HttpFetchingError,
		//Signature error of offline working machine
		OffchainSignedTxError,

		OffchainUnSignedTxError,
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
	}
	#[pallet::storage]
	#[pallet::getter(fn next_unsigned_at)]
	pub(super) type NextUnsignedAt<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn file)]
	pub(super) type File<T: Config> =
		StorageMap<_, Blake2_128Concat, BoundedString<T>, FileInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn invoice)]
	pub(super) type Invoice<T: Config> =
		StorageMap<_, Blake2_128Concat, BoundedString<T>, u8, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn user_hold_file_list)]
	pub(super) type UserHoldFileList<T: Config> =
		StorageMap<
			_, 
			Blake2_128Concat, 
			T::AccountId, 
			BoundedVec<UserFileSliceInfo<T>, T::ItemLimit>, 
			ValueQuery
		>;

	#[pallet::storage]
	#[pallet::getter(fn user_hold_storage_space)]
	pub(super) type UserHoldSpaceDetails<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, StorageSpace>;

	#[pallet::storage]
	#[pallet::getter(fn user_spance_details)]
	pub(super) type UserSpaceList<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<SpaceInfo<T>, T::ItemLimit>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn file_recovery)]
	pub(super) type FileRecovery<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, BoundedList<T>, ValueQuery>;

	#[pallet::storage]
	pub(super) type UserFreeRecord<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u8, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn unit_price)]
	pub(super) type UnitPrice<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn filler_map)]
	pub(super) type FillerMap<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AccountOf<T>,
		Blake2_128Concat,
		BoundedString<T>,
		FillerInfo<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn invalid_file)]
	pub(super) type InvalidFile<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, BoundedList<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn members)]
	pub(super) type Members<T: Config> =
		StorageValue<_, BoundedVec<AccountOf<T>, T::StringLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn lock_time)]
	pub(super) type LockTime<T: Config> = StorageValue<_, BlockNumberOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn file_keys_map)]
	pub(super) type FileKeysMap<T: Config> = CountedStorageMap<_, Blake2_128Concat, u32, (bool, BoundedString<T>)>;

	#[pallet::storage]
	#[pallet::getter(fn file_index_count)]
	pub(super) type FileIndexCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn filler_keys_map)]
	pub(super) type FillerKeysMap<T: Config> = CountedStorageMap<_, Blake2_128Concat, u32, (AccountOf<T>, BoundedString<T>)>;

	#[pallet::storage]
	#[pallet::getter(fn filler_index_count)]
	pub(super) type FillerIndexCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::pallet]
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
			let oneday: u128 = block_oneday.saturated_into();
			let mut count: u8 = 0;
			if number % oneday == 0 {
				for (key, value) in <UserSpaceList<T>>::iter() {
					let mut k = 0;
					let mut list = <UserSpaceList<T>>::get(&key);
					for s in value.iter() {
						let size = s.size;
						if now >= s.deadline {
							list.remove(k);
							<UserHoldSpaceDetails<T>>::mutate(&key, |s_opt| {
								let s = s_opt.as_mut().unwrap();
								s.purchased_space = s.purchased_space - size;
							});
							let v = <UserHoldSpaceDetails<T>>::get(&key).unwrap();
							if v.purchased_space > v.used_space {
								<UserHoldSpaceDetails<T>>::mutate(&key, |s_opt| {
									let s = s_opt.as_mut().unwrap();
									s.remaining_space = s.purchased_space - s.used_space;
								});
							} else {
								let _ = Self::clear_expired_file(&key, v.used_space.clone(), v.purchased_space.clone());
							}
							let div2space = size / 2;
							let _ = pallet_sminer::Pallet::<T>::sub_purchased_space(size + div2space);
							Self::deposit_event(Event::<T>::LeaseExpired {
								acc: key.clone(),
								size,
							});
							k -= 1;
						} else if s.deadline < now && now >= s.deadline - block_oneday {
							count += 1;
						}
						k += 1;
					}
					<UserSpaceList<T>>::insert(&key, list);
					Self::deposit_event(Event::<T>::LeaseExpireIn24Hours {
						acc: key.clone(),
						size: 1024 * (count as u128),
					});
				}
			}
			0
		}

		fn offchain_worker(block_number: T::BlockNumber) {
			let _signer = Signer::<T, T::AuthorityId>::all_accounts();
			let number: u128 = block_number.saturated_into();
			let one_day: u128 = <T as Config>::OneDay::get().saturated_into();
			if number % one_day == 0 || number == 500 {
				//Query price
				let mut counter = 0;
				loop {
					log::info!("Current offchain machine execution rounds: {:?}", counter + 1);
					if counter == 5 {
						break
					}
					let result = Self::offchain_fetch_price(block_number);
					if let Err(e) = result {
						log::error!("offchain_worker error: {:?}", e);
					} else {
						log::info!("Update price succeeded");
						break
					}
					counter = counter + 1;
				}
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(6_231_000)]
		pub fn upload_declaration(
			origin: OriginFor<T>,
			file_hash: Vec<u8>,
			file_name: Vec<u8>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let file_hash_bound: BoundedString<T> = file_hash.clone().try_into().map_err(|_| Error::<T>::Overflow)?;
			let file_name_bound: BoundedString<T> = file_name.clone().try_into().map_err(|_| Error::<T>::Overflow)?;
			if <File<T>>::contains_key(&file_hash_bound) {
				<File<T>>::try_mutate(&file_hash_bound, |s_opt| -> DispatchResult {
					let s = s_opt.as_mut().ok_or(Error::<T>::FileNonExistent)?;
					if s.user.contains(&sender) {
						Err(Error::<T>::Declarated)?;
					}
					Self::update_user_space(sender.clone(), 1, s.file_size.into())?;
					Self::add_user_hold_fileslice(sender.clone(), file_hash_bound.clone(), s.file_size)?;
					s.user.try_push(sender.clone()).map_err(|_| Error::<T>::StorageLimitReached)?;
					s.file_name.try_push(file_name_bound.clone()).map_err(|_| Error::<T>::StorageLimitReached)?;
					Ok(())
				})?;
			} else {
				let count = <FileIndexCount<T>>::get().checked_add(1).ok_or(Error::<T>::Overflow)?;
				<File<T>>::insert(
					&file_hash_bound,
					FileInfo::<T>{
						file_size: 0,
						index: count,
						file_state: "pending".as_bytes().to_vec().try_into().map_err(|_| Error::<T>::BoundedVecError)?,
						user: vec![sender.clone()].try_into().map_err(|_| Error::<T>::BoundedVecError)?,
						file_name: vec![file_name_bound].try_into().map_err(|_| Error::<T>::BoundedVecError)?,
						slice_info: Default::default(),
					},
				);
				<FileIndexCount<T>>::put(count);
				<FileKeysMap<T>>::insert(count, (false, file_hash_bound.clone()));
			}
			Self::deposit_event(Event::<T>::UploadDeclaration { acc: sender, file_hash: file_hash, file_name: file_name });
			Ok(())
		}
		/// Upload info of stored file.
		///
		/// The dispatch origin of this call must be _Signed_.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upload())]
		pub fn upload(
			origin: OriginFor<T>,
			file_hash: Vec<u8>,
			file_size: u64,
			slice_info: Vec<SliceInfo<T>>,
			user: AccountOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			if !T::Scheduler::contains_scheduler(sender.clone()) {
				Err(Error::<T>::ScheduleNonExistent)?;
			}
			let file_hash_bounded: BoundedString<T> = file_hash.try_into().map_err(|_| Error::<T>::BoundedVecError)?;
			if !<File<T>>::contains_key(&file_hash_bounded) {
				Err(Error::<T>::FileNonExistent)?;
			}
			Self::update_user_space(user.clone(), 1, file_size.into())?;

			<File<T>>::try_mutate(&file_hash_bounded, |s_opt| -> DispatchResult {
				let s = s_opt.as_mut().ok_or(Error::<T>::FileNonExistent)?;
				if !s.user.contains(&user) {
					Err(Error::<T>::UserNotDeclared)?;
				}
				if s.file_state.to_vec() == "active".as_bytes().to_vec() {
					Err(Error::<T>::FileExistent)?;
				}
				s.file_size = file_size;
				s.slice_info = slice_info.clone().try_into().map_err(|_| Error::<T>::BoundedVecError)?;
				s.file_state = "active".as_bytes().to_vec().try_into().map_err(|_| Error::<T>::BoundedVecError)?;
				<FileKeysMap<T>>::try_mutate(s.index, |v_opt| -> DispatchResult{
					let v = v_opt.as_mut().ok_or(Error::<T>::FileNonExistent)?;
					v.0 = true;
					Ok(())
				})?;
				Ok(())
			})?;

			Self::add_user_hold_fileslice(user.clone() ,file_hash_bounded.clone(), file_size)?;
			//To be tested
			for v in slice_info.iter() {
				Self::replace_file(v.miner_acc.clone(), v.shard_size)?;
			}
		
			Self::deposit_event(Event::<T>::FileUpload { acc: user.clone() });
			Ok(())
		}

		//The filler upload interface can only be called by scheduling, and the list has a maximum
		// length limit
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upload_filler(filler_list.len() as u32))]
		pub fn upload_filler(
			origin: OriginFor<T>,
			miner: AccountOf<T>,
			filler_list: Vec<FillerInfo<T>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			if filler_list.len() > 10 {
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
				if <FillerMap<T>>::contains_key(&miner, i.filler_id.clone()) {
					Err(Error::<T>::FileExistent)?;
				}
				let mut value = i.clone();
				value.index = count;
				<FillerMap<T>>::insert(miner.clone(), i.filler_id.clone(), value);
				<FillerKeysMap<T>>::insert(count, (miner.clone(), i.filler_id.clone()));
			}
			<FillerIndexCount<T>>::put(count);

			let power = M_BYTE
				.checked_mul(8)
				.ok_or(Error::<T>::Overflow)?
				.checked_mul(filler_list.len() as u128)
				.ok_or(Error::<T>::Overflow)?;
			T::MinerControl::add_power(miner.clone(), power)?;
			Self::deposit_event(Event::<T>::FillerUpload { acc: sender, file_size: power as u64 });
			Ok(())
		}

		#[pallet::weight(2_000_000)]
		pub fn delete_file(origin: OriginFor<T>, fileid: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let bounded_fileid = Self::vec_to_bound::<u8>(fileid.clone())?;
			ensure!(<File<T>>::contains_key(bounded_fileid.clone()), Error::<T>::FileNonExistent);
			//The above has been judged. Unwrap will be performed only if the key exists
			Self::clear_user_file(bounded_fileid, &sender)?;

			Self::deposit_event(Event::<T>::DeleteFile { acc: sender, fileid });
			Ok(())
		}

		//**********************************************************************************************************************************************
		//************************************************************Storage space lease***********
		//************************************************************Storage **********************
		//************************************************************Storage **********************
		//************************************************************Storage ********
		//**********************************************************************************************************************************************
		//The parameter "space_count" is calculated in gigabyte.
		//parameter "lease_count" is calculated on the monthly basis.
		#[pallet::weight(2_000_000)]
		pub fn buy_space(
			origin: OriginFor<T>,
			space_count: u128,
			lease_count: u128,
			max_price: u128,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let acc = T::FilbakPalletId::get().into_account();
			let cur_price = <UnitPrice<T>>::get();
			if cur_price == 0u32.saturated_into() {
				Err(Error::<T>::IsZero)?;
			}
			let unit_price =
				TryInto::<u128>::try_into(cur_price).map_err(|_e| Error::<T>::Overflow)?;
			if unit_price > max_price * 1000000000000 && 0 != max_price {
				Err(Error::<T>::ExceedExpectations)?;
			}
			//space_count, Represents how many GB of space the user wants to buy
			let space = space_count.checked_mul(1024).ok_or(Error::<T>::Overflow)?;
			//Because there are three backups, it is charged at one-third of the price
			let price: u128 = unit_price
				.checked_mul(space)
				.ok_or(Error::<T>::Overflow)?
				.checked_mul(lease_count)
				.ok_or(Error::<T>::Overflow)?
				.checked_div(3)
				.ok_or(Error::<T>::Overflow)?;
			//Increase the space purchased by users
			//and judge whether there is still space available for purchase
			let money: BalanceOf<T> = price.try_into().map_err(|_e| Error::<T>::ConversionError)?;
			<T as pallet::Config>::Currency::transfer(&sender, &acc, money, AllowDeath)?;
			let now = <frame_system::Pallet<T>>::block_number();
			let one_day: u128 = <T as Config>::OneDay::get().saturated_into();
			let deadline: BlockNumberOf<T> =
				((lease_count.checked_mul(one_day * 30).ok_or(Error::<T>::Overflow)?) as u32)
					.into();
			let list: SpaceInfo<T> = SpaceInfo::<T> {
				size: space.checked_mul(M_BYTE).ok_or(Error::<T>::Overflow)?,
				deadline: now.checked_add(&deadline).ok_or(Error::<T>::Overflow)?,
			};

			<UserSpaceList<T>>::try_mutate(&sender, |s| -> DispatchResult {
				s.try_push(list).map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;
			//Convert MB to BYTE
			Self::user_buy_space_update(
				sender.clone(),
				space.checked_mul(M_BYTE).ok_or(Error::<T>::Overflow)?,
			)?;
			let div2space = space.checked_div(2).ok_or(Error::<T>::Overflow)?;
			let true_space = space.checked_add(div2space).ok_or(Error::<T>::Overflow)?;
			pallet_sminer::Pallet::<T>::add_purchased_space(
				true_space.checked_mul(M_BYTE).ok_or(Error::<T>::Overflow)?
			)?;
			Self::deposit_event(Event::<T>::BuySpace {
				acc: sender.clone(),
				size: space,
				fee: money,
			});
			Ok(())
		}

		//Free space for users
		#[pallet::weight(2_000_000)]
		pub fn receive_free_space(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!<UserFreeRecord<T>>::contains_key(&sender), Error::<T>::AlreadyReceive);
			pallet_sminer::Pallet::<T>::add_purchased_space(
				M_BYTE.checked_mul(1024 + 512).ok_or(Error::<T>::Overflow)?,
			)?;

			let deadline: BlockNumberOf<T> = 999999999u32.into();
			let list: SpaceInfo<T> = SpaceInfo::<T> { size: 1024, deadline };

			<UserSpaceList<T>>::try_mutate(&sender, |s| -> DispatchResult {
				s.try_push(list).map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;

			Self::user_buy_space_update(
				sender.clone(),
				M_BYTE.checked_mul(1024).ok_or(Error::<T>::Overflow)?,
			)?;
			Self::deposit_event(Event::<T>::ReceiveSpace { acc: sender.clone() });
			<UserFreeRecord<T>>::insert(&sender, 1);
			Ok(())
		}

		//Update current storage unit price
		#[pallet::weight(0)]
		pub fn update_price(origin: OriginFor<T>, price: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let member_list = Self::members();
			if !member_list.contains(&sender) {
				Err(Error::<T>::NotQualified)?;
			}
			let now = <frame_system::Pallet<T>>::block_number();
			//Only accept the first price from offchain worker
			let lock_time = <LockTime<T>>::get();
			if lock_time > now {
				Err(Error::<T>::Locked)?;
			}
			//Convert price of string type to balance
			//Vec<u8> -> str
			let str_price = str::from_utf8(&price).unwrap_or_default();
			//str -> u128
			let mut price_u128: u128 =
				str_price.parse().map_err(|_e| Error::<T>::ConversionError)?;

			//One third of the price
			price_u128 = price_u128.checked_div(3).ok_or(Error::<T>::Overflow)?;
			//Get the current price on the chain
			let our_price = Self::get_price()?;
			//Which pricing is cheaper
			if our_price < price_u128 {
				price_u128 = our_price;
			}
			//u128 -> balance
			let price_balance: BalanceOf<T> =
				price_u128.try_into().map_err(|_e| Error::<T>::ConversionError)?;
			<UnitPrice<T>>::put(price_balance);
			let deadline = now.checked_add(&5000u32.into()).ok_or(Error::<T>::Overflow)?;
			<LockTime<T>>::put(deadline);
			Ok(())
		}

		//Feedback results after the miner clears the invalid files
		#[pallet::weight(10_000)]
		pub fn clear_invalid_file(
			origin: OriginFor<T>,
			file_hash: Vec<u8>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let bounded_string: BoundedString<T> =
				file_hash.clone().try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
			<InvalidFile<T>>::try_mutate(&sender, |o| -> DispatchResult {
				o.retain(|x| *x != bounded_string);
				Ok(())
			})?;
			Self::deposit_event(Event::<T>::ClearInvalidFile { acc: sender, file_hash });
			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn add_member(origin: OriginFor<T>, acc: AccountOf<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;
			let member_list = Self::members();
			if member_list.contains(&acc) {
				Err(Error::<T>::AlreadyExist)?;
			}
			<Members<T>>::try_mutate(|o| -> DispatchResult {
				o.try_push(acc).map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;
			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn del_member(origin: OriginFor<T>, acc: AccountOf<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;
			<Members<T>>::try_mutate(|o| -> DispatchResult {
				o.retain(|x| x != &acc);
				Ok(())
			})?;
			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn clear_all_filler(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let state = T::MinerControl::get_miner_state(sender.clone())?;
			if state != "exit".as_bytes().to_vec() {
				Err(Error::<T>::NotQualified)?;
			}
			for (_, value) in FillerMap::<T>::iter_prefix(&sender) {
				<FillerKeysMap<T>>::remove(value.index);
			}
			
			let _ = FillerMap::<T>::remove_prefix(&sender, Option::None);
			Ok(())
		}

		//Scheduling is the notification chain after file recovery
		#[pallet::weight(10_000)]
		pub fn recover_file(origin: OriginFor<T>, shard_id: Vec<u8>, slice_info: SliceInfo<T>, avail: bool) -> DispatchResult {
			//Get fileid from shardid,
			let length = shard_id.len().checked_sub(4).ok_or(Error::<T>::Overflow)?;
			let file_id = shard_id[0..length].to_vec();
			//Vec to BoundedVec
			let file_id_bounded: BoundedString<T> =
				file_id.clone().try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
			let sender = ensure_signed(origin)?;
			let bounded_string: BoundedString<T> =
			shard_id.clone().try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
			//Delete the corresponding recovery slice request pool
			<FileRecovery<T>>::try_mutate(&sender, |o| -> DispatchResult {
				o.retain(|x| *x != bounded_string);
				Ok(())
			})?;
			if !<File<T>>::contains_key(&file_id_bounded) {
				Err(Error::<T>::FileNonExistent)?;
			}
			if avail {
				<File<T>>::try_mutate(&file_id_bounded, |opt| -> DispatchResult {
					let o = opt.as_mut().unwrap();
					o.slice_info.try_push(slice_info).map_err(|_| Error::<T>::StorageLimitReached)?;
					o.file_state = "active"
						.as_bytes()
						.to_vec()
						.try_into()
						.map_err(|_e| Error::<T>::BoundedVecError)?;
					Ok(())
				})?;
			} else {
				Self::clear_file(file_id.clone())?;
			}
			
			Self::deposit_event(Event::<T>::RecoverFile { acc: sender, file_hash: shard_id });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {

		//operation: 1 upload files, 2 delete file
		fn update_user_space(acc: AccountOf<T>, operation: u8, size: u128) -> DispatchResult {
			match operation {
				1 => <UserHoldSpaceDetails<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
					let s = s_opt.as_mut().unwrap();
					if size > s.purchased_space - s.used_space {
						Err(Error::<T>::InsufficientStorage)?;
					}
					if false == Self::check_lease_expired(acc.clone()) {
						Self::deposit_event(Event::<T>::LeaseExpired { acc: acc.clone(), size: 0 });
						Err(Error::<T>::LeaseExpired)?;
					}
					s.remaining_space =
						s.remaining_space.checked_sub(size).ok_or(Error::<T>::Overflow)?;
					s.used_space = s.used_space.checked_add(size).ok_or(Error::<T>::Overflow)?;
					Ok(())
				})?,
				2 => <UserHoldSpaceDetails<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
					let s = s_opt.as_mut().unwrap();
					s.used_space = s.used_space.checked_sub(size).ok_or(Error::<T>::Overflow)?;
					if s.purchased_space > s.used_space {
						s.remaining_space = s
							.purchased_space
							.checked_sub(s.used_space)
							.ok_or(Error::<T>::Overflow)?;
					} else {
						s.remaining_space = 0;
					}
					Ok(())
				})?,
				_ => Err(Error::<T>::WrongOperation)?,
			}
			Ok(())
		}

		fn user_buy_space_update(acc: AccountOf<T>, size: u128) -> DispatchResult {
			if <UserHoldSpaceDetails<T>>::contains_key(&acc) {
				<UserHoldSpaceDetails<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
					let s = s_opt.as_mut().unwrap();
					s.purchased_space =
						s.purchased_space.checked_add(size).ok_or(Error::<T>::Overflow)?;
					if s.purchased_space > s.used_space {
						s.remaining_space = s
							.purchased_space
							.checked_sub(s.used_space)
							.ok_or(Error::<T>::Overflow)?;
					} else {
						s.remaining_space = 0;
					}

					Ok(())
				})?;
			} else {
				let value =
					StorageSpace { purchased_space: size, used_space: 0, remaining_space: size };
				<UserHoldSpaceDetails<T>>::insert(&acc, value);
			}
			Ok(())
		}

		//Available space divided by 1024 is the unit price
		fn get_price() -> Result<u128, DispatchError> {
			//Get the available space on the current chain
			let space = pallet_sminer::Pallet::<T>::get_space()?;
			//If it is not 0, the logic is executed normally
			if space != 0 {
				//Calculation rules
				//The price is based on 1024 / available space on the current chain
				//Multiply by the base value 1 tcess * 1_000 (1_000_000_000_000 * 1_000)
				let price: u128 = M_BYTE
					.checked_mul(1024)
					.ok_or(Error::<T>::Overflow)?
					.checked_mul(1_000_000_000_000)
					.ok_or(Error::<T>::Overflow)?
					.checked_mul(1000)
					.ok_or(Error::<T>::Overflow)?
					.checked_div(space)
					.ok_or(Error::<T>::Overflow)?;
				return Ok(price)
			}
			//If it is 0, an extra large price will be returned
			Ok(1_000_000_000_000_000_000)
		}

		//Before using this method, you must determine whether the primary key fileid exists
		fn check_lease_expired(acc: AccountOf<T>) -> bool {
			let details = <UserHoldSpaceDetails<T>>::get(&acc).unwrap();
			if details.used_space > details.purchased_space {
				false
			} else {
				true
			}
		}

		fn vec_to_bound<P>(param: Vec<P>) -> Result<BoundedVec<P, T::StringLimit>, DispatchError> {
			let result: BoundedVec<P, T::StringLimit> =
				param.try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
			Ok(result)
		}

		//offchain helper
		//Signature chaining method
		fn offchain_signed_tx(
			_block_number: T::BlockNumber,
			price: Vec<u8>,
		) -> Result<(), Error<T>> {
			let signer = Signer::<T, T::AuthorityId>::any_account();

			let result = signer
				.send_signed_transaction(|_account| Call::update_price { price: price.clone() });

			if let Some((acc, res)) = result {
				if res.is_err() {
					log::error!("failure: offchain_signed_tx: tx sent: {:?}", acc.id);
					return Err(<Error<T>>::OffchainSignedTxError)
				}
				// Transaction is sent successfully
				return Ok(())
			}

			// The case of `None`: no account is available for sending
			log::error!("No local account available");
			Err(<Error<T>>::NoLocalAcctForSigning)
		}

		pub fn offchain_fetch_price(block_number: T::BlockNumber) -> Result<(), Error<T>> {
			//Call HTTP request method
			let resp_bytes = Self::offchain_http_req().map_err(|e| {
				log::error!("fetch_from_remote error: {:?}", e);
				<Error<T>>::HttpFetchingError
			})?;
			//Pass the value returned from the request to the signing authority
			let _ = Self::offchain_signed_tx(block_number, resp_bytes)?;

			Ok(())
		}
		//Offline worker, HTTP request method.
		fn offchain_http_req() -> Result<Vec<u8>, Error<T>> {
			log::info!("send request to {}", HTTP_REQUEST_STR);
			//Create request
			//The request address is: https://arweave.net/price/1048576
			//Arweave's official API is used to query prices
			//Request price of 1MB
			let request = rt_offchain::http::Request::get(HTTP_REQUEST_STR);
			//Set timeout wait
			let timeout = sp_io::offchain::timestamp()
				.add(rt_offchain::Duration::from_millis(FETCH_TIMEOUT_PERIOD));

			log::info!("send request");
			//Send request
			let pending = request
				.add_header("User-Agent", "PostmanRuntime/7.28.4")
				.deadline(timeout) // Setting the timeout time
				.send() // Sending the request out by the host
				.map_err(|_| <Error<T>>::HttpFetchingError)?;

			log::info!("wating response");
			//Waiting for response
			let response = pending.wait().map_err(|_| <Error<T>>::HttpFetchingError)?;
			//Determine whether the response is wrong
			if response.code != 200 {
				log::error!("Unexpected http request status code: {}", response.code);
				Err(<Error<T>>::HttpFetchingError)?;
			}

			Ok(response.body().collect::<Vec<u8>>())
		}

		pub fn get_random_challenge_data(
		) -> Result<Vec<(AccountOf<T>, Vec<u8>, Vec<u8>, u64, u8)>, DispatchError> {
			let filler_list = Self::get_random_filler()?;
			let mut data: Vec<(AccountOf<T>, Vec<u8>, Vec<u8>, u64, u8)> = Vec::new();
			for v in filler_list {
				let length = v.block_num;
				let number_list = Self::get_random_numberlist(length, 3, length)?;
				let miner_acc = v.miner_address.clone();
				let filler_id = v.filler_id.clone().to_vec();
				let file_size = v.filler_size.clone();
				let mut block_list: Vec<u8> = Vec::new();
				for i in number_list.iter() {
					block_list.push((*i as u8) + 1);
				}
				data.push((miner_acc, filler_id, block_list, file_size, 1));
			}

			let file_list = Self::get_random_file()?;
			for (_, file) in file_list {
				let slice_number_list = Self::get_random_numberlist(file.slice_info.len() as u32, 3, file.slice_info.len() as u32)?;
				for slice_index in slice_number_list.iter() {
					let miner_id = T::MinerControl::get_miner_id(file.slice_info[*slice_index as usize].miner_acc.clone())?;
					if file.slice_info[*slice_index as usize].miner_id != miner_id {
						continue;
					}
					let mut block_list: Vec<u8> = Vec::new();
					let length = file.slice_info[*slice_index as usize].block_num;
					let number_list = Self::get_random_numberlist(length, 3, length)?;
					let file_hash = file.slice_info[*slice_index as usize].shard_id.to_vec();
					let miner_acc = file.slice_info[*slice_index as usize].miner_acc.clone();
					let slice_size = file.slice_info[*slice_index as usize].shard_size;
					for i in number_list.iter() {
						block_list.push((*i as u8) + 1);
					}
					data.push((miner_acc, file_hash, block_list, slice_size, 2));
				}		
			}

			Ok(data)
		}
		//Get random file block list
		fn get_random_filler() -> Result<Vec<FillerInfo<T>>, DispatchError> {
			let length = Self::get_fillermap_length()?;
			let limit = <FillerIndexCount<T>>::get();
			let number_list = Self::get_random_numberlist(length, 1, limit)?;
			let mut filler_list: Vec<FillerInfo<T>> = Vec::new();
			for i in number_list.iter() {
				let result = <FillerKeysMap<T>>::get(i);
				let (acc, filler_id) = match result {
					Some(x) => x,
					None => {
						Err(Error::<T>::BugInvalid)?
					},
				};
				let filler = <FillerMap<T>>::try_get(acc, filler_id).map_err(|_e| Error::<T>::BugInvalid)?;
				filler_list.push(filler);
			}
			Ok(filler_list)
		}

		fn get_random_file() -> Result<Vec<(BoundedString<T>, FileInfo<T>)>, DispatchError> {
			let length = Self::get_file_map_length()?;
			let limit = <FileIndexCount<T>>::get();
			let number_list = Self::get_random_numberlist(length, 2, limit)?;
			let mut file_list: Vec<(BoundedString<T>, FileInfo<T>)> = Vec::new();
			for i in number_list.iter() {
				let file_id = <FileKeysMap<T>>::try_get(i).map_err(|_e| Error::<T>::FileNonExistent)?.1;
				let value = <File<T>>::try_get(&file_id).map_err(|_e| Error::<T>::FileNonExistent)?;
				if value.file_state.to_vec() == "active".as_bytes().to_vec() {
					file_list.push(
						(
							file_id.clone(),
							value,
						)
					);
				}
			}
			Ok(file_list)
		}

		fn get_random_numberlist(length: u32, random_type: u8, limit: u32) -> Result<Vec<u32>, DispatchError> {
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
					continue;
				}
				log::info!("List addition: {}", random);
				number_list.push(random);
			}
			Ok(number_list)
		}

		fn judge_file_exist(index: u32) -> bool {
			let result = <FileKeysMap<T>>::get(index);
			let result = match result {
				Some(x) => x.0,
				None => false,
			};
			result 
		}

		fn judge_filler_exist(index: u32) -> bool {
			let result = <FillerKeysMap<T>>::get(index);
			let result = match result {
				Some(x) => true,
				None => false,
			};
			result 
		}

		//Get storagemap filler length
		fn get_fillermap_length() -> Result<u32, DispatchError> {
			let count = <FillerKeysMap<T>>::count();
			Ok(count)
		}

		//Get Storage FillerMap Length
		fn get_file_map_length() -> Result<u32, DispatchError> {
			Ok(<FileKeysMap<T>>::count())
		}

		//Get random number
		pub fn generate_random_number(seed: u32) -> Result<u32, DispatchError> {
			let mut counter = 0;
			loop {
				let (random_seed, _) =
					T::MyRandomness::random(&(T::FilbakPalletId::get(), seed + counter).encode());
				let random_number = <u32>::decode(&mut random_seed.as_ref()).unwrap_or(0);
				if random_number != 0 {
					return Ok(random_number)
				}
				counter = counter.checked_add(1).ok_or(Error::<T>::Overflow)?;
			}
		}

		//Specific implementation method of deleting filler file
		pub fn delete_filler(miner_acc: AccountOf<T>, filler_id: Vec<u8>) -> DispatchResult {
			let filler_boud: BoundedString<T> =
				filler_id.try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
			if !<FillerMap<T>>::contains_key(&miner_acc, filler_boud.clone()) {
				Err(Error::<T>::FileNonExistent)?;
			}
			let value = <FillerMap<T>>::try_get(&miner_acc, filler_boud.clone()).map_err(|_e| Error::<T>::FileNonExistent)?;
			<FillerKeysMap<T>>::remove(value.index);
			<FillerMap<T>>::remove(miner_acc, filler_boud.clone());

			Ok(())
		}

		//Delete the next backup under the file
		pub fn clear_file(file_hash: Vec<u8>) -> DispatchResult {
			let file_hash_bounded: BoundedString<T> =
				file_hash.try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
			let file = <File<T>>::try_get(&file_hash_bounded).map_err(|_| Error::<T>::FileNonExistent)?;
			for user in file.user.iter() {
				Self::update_user_space(user.clone(), 2, file.file_size.into())?;
			}
			for slice in file.slice_info.iter() {
				Self::add_invalid_file(slice.miner_acc.clone(), slice.shard_id.to_vec())?;
				T::MinerControl::sub_space(slice.miner_acc.clone(), slice.shard_size.into())?;
			}
			<File<T>>::remove(file_hash_bounded);
			<FileKeysMap<T>>::remove(file.index);
			Ok(())
		}

		pub fn clear_user_file(file_hash: BoundedVec<u8, T::StringLimit>, user: &AccountOf<T>) -> DispatchResult {
			let file = <File<T>>::get(&file_hash).unwrap();
			ensure!(file.user.contains(user),  Error::<T>::NotOwner);
			Self::update_user_space(
				user.clone(),
				2,
				file.file_size.clone().into(),
			)?;
			//If the file still has an owner, only the corresponding owner will be cleared. 
			//If the owner is unique, the file meta information will be cleared.
			if file.user.len() > 1 {
				<File<T>>::try_mutate(&file_hash, |s_opt| -> DispatchResult {
					let s = s_opt.as_mut().unwrap();
					let mut index = 0;
					for acc in s.user.iter() {
						if *acc == user.clone() {
							break;
						}
						index = index.checked_add(&1).ok_or(Error::<T>::Overflow)?;
					}
					s.user.remove(index);
					s.file_name.remove(index);
					Ok(())
				})?;
			} else {
				Self::clear_file(file_hash.clone().to_vec())?;
			}
			
			<UserHoldFileList<T>>::try_mutate(&user, |s| -> DispatchResult {
				s.retain(|x| x.file_hash != file_hash.clone());
				Ok(())
			})?;
			Ok(())
		}

		//Add the list of files to be recovered and notify the scheduler to recover
		pub fn add_recovery_file(shard_id: Vec<u8>) -> DispatchResult {
			let acc = Self::get_current_scheduler();
			let length = shard_id.len().checked_sub(4).ok_or(Error::<T>::Overflow)?;
			let file_id = shard_id[0..length].to_vec();
			let file_id_bounded: BoundedString<T> =
				file_id.try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
			if !<File<T>>::contains_key(&file_id_bounded) {
				Err(Error::<T>::FileNonExistent)?;
			}
			<File<T>>::try_mutate(&file_id_bounded, |opt| -> DispatchResult {
				let o = opt.as_mut().unwrap();
				o.slice_info.retain(|x| x.shard_id.to_vec() != shard_id);
				Ok(())
			})?;
			<FileRecovery<T>>::try_mutate(&acc, |o| -> DispatchResult {
				o.try_push(shard_id.try_into().map_err(|_e| Error::<T>::BoundedVecError)?)
					.map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;

			Ok(())
		}

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
			let mut filler_id_list: BoundedList<T> = Default::default();
			for (filler_id, _) in <FillerMap<T>>::iter_prefix(miner_acc.clone()) {
				if counter == replace_num {
					break
				}
				filler_id_list.try_push(filler_id.clone()).map_err(|_| Error::<T>::StorageLimitReached)?;
				
				counter = counter.checked_add(1).ok_or(Error::<T>::Overflow)?;
				//Clear information on the chain
				Self::delete_filler(miner_acc.clone(), filler_id.to_vec())?;
			}
			
			//Notify the miner to clear the corresponding data segment
			<InvalidFile<T>>::try_mutate(&miner_acc, |o| -> DispatchResult {
				for file_hash in filler_id_list {
					o.try_push(file_hash).map_err(|_e| Error::<T>::StorageLimitReached)?;
				}
				Ok(())
			})?;
			//add space
			T::MinerControl::add_space(miner_acc.clone(), file_size.into())?;
			T::MinerControl::sub_power(miner_acc.clone(), replace_num * M_BYTE * 8)?;
			Ok(())
		}

		//Add invalid file list, notify miner to delete
		pub fn add_invalid_file(miner_acc: AccountOf<T>, file_hash: Vec<u8>) -> DispatchResult {
			<InvalidFile<T>>::try_mutate(&miner_acc, |o| -> DispatchResult {
				o.try_push(file_hash.try_into().map_err(|_e| Error::<T>::BoundedVecError)?)
					.map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;

			Ok(())
		}

		pub fn update_price_for_tests() -> DispatchResult {
			let price: BalanceOf<T> = 100u128.try_into().map_err(|_| Error::<T>::Overflow)?;
			UnitPrice::<T>::put(price);
			Ok(())
		}

		fn add_user_hold_fileslice(user: AccountOf<T>, file_hash_bound: BoundedVec<u8, T::StringLimit>, file_size: u64) -> DispatchResult {
			let file_info = UserFileSliceInfo::<T>{
				file_hash: file_hash_bound.clone(),
				file_size: file_size,
			};
			<UserHoldFileList<T>>::try_mutate(&user, |v| -> DispatchResult {
				v.try_push(file_info).map_err(|_| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;

			Ok(())
		}

		//Obtain the consensus of the current block
		fn get_current_scheduler() -> AccountOf<T> {
			//Current block information
			let digest = <frame_system::Pallet<T>>::digest();
			let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
			//TODO
			let acc = T::FindAuthor::find_author(pre_runtime_digests).map(|a| a);
			T::Scheduler::get_controller_acc(acc.unwrap())
		}
	
		fn clear_expired_file(acc: &AccountOf<T>, used_space: u128, purchased_space: u128) -> DispatchResult {
			let diff = used_space.checked_div(purchased_space).ok_or(Error::<T>::Overflow)?;
			let mut clear_file_size: u128 = 0;
			let file_list = <UserHoldFileList<T>>::try_get(&acc).map_err(|_| Error::<T>::Overflow)?;
			for v in file_list.iter() {
				if clear_file_size > diff {
					break;
				}
				Self::clear_user_file(v.file_hash.clone(), acc)?;
				
				clear_file_size = clear_file_size.checked_add(v.file_size.into()).ok_or(Error::<T>::Overflow)?;
			}

			Ok(())
		}
	}
}

pub trait RandomFileList<AccountId> {
	//Get random challenge data
	fn get_random_challenge_data(
	) -> Result<Vec<(AccountId, Vec<u8>, Vec<u8>, u64, u8)>, DispatchError>;
	//Delete filler file
	fn delete_filler(miner_acc: AccountId, filler_id: Vec<u8>) -> DispatchResult;
	//Delete all filler according to miner_acc
	fn delete_miner_all_filler(miner_acc: AccountId) -> DispatchResult;
	//Delete file backup
	fn clear_file(file_hash: Vec<u8>) -> DispatchResult;
	//The function executed when the challenge fails, allowing the miner to delete invalid files
	fn add_recovery_file(file_id: Vec<u8>) -> DispatchResult;
	//The function executed when the challenge fails to let the consensus schedule recover the file
	fn add_invalid_file(miner_acc: AccountId, file_hash: Vec<u8>) -> DispatchResult;
	//Judge whether it is a user who can initiate transactions on the off chain machine
	fn contains_member(acc: AccountId) -> bool;
}

impl<T: Config> RandomFileList<<T as frame_system::Config>::AccountId> for Pallet<T> {
	fn get_random_challenge_data(
	) -> Result<Vec<(AccountOf<T>, Vec<u8>, Vec<u8>, u64, u8)>, DispatchError> {
		let result = Pallet::<T>::get_random_challenge_data()?;
		Ok(result)
	}

	fn delete_filler(miner_acc: AccountOf<T>, filler_id: Vec<u8>) -> DispatchResult {
		Pallet::<T>::delete_filler(miner_acc, filler_id)?;
		Ok(())
	}
	
	fn delete_miner_all_filler(miner_acc: AccountOf<T>) -> DispatchResult {
		let _ = FillerMap::<T>::remove_prefix(&miner_acc, Option::None);
		Ok(())
	}

	fn clear_file(file_hash: Vec<u8>) -> DispatchResult {
		Pallet::<T>::clear_file(file_hash)?;
		Ok(())
	}

	fn add_recovery_file(file_id: Vec<u8>) -> DispatchResult {
		Pallet::<T>::add_recovery_file(file_id)?;
		Ok(())
	}

	fn add_invalid_file(miner_acc: AccountOf<T>, file_hash: Vec<u8>) -> DispatchResult {
		Pallet::<T>::add_invalid_file(miner_acc, file_hash)?;
		Ok(())
	}

	fn contains_member(acc: AccountOf<T>) -> bool {
		let member_list = Self::members();
		if member_list.contains(&acc) {
			return true
		} else {
			return false
		}
	}
}

impl<T: Config> BlockNumberProvider for Pallet<T> {
	type BlockNumber = T::BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		<frame_system::Pallet<T>>::block_number()
	}
}
