#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

pub mod weights;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use frame_system::pallet_prelude::*;
use frame_support::{
	pallet_prelude::*,
	transactional,
	traits::{Currency, ReservableCurrency},
};
use cp_cess_common::{
	IpAddress,
};

pub use pallet::*;

pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use crate::*;
	use frame_system::ensure_signed;

	#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct CacherInfo<Balance> {
		pub ip: IpAddress,
		pub byte_price: Balance,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + sp_std::fmt::Debug {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		//Successful Authorization Events
		Authorize { acc: AccountOf<T>, operator: AccountOf<T> },
		//Cancel authorization success event
		CancelAuthorize { acc: AccountOf<T> },
		//The event of successful Cacher registration
		Register { acc: AccountOf<T>, info: CacherInfo<BalanceOf<T>> },
		//Cacher information change success event
		Update { acc: AccountOf<T>, info: CacherInfo<BalanceOf<T>> },
		//Cacher account logout success event
		Logout { acc: AccountOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		//No errors authorizing any use
		NoAuthorization,
		//Registered Error
		Registered,
		//Unregistered Error
		UnRegister,
		//Option parse Error
		OptionParseError,
	}

	#[pallet::storage]
	#[pallet::getter(fn authority_list)]
	pub(super) type AuthorityList<T: Config> = StorageMap<_, Blake2_128Concat, AccountOf<T>, AccountOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn cacher)]
	pub(super) type Cachers<T: Config> = StorageMap<_, Blake2_128Concat, AccountOf<T>, CacherInfo<BalanceOf<T>>>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::authorize())]
		pub fn authorize(origin: OriginFor<T>, operator: AccountOf<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			AuthorityList::<T>::insert(&sender, &operator);

			Self::deposit_event(Event::<T>::Authorize {
				acc: sender,
				operator,
			});

			Ok(())
		}

		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::cancel_authorize())]
		pub fn cancel_authorize(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<AuthorityList<T>>::contains_key(&sender), Error::<T>::NoAuthorization);

			<AuthorityList<T>>::remove(&sender);

			Self::deposit_event(Event::<T>::CancelAuthorize {
				acc: sender,
			});

			Ok(())
		}

		#[pallet::weight(<T as pallet::Config>::WeightInfo::register())]
		pub fn register(origin: OriginFor<T>, info: CacherInfo<BalanceOf<T>>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!<Cachers<T>>::contains_key(&sender), Error::<T>::Registered);
			<Cachers<T>>::insert(&sender, info.clone());

			Self::deposit_event(Event::<T>::Register {acc: sender, info});

			Ok(())
		}

		#[pallet::weight(<T as pallet::Config>::WeightInfo::update())]
		pub fn update(origin: OriginFor<T>, info: CacherInfo<BalanceOf<T>>) -> DispatchResult {
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

		#[pallet::weight(<T as pallet::Config>::WeightInfo::logout())]
		pub fn logout(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Cachers<T>>::contains_key(&sender), Error::<T>::UnRegister);

			<Cachers<T>>::remove(&sender);

			Self::deposit_event(Event::<T>::Logout { acc: sender });

			Ok(())
		}
	}
}

