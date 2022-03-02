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

use frame_support::traits::{Currency, ReservableCurrency, ExistenceRequirement::AllowDeath};
pub use pallet::*;
mod benchmarking;
pub mod weights;
use sp_std::convert::TryInto;
use sp_std::fmt::Debug;

use scale_info::TypeInfo;
use sp_runtime::{
	RuntimeDebug,
	traits::{AccountIdConversion,SaturatedConversion}
};
use sp_std::prelude::*;
use codec::{Encode, Decode};
use frame_support::{dispatch::DispatchResult, PalletId};
pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

/// The custom struct for storing file info.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct FileInfo<T: pallet::Config> {
	file_name: Vec<u8>,
	file_size: u128,
	file_hash: Vec<u8>,
	//Public or not
	public: bool,
	user_addr: AccountOf<T>,
	//normal or repairing
	file_state: Vec<u8>,
	//Number of backups
	backups: u8,
	downloadfee: BalanceOf<T>,
	//Backup information
	file_dupl: Vec<FileDuplicateInfo<T>>,
}

//backups info struct
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct FileDuplicateInfo<T: pallet::Config> {
	dupl_id: Vec<u8>,
	rand_key: Vec<u8>,
	slice_num: u16,
	file_slice: Vec<FileSliceInfo<T>>,
}

//slice info
//Slice consists of shard
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct FileSliceInfo<T: pallet::Config> {
	slice_id: Vec<u8>,
	slice_size: u16,
	slice_hash: Vec<u8>,
	file_shard: FileShardInfo<T>,
}

//shard info
//Slice consists of shard
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct FileShardInfo<T: pallet::Config> {
	data_shard_num: u8,
	redun_shard_num: u8,
	shard_hash: Vec<Vec<u8>>,
	shard_addr: Vec<AccountOf<T>>,
}




#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
pub struct StorageSpace {
	purchased_space: u128,
	used_space: u128,
	remaining_space: u128,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct SpaceInfo<T: pallet::Config> {
	size: u128,
	deadline: BlockNumberOf<T>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
pub struct FileSlice {
	peer_id: Vec<Vec<u8>>,
	fill_zero: u32,
	slice_id: u64,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		ensure,
		pallet_prelude::*,
		traits::Get,
	};
	//pub use crate::weights::WeightInfo;
	use frame_system::{ensure_signed, pallet_prelude::*};

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_sminer::Config + sp_std::fmt::Debug {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		type WeightInfo: WeightInfo;
		/// pallet address.
		#[pallet::constant]
		type FilbakPalletId: Get<PalletId>;
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
	}
	#[pallet::storage]
	#[pallet::getter(fn file)]
	pub(super) type File<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, FileInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn invoice)]
	pub(super) type Invoice<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, u8, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn seg_info)]
	pub(super) type UserFileSize<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, u128, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn file_slice_location)]
	pub(super) type FileSliceLocation<T: Config> = StorageMap<_, Twox64Concat, Vec<u8>, Vec<FileSlice>>;


	#[pallet::storage]
	#[pallet::getter(fn user_hold_file_list)]
	pub(super) type UserHoldFileList<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Vec<Vec<u8>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn user_hold_storage_space)]
	pub(super) type UserHoldSpaceDetails<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, StorageSpace>;

	#[pallet::storage]
	#[pallet::getter(fn user_spance_details)]
	pub(super) type UserSpaceList<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Vec<SpaceInfo<T>>, ValueQuery>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

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
						if now >= s.deadline {
							list.remove(k);
							<UserHoldSpaceDetails<T>>::mutate(&key, |s_opt|{
								let v = s_opt.as_mut().unwrap();
								v.purchased_space = v.purchased_space - 512 * 1024;
								if v.remaining_space > 512 * 1024 {
									v.remaining_space = v.remaining_space - 512 * 1024;
								}
							});
							let _ = pallet_sminer::Pallet::<T>::sub_purchased_space(512);
							Self::deposit_event(Event::<T>::LeaseExpired{acc: key.clone(), size: 512});
							k-= 1;
						} else if s.deadline < now && now >= s.deadline - block_oneday {
							count += 1;
						} 
						k+= 1;
					}
					<UserSpaceList<T>>::insert(&key, list);
					Self::deposit_event(Event::<T>::LeaseExpireIn24Hours{acc: key.clone(), size: 512 * (count as u128)});
				}
			}
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		/// Upload info of stored file.
		/// 
		/// The dispatch origin of this call must be _Signed_.
		/// 
		/// Parameters:
		/// - `filename`: name of file.
		/// - `address`: address of file.
		/// - `fileid`: id of file, each file will have different number, even for the same file.
		/// - `filehash`: hash of file.
		/// - `similarityhash`: hash of file, used for checking similarity.
		/// - `is_public`: public or private.
		/// - `backups`: number of duplicate.
		/// - `creator`: creator of file.
		/// - `file_size`: the file size.
		/// - `keywords`: keywords of file.
		/// - `email`: owner's email.
		/// - `uploadfee`: the upload fees.
		/// - `downloadfee`: the download fees.
		/// - `deadline`: expiration time.
		/// 
		// pub struct FileInfo<T: pallet::Config> {
		// 	file_name: Vec<u8>,
		// 	file_size: u128,
		// 	file_hash: Vec<u8>,
		// 	public: bool,
		// 	user_addr: AccountOf<T>,
		// 	file_state: Vec<u8>,
		// 	backups: u8,
		// 	downloadfee: BalanceOf<T>,
		// 	file_dupl: Vec<FileDuplicateInfo<T>>,
		// }
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upload())]
		pub fn upload(
			origin: OriginFor<T>,
			address: Vec<u8>,
			filename:Vec<u8>,
			fileid: Vec<u8>,
			filehash: Vec<u8>,
			public: bool,
			file_state: Vec<u8>,
			backups: u8,
			filesize: u128,
			downloadfee:BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			// let acc = T::FilbakPalletId::get().into_account();
			// T::Currency::transfer(&sender, &acc, uploadfee, AllowDeath)?;

			ensure!(<UserHoldSpaceDetails<T>>::contains_key(&sender), Error::<T>::NotPurchasedSpace);
			ensure!(!<File<T>>::contains_key(&fileid), Error::<T>::FileExistent);
			Self::update_user_space(sender.clone(), 1, filesize * (backups as u128))?;

			let mut invoice: Vec<u8> = Vec::new();
			for i in &fileid {
				invoice.push(*i);
			}
			for i in &address {
				invoice.push(*i);
			}

			<Invoice<T>>::insert(
				invoice,
				0 
			);
			<File<T>>::insert(
				fileid.clone(),
				FileInfo::<T> {
					file_name: filename,
					file_size: filesize,
					file_hash: filehash,
					public: public,
					user_addr: sender.clone(),
					file_state: file_state,
					backups: backups,
					downloadfee: downloadfee,
					file_dupl: Vec::new(),
				}
			);
			UserFileSize::<T>::try_mutate(sender.clone(), |s| -> DispatchResult{
				*s = (*s).checked_add(filesize).ok_or(Error::<T>::Overflow)?;
				Ok(())
			})?;
			Self::add_user_hold_file(sender.clone(), fileid.clone());
			Self::deposit_event(Event::<T>::FileUpload{acc: sender.clone()});
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
		pub fn update_dupl(origin: OriginFor<T>, fileid: Vec<u8>, file_dupl: Vec<FileDuplicateInfo<T>>) -> DispatchResult{
			let sender = ensure_signed(origin)?;
			ensure!((<File<T>>::contains_key(fileid.clone())), Error::<T>::FileNonExistent);
			//Judge whether it is a consensus node

			<File<T>>::mutate(fileid.clone(), |s_opt| {
				let s = s_opt.as_mut().unwrap();
				s.file_dupl = file_dupl;
			});
			Self::deposit_event(Event::<T>::FileUpdate{acc: sender.clone(), fileid: fileid});

			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn update_file_state(origin: OriginFor<T>, fileid: Vec<u8>, state: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!((<File<T>>::contains_key(fileid.clone())), Error::<T>::FileNonExistent);
			//Judge whether it is a consensus node

			<File<T>>::try_mutate(fileid.clone(), |s_opt| -> DispatchResult{
				let s = s_opt.as_mut().unwrap();
				//To prevent multiple scheduling
				if s.file_state == "repairing".as_bytes().to_vec() && state == "repairing".as_bytes().to_vec() {
					Err(Error::<T>::AlreadyRepair)?;
				}

				s.file_state = state;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::FileChangeState{acc: sender.clone(), fileid: fileid});
			Ok(())
		}

		#[pallet::weight(2_000_000)]
		pub fn delete_file(origin: OriginFor<T>, fileid: Vec<u8>) -> DispatchResult{
			let sender = ensure_signed(origin)?;
			ensure!((<File<T>>::contains_key(fileid.clone())), Error::<T>::FileNonExistent);
			let file = <File<T>>::get(&fileid).unwrap();
			if file.user_addr != sender {
				Err(Error::<T>::NotOwner)?;
			}

			Self::update_user_space(sender, 2, file.file_size)?;
			<File::<T>>::remove(fileid);

			Ok(())
		}

		#[pallet::weight(2_000_000)]
		pub fn buyfile(origin: OriginFor<T>, fileid: Vec<u8>, address: Vec<u8>) -> DispatchResult{
			let sender = ensure_signed(origin)?;

			ensure!((<File<T>>::contains_key(fileid.clone())), Error::<T>::FileNonExistent);
			ensure!(Self::check_lease_expired_forfileid(fileid.clone()), Error::<T>::LeaseExpired);
			let group_id = <File<T>>::get(fileid.clone()).unwrap();

			let mut invoice: Vec<u8> = Vec::new();
			for i in &fileid {
				invoice.push(*i);
			}
			for i in &address {
				invoice.push(*i);
			}
				
			if <Invoice<T>>::contains_key(fileid.clone()) {
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
					invoice,
					0
				);
				Self::deposit_event(Event::<T>::BuyFile{acc: sender.clone(), money: group_id.downloadfee.clone(), fileid: fileid.clone()});
			}
			
			Ok(())
		}

		#[pallet::weight(2_000_000)]
		pub fn insert_file_slice_location(origin: OriginFor<T>, fileid: Vec<u8>, slice: Vec<FileSlice>) -> DispatchResult {
			let _ = ensure_signed(origin)?;

			<FileSliceLocation<T>>::insert(&fileid, slice);
			Self::deposit_event(Event::<T>::InsertFileSlice{fileid: fileid});
			Ok(())
		}

		//**********************************************************************************************************************************************
		//************************************************************Storage space lease***************************************************************
		//**********************************************************************************************************************************************
		#[pallet::weight(2_000_000)]
		pub fn buy_space(origin: OriginFor<T>, count: u128, max_price: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let acc = T::FilbakPalletId::get().into_account();
			let unit_price = Self::get_price();
			if unit_price > max_price * 1000000000000 && 0 != max_price {
				Err(Error::<T>::ExceedExpectations)?;
			}
			let space = 512 * count;
			//Because there are three backups, it is charged at one-third of the price
			let price = unit_price * (space) / 3;
			//Increase the space purchased by users 
			//and judge whether there is still space available for purchase
			pallet_sminer::Pallet::<T>::add_purchased_space(space)?;

			let money: Option<BalanceOf<T>> = price.try_into().ok();
			<T as pallet::Config>::Currency::transfer(&sender, &acc, money.unwrap(), AllowDeath)?;
			let now = <frame_system::Pallet<T>>::block_number();
			let deadline: BlockNumberOf<T> = (864000 as u32).into();
			let mut list: Vec<SpaceInfo<T>> = vec![SpaceInfo::<T>{size: 512, deadline: now + deadline}; count as usize];

			<UserSpaceList<T>>::mutate(&sender, |s|{
				s.append(&mut list);
			});
			Self::user_buy_space_update(sender.clone(), space * 1024)?;

			Self::deposit_event(Event::<T>::BuySpace{acc: sender.clone(), size: space, fee: money.unwrap()});
			Ok(())
		}

		#[pallet::weight(2_000_000)]
		pub fn initi_acc(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			<UserSpaceList<T>>::remove(&sender);
			<UserHoldSpaceDetails<T>>::remove(&sender);
			Ok(())
		}

		// #[pallet::weight(2_000_000)]
		// pub fn clean_file(origin: OriginFor<T>) -> DispatchResult {
		// 	let _ = ensure_signed(origin)?;
		// 	for (key, _) in <File<T>>::iter() {
		// 		<File<T>>::remove(&key);
		// 	}
		// 	Ok(())
		// }
		
	}
}

impl<T: Config> Pallet<T> {
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

	fn add_user_hold_file(acc: AccountOf<T>, fileid: Vec<u8>) {
		<UserHoldFileList<T>>::mutate(&acc, |s|{
			s.push(fileid);
		});
	}
	//Available space divided by 1024 is the unit price
	fn get_price() -> u128 {
		let space = pallet_sminer::Pallet::<T>::get_space();
		let price: u128 = 1024 / space * 1000000000000 * 1000;
		price
	}

	fn check_lease_expired_forfileid(fileid: Vec<u8>) -> bool {
		let file = <File<T>>::get(&fileid).unwrap();
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
		if <File<T>>::contains_key(fileid) {
			true
		} else {
			false
		}
	}
}


