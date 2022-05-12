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

use frame_support::traits::{Currency, ReservableCurrency, ExistenceRequirement::AllowDeath, Randomness, FindAuthor};
pub use pallet::*;
mod benchmarking;
pub mod weights;

mod types;
pub use types::*;

use scale_info::TypeInfo;
use sp_runtime::{
	RuntimeDebug,
	traits::{AccountIdConversion,SaturatedConversion}
};
use sp_std::{
	prelude::*,
	convert::TryInto,
	str,
};
use codec::{Encode, Decode};

use frame_support::{
	pallet_prelude::*,
	dispatch::DispatchResult, 
	PalletId,
};
use sp_runtime::{
	traits::{
		BlockNumberProvider
	},
	offchain as rt_offchain,
};
use frame_system::{
	offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer, 
	},
};
pub use weights::WeightInfo;
use sp_core::{crypto::KeyTypeId};

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
type BoundedString<T> = BoundedVec<u8, <T as Config>::StringLimit>;
type BoundedList<T> = BoundedVec<BoundedVec<u8, <T as Config>::StringLimit>, <T as Config>::StringLimit>;



#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		ensure,
		traits::Get,
	};
	use pallet_file_map::ScheduleFind;
	use pallet_sminer::MinerControl;
	//pub use crate::weights::WeightInfo;
	use frame_system::{ensure_signed, pallet_prelude::*};
	

	const HTTP_REQUEST_STR: &str = "https://arweave.net/price/1048576";
		// const HTTP_REQUEST_STR: &str = "https://api.coincap.io/v2/assets/polkadot";
	pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"demo");
	const FETCH_TIMEOUT_PERIOD: u64 = 60_000; // in milli-seconds

	pub mod crypto {
		use super::KEY_TYPE;
		use sp_core::sr25519::Signature as Sr25519Signature;
		use sp_runtime::app_crypto::{app_crypto, sr25519};
		use sp_runtime::{traits::Verify, MultiSignature, MultiSigner};

		app_crypto!(sr25519, KEY_TYPE);

		pub struct TestAuthId;
		// implemented for ocw-runtime
		impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::sr25519::Signature;
			type GenericPublic = sp_core::sr25519::Public;
		}

		// implemented for mock runtime in test
		impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
		{
			type RuntimeAppPublic = Public;
			type GenericSignature = sp_core::sr25519::Signature;
			type GenericPublic = sp_core::sr25519::Public;
		}
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_sminer::Config + sp_std::fmt::Debug  + CreateSignedTransaction<Call<Self>> {
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
		type MinerControl: MinerControl;
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
		//file uploaded.
		FileUpload{acc: AccountOf<T>},
		//file updated.
		FileUpdate{acc: AccountOf<T>, fileid: Vec<u8>},

		FileChangeState{acc: AccountOf<T>, fileid: Vec<u8>},
		//file bought.
		BuyFile{acc: AccountOf<T>, money: BalanceOf<T>, fileid: Vec<u8>},
		//file purchased before.
		Purchased{acc: AccountOf<T>, fileid: Vec<u8>},
		//Storage information of scheduling storage file slice
		InsertFileSlice{fileid: Vec<u8>},		
		//User purchase space
		BuySpace{acc: AccountOf<T>, size: u128, fee: BalanceOf<T>},
		//Expired storage space
		LeaseExpired{acc: AccountOf<T>, size: u128},
		//Storage space expiring within 24 hours
		LeaseExpireIn24Hours{acc: AccountOf<T>, size: u128},
		//File deletion event
		DeleteFile{acc: AccountOf<T>, fileid: Vec<u8>},

		UserAuth{user: AccountOf<T>, collaterals: BalanceOf<T>, random: u32},
		//Filler chain success event
		FillerUpload{acc: AccountOf<T>, file_size: u64},

		RecoverFile{acc: AccountOf<T>, file_id: Vec<u8>},

		ClearInvalidFile{acc: AccountOf<T>, file_id: Vec<u8>},
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

		NotUser,
		//HTTP interaction error of offline working machine
		HttpFetchingError,
		//Signature error of offline working machine
		OffchainSignedTxError,
		//Signature error of offline working machine
		NoLocalAcctForSigning,
		//It is not an error message for scheduling operation
		ScheduleNonExistent,
		//Error reporting when boundedvec is converted to VEC
		BoundedVecError,
		//Error that the storage has reached the upper limit.
		StorageLimitReached,
		//The miner's calculation power is insufficient, resulting in an error that cannot be replaced
		MinerPowerInsufficient
	}
	#[pallet::storage]
	#[pallet::getter(fn file)]
	pub(super) type File<T: Config> = StorageMap<_, Twox64Concat, BoundedString<T>, FileInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn invoice)]
	pub(super) type Invoice<T: Config> = StorageMap<_, Twox64Concat, BoundedString<T>, u8, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn seg_info)]
	pub(super) type UserFileSize<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, u128, ValueQuery>;


	#[pallet::storage]
	#[pallet::getter(fn user_hold_file_list)]
	pub(super) type UserHoldFileList<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BoundedList<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn user_hold_storage_space)]
	pub(super) type UserHoldSpaceDetails<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, StorageSpace>;

	#[pallet::storage]
	#[pallet::getter(fn user_spance_details)]
	pub(super) type UserSpaceList<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<SpaceInfo<T>, T::ItemLimit>, ValueQuery>;

	#[pallet::storage]
	pub(super) type UserFreeRecord<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, u8, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn user_info_map)]
	pub(super) type UserInfoMap<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, UserInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn unit_price)]
	pub(super) type UnitPrice<T: Config> = StorageValue<_, BalanceOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn filler_map)]
	pub(super) type FillerMap<T: Config> = StorageDoubleMap<_, Twox64Concat, u64, Twox64Concat, BoundedString<T>, FillerInfo<T> >;

	#[pallet::storage]
	#[pallet::getter(fn file_recovery)]
	pub(super) type FileRecovery<T: Config> = StorageMap<_, Twox64Concat, AccountOf<T>, BoundedList<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn invalid_file)]
	pub(super) type InvalidFile<T: Config> = StorageMap<_, Twox64Concat, u64, BoundedList<T>, ValueQuery>;


	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberOf<T>> for Pallet<T> {
		//Used to calculate whether it is implied to submit spatiotemporal proof
		//Cycle every 7.2 hours
		//When there is an uncommitted space-time certificate, the corresponding miner will be punished 
		//and the corresponding data segment will be removed
		fn on_initialize(now: BlockNumberOf<T>) -> Weight {
			let number: u128 = now.saturated_into();
			let block_oneday: BlockNumberOf<T> = (28800 as u32).into();
			let mut count: u8 = 0;
			if number % 28800 == 0 {
				for (key, value) in <UserSpaceList<T>>::iter() {
					let mut k = 0;
					let mut list = <UserSpaceList<T>>::get(&key);
					for s in value.iter() {
						let size = s.size;
						if now >= s.deadline {
							list.remove(k);
							<UserHoldSpaceDetails<T>>::mutate(&key, |s_opt|{
								let v = s_opt.as_mut().unwrap();
								v.purchased_space = v.purchased_space - size * 1024;
								if v.remaining_space > size * 1024 {
									v.remaining_space = v.remaining_space - size * 1024;
								}
							});
							let _ = pallet_sminer::Pallet::<T>::sub_purchased_space(size);
							Self::deposit_event(Event::<T>::LeaseExpired{acc: key.clone(), size: size});
							k-= 1;
						} else if s.deadline < now && now >= s.deadline - block_oneday {
							count += 1;
							
						} 
						k+= 1;
					}
					<UserSpaceList<T>>::insert(&key, list);
					Self::deposit_event(Event::<T>::LeaseExpireIn24Hours{acc: key.clone(), size: 1024 * (count as u128)});
				}
			}
			0
		}

		fn offchain_worker(block_number: T::BlockNumber) {
			let _signer = Signer::<T, T::AuthorityId>::all_accounts();

			let number: u128 = block_number.saturated_into();
			let one_day: u128 = <T as Config>::OneDay::get().saturated_into();
			if number % one_day == 0 {
				//Query price
				let result = Self::offchain_fetch_price(block_number);
				if let Err(e) = result {
					log::error!("offchain_worker error: {:?}", e);
				}
			}
		}

	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		/// Upload info of stored file.
		/// 
		/// The dispatch origin of this call must be _Signed_.
		/// 
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upload())]
		pub fn upload(
			origin: OriginFor<T>,
			address: Vec<u8>,
			filename:Vec<u8>,
			fileid: Vec<u8>,
			filehash: Vec<u8>,
			public: bool,
			backups: u8,
			filesize: u64,
			downloadfee:BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			// let acc = T::FilbakPalletId::get().into_account();
			// T::Currency::transfer(&sender, &acc, uploadfee, AllowDeath)?;
			
			Self::upload_file(&sender, &address, &filename, &fileid, &filehash, public, backups, filesize, downloadfee)?;
			Self::deposit_event(Event::<T>::FileUpload{acc: sender.clone()});
			Ok(())
		}

		//The filler upload interface can only be called by scheduling, and the list has a maximum length limit
		#[pallet::weight(1_000_000)]
		pub fn upload_filler(
			origin: OriginFor<T>,
			miner_id: u64,
			filler_list: Vec<FillerInfo<T>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			if !T::Scheduler::contains_scheduler(sender.clone()) {
				Err(Error::<T>::ScheduleNonExistent)?;
			}

			for i in filler_list.iter() {
				<FillerMap<T>>::insert(
					miner_id,
					i.filler_id.clone(),
					i
				);
			}
			T::MinerControl::add_power(miner_id.clone(), 8 * filler_list.len() as u128 )?;
			Self::deposit_event(Event::<T>::FillerUpload{acc: sender, file_size: 8 * filler_list.len() as u64});
			Ok(())

		}

		/// Update info of uploaded file.
		/// 
		/// The dispatch origin of this call must be _Signed_.
		/// 
		/// Parameters:
		/// - `fileid`: id of file, each file will have different number, even for the same file.
		/// - `is_public`: public or private.
		/// - `similarityhash`: hash of file, used for checking similarity.
		#[pallet::weight(1_000_000)]
		pub fn update_dupl(origin: OriginFor<T>, fileid: Vec<u8>, file_dupl: Vec<FileDuplicateInfo<T>> ) -> DispatchResult{
			let sender = ensure_signed(origin)?;
			let bounded_fileid = Self::vec_to_bound::<u8>(fileid.clone())?;
			ensure!((<File<T>>::contains_key(bounded_fileid.clone())), Error::<T>::FileNonExistent);
			//Judge whether it is a consensus node

			<File<T>>::try_mutate(bounded_fileid.clone(), |s_opt| -> DispatchResult {
				let s = s_opt.as_mut().unwrap();
				//Replace idle files with service files
				Self::replace_file(file_dupl.clone(), s.file_size)?;
				
				s.file_state = Self::vec_to_bound::<u8>("active".as_bytes().to_vec())?;
				s.file_dupl = Self::vec_to_bound::<FileDuplicateInfo<T>>(file_dupl)?;
				Ok(())
			})?;
			Self::deposit_event(Event::<T>::FileUpdate{acc: sender.clone(), fileid: fileid});

			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn update_file_state(origin: OriginFor<T>, fileid: Vec<u8>, state: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let bounded_fileid = Self::vec_to_bound::<u8>(fileid.clone())?;
			ensure!((<File<T>>::contains_key(bounded_fileid.clone())), Error::<T>::FileNonExistent);
			//Judge whether it is a consensus node

			<File<T>>::try_mutate(bounded_fileid.clone(), |s_opt| -> DispatchResult{
				let s = s_opt.as_mut().unwrap();
				//To prevent multiple scheduling
				if s.file_state == "repairing".as_bytes().to_vec() && state == "repairing".as_bytes().to_vec() {
					Err(Error::<T>::AlreadyRepair)?;
				}

				s.file_state = Self::vec_to_bound::<u8>(state)?;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::FileChangeState{acc: sender.clone(), fileid: fileid});
			Ok(())
		}

		#[pallet::weight(2_000_000)]
		pub fn delete_file(origin: OriginFor<T>, fileid: Vec<u8>) -> DispatchResult{
			let sender = ensure_signed(origin)?;
			let bounded_fileid = Self::vec_to_bound::<u8>(fileid.clone())?;
			ensure!((<File<T>>::contains_key(bounded_fileid.clone())), Error::<T>::FileNonExistent);
			let file = <File<T>>::get(&bounded_fileid).unwrap();
			if file.user_addr != sender.clone() {
				Err(Error::<T>::NotOwner)?;
			}

			Self::update_user_space(sender.clone(), 2, file.file_size.into())?;
			<File::<T>>::remove(&bounded_fileid);

			Self::deposit_event(Event::<T>::DeleteFile{acc: sender, fileid: fileid});
			Ok(())
		}

		#[pallet::weight(2_000_000)]
		pub fn buyfile(origin: OriginFor<T>, fileid: Vec<u8>, address: Vec<u8>) -> DispatchResult{
			let sender = ensure_signed(origin)?;
			let bounded_fileid = Self::vec_to_bound::<u8>(fileid.clone())?;
			ensure!((<File<T>>::contains_key(bounded_fileid.clone())), Error::<T>::FileNonExistent);
			ensure!(Self::check_lease_expired_forfileid(fileid.clone()), Error::<T>::LeaseExpired);
			let group_id = <File<T>>::get(bounded_fileid.clone()).unwrap();

			let mut invoice: Vec<u8> = Vec::new();
			for i in &fileid {
				invoice.push(*i);
			}
			for i in &address {
				invoice.push(*i);
			}
			let bounded_invoice = Self::vec_to_bound::<u8>(invoice.clone())?;
			if <Invoice<T>>::contains_key(bounded_invoice.clone()) {
				Self::deposit_event(Event::<T>::Purchased{acc: sender.clone(), fileid: fileid.clone()});
			} else {
				let zh = TryInto::<u128>::try_into(group_id.downloadfee).ok().unwrap();
				//let umoney = zh * 8 / 10;
				let umoney = zh.checked_mul(8).ok_or(Error::<T>::Overflow)?
					.checked_div(10).ok_or(Error::<T>::Overflow)?;
				let money: Option<BalanceOf<T>> = umoney.try_into().ok();
				let acc = T::FilbakPalletId::get().into_account();
				<T as pallet::Config>::Currency::transfer(&sender, &group_id.user_addr, money.unwrap(), AllowDeath)?;
				<T as pallet::Config>::Currency::transfer(&sender, &acc, group_id.downloadfee - money.unwrap(), AllowDeath)?;
				<Invoice<T>>::insert(
					bounded_invoice,
					0
				);
				Self::deposit_event(Event::<T>::BuyFile{acc: sender.clone(), money: group_id.downloadfee.clone(), fileid: fileid.clone()});
			}
			
			Ok(())
		}

		//**********************************************************************************************************************************************
		//************************************************************Storage space lease***************************************************************
		//**********************************************************************************************************************************************
		//The parameter "space_count" is calculated in gigabyte.
		//parameter "lease_count" is calculated on the monthly basis.
		#[pallet::weight(2_000_000)]
		pub fn buy_space(origin: OriginFor<T>, space_count: u128, lease_count: u128, max_price: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let acc = T::FilbakPalletId::get().into_account();
			let unit_price = TryInto::<u128>::try_into(<UnitPrice<T>>::get().unwrap()).ok().unwrap();
			if unit_price > max_price * 1000000000000 && 0 != max_price {
				Err(Error::<T>::ExceedExpectations)?;
			}
			//space_count, Represents how many GB of space the user wants to buy
			let space = 1024 * space_count;
			//Because there are three backups, it is charged at one-third of the price
			let price = unit_price
				.checked_mul(space).ok_or(Error::<T>::Overflow)?
				.checked_mul(lease_count).ok_or(Error::<T>::Overflow)?
				.checked_div(3).ok_or(Error::<T>::Overflow)?;
			//Increase the space purchased by users 
			//and judge whether there is still space available for purchase
			pallet_sminer::Pallet::<T>::add_purchased_space(space)?;

			let money: BalanceOf<T> = price.try_into().map_err(|_e| Error::<T>::ConversionError)?;
			<T as pallet::Config>::Currency::transfer(&sender, &acc, money, AllowDeath)?;
			let now = <frame_system::Pallet<T>>::block_number();
			let deadline: BlockNumberOf<T> = ((864000 * lease_count) as u32).into();
			let list: SpaceInfo<T> = SpaceInfo::<T>{
				size: space, 
				deadline: now + deadline,
			};

			<UserSpaceList<T>>::try_mutate(&sender, |s| -> DispatchResult{
				s.try_push(list).expect("Length exceeded");
				Ok(())
			})?;
			//Convert MB to KB
			Self::user_buy_space_update(sender.clone(), space * 1024)?;

			Self::deposit_event(Event::<T>::BuySpace{acc: sender.clone(), size: space, fee: money});
			Ok(())
		}

		//Free space for users
		#[pallet::weight(2_000_000)]
		pub fn receive_free_space(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!<UserFreeRecord<T>>::contains_key(&sender), Error::<T>::AlreadyReceive);
			pallet_sminer::Pallet::<T>::add_purchased_space(1024)?;
			
			let deadline: BlockNumberOf<T> = 999999999u32.into();
			let list: SpaceInfo<T> = SpaceInfo::<T>{size: 1024, deadline};
		
			<UserSpaceList<T>>::try_mutate(&sender, |s| -> DispatchResult{
				s.try_push(list).expect("Length exceeded");
				Ok(())
			})?;

			Self::user_buy_space_update(sender.clone(), 1024 * 1024)?;
			<UserFreeRecord<T>>::insert(&sender, 1);
			Ok(())
		}

		//Test method：Clear the storage space owned by the user
		#[pallet::weight(2_000_000)]
		pub fn initi_acc(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			<UserSpaceList<T>>::remove(&sender);
			<UserHoldSpaceDetails<T>>::remove(&sender);
			Ok(())
		}

		//Update current storage unit price
		#[pallet::weight(2_000_000)]
		pub fn update_price(
			origin: OriginFor<T>,
			price: Vec<u8>
		) -> DispatchResult {
			let _sender = ensure_signed(origin)?;
			//Convert price of string type to balance
			//Vec<u8> -> str
			let str_price = str::from_utf8(&price).unwrap();
			//str -> u128
			let mut price_u128: u128 = str_price
				.parse()
				.map_err(|_e| Error::<T>::ConversionError)?;
			
			//One third of the price
			price_u128 = price_u128.checked_mul(3).ok_or(Error::<T>::Overflow)?;
				
			//Get the current price on the chain
			let our_price = Self::get_price();
			//Which pricing is cheaper
			if our_price < price_u128 {
				price_u128 = our_price;
			}
			//u128 -> balance
			let price_balance: BalanceOf<T> = price_u128.try_into().map_err(|_e| Error::<T>::ConversionError)?;
			<UnitPrice<T>>::put(price_balance);

			Ok(())
		}

		//Scheduling is the notification chain after file recovery
		#[pallet::weight(10_000)]
		pub fn recover_file(origin: OriginFor<T>, dupl_id: Vec<u8>) -> DispatchResult {
			let length = dupl_id.len() - 4;
			let file_id = dupl_id[0..length].to_vec();
			let file_id_bounded: BoundedString<T> = file_id.try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
			let sender = ensure_signed(origin)?;
			let bounded_string: BoundedString<T> = dupl_id.clone().try_into().map_err(|_e| Error::<T>::BoundedVecError)?;

			<FileRecovery<T>>::try_mutate(&sender, |o| -> DispatchResult {
				o.retain(|x| *x != bounded_string);
				Ok(())
			})?;
			if !<File<T>>::contains_key(&file_id_bounded) {
				Err(Error::<T>::FileNonExistent)?;
			}
			<File<T>>::try_mutate(&file_id_bounded, |opt| -> DispatchResult {
				let o = opt.as_mut().unwrap();
				o.file_state = "active".as_bytes().to_vec().try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
				Ok(())
			})?;
			Self::deposit_event(Event::<T>::RecoverFile{acc: sender, file_id: dupl_id});
			Ok(())
		}

		//Feedback results after the miner clears the invalid files
		#[pallet::weight(10_000)]
		pub fn clear_invalid_file(origin: OriginFor<T>, miner_id: u64, file_id: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let bounded_string: BoundedString<T> = file_id.clone().try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
			<InvalidFile<T>>::try_mutate(&miner_id, |o| -> DispatchResult {
				o.retain(|x| *x != bounded_string);
				Ok(())
			})?;
			Self::deposit_event(Event::<T>::ClearInvalidFile{acc: sender, file_id: file_id});
			Ok(())
		}
		
	}

	impl<T: Config> Pallet<T> {
		fn upload_file(
			acc: &AccountOf<T>, 
			address: &Vec<u8>,
			filename:&Vec<u8>,
			fileid: &Vec<u8>,
			filehash: &Vec<u8>,
			public: bool,
			backups: u8,
			filesize: u64,
			downloadfee: BalanceOf<T>
		) -> DispatchResult {
			ensure!(<UserHoldSpaceDetails<T>>::contains_key(&acc), Error::<T>::NotPurchasedSpace);
			let bounded_fileid = Self::vec_to_bound::<u8>(fileid.to_vec())?;
			ensure!(!<File<T>>::contains_key(bounded_fileid.clone()), Error::<T>::FileExistent);		
			let mut invoice: Vec<u8> = Vec::new();
			for i in fileid {
				invoice.push(*i);
			}
			for i in address {
				invoice.push(*i);
			}
	
			let bounded_invoice = Self::vec_to_bound::<u8>(invoice)?;
			<Invoice<T>>::insert(
				bounded_invoice,
				0 
			);
			<File<T>>::insert(
				bounded_fileid.clone(),
				FileInfo::<T> {
					file_name: Self::vec_to_bound::<u8>(filename.to_vec())?,
					file_size: filesize,
					file_hash: Self::vec_to_bound::<u8>(filehash.to_vec())?,
					public: public,
					user_addr: acc.clone(),
					file_state: Self::vec_to_bound::<u8>("normal".as_bytes().to_vec())?,
					backups: backups,
					downloadfee: downloadfee,
					file_dupl: BoundedVec::default(),
				}
			);
			UserFileSize::<T>::try_mutate(acc.clone(), |s| -> DispatchResult{
				*s = (*s).checked_add(filesize as u128).ok_or(Error::<T>::Overflow)?;
				Ok(())
			})?;
			Self::update_user_space(acc.clone(), 1, (filesize as u128) * (backups as u128))?;
			Self::add_user_hold_file(acc.clone(), fileid.clone());
			Ok(())
		}
	
		//operation: 1 upload files, 2 delete file
		fn update_user_space(acc: AccountOf<T>, operation: u8, size: u128) -> DispatchResult{
			match operation {
				1 => {
					<UserHoldSpaceDetails<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
						let s = s_opt.as_mut().unwrap();
						if size > s.remaining_space {
							Err(Error::<T>::InsufficientStorage)?;
						}
						if false == Self::check_lease_expired(acc.clone()) {
							Self::deposit_event(Event::<T>::LeaseExpired{acc: acc.clone(), size: 0});
							Err(Error::<T>::LeaseExpired)?;
						}
						s.remaining_space = s.remaining_space.checked_sub(size).ok_or(Error::<T>::Overflow)?;
						s.used_space = s.used_space.checked_add(size).ok_or(Error::<T>::Overflow)?;
						Ok(())
					})?
				}
				2 => {
					<UserHoldSpaceDetails<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
						let s = s_opt.as_mut().unwrap();
						s.remaining_space = s.remaining_space.checked_add(size).ok_or(Error::<T>::Overflow)?;
						s.used_space = s.used_space.checked_sub(size).ok_or(Error::<T>::Overflow)?;
						Ok(())
					})?
				}
				_ => Err(Error::<T>::WrongOperation)?			
			}
			Ok(())
		}
	
		fn user_buy_space_update(acc: AccountOf<T>, size: u128) -> DispatchResult{
			
			if <UserHoldSpaceDetails<T>>::contains_key(&acc) {
				<UserHoldSpaceDetails<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
					let s = s_opt.as_mut().unwrap();
					s.purchased_space = s.purchased_space.checked_add(size).ok_or(Error::<T>::Overflow)?;
					s.remaining_space = s.remaining_space.checked_add(size).ok_or(Error::<T>::Overflow)?;
					Ok(())
				})?;
			} else {
				let value = StorageSpace {
					purchased_space: size,
					used_space: 0,
					remaining_space: size,
				};
				<UserHoldSpaceDetails<T>>::insert(&acc, value);
			}
			Ok(())
		}
	
		fn add_user_hold_file(acc: AccountOf<T>, fileid: Vec<u8>){
			let bounded_fileid = Self::vec_to_bound::<u8>(fileid).unwrap();
			<UserHoldFileList<T>>::mutate(&acc, |s|{
				s.try_push(bounded_fileid).expect("Length exceeded");
			});
		}
	
		// fn remove_user_hold_file(acc: &AccountOf<T>, fileid: Vec<u8>) {
		// 	<UserHoldFileList<T>>::mutate(&acc, |s|{
		// 		s.drain_filter(|v| *v == fileid);
		// 	});
		// }
		//Available space divided by 1024 is the unit price
		fn get_price() -> u128 {
			//Get the available space on the current chain
			let space = pallet_sminer::Pallet::<T>::get_space();
			//If it is not 0, the logic is executed normally
			if space != 0 {
				//Calculation rules
				//The price is based on 1024 / available space on the current chain
				//Multiply by the base value 1 tcess * 1_000 (1_000_000_000_000 * 1_000)
				let price: u128 = 1024 * 1_000_000_000_000 * 1000 / space ;
				return price;
			}
			//If it is 0, an extra large price will be returned
			1_000_000_000_000_000_000
		}
	
		fn check_lease_expired_forfileid(fileid: Vec<u8>) -> bool {
			let bounded_fileid = Self::vec_to_bound::<u8>(fileid).unwrap();
			let file = <File<T>>::get(&bounded_fileid).unwrap();
			Self::check_lease_expired(file.user_addr)
		}
		//ture is Not expired;  false is expired
		fn check_lease_expired(acc: AccountOf<T>) -> bool {
			let details = <UserHoldSpaceDetails<T>>::get(&acc).unwrap();
			if details.used_space + details.remaining_space > details.purchased_space {
				false
			} else {
				true
			}
		}
	
		pub fn check_file_exist(fileid: Vec<u8>) -> bool {
			let bounded_fileid = Self::vec_to_bound::<u8>(fileid).unwrap();
			if <File<T>>::contains_key(bounded_fileid) {
				true
			} else {
				false
			}
		}
	
		fn vec_to_bound<P>(param: Vec<P>) -> Result<BoundedVec<P, T::StringLimit>, DispatchError> {
			let result: BoundedVec<P, T::StringLimit> = param.try_into().expect("too long");
			Ok(result)
		}
	
		//offchain helper
		//Signature chaining method
		fn offchain_signed_tx(block_number: T::BlockNumber, value: Vec<u8>) -> Result<(), Error<T>> {
			// We retrieve a signer and check if it is valid.
			//   Since this pallet only has one key in the keystore. We use `any_account()1 to
			//   retrieve it. If there are multiple keys and we want to pinpoint it, `with_filter()` can be chained,
			let signer = Signer::<T, T::AuthorityId>::any_account();
			//Data not currently used
			let _number: u64 = block_number.try_into().unwrap_or(0);
			//Send signed transaction
			//call update_price transaction
			let result = signer.send_signed_transaction(|_acct| 
				Call::update_price{price: value.clone()}
			);
	
			// Display error if the signed tx fails.
			if let Some((acc, res)) = result {
				if res.is_err() {
					log::error!("failure: offchain_signed_tx: tx sent: {:?}", acc.id);
					Err(<Error<T>>::OffchainSignedTxError)?;
				}
	
				return Ok(());
			}

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
			.add_header("User-Agent","PostmanRuntime/7.28.4")
				.deadline(timeout) // Setting the timeout time
				.send() // Sending the request out by the host
				.map_err(|_| <Error<T>>::HttpFetchingError)?;
			
			log::info!("wating response");
			//Waiting for response
			let response = pending
				.wait()
				.map_err(|_| <Error<T>>::HttpFetchingError)?;
			//Determine whether the response is wrong
			if response.code != 200 {
				log::error!("Unexpected http request status code: {}", response.code);
				Err(<Error<T>>::HttpFetchingError)?;
			}
					
			Ok(response.body().collect::<Vec<u8>>())
		}

		pub fn get_random_challenge_data() -> Result<Vec<(u64, Vec<u8>, Vec<u32>, u64, u8, u32)>, DispatchError> {
			let filler_list = Self::get_random_filler()?;
			let mut data: Vec<(u64, Vec<u8>, Vec<u32>, u64, u8, u32)> = Vec::new();
			for v in filler_list {
				let length = v.block_num;
				let number_list = Self::get_random_numberlist(length)?;
				let miner_id = v.miner_id.clone();
				let filler_id = v.filler_id.clone().to_vec();
				let file_size = v.filler_size.clone();
				let segment_size = v.segment_size.clone();
				let mut block_list:Vec<u32> = Vec::new();
				for i in number_list.iter() {
					let filler_block = v.filler_block[*i as usize].clone();
					block_list.push(filler_block.block_index);
				}
				data.push((miner_id, filler_id, block_list, file_size, 1, segment_size));
			}

			let file_list = Self::get_random_file()?;
			for (size, v) in file_list {
				let length = v.block_num;
				let number_list = Self::get_random_numberlist(length)?;
				let miner_id = v.miner_id.clone();
				let file_id = v.dupl_id.clone().to_vec();
				let file_size = size.clone();
				let segment_size = v.segment_size.clone();
				let mut block_list:Vec<u32> = Vec::new();
				for i in number_list.iter() {
					let file_block = v.block_info[*i as usize].clone();
					block_list.push(file_block.block_index);
				}
				data.push((miner_id, file_id, block_list, file_size, 2, segment_size));
			}

			Ok(data)
		}
		//Get random file block list
		fn get_random_filler() -> Result<Vec<FillerInfo<T>>, DispatchError> {
			let length = Self::get_fillermap_length()?;
			let number_list = Self::get_random_numberlist(length)?;
			let mut filler_list: Vec<FillerInfo<T>> = Vec::new();
			for i in number_list.iter() {
				let mut counter: u32 = 0;
				for (_, _, value) in <FillerMap<T>>::iter() {	
					if counter == *i {
						filler_list.push(value);
						break;
					}
					counter = counter + 1;
				}
			}
			Ok(filler_list)
		}

		fn get_random_file() -> Result<Vec<(u64, FileDuplicateInfo<T>)>, DispatchError> {
			let length = Self::get_file_map_length()?;
			let number_list = Self::get_random_numberlist(length)?;
			let mut file_list: Vec<(u64, FileDuplicateInfo<T>)> = Vec::new();
			for i in number_list.iter() {
				let mut counter: u32 = 0;
				for (_, value) in <File<T>>::iter() {	
					if value.file_state.to_vec() == "active".as_bytes().to_vec() {
						if counter == *i {
							file_list.push((value.file_size, value.file_dupl[0].clone()));
							file_list.push((value.file_size, value.file_dupl[1].clone()));
							file_list.push((value.file_size, value.file_dupl[2].clone()));
							break;
						}
						counter = counter + 1;
					}
				}
			}
			Ok(file_list)
		}

		fn get_random_numberlist(length: u32) -> Result<Vec<u32>, DispatchError> {		
			let seed: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();
			let num: u32 = length
				.checked_mul(1000).ok_or(Error::<T>::Overflow)?
				.checked_div(46).ok_or(Error::<T>::Overflow)?
				.checked_add(1).ok_or(Error::<T>::Overflow)?;			
			let mut number_list: Vec<u32> = Vec::new();
			loop {
				if number_list.len() >= num as usize {
					number_list.sort();
					number_list.dedup();
					if number_list.len() >= num as usize {
						break;
					}
				}
				let random = Self::generate_random_number(seed) % length;
				number_list.push(random);
			}
			Ok(number_list)
		}

		//Get storagemap filler length 
		fn get_fillermap_length() -> Result<u32, DispatchError> {
			let mut length: u32 = 0;
			for _ in <FillerMap<T>>::iter() {
				length = length.checked_add(1).ok_or(Error::<T>::Overflow)?;
			}
			Ok(length)
		}

		//Get Storage FillerMap Length
		fn get_file_map_length() -> Result<u32, DispatchError> {
			let mut length: u32 = 0;
			for (_, v) in <File<T>>::iter() {
				if v.file_state.to_vec() == "active".as_bytes().to_vec() {
					length = length.checked_add(1).ok_or(Error::<T>::Overflow)?;
				}
			}
			Ok(length)
		}

		//Get random number
		pub fn generate_random_number(seed: u32) -> u32 {
			let (random_seed, _) = T::MyRandomness::random(&(T::FilbakPalletId::get(), seed).encode());
			let random_number = <u32>::decode(&mut random_seed.as_ref())
				.expect("secure hashes should always be bigger than u32; qed");
			random_number
		}

		//Specific implementation method of deleting filler file
		pub fn delete_filler(miner_id: u64, filler_id: Vec<u8>) -> DispatchResult {
			let filler_boud: BoundedString<T> = filler_id.try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
			if !<FillerMap<T>>::contains_key(miner_id, filler_boud.clone()) {
				Err(Error::<T>::FileNonExistent)?;
			}
			<FillerMap<T>>::remove(miner_id, filler_boud.clone());

			Ok(())
		}

		//Delete the next backup under the file
		pub fn delete_file_dupl(dupl_id: Vec<u8>) -> DispatchResult {
			let length = dupl_id.len() - 4;
			let file_id = dupl_id[0..length].to_vec();
			let file_id_bounded: BoundedString<T> = file_id.try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
			if !<File<T>>::contains_key(&file_id_bounded) {
				Err(Error::<T>::FileNonExistent)?;
			} 

			<File<T>>::try_mutate(&file_id_bounded, |opt| -> DispatchResult {
				let o = opt.as_mut().unwrap();
				o.file_dupl.retain(|x| x.dupl_id.to_vec() != dupl_id);
				Ok(())
			})?;

			Ok(())
		}
		
		//Add the list of files to be recovered and notify the scheduler to recover
		pub fn add_recovery_file(dupl_id: Vec<u8>) -> DispatchResult {
			let acc = Self::get_current_scheduler();
			let length = dupl_id.len() - 4;
			let file_id = dupl_id[0..length].to_vec();
			let file_id_bounded: BoundedString<T> = file_id.try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
			if !<File<T>>::contains_key(&file_id_bounded) {
				Err(Error::<T>::FileNonExistent)?;
			}
			<File<T>>::try_mutate(&file_id_bounded, |opt| -> DispatchResult {
				let o = opt.as_mut().unwrap();
				o.file_state = "repairing".as_bytes().to_vec().try_into().map_err(|_e| Error::<T>::BoundedVecError)?;
				Ok(())
			})?;
			<FileRecovery<T>>::try_mutate(&acc, |o| -> DispatchResult {
				o.try_push(dupl_id.try_into().map_err(|_e| Error::<T>::BoundedVecError)?).map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;

			Ok(())
		}

		//Add invalid file list, notify miner to delete
		pub fn add_invalid_file(miner_id: u64, file_id: Vec<u8>) -> DispatchResult {
			<InvalidFile<T>>::try_mutate(&miner_id, |o| -> DispatchResult {
				o.try_push(file_id.try_into().map_err(|_e| Error::<T>::BoundedVecError)?).map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;

			Ok(())
		}

		//Obtain the consensus of the current block
		fn get_current_scheduler() -> AccountOf<T> {
			//Current block information
			let digest = <frame_system::Pallet<T>>::digest();
				let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
				let acc = T::FindAuthor::find_author(pre_runtime_digests).map(|a| {
					a
				});
				acc.unwrap()
		}

		//The file replacement method will increase the miner's space at the same time
		fn replace_file(file_dupl: Vec<FileDuplicateInfo<T>>, file_size: u64) -> DispatchResult {
			for v in file_dupl {
				//add space
				T::MinerControl::add_space(v.miner_id, file_size.into())?;
				let (power, space) = T::MinerControl::get_power_and_space(v.miner_id)?;
				//Judge whether the current miner's remaining is enough to store files
				if power - space < file_size.into() {
					Err(Error::<T>::MinerPowerInsufficient)?;
				}
				//How many files to replace, round up
				let replace_num = file_size / 8 + 1;
				let mut counter = 0;
				for (filler_id, _) in <FillerMap<T>>::iter_prefix(v.miner_id) {
					if counter == replace_num {
						break;
					}
					//Clear information on the chain
					Self::delete_filler(v.miner_id, filler_id.to_vec())?;
					//Notify the miner to clear the corresponding data segment
					Self::add_invalid_file(v.miner_id, filler_id.to_vec())?;
					counter = counter + 1;
				}
			}
			
			Ok(())
		}
	}
}

pub trait RandomFileList {
	//Get random challenge data
	fn get_random_challenge_data() -> Result<Vec<(u64, Vec<u8>, Vec<u32>, u64, u8, u32)>, DispatchError>;
	//Delete filler file
	fn delete_filler(miner: u64, filler_id: Vec<u8>) -> DispatchResult;
	//Delete file backup
	fn delete_file_dupl(dupl_id: Vec<u8>) -> DispatchResult;
	//The function executed when the challenge fails, allowing the miner to delete invalid files
	fn add_recovery_file(file_id: Vec<u8>) -> DispatchResult;
	//The function executed when the challenge fails to let the consensus schedule recover the file
	fn add_invalid_file(miner_id: u64, file_id: Vec<u8>) -> DispatchResult;
}

impl<T: Config> RandomFileList for Pallet<T> {
	fn get_random_challenge_data() -> Result<Vec<(u64, Vec<u8>, Vec<u32>, u64, u8, u32)>, DispatchError> {
		let result = Pallet::<T>::get_random_challenge_data()?;
		Ok(result)
	}

	fn delete_filler(miner_id: u64, filler_id: Vec<u8>) -> DispatchResult {
		Pallet::<T>::delete_filler(miner_id, filler_id)?;
		Ok(())
	}

	fn delete_file_dupl(dupl_id: Vec<u8>) -> DispatchResult {
		Pallet::<T>::delete_file_dupl(dupl_id)?;
		Ok(())
	}

	fn add_recovery_file(file_id: Vec<u8>) -> DispatchResult {
		Pallet::<T>::add_recovery_file(file_id)?;
		Ok(())
	}

	fn add_invalid_file(miner_id: u64, file_id: Vec<u8>) -> DispatchResult {
		Pallet::<T>::add_invalid_file(miner_id, file_id)?;
		Ok(())
	}
}




impl<T: Config> BlockNumberProvider for Pallet<T> {
	type BlockNumber = T::BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		<frame_system::Pallet<T>>::block_number()
	}
}


