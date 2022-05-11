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

use frame_support::traits::{Currency, ReservableCurrency, ExistenceRequirement::AllowDeath, Randomness};
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
use sp_core::{crypto::KeyTypeId, U256};

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

		DeleteFile{acc: AccountOf<T>, fileid: Vec<u8>},

		UserAuth{user: AccountOf<T>, collaterals: BalanceOf<T>, random: u32},
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
	pub(super) type FillerMap<T: Config> = StorageMap<_, Twox64Concat, BoundedString<T>, FillerInfo<T> >;


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
			let signer = Signer::<T, T::AuthorityId>::all_accounts();

			let number: u128 = block_number.saturated_into();
			let one_day: u128 = <T as Config>::OneDay::get().saturated_into();
			if number % one_day == 0 {
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
			filler_list: Vec<FillerInfo<T>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			if T::Scheduler::contains_scheduler(sender) {
				Err(Error::<T>::ScheduleNonExistent)?;
			}

			for i in filler_list.iter() {
				<FillerMap<T>>::insert(
					i.filler_id.clone(),
					i
				);

				T::MinerControl::add_temp_power(i.miner_id.clone(), 8)?;
			}

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
		#[pallet::weight(2_000_000)]
		pub fn buy_space(origin: OriginFor<T>, space_count: u128, lease_count: u128, max_price: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let acc = T::FilbakPalletId::get().into_account();
			let unit_price = TryInto::<u128>::try_into(<UnitPrice<T>>::get().unwrap()).ok().unwrap();
			if unit_price > max_price * 1000000000000 && 0 != max_price {
				Err(Error::<T>::ExceedExpectations)?;
			}
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
			Self::user_buy_space_update(sender.clone(), space * 1024)?;

			Self::deposit_event(Event::<T>::BuySpace{acc: sender.clone(), size: space, fee: money});
			Ok(())
		}

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
		// #[

		//Test method：Clear the storage space owned by the user
		#[pallet::weight(2_000_000)]
		pub fn initi_acc(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			<UserSpaceList<T>>::remove(&sender);
			<UserHoldSpaceDetails<T>>::remove(&sender);
			Ok(())
		}

		#[pallet::weight(2_000_000)]
		pub fn update_price(
			origin: OriginFor<T>,
			price: Vec<u8>
		) -> DispatchResult {
			let _sender = ensure_signed(origin)?;
			//Vec<u8> -> str
			let str_price = str::from_utf8(&price).unwrap();

			//str -> u128
			let mut price_u128: u128 = str_price
				.parse()
				.map_err(|_e| Error::<T>::ConversionError)?;
			
			//One third of the price
			price_u128 = price_u128.checked_mul(3).ok_or(Error::<T>::Overflow)?;
				
			//Which pricing is cheaper
			let our_price = Self::get_price();
			if our_price < price_u128 {
				price_u128 = our_price;
			}
			//u128 -> balance
			let price_balance: BalanceOf<T> = price_u128.try_into().map_err(|_e| Error::<T>::ConversionError)?;

			<UnitPrice<T>>::put(price_balance);

			Ok(())
		}


		//#[pallet::weight(2_000_000)]
		// pub fn clean_file(origin: OriginFor<T>) -> DispatchResult {
		// 	let _ = ensure_signed(origin)?;
		// 	for (key, _) in <File<T>>::iter() {
		// 		<File<T>>::remove(&key);
		// 	}
		// 	Ok(())
		// }
	
		// #[pallet::weight(2_000_000)]
		// pub fn user_auth(origin: OriginFor<T>, user: AccountOf<T>, collaterals: BalanceOf<T>, random: u32) -> DispatchResult {
		// 	let _who = ensure_signed(origin)?;
		// 	<T as pallet::Config>::Currency::reserve(&user, collaterals.clone())?;
		// 	// if who == b"5CkMk8pNuvZsZpYKi1HpTEajVLuM1EzRDUnj9e9Rbuxmit7M".to_owner() {

		// 	// }
		// 	if !<UserInfoMap<T>>::contains_key(&user) {
		// 		UserInfoMap::<T>::insert(&user,
		// 			UserInfo::<T>{
		// 				collaterals: collaterals.clone()
		// 		});
		// 	} else {
		// 		UserInfoMap::<T>::try_mutate(&user, |opt| -> DispatchResult {
		// 			let o = opt.as_mut().unwrap();
		// 			o.collaterals = o.collaterals.checked_add(&collaterals).ok_or(Error::<T>::Overflow)?;
		// 			Ok(())
		// 		})?;
		// 	}

		// 	Self::deposit_event(Event::<T>::UserAuth{user: user, collaterals: collaterals, random: random});
		// 	Ok(())
		// }
		
		// #[pallet::weight(2_000_000)]
		// pub fn http_upload(
		// 	origin: OriginFor<T>, 
		// 	user: AccountOf<T>, 
		// 	address: Vec<u8>,
		// 	filename:Vec<u8>,
		// 	fileid: Vec<u8>,
		// 	filehash: Vec<u8>,
		// 	public: bool,
		// 	backups: u8,
		// 	filesize: u64,
		// 	downloadfee:BalanceOf<T>
		// ) -> DispatchResult {
		// 	let sender = ensure_signed(origin)?;

		// 	if !<UserInfoMap<T>>::contains_key(&user) {
		// 		Err(Error::<T>::NotUser)?;
		// 	}
		// 	let deposit: BalanceOf<T> = 780_000_000_000u128.try_into().map_err(|_e| Error::<T>::ConversionError)?;
		
		// 	Self::upload_file(&user, &address, &filename, &fileid, &filehash, public, backups, filesize, downloadfee)?;
		// 	<T as pallet::Config>::Currency::unreserve(&user, deposit);
		// 	<T as pallet::Config>::Currency::transfer(&user, &sender, deposit, AllowDeath)?;
			
		// 	Self::deposit_event(Event::<T>::FileUpload{acc: user.clone()});
		// 	Ok(())
		// }

		// #[pallet::weight(2_000_000)]
		// pub fn http_delete(
		// 	origin: OriginFor<T>,
		// 	user: AccountOf<T>,
		// 	fileid: Vec<u8>
		// ) -> DispatchResult {
		// 	let sender = ensure_signed(origin)?;
		// 	if !<UserInfoMap<T>>::contains_key(&user) {
		// 		Err(Error::<T>::NotUser)?;
		// 	}
		// 	let bounded_fileid = Self::vec_to_bound::<u8>(fileid.clone())?;
		// 	ensure!((<File<T>>::contains_key(bounded_fileid.clone())), Error::<T>::FileNonExistent);
			
		// 	let file = <File<T>>::get(&bounded_fileid).unwrap();
		// 	if file.user_addr != user.clone() {
		// 		Err(Error::<T>::NotOwner)?;
		// 	}

		// 	Self::update_user_space(user.clone(), 2, file.file_size.into())?;
		// 	<File<T>>::remove(&bounded_fileid);

		// 	let deposit: BalanceOf<T> = 780_000_000_000u128.try_into().map_err(|_e| Error::<T>::ConversionError)?;
		// 	<T as pallet::Config>::Currency::unreserve(&user, deposit);
		// 	<T as pallet::Config>::Currency::transfer(&user, &sender, deposit, AllowDeath)?;
		// 	Self::deposit_event(Event::<T>::DeleteFile{acc: user, fileid: fileid});
		// 	Ok(())
		// }
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
			let space = pallet_sminer::Pallet::<T>::get_space();
			if space != 0 {
				let price: u128 = 1024 * 1_000_000_000_000 * 1000 / space ;
				return price;
			}
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
		fn offchain_signed_tx(block_number: T::BlockNumber, value: Vec<u8>) -> Result<(), Error<T>> {
			// We retrieve a signer and check if it is valid.
			//   Since this pallet only has one key in the keystore. We use `any_account()1 to
			//   retrieve it. If there are multiple keys and we want to pinpoint it, `with_filter()` can be chained,
			let signer = Signer::<T, T::AuthorityId>::any_account();
	
	
			let _number: u64 = block_number.try_into().unwrap_or(0);
	
			let result = signer.send_signed_transaction(|_acct| 
				Call::update_price{price: value.clone()}
			);
	
			// Display error if the signed tx fails.
			if let Some((acc, res)) = result {
				if res.is_err() {
					log::error!("failure: offchain_signed_tx: tx sent: {:?}", acc.id);
					return Err(<Error<T>>::OffchainSignedTxError);
				}
	
				return Ok(());
			}
	
	
			log::error!("No local account available");
			Err(<Error<T>>::NoLocalAcctForSigning)
		}
	
		pub fn offchain_fetch_price(block_number: T::BlockNumber) -> Result<(), Error<T>> {
	
		
			let resp_bytes = Self::offchain_http_req().map_err(|e| {
				log::error!("fetch_from_remote error: {:?}", e);
				<Error<T>>::HttpFetchingError
			})?;
			let _ = Self::offchain_signed_tx(block_number, resp_bytes)?;
			
	
			Ok(())
		}
	
		fn offchain_http_req() -> Result<Vec<u8>, Error<T>> {
			log::info!("send request to {}", HTTP_REQUEST_STR);
			
			let request = rt_offchain::http::Request::get(HTTP_REQUEST_STR);
			
			let timeout = sp_io::offchain::timestamp()
			.add(rt_offchain::Duration::from_millis(FETCH_TIMEOUT_PERIOD));
	
			log::info!("send request");
			let pending = request
			.add_header("User-Agent","PostmanRuntime/7.28.4")
				.deadline(timeout) // Setting the timeout time
				.send() // Sending the request out by the host
				.map_err(|_| <Error<T>>::HttpFetchingError)?;
			
			log::info!("wating response");
			let response = pending
				.wait()
				.map_err(|_| <Error<T>>::HttpFetchingError)?;
	
			if response.code != 200 {
				log::error!("Unexpected http request status code: {}", response.code);
				return Err(<Error<T>>::HttpFetchingError);
			}
					
			Ok(response.body().collect::<Vec<u8>>())
		}

		pub fn get_random_challenge_data() -> Result<Vec<(u64, Vec<u8>, Vec<u32>)>, DispatchError> {
			let file_list = Self::get_random_file()?;
			let mut data: Vec<(u64, Vec<u8>, Vec<u32>)> = Vec::new();
			for v in file_list {
				let length = v.block_num;
				let number_list = Self::get_random_numberlist(length)?;
				let miner_id = v.miner_id.clone();
				let filler_id = v.filler_id.clone().to_vec();
				let mut block_list:Vec<u32> = Vec::new();
				for i in number_list.iter() {
					let filler_block = v.filler_block[*i as usize].clone();
					block_list.push(filler_block.block_index);
				}
				data.push((miner_id, filler_id, block_list));
			} 

			Ok(data)
		}
		//Get random file block list
		fn get_random_file() -> Result<Vec<FillerInfo<T>>, DispatchError> {
			
			let length = Self::get_fillermap_length()?;
			let number_list = Self::get_random_numberlist(length)?;
			let mut filler_list: Vec<FillerInfo<T>> = Vec::new();
			for i in number_list.iter() {
				let mut counter: u32 = 0;
				for (_, value) in <FillerMap<T>>::iter() {
					counter = counter + 1;
					if counter == *i {
						filler_list.push(value);
						break;
					}
				}
			}
			Ok(filler_list)
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

		//Get random number
		pub fn generate_random_number(seed: u32) -> u32 {
			let (random_seed, _) = T::MyRandomness::random(&(T::FilbakPalletId::get(), seed).encode());
			let random_number = <u32>::decode(&mut random_seed.as_ref())
				.expect("secure hashes should always be bigger than u32; qed");
			random_number
		}
		//Get Storage FillerMap Length
		
	}
}

pub trait RandomFileList {
	fn get_random_challenge_data() -> Result<Vec<(u64, Vec<u8>, Vec<u32>)>, DispatchError>;
}

impl<T: Config> RandomFileList for Pallet<T> {
	fn get_random_challenge_data() -> Result<Vec<(u64, Vec<u8>, Vec<u32>)>, DispatchError> {
		let result = Pallet::<T>::get_random_challenge_data()?;
		Ok(result)
	}
}




impl<T: Config> BlockNumberProvider for Pallet<T> {
	type BlockNumber = T::BlockNumber;

	fn current_block_number() -> Self::BlockNumber {
		<frame_system::Pallet<T>>::block_number()
	}
}


