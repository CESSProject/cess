#![cfg_attr(not(feature = "std"), no_std)]

mod benchmarking;

use frame_system::pallet_prelude::*;
use frame_support::{
	pallet_prelude::*, transactional
};
use cp_cess_common::{
	IpAddress,
};

pub use pallet::*;

type AccountOf<T> = <T as frame_system::Config>::AccountId;

#[frame_support::pallet]
pub mod pallet {
	use crate::*;
	use frame_system::ensure_signed;

	#[pallet::config]
	pub trait Config: frame_system::Config + sp_std::fmt::Debug {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		//Successful Authorization Events
		Authorize { acc: AccountOf<T>, operator: AccountOf<T> },
		//Cancel authorization success event
		CancelAuthorize { acc: AccountOf<T> },
		//The event of successful Oss registration
		OssRegister { acc: AccountOf<T>, endpoint: IpAddress },
		//Oss information change success event
		OssUpdate { acc: AccountOf<T>, new_endpoint: IpAddress },
		//Oss account destruction success event
		OssDestroy { acc: AccountOf<T> },
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
	#[pallet::getter(fn oss)]
	pub(super) type Oss<T: Config> = StorageMap<_, Blake2_128Concat, AccountOf<T>, IpAddress>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[transactional]
		#[pallet::weight(1_000_000)]
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
		#[pallet::weight(1_000_000)]
		pub fn cancel_authorize(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<AuthorityList<T>>::contains_key(&sender), Error::<T>::NoAuthorization);

			<AuthorityList<T>>::remove(&sender);

			Self::deposit_event(Event::<T>::CancelAuthorize {
				acc: sender,
			});

			Ok(())
		}

		#[transactional]
		#[pallet::weight(3_000_000)]
		pub fn register(origin: OriginFor<T>, endpoint: IpAddress) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!<Oss<T>>::contains_key(&sender), Error::<T>::Registered);
			<Oss<T>>::insert(&sender, endpoint.clone());

			Self::deposit_event(Event::<T>::OssRegister {acc: sender, endpoint});

			Ok(())
		}

		#[transactional]
		#[pallet::weight(3_000_000)]
		pub fn update(origin: OriginFor<T>, endpoint: IpAddress) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Oss<T>>::contains_key(&sender), Error::<T>::UnRegister);

			<Oss<T>>::try_mutate(&sender, |endpoint_opt| -> DispatchResult {
				let p_endpoint = endpoint_opt.as_mut().ok_or(Error::<T>::OptionParseError)?;
				*p_endpoint = endpoint.clone();
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::OssUpdate {acc: sender, new_endpoint: endpoint});

			Ok(())
		}

		#[transactional]
		#[pallet::weight(3_000_000)]
		pub fn destroy(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Oss<T>>::contains_key(&sender), Error::<T>::UnRegister);

			<Oss<T>>::remove(&sender);

			Self::deposit_event(Event::<T>::OssDestroy { acc: sender });

			Ok(())
		}
	}
}

pub trait OssFindAuthor<AccountId> {
	fn is_authorized(owner: AccountId, operator: AccountId) -> bool;
}

impl<T: Config> OssFindAuthor<AccountOf<T>> for Pallet<T> {
	fn is_authorized(owner: AccountOf<T>, operator: AccountOf<T>) -> bool {
		if let Some(acc) = <AuthorityList<T>>::get(&owner) {
			return acc == operator;
		}
		false
	}
}
