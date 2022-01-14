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

use frame_support::traits::{Currency, ReservableCurrency, ExistenceRequirement::AllowDeath};
pub use pallet::*;
mod benchmarking;
pub mod weights;
use sp_std::convert::TryInto;

use scale_info::TypeInfo;
use sp_runtime::{
	RuntimeDebug,
	traits::{AccountIdConversion}
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
	filename: Vec<u8>,
	owner: AccountOf<T>,
	filehash: Vec<u8>,
	backups: u8,
	filesize: u128,
	downloadfee: BalanceOf<T>,
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
	peer_id: u64,
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
	pub trait Config: frame_system::Config + pallet_sminer::Config {
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
		FileUpload(AccountOf<T>),
		//file updated.
		FileUpdate(AccountOf<T>),
		//file bought.
		BuyFile(AccountOf<T>, BalanceOf<T>, Vec<u8>),
		//file purchased before.
		Purchased(AccountOf<T>, Vec<u8>),

		InsertFileSlice(Vec<u8>),		

		BuySpace(AccountOf<T>, u128, BalanceOf<T>),
	}
	#[pallet::error]
	pub enum Error<T> {
		//file doesn't exist.
		FileNonExistent,
		//overflow.
		Overflow,

		InsufficientStorage,

		WrongOperation,

		NotPurchasedSpace,

		LeaseExpired,
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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::upload())]
		pub fn upload(
			origin: OriginFor<T>,
			address: Vec<u8>,
			filename:Vec<u8>,
			fileid: Vec<u8>,
			filehash: Vec<u8>,
			backups: u8,
			filesize: u128,
			downloadfee:BalanceOf<T>
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			// let acc = T::FilbakPalletId::get().into_account();
			// T::Currency::transfer(&sender, &acc, uploadfee, AllowDeath)?;

			ensure!(<UserHoldSpaceDetails<T>>::contains_key(&sender), Error::<T>::NotPurchasedSpace);
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
					filename,
					owner: sender.clone(),
					filehash,
					backups,
					filesize,
					downloadfee: downloadfee.clone(),
				}
			);
			UserFileSize::<T>::try_mutate(sender.clone(), |s| -> DispatchResult{
				*s = (*s).checked_add(filesize).ok_or(Error::<T>::Overflow)?;
				Ok(())
			})?;
			Self::add_user_hold_file(sender.clone(), fileid.clone());
			Self::deposit_event(Event::<T>::FileUpload(sender.clone()));
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
		// #[pallet::weight(T::WeightInfo::update())]
		// pub fn update(origin: OriginFor<T>, fileid: Vec<u8>, ispublic: u8, similarityhash: Vec<u8>) -> DispatchResult{
		// 	let sender = ensure_signed(origin)?;
		// 	ensure!((<File<T>>::contains_key(fileid.clone())), Error::<T>::FileNonExistent);

		// 	<File<T>>::mutate(fileid, |s_opt| {
		// 		let s = s_opt.as_mut().unwrap();
		// 		s.ispublic = ispublic;
		// 		s.similarityhash = similarityhash;
		// 	});
		// 	Self::deposit_event(Event::<T>::FileUpdate(sender.clone()));

		// 	Ok(())
		// }

		/// Update info of uploaded file.
		/// 
		/// The dispatch origin of this call must be _Signed_.
		/// 
		/// Parameters:
		/// - `fileid`: id of file, each file will have different number, even for the same file.
		/// - `address`: address of file.
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
				Self::deposit_event(Event::<T>::Purchased(sender.clone(), fileid.clone()));
			} else {
				let zh = TryInto::<u128>::try_into(group_id.downloadfee).ok().unwrap();
				//let umoney = zh * 8 / 10;
				let umoney = zh.checked_mul(8).ok_or(Error::<T>::Overflow)?
					.checked_div(10).ok_or(Error::<T>::Overflow)?;
				let money: Option<BalanceOf<T>> = umoney.try_into().ok();
				let acc = T::FilbakPalletId::get().into_account();
				<T as pallet::Config>::Currency::transfer(&sender, &group_id.owner, money.unwrap(), AllowDeath)?;
				<T as pallet::Config>::Currency::transfer(&sender, &acc, group_id.downloadfee - money.unwrap(), AllowDeath)?;
				<Invoice<T>>::insert(
					invoice,
					0
				);
				Self::deposit_event(Event::<T>::BuyFile(sender.clone(), group_id.downloadfee.clone(), fileid.clone()));
			}
			
			Ok(())
		}

		#[pallet::weight(2_000_000)]
		pub fn insert_file_slice_location(origin: OriginFor<T>, fileid: Vec<u8>, slice: Vec<FileSlice>) -> DispatchResult {
			let _ = ensure_signed(origin)?;

			<FileSliceLocation<T>>::insert(&fileid, slice);
			Self::deposit_event(Event::<T>::InsertFileSlice(fileid));
			Ok(())
		}

		//**********************************************************************************************************************************************
		//************************************************************Storage space lease***************************************************************
		//**********************************************************************************************************************************************
		#[pallet::weight(2_000_000)]
		pub fn buy_space(origin: OriginFor<T>, count: u128) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let acc = T::FilbakPalletId::get().into_account();
			// const k: usize = count.clone() as usize;
			let unit_price = Self::get_price();
			let space = 512 * count;
			let price = unit_price * (space) / 3;

			let money: Option<BalanceOf<T>> = price.try_into().ok();
			<T as pallet::Config>::Currency::transfer(&sender, &acc, money.unwrap(), AllowDeath)?;
			let now = <frame_system::Pallet<T>>::block_number();
			let deadline: BlockNumberOf<T> = (864000 as u32).into();
			let mut list: Vec<SpaceInfo<T>> = vec![SpaceInfo::<T>{size: 512, deadline: now + deadline}; count as usize];

			<UserSpaceList<T>>::mutate(&sender, |s|{
				s.append(&mut list);
			});
			Self::user_buy_space_update(sender.clone(), space)?;

			

			Self::deposit_event(Event::<T>::BuySpace(sender.clone(), space, money.unwrap()));
			Ok(())
		}
		
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
		let price = space / 1024 * 1000;
		price
	}

	fn check_lease_expired_forfileid(fileid: Vec<u8>) -> bool {
		let file = <File<T>>::get(&fileid).unwrap();
		Self::check_lease_expired(file.owner)
	}
	//ture is Not expired;  false is expired
	fn check_lease_expired(acc: AccountOf<T>) -> bool {
		let details = <UserHoldSpaceDetails<T>>::get(&acc).unwrap();
		if details.used_space < details.purchased_space {
			true
		} else {
			false
		}
	}
}


