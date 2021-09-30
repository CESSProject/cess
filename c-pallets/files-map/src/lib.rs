//! # Files Map Module
//!
//! Management to stored file routing.
//!
//! ### Terminology
//!
//! * **Peer ID:** The storage miner account address.
//! * **Storage Address:** The IP address used by the storage miner to actually store data.
//! 
//! ### Interface
//!
//! ### Dispatchable Functions
//!
//! * `add` - Add stored file route.
//! * `update` - Update stored file route.
//! * `del` - Delete stored file route.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;

//use codec::{Encode, Decode};
use frame_support::{
	decl_module, decl_event, decl_storage, ensure, decl_error,
};
use frame_system::ensure_signed;


pub trait Config: frame_system::Config {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
}


decl_storage! {
	trait Store for Module<T: Config> as FilesMap {
		/// The hashmap for route info of stored files.
        FilesRoute get(fn files_route): map hasher(twox_64_concat) u128 => Option<(T::AccountId, u32)>;
	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Config>::AccountId {
		/// An files map was updated.
		Updated(u128, AccountId),

        /// An files map was added.
        Added(u128, AccountId),

		/// An files map was deleted.
        Deleted(u128),
	}
);

decl_error! {
	/// Error for the sminer module.
	pub enum Error for Module<T: Config> {
		/// An account doesn't registered.
		NotExisted,

		/// An account already existed.
		AlreadyExisted,

		/// An account already existed.
		SameInfo,
	}
}

decl_module! {
	/// Files Map module declaration.
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

        /// Add stored file route.
        /// 
        /// The dispatch origin of this call must be _Signed_.
        /// 
        /// Parameters:
        /// - `file_hash`: The hash for a stored file.
        /// - `peer_id`: The storage miner account address.
		/// - `storage_addr`: The IP address used by the storage miner to actually store data.
		#[weight = 50_000_000]
		fn add(origin, file_hash: u128, peer_id: T::AccountId, storage_addr: u32) {
			let _ = ensure_signed(origin)?;

            ensure!(!FilesRoute::<T>::contains_key(file_hash), Error::<T>::AlreadyExisted);

            FilesRoute::<T>::insert(file_hash, (peer_id.clone(), storage_addr));

			Self::deposit_event(RawEvent::Added(file_hash, peer_id.clone()));
		}

        /// Update stored file route.
        /// 
        /// The dispatch origin of this call must be _Signed_.
        /// 
        /// Parameters:
        /// - `file_hash`: The hash for a stored file.
        /// - `peer_id`: The storage miner account address.
		/// - `storage_addr`: The IP address used by the storage miner to actually store data.
		#[weight = 50_000_000]
		fn update(origin, file_hash: u128, peer_id: T::AccountId, storage_addr: u32) {
			let _ = ensure_signed(origin)?;

            ensure!(FilesRoute::<T>::contains_key(file_hash), Error::<T>::NotExisted);

			ensure!(FilesRoute::<T>::get(file_hash) != Some((peer_id.clone(), storage_addr)), Error::<T>::SameInfo);

			FilesRoute::<T>::mutate(file_hash, |s| {
				*s = Some((peer_id.clone(), storage_addr));
			});

			Self::deposit_event(RawEvent::Updated(file_hash, peer_id.clone()));
		}

		/// Delete stored file route.
        /// 
        /// The dispatch origin of this call must be _Signed_.
        /// 
        /// Parameters:
        /// - `file_hash`: The hash for a stored file.
        /// - `peer_id`: The storage miner account address.
		/// - `storage_addr`: The IP address used by the storage miner to actually store data.
		#[weight = 50_000_000]
		fn del(origin, file_hash: u128) {
			let _ = ensure_signed(origin)?;

            ensure!(FilesRoute::<T>::contains_key(file_hash), Error::<T>::NotExisted);

			FilesRoute::<T>::remove(file_hash);

			Self::deposit_event(RawEvent::Deleted(file_hash));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate as pallet_files_map;

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
			FilesMap: pallet_files_map::{Module, Call, Storage, Event<T>},
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

	impl Config for Test {
		type Event = Event;
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
	fn add_should_work() {
		new_test_ext().execute_with(|| {
			let file_hash1: u128 = 0x4a5c456435;
			let file_hash2: u128 = 0xb249556391;
			let storage_addr: u32 = 127001;
			assert_ok!(FilesMap::add(Origin::signed(2), file_hash1, 1, storage_addr));
			assert_noop!(FilesMap::add(Origin::signed(2), file_hash1, 1, storage_addr), Error::<Test>::AlreadyExisted);
			assert_noop!(FilesMap::add(Origin::signed(1), file_hash1, 1, storage_addr), Error::<Test>::AlreadyExisted);
			assert_noop!(FilesMap::add(Origin::signed(2), file_hash1, 2, storage_addr), Error::<Test>::AlreadyExisted);
			assert_noop!(FilesMap::add(Origin::signed(1), file_hash1, 2, storage_addr), Error::<Test>::AlreadyExisted);
			assert_ok!(FilesMap::add(Origin::signed(2), file_hash2, 1, storage_addr));
		});
	}

	#[test]
	fn update_should_work() {
		new_test_ext().execute_with(|| {
			let file_hash: u128 = 0x5bd456909;
			let old_storage_addr: u32 = 127001;
			let new_storage_addr: u32 = 943746;
			assert_noop!(FilesMap::update(Origin::signed(2), file_hash, 1, new_storage_addr), Error::<Test>::NotExisted);
			assert_ok!(FilesMap::add(Origin::signed(2), file_hash, 1, old_storage_addr));
			assert_ok!(FilesMap::update(Origin::signed(2), file_hash, 1, new_storage_addr));
			assert_noop!(FilesMap::update(Origin::signed(2), file_hash, 1, new_storage_addr), Error::<Test>::SameInfo);
		});
	}

	#[test]
	fn del_should_work() {
		new_test_ext().execute_with(|| {
			let file_hash: u128 = 0xdf234354;
			let storage_addr: u32 = 127001;
			assert_noop!(FilesMap::del(Origin::signed(2), file_hash), Error::<Test>::NotExisted);
			assert_ok!(FilesMap::add(Origin::signed(2), file_hash, 1, storage_addr));
			assert_ok!(FilesMap::del(Origin::signed(2), file_hash));
			assert_noop!(FilesMap::del(Origin::signed(2), file_hash), Error::<Test>::NotExisted);
		});
	}

}

