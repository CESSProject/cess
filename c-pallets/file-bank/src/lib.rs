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

use frame_support::traits::{Currency, ReservableCurrency, ExistenceRequirement::AllowDeath};
pub use pallet::*;
mod benchmarking;
pub mod weights;
use sp_std::convert::TryInto;

use scale_info::TypeInfo;
use sp_runtime::{
	RuntimeDebug,
	traits::AccountIdConversion,
};
use sp_std::prelude::*;
use codec::{Encode, Decode};
use frame_support::{dispatch::DispatchResult, PalletId};
pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;


/// The custom struct for storing file info.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct FileInfo<T: pallet::Config> {
	filename: Vec<u8>,
	owner: AccountOf<T>,
	filehash: Vec<u8>,
	similarityhash: Vec<u8>,
	//two status: 0(private) 1(public)
	ispublic: u8,
	backups: u8,
	creator: Vec<u8>,
	filesize: u128,
	keywords: Vec<u8>,
	email: Vec<u8>,
	uploadfee: BalanceOf<T>,
	downloadfee: BalanceOf<T>,
	//expiration time
	deadline: u128,
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
	pub trait Config: frame_system::Config {
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
	}
	#[pallet::error]
	pub enum Error<T> {
		//file doesn't exist.
		FileNonExistent,
		//overflow.
		Overflow,
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
		#[pallet::weight(T::WeightInfo::upload())]
		pub fn upload(
			origin: OriginFor<T>,
			filename:Vec<u8>,
			address:Vec<u8>,
			fileid: Vec<u8>,
			filehash: Vec<u8>,
			similarityhash: Vec<u8>,
			ispublic: u8,
			backups: u8,
			creator: Vec<u8>,
			filesize: u128,
			keywords: Vec<u8>,
			email: Vec<u8>,
			uploadfee:BalanceOf<T>,
			downloadfee:BalanceOf<T>,
			deadline: u128
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let acc = T::FilbakPalletId::get().into_account();
			T::Currency::transfer(&sender, &acc, uploadfee, AllowDeath)?;
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
					similarityhash,
					ispublic,
					backups,
					creator,
					filesize,
					keywords,
					email,
					uploadfee: uploadfee.clone(),
					downloadfee: downloadfee.clone(),
					deadline: deadline,
				}
			);
			UserFileSize::<T>::try_mutate(sender.clone(), |s| -> DispatchResult{
				*s = (*s).checked_add(filesize).ok_or(Error::<T>::Overflow)?;
				Ok(())
			})?;
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
		#[pallet::weight(T::WeightInfo::update())]
		pub fn update(origin: OriginFor<T>, fileid: Vec<u8>, ispublic: u8, similarityhash: Vec<u8>) -> DispatchResult{
			let sender = ensure_signed(origin)?;
			ensure!((<File<T>>::contains_key(fileid.clone())), Error::<T>::FileNonExistent);

			<File<T>>::mutate(fileid, |s_opt| {
				let s = s_opt.as_mut().unwrap();
				s.ispublic = ispublic;
				s.similarityhash = similarityhash;
			});
			Self::deposit_event(Event::<T>::FileUpdate(sender.clone()));

			Ok(())
		}

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
				T::Currency::transfer(&sender, &group_id.owner, money.unwrap(), AllowDeath)?;
				T::Currency::transfer(&sender, &acc, group_id.downloadfee - money.unwrap(), AllowDeath)?;
				<Invoice<T>>::insert(
					invoice,
					0
				);
				Self::deposit_event(Event::<T>::BuyFile(sender.clone(), group_id.downloadfee.clone(), fileid.clone()));
			}
			
			Ok(())
		}

	}
}
