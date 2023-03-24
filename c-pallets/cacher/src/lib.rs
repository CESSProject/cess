#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

use frame_system::pallet_prelude::*;
use frame_support::{
	pallet_prelude::*,
	traits::{
		Currency, LockableCurrency,
		ExistenceRequirement::KeepAlive,
	},
};
use cp_cess_common::{
	IpAddress,
};

pub use pallet::*;
use sp_runtime::traits::Zero;
use sp_std::prelude::*;
pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

// Can we create `types.rs` and move these structs there to make it more clean
/// The custom struct for cacher info.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
// Can we rename to AccoutId --> AccountId
pub struct CacherInfo<AccoutId, Balance> {
	// What type of account is this? Can we rename this to appropriate variable name like member, account etc?
	pub acc: AccoutId,
	pub ip: IpAddress,
	pub byte_price: Balance,
}

/// The custom struct for bill info.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
// Can we rename to AccoutId --> AccountId
pub struct Bill<AccoutId, Balance, Hash> {
	pub id: [u8; 16],
	pub to: AccoutId,
	pub amount: Balance,
	pub file_hash: Hash,
	pub slice_hash: Hash,
	pub expiration_time: u64,
}

#[frame_support::pallet]
pub mod pallet {
	use crate::*;
	use frame_system::ensure_signed;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency trait.
		type Currency: LockableCurrency<Self::AccountId>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		//The event of successful Cacher registration
		Register { acc: AccountOf<T>, info: CacherInfo<AccountOf<T>, BalanceOf<T>> },
		//Cacher information change success event
		Update { acc: AccountOf<T>, info: CacherInfo<AccountOf<T>, BalanceOf<T>> },
		//Cacher account logout success event
		Logout { acc: AccountOf<T> },
		//Pay to cacher success event
		Pay { acc: AccountOf<T>, bills: Vec<Bill<AccountOf<T>, BalanceOf<T>, T::Hash>> },
	}

	#[pallet::error]
	pub enum Error<T> {
		// Can we rename it to `AlreadyRegistered`
		//Registered Error
		Registered,
		// Can we rename it to `UnRegistered`
		//Unregistered Error
		UnRegister,
		//Option parse Error
		OptionParseError,
		//Insufficient balance Error
		InsufficientBalance,
	}

	/// Store all cacher info
	#[pallet::storage]
	#[pallet::getter(fn cacher)]
	// Please check https://github.com/CESSProject/cess/issues/42. Can we use `Blake2`?
	pub(super) type Cachers<T: Config> = StorageMap<_, Twox64Concat, AccountOf<T>, CacherInfo<AccountOf<T>, BalanceOf<T>>>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {

		/// Register for cacher.
		///
		/// Parameters:
		/// - `info`: The cacher info related to signer account.
		#[pallet::weight(T::WeightInfo::register())]
		pub fn register(origin: OriginFor<T>, info: CacherInfo<AccountOf<T>, BalanceOf<T>>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!<Cachers<T>>::contains_key(&sender), Error::<T>::Registered);
			<Cachers<T>>::insert(&sender, info.clone());

			Self::deposit_event(Event::<T>::Register {acc: sender, info});

			Ok(())
		}

		/// Update cacher info.
		///
		/// Parameters:
		/// - `info`: The cacher info related to signer account.
		#[pallet::weight(T::WeightInfo::update())]
		pub fn update(origin: OriginFor<T>, info: CacherInfo<AccountOf<T>, BalanceOf<T>>) -> DispatchResult {
			// Is anyone allowed to update cacher information? Can we put some validation for origin?
			let sender = ensure_signed(origin)?;
			ensure!(<Cachers<T>>::contains_key(&sender), Error::<T>::UnRegister);

			<Cachers<T>>::try_mutate(&sender, |info_opt| -> DispatchResult {
				let p_info = info_opt.as_mut().ok_or(Error::<T>::OptionParseError)?;
				*p_info = info.clone();
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::Update {acc: sender, info});

			Ok(())
		}

		/// Cacher exit method, Irreversible process.
		#[pallet::weight(T::WeightInfo::logout())]
		pub fn logout(origin: OriginFor<T>) -> DispatchResult {
			// Can we add validation for origin?
			let sender = ensure_signed(origin)?;
			ensure!(<Cachers<T>>::contains_key(&sender), Error::<T>::UnRegister);

			<Cachers<T>>::remove(&sender);

			Self::deposit_event(Event::<T>::Logout { acc: sender });

			Ok(())
		}

		/// Pay to cachers for downloading files.
		///
		/// Parameters:
		/// - `bills`: list of bill.
		#[pallet::weight(T::WeightInfo::pay(bills.len() as u32))]
		pub fn pay(origin: OriginFor<T>, bills: Vec<Bill<AccountOf<T>, BalanceOf<T>, T::Hash>>) -> DispatchResult {
			// Can we add validation for origin? This should not be allowed by everyone.
			let sender = ensure_signed(origin)?;

			// We don't need this. `T::Currency::transfer` is already taking care of `InsufficientBalance`.
			// let mut total_amount: BalanceOf<T> = Zero::zero();
			// for bill in bills.iter() {
			// 	total_amount += bill.amount;
			// }
			// ensure!(T::Currency::free_balance(&sender) >= total_amount, Error::<T>::InsufficientBalance);

			// This method looks like a simple balance transfer.
			// Can we make it more secure and add checks to verify biller information?
			// For ex: We can match payment receiving account from `Bill` with `CacherInfo`.

			// What is the use of `expiration_time` in `Bill`? Can we add a check for the same?

			// What is the use of `file_hash` and `slice_hash`? I don't see its usage anywhere.

			// We can remove `.iter()`.

			for bill in &bills {
				T::Currency::transfer(&sender, &bill.to, bill.amount, KeepAlive)?;
			}

			Self::deposit_event(Event::<T>::Pay { acc: sender, bills });

			Ok(())
		}
	}
}

