//! # Sminer Module
//!
//! Contain operations related storage miners.
//!
//! ### Terminology
//!
//! * **Collateral:** The Staking amount when registering storage miner.
//! * **Earnings:** Store the storage miner's earnings during mining.
//! * **Locked:** Store the locked amount of the storage miner during mining.
//! 
//! ### Interface
//!
//! ### Dispatchable Functions
//!
//! * `register` - Staking and register for storage miner.
//! * `redeem` - Redeem and exit for storage miner.
//! * `claim` - Claim the rewards from storage miner's earnings.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;

use codec::{Encode, Decode};
use frame_support::{
	decl_module, decl_event, decl_storage, ensure, decl_error,
	traits::{Currency, ReservableCurrency, Get, ExistenceRequirement::AllowDeath},
};
use frame_system::ensure_signed;
use sp_runtime::{
	RuntimeDebug, ModuleId,
	traits::{AccountIdConversion},
};	

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;


pub trait Config: frame_system::Config {
	/// The overarching event type.
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

	/// The currency trait.
	type Currency: ReservableCurrency<Self::AccountId>;

	type ModuleId: Get<ModuleId>;
}

/// The custom struct for storing info of storage miners.
#[derive(PartialEq, Eq, Default, Encode, Decode, Clone, RuntimeDebug)]
pub struct Mr<AccountId, Balance> {
    signer: AccountId,
    ip: u32,
    collateral: Balance,
    earnings: Balance,
    locked: Balance,
}

decl_storage! {
	trait Store for Module<T: Config> as Sminer {
		/// The hashmap for info of storage miners.
        MinerItems get(fn miner_items): map hasher(twox_64_concat) T::AccountId => Option<Mr<T::AccountId, BalanceOf<T>>>;

		/// The hashmap for checking registered or not.
        WalletMiners get(fn wallet_miners): map hasher(twox_64_concat) T::AccountId => Option<i8>;
	}
}

decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Config>::AccountId, Balance = BalanceOf<T> {
		/// A new account was set.
		Registered(AccountId, Balance),

		/// An account was redeemed.
		Redeemed(AccountId, Balance),

		/// An account was claimed.
		Claimed(AccountId, Balance),
	}
);

decl_error! {
	/// Error for the sminer module.
	pub enum Error for Module<T: Config> {
		/// An account doesn't registered.
		UnregisteredAccountId,

		/// An account has locked balances.
		LockedNotEmpty,

		/// An account already registered.
		AlreadyRegistered,

		/// An account's earnings is empty.
		EarningsIsEmpty,
	}
}

decl_module! {
	/// Sminer module declaration.
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		type Error = Error<T>;

		fn deposit_event() = default;

		const ModuleId: ModuleId = T::ModuleId::get();

		/// Staking and register for storage miner.
        /// 
        /// The dispatch origin of this call must be _Signed_.
        /// 
        /// Parameters:
        /// - `ip`: The registered IP of storage miner.
        /// - `staking_val`: The number of staking.
        #[weight = 50_000_000]
		fn register(origin, ip: u32, staking_val: BalanceOf<T>) {
			let sender = ensure_signed(origin)?;

			ensure!(!(<WalletMiners<T>>::contains_key(&sender)), Error::<T>::AlreadyRegistered);

            T::Currency::reserve(&sender, staking_val.clone())?;

			let value = BalanceOf::<T>::from(0 as u32);

            <MinerItems<T>>::insert(
				&sender, 
				Mr::<T::AccountId, BalanceOf<T>> {
					signer: sender.clone(),
					ip,
					collateral: staking_val.clone(),
					earnings: value.clone(),
					locked: value.clone(),
				}
			);

            <WalletMiners<T>>::insert(&sender, 1);

			Self::deposit_event(RawEvent::Registered(sender.clone(), staking_val.clone()));
		}

		/// Redeem and exit for storage miner.
        /// 
        /// The dispatch origin of this call must be _Signed_.
		#[weight = 50_000_000]
		fn redeem(origin) {
			let sender = ensure_signed(origin)?;

			ensure!(<WalletMiners<T>>::contains_key(&sender), Error::<T>::UnregisteredAccountId);

			ensure!(MinerItems::<T>::get(&sender).unwrap().locked == BalanceOf::<T>::from(0 as u32), Error::<T>::LockedNotEmpty);

			let deposit = MinerItems::<T>::get(&sender).unwrap().collateral;

            let _ = T::Currency::unreserve(&sender, deposit.clone());

            <WalletMiners<T>>::remove(&sender);

			<MinerItems<T>>::remove(&sender);

			Self::deposit_event(RawEvent::Redeemed(sender.clone(), deposit.clone()));
		}

		/// Claim the rewards from storage miner's earnings.
        /// 
        /// The dispatch origin of this call must be _Signed_.
		#[weight = 50_000_000]
		fn claim(origin) {
			let sender = ensure_signed(origin)?;

			ensure!(<WalletMiners<T>>::contains_key(&sender), Error::<T>::UnregisteredAccountId);

			ensure!(MinerItems::<T>::get(&sender).unwrap().earnings != BalanceOf::<T>::from(0 as u32), Error::<T>::EarningsIsEmpty);

			let deposit = MinerItems::<T>::get(&sender).unwrap().earnings;

			let reward_pot = T::ModuleId::get().into_account();

            let _ = T::Currency::transfer(&reward_pot, &sender, deposit.clone(), AllowDeath);

			Self::deposit_event(RawEvent::Claimed(sender.clone(), deposit.clone()));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate as pallet_sminer;

	use frame_support::{assert_ok, parameter_types, ord_parameter_types};
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
			Sminer: pallet_sminer::{Module, Call, Storage, Event<T>},
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
		pub const RewardModuleId: ModuleId = ModuleId(*b"rewardpt");
	}
	ord_parameter_types! {
		pub const One: u64 = 1;
	}
	impl Config for Test {
		type Event = Event;
		type Currency = Balances;
		type ModuleId = RewardModuleId;
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
	fn register_should_work() {
		new_test_ext().execute_with(|| {
			let ip: u32 = 127001;
			assert_ok!(Sminer::register(Origin::signed(2), ip, 5));
			assert_eq!(Balances::free_balance(&2), 5);
		});
	}

	#[test]
	fn redeem_should_work() {
		new_test_ext().execute_with(|| {
			let ip: u32 = 127001;
			assert_ok!(Sminer::register(Origin::signed(2), ip, 5));
			assert_eq!(Balances::free_balance(&2), 5);
			assert_ok!(Sminer::redeem(Origin::signed(2)));
			assert_eq!(Balances::free_balance(&2), 10);
		});
	}

}


