#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

// type AccountOf<T> = <T as frame_system::Config>::AccountId;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod types;
use types::*;

use frame_support::{dispatch::*, pallet_prelude::*};
// use sp_std::prelude::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;
	use frame_support::storage::bounded_vec::BoundedVec;

	
	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type StringLimit: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	pub type StringList<T> = BoundedVec<BoundedVec<u8, <T as Config>::StringLimit>, <T as Config>::StringLimit>;
	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage

	#[pallet::storage]
	#[pallet::getter(fn file_storage)]
	pub type FileStorage<T: Config> = StorageDoubleMap<_, Blake2_128Concat, T::AccountId, Blake2_128Concat, BoundedVec<u8, T::StringLimit>, FileInfo<BoundedVec<u8, T::StringLimit>, StringList<T>> >;

	#[pallet::storage]
	pub type TestStorage<T: Config> = StorageMap<_, Blake2_128Concat, [u8; 32], TestInfo<T::AccountId>>;
	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),

		Store{acc: T::AccountId, fileid: Vec<u8>},

		NotOwner{acc: T::AccountId, fileid: Vec<u8>},

		IsOwner{acc: T::AccountId, fileid: Vec<u8>},

		Replace{acc: T::AccountId, old_fileid: Vec<u8>, new_fileid: Vec<u8>},

		Delete{acc: T::AccountId, fileid: Vec<u8>},

		Edit{acc: T::AccountId, fileid: Vec<u8>, new_filename: Vec<u8>, new_keywords: Vec<Vec<u8>>},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		//Error that file ID already exists
		FileExist,
		//File does not exist error
		FileNonExist,
		//Error not file owner
		NotOwner,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000_000)]
		pub fn store(origin: OriginFor<T>, pfileid:Vec<u8>, pfilename: Vec<u8>, filesize: u128, pkeywords: Vec<Vec<u8>>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::insert_file(&who, pfileid.clone(), pfilename, filesize, pkeywords)?;

			Self::deposit_event(Event::<T>::Store{acc: who, fileid: pfileid});
			Ok(())
		}

		//Determine whether it is the file owner
		#[pallet::weight(1_000_000)]
		pub fn retrieve(origin: OriginFor<T>, pfileid: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let fileid:BoundedVec<u8, T::StringLimit> = pfileid.clone().try_into().expect("fileid too long");

			if <FileStorage<T>>::contains_key(&who, &fileid) {
				Self::deposit_event(Event::<T>::IsOwner{acc: who, fileid: pfileid});
			} else {
				Err(Error::<T>::NotOwner)?;
			}

			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn replace(origin: OriginFor<T>, old_fileid: Vec<u8>, new_fileid: Vec<u8>, pfilename: Vec<u8>, filesize: u128, pkeywords: Vec<Vec<u8>>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let o_fileid: BoundedVec<u8, T::StringLimit> = old_fileid.clone().try_into().expect("fileid too long");
			//Determine whether it is the file owner
			if !<FileStorage<T>>::contains_key(&who, &o_fileid) {
				Err(Error::<T>::FileNonExist)?;
			}

			Self::insert_file(&who, new_fileid.clone(), pfilename, filesize, pkeywords)?;
			Self::deposit_event(Event::<T>::Replace{acc: who, old_fileid: old_fileid, new_fileid: new_fileid});
			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn delete(origin: OriginFor<T>, pfileid: Vec<u8>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let fileid: BoundedVec<u8, T::StringLimit> = pfileid.clone().try_into().expect("fileid too long");
			//Determine whether it is the file owner
			if !<FileStorage<T>>::contains_key(&who, &fileid) {
				Err(Error::<T>::FileNonExist)?;
			}

			Self::deposit_event(Event::<T>::Delete{acc: who, fileid: pfileid});
			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn edit(origin: OriginFor<T>, pfileid: Vec<u8>, new_filename: Vec<u8>, new_keywords: Vec<Vec<u8>>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let fileid: BoundedVec<u8, T::StringLimit> = pfileid.clone().try_into().expect("fileid too long");
			if !<FileStorage<T>>::contains_key(&who, &fileid) {
				Err(Error::<T>::FileNonExist)?;
			}

			let filename: BoundedVec<u8, T::StringLimit> = new_filename.clone().try_into().expect("new filename too long");
			let keywords = Self::vec_to_bounde(new_keywords.clone())?;

			<FileStorage<T>>::try_mutate(&who, &fileid, |o_opt| -> DispatchResult {
				let o = o_opt.as_mut().unwrap();
				o.filename = filename;
				o.keywords = keywords;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::Edit{acc: who, fileid: pfileid, new_filename: new_filename, new_keywords: new_keywords});
			Ok(())
		}

		

		
	}
}

impl <T: Config> Pallet<T> {
	fn vec_to_bounde(param: Vec<Vec<u8>>) -> Result<StringList<T>, DispatchError> {
		let mut result: StringList<T> = Vec::new().try_into().expect("...");

		for v in param {
			let string: BoundedVec<u8, T::StringLimit> = v.try_into().expect("keywords too long");
			result.try_push(string).expect("keywords too long");
		}

		Ok(result)
	}
	//Storage file
	fn insert_file(who: &T::AccountId, pfileid: Vec<u8>, pfilename: Vec<u8>, filesize: u128, pkeywords: Vec<Vec<u8>>) -> DispatchResult {
		let fileid: BoundedVec<u8, T::StringLimit> = pfileid.try_into().expect("fileid too long");
		//Determine that the file you want to change does not currently exist
		if FileStorage::<T>::contains_key(who, &fileid) {
			Err(Error::<T>::FileExist)?;
		}

		let filename: BoundedVec<u8, T::StringLimit> = pfilename.try_into().expect("filename too long");
		let keywords = Self::vec_to_bounde(pkeywords)?;
			
		let file = FileInfo::<BoundedVec<u8, T::StringLimit>, StringList<T>>::new(filename, filesize, keywords);
		FileStorage::<T>::insert(
			who,
			&fileid,
			file,
		);

		Ok(())
	}

	pub fn extension_test_insert(who: &T::AccountId, fileid: &[u8; 32], filename: &[u8; 32], filesize: u32) {
		let info = TestInfo::<T::AccountId> {
			owner: who.clone(),
			filename: *filename,
			filesize: filesize,
		};

		TestStorage::<T>::insert(&fileid, info);
	}
}