//! # Files Bank Module
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
//! * `cleared` - Clear info of uploaded file.
//! * `changed` - Update the file expiration time.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;

use codec::{Encode, Decode};
use frame_support::{
	decl_module, decl_event, decl_storage, ensure, decl_error,
	traits::{Currency, ReservableCurrency, Get, ExistenceRequirement::AllowDeath},
};
use frame_system::ensure_signed;
use sp_runtime::{ RuntimeDebug, ModuleId, traits::{
	 AccountIdConversion
  }};

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;


pub trait Config: frame_system::Config {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	/// The currency trait.
	type Currency: ReservableCurrency<Self::AccountId>;
	
	type ModuleId: Get<ModuleId>;
}


/// The custom struct for storing info of storage fileInfo.
#[derive(PartialEq, Eq, Default, Encode, Decode, Clone, RuntimeDebug)]
pub struct FileInfo<AccountId,Balance,BlockNumber>{
	is_public:bool,
	backups:u8,
	user:AccountId,
	file_size:u128,
	start_t:BlockNumber,
	deadline:BlockNumber,
	upload_fee:Balance,
	download_fee:Balance,
}

decl_storage! {
	trait Store for Module<T: Config> as Sminer {
		/// The hashMap structure declaration of FileInfo
		FileInfoMap: map hasher(twox_64_concat) u128 => Option<FileInfo<T::AccountId, BalanceOf<T>,T::BlockNumber>>;
	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Config>::AccountId {
		/// An account Uploads files
		FileInfoUpload(AccountId),
		/// An account Clears files
		FileInfoCleared(AccountId),
		/// An account changes the file
		FileInfoChanged(AccountId),
	}
);

decl_error! {
	/// Error for the nicks module.
	pub enum Error for Module<T: Config> {

		NotExisted,	

		LackOfPermissions,
	}
}

decl_module! {
	/// Nicks module declaration.
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		const ModuleId: ModuleId = T::ModuleId::get();
		
		/// Upload info of stored file.
        /// 
        /// The dispatch origin of this call must be _Signed_.
        /// 
        /// Parameters:
        /// - `file`: Upload hash of file.
        /// - `is_public`: Public or private.
		/// - `backups`: Number of duplicate.
        /// - `file_size`: The file size.
		/// - `upload_fee`: The upload cost.
        /// - `download_fee`: The download fees.
		/// - `deadline`: Expiration time.
		#[weight = 50_000_000]
		pub fn upload(origin, file: u128, is_public: bool, backups: u8, file_size: u128, upload_fee: BalanceOf<T>, download_fee: BalanceOf<T>, deadline: T::BlockNumber) {
			let sender = ensure_signed(origin)?;

			let now = <frame_system::Module<T>>::block_number();

			let acc = T::ModuleId::get().into_account();

		    T::Currency::transfer(&sender, &acc, upload_fee, AllowDeath)?;

			<FileInfoMap<T>>::insert(
				file, 
				FileInfo::<T::AccountId, BalanceOf<T>, T::BlockNumber> {
					is_public,
					backups,
					user: sender.clone(),
					file_size,
					start_t: now,
					deadline: now + deadline,
					upload_fee,
					download_fee,
				}
			);

			Self::deposit_event(RawEvent::FileInfoUpload(sender.clone()));
		}

		/// Clear info of uploaded file.
        /// 
        /// The dispatch origin of this call must be _Signed_.
        /// 
        /// Parameters:
        /// - `file`: Upload file name.
		#[weight = 50_000_000]
		pub fn cleared(origin,file:u128) {
			let sender = ensure_signed(origin)?;

			ensure!(FileInfoMap::<T>::contains_key(file), Error::<T>::NotExisted);

			let group_id = <FileInfoMap<T>>::get(file).unwrap();
			
			ensure!(group_id.user == sender.clone(), Error::<T>::LackOfPermissions);

			<FileInfoMap<T>>::remove(file);

			Self::deposit_event(RawEvent::FileInfoCleared(sender.clone()));
		}

		/// Update the file expiration time.
        /// 
        /// The dispatch origin of this call must be _Signed_.
        /// 
        /// Parameters:
        /// - `file`: Upload file name.
		/// - `deadline`: Expiration time.
		#[weight = 50_000_000]
		pub fn changed(origin, file: u128, deadline: T::BlockNumber) {
			let sender = ensure_signed(origin)?;

			ensure!(FileInfoMap::<T>::contains_key(file), Error::<T>::NotExisted);

			let group_id = <FileInfoMap<T>>::get(file).unwrap();

			ensure!(group_id.user == sender.clone(), Error::<T>::LackOfPermissions);

			ensure!(deadline != group_id.deadline, "Expiration time is invalid");

			let file_info1 = FileInfo::<T::AccountId, BalanceOf<T>, T::BlockNumber> {
				is_public: group_id.is_public,
				backups:group_id.backups,
				user: group_id.user,
				file_size: group_id.file_size,
				start_t: group_id.start_t,
				deadline: deadline,
				upload_fee: group_id.upload_fee,
				download_fee: group_id.download_fee,
			};

			<FileInfoMap<T>>::remove(file);

			<FileInfoMap<T>>::insert(
				file, 
				file_info1
			);

			Self::deposit_event(RawEvent::FileInfoChanged(sender.clone()));
		}
		
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate as pallet_files_bank;

	use frame_support::{assert_ok, assert_noop, parameter_types};
	use sp_core::H256;
	use sp_runtime::{
		testing::Header, traits::{BlakeTwo256, IdentityLookup},
	};

	type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
	type Block = frame_system::mocking::MockBlock<Test>;

	frame_support::construct_runtime!(
		pub enum Test where
			Block = Block,
			NodeBlock = Block,
			UncheckedExtrinsic = UncheckedExtrinsic,
		{
			System: frame_system::{Module, Call, Config, Storage, Event<T>},
			Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
			FilesBank: pallet_files_bank::{Module, Call, Storage, Event<T>},
		}
	);

	parameter_types! {
		pub const BlockHashCount: u64 = 250;
		pub BlockWeights: frame_system::limits::BlockWeights =
			frame_system::limits::BlockWeights::simple_max(1024);
	}
	impl frame_system::Config for Test {
		type BaseCallFilter = ();
		type BlockWeights = ();
		type BlockLength = ();
		type DbWeight = ();
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Call = Call;
		type Hashing = BlakeTwo256;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = Event;
		type BlockHashCount = BlockHashCount;
		type Version = ();
		type PalletInfo = PalletInfo;
		type AccountData = pallet_balances::AccountData<u64>;
		type OnNewAccount = ();
		type OnKilledAccount = ();
		type SystemWeightInfo = ();
		type SS58Prefix = ();
	}
	parameter_types! {
		pub const ExistentialDeposit: u64 = 1;
	}
	impl pallet_balances::Config for Test {
		type MaxLocks = ();
		type Balance = u64;
		type Event = Event;
		type DustRemoval = ();
		type ExistentialDeposit = ExistentialDeposit;
		type AccountStore = System;
		type WeightInfo = ();
	}

	parameter_types! {
		pub const FBModuleId: ModuleId = ModuleId(*b"filebank");
	}
	impl Config for Test {
		type Event = Event;
		type Currency = Balances;
		type ModuleId = FBModuleId;
	}

	fn new_test_ext() -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
		pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 10),
				(2, 10),
			],
		}.assimilate_storage(&mut t).unwrap();
		t.into()
	}

	#[test]
	fn upload_should_work() {
		new_test_ext().execute_with(|| {
			let file: u128 = 0x4a5c456435;
			let is_public: bool = true;
			let backups: u8 = 3;
			let file_size: u128 = 393725;
			assert_ok!(FilesBank::upload(Origin::signed(2), file, is_public, backups, file_size, 5, 7, 100));
			assert_eq!(Balances::free_balance(&2), 5);
		});
	}

	#[test]
	fn cleared_should_work() {
		new_test_ext().execute_with(|| {
			let file: u128 = 0xdf234354;
			let is_public: bool = true;
			let backups: u8 = 3;
			let file_size: u128 = 393725;
			assert_noop!(FilesBank::cleared(Origin::signed(2), file), Error::<Test>::NotExisted);
			assert_ok!(FilesBank::upload(Origin::signed(2), file, is_public, backups, file_size, 5, 7, 100));
			assert_noop!(FilesBank::cleared(Origin::signed(1), file), Error::<Test>::LackOfPermissions);
			assert_ok!(FilesBank::cleared(Origin::signed(2), file));
		});
	}

	#[test]
	fn changed_should_work() {
		new_test_ext().execute_with(|| {
			let file: u128 = 0xaf2097394;
			let is_public: bool = true;
			let backups: u8 = 3;
			let file_size: u128 = 393725;
			assert_noop!(FilesBank::changed(Origin::signed(2), file, 200), Error::<Test>::NotExisted);
			assert_ok!(FilesBank::upload(Origin::signed(2), file, is_public, backups, file_size, 5, 7, 100));
			assert_noop!(FilesBank::changed(Origin::signed(1), file, 200), Error::<Test>::LackOfPermissions);
			assert_ok!(FilesBank::changed(Origin::signed(2), file, 200));
		});
	}

}