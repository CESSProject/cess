#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

pub mod weights;

mod types;
use types::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use codec::alloc::string::ToString;
use sp_core::H256;
use frame_system::pallet_prelude::*;
use frame_support::{
	pallet_prelude::*, transactional,
};
use cp_cess_common::*;
use sp_std::vec::Vec;
use sp_runtime::traits::TrailingZeroInput;

pub use pallet::*;

pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;

#[frame_support::pallet]
pub mod pallet {
	use crate::*;
	use frame_support::Hashable;
use frame_system::{ensure_signed, Origin};

	#[pallet::config]
	pub trait Config: frame_system::Config + sp_std::fmt::Debug {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		type WeightInfo: WeightInfo;

		#[pallet::constant]
		type P2PLength: Get<u32> + Clone;

		#[pallet::constant]
		type AuthorLimit: Get<u32> + Clone;

		// type AccountIdConvertor: AccountIdConvertor<Self::AccountId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		//Successful Authorization Events
		Authorize { acc: AccountOf<T>, operator: AccountOf<T> },
		//Cancel authorization success event
		CancelAuthorize { acc: AccountOf<T>, oss: AccountOf<T> },
		//The event of successful Oss registration
		OssRegister { acc: AccountOf<T>, endpoint: PeerId },
		//Oss information change success event
		OssUpdate { acc: AccountOf<T>, new_endpoint: PeerId },
		//Oss account destruction success event
		OssDestroy { acc: AccountOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// No errors authorizing any use
		NoAuthorization,
		/// Registered Error
		Registered,
		/// Unregistered Error
		UnRegister,
		/// Option parse Error
		OptionParseError,
		/// Convert bounded vector Error
		BoundedVecError,
		/// Already Exists Error
		Existed,

		MalformedSignature,

		VerifySigFailed,
	}

	#[pallet::storage]
	#[pallet::getter(fn authority_list)]
	pub(super) type AuthorityList<T: Config> = StorageMap<_, Blake2_128Concat, AccountOf<T>, BoundedVec<AccountOf<T>, T::AuthorLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn oss)]
	pub(super) type Oss<T: Config> = StorageMap<_, Blake2_128Concat, AccountOf<T>, OssInfo>;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		
		/// Authorize Operator
		///
		/// This function allows an account to authorize another account as an operator, 
		/// granting them specific permissions or access rights to perform actions on behalf of the authorizing account.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization. Typically, this is the authorizing account.
		/// - `operator`: The account that will be authorized as an operator by the authorizing account.
		#[pallet::call_index(0)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::authorize())]
		pub fn authorize(origin: OriginFor<T>, operator: AccountOf<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			AuthorityList::<T>::try_mutate(&sender, |authority_list| -> DispatchResult {
				if !authority_list.contains(&operator) {
					authority_list.try_push(operator.clone()).map_err(|_| Error::<T>::BoundedVecError)?;
				}

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::Authorize {
				acc: sender,
				operator,
			});

			Ok(())
		}

		/// Cancel Authorization
		///
		/// This function allows an account to cancel the authorization of another account (operator), 
		/// revoking the operator's permissions or access rights to perform actions on behalf of the authorizing account.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization. Typically, this is the authorizing account.
		/// - `oss`: The account for which the authorization is canceled by the authorizing account.
		#[pallet::call_index(1)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::cancel_authorize())]
		pub fn cancel_authorize(origin: OriginFor<T>, oss: AccountOf<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<AuthorityList<T>>::contains_key(&sender), Error::<T>::NoAuthorization);

			AuthorityList::<T>::try_mutate(&sender, |authority_list| -> DispatchResult {
				authority_list.retain(|authority| authority != &oss); 

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::CancelAuthorize {
				acc: sender,
				oss,
			});

			Ok(())
		}

		/// Register an OSS
		///
		/// This function allows an account to register as an OSS in the pallet. 
		/// An OSS is a service provider that offers its services via a defined endpoint. 
		/// Registering as an OSS enables the account to make its services accessible to other users and be authorized for certain actions.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization. Typically, this is the account registering as an OSS.
		/// - `endpoint`: The unique peer ID or endpoint that identifies the OSS and its services.
		/// - `domain`: A bounded vector of up to 50 bytes representing the domain or description of the OSS.
		#[pallet::call_index(2)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::register())]
		pub fn register(origin: OriginFor<T>, endpoint: PeerId, domain: BoundedVec<u8, ConstU32<50>>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!<Oss<T>>::contains_key(&sender), Error::<T>::Registered);
			let oss_info = OssInfo {
				peer_id: endpoint.clone(),
				domain,
			};
			<Oss<T>>::insert(&sender, oss_info);

			Self::deposit_event(Event::<T>::OssRegister {acc: sender, endpoint});

			Ok(())
		}

		/// Update OSS Information
		///
		/// This function allows a registered OSS to update its information, 
		/// including its endpoint and domain. 
		/// An OSS is a service provider that offers its services via a defined endpoint. 
		/// This function enables the OSS to modify its endpoint and provide an updated description of its services.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization. Typically, this is the registered OSS that wishes to update its information.
		/// - `endpoint`: The new unique peer ID or endpoint that identifies the OSS and its services.
		/// - `domain`: A bounded vector of up to 50 bytes representing the updated domain or description of the OSS's services.
		#[pallet::call_index(3)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::update())]
		pub fn update(origin: OriginFor<T>, endpoint: PeerId, domain: BoundedVec<u8, ConstU32<50>>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Oss<T>>::contains_key(&sender), Error::<T>::UnRegister);

			<Oss<T>>::try_mutate(&sender, |oss_info_opt| -> DispatchResult {
				let oss_info = oss_info_opt.as_mut().ok_or(Error::<T>::OptionParseError)?;
				oss_info.peer_id = endpoint;
				oss_info.domain = domain;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::OssUpdate {acc: sender, new_endpoint: endpoint});

			Ok(())
		}

		/// Destroy OSS Registration
		///
		/// This function allows a registered OSS to voluntarily destroy its registration, 
		/// effectively unregistering from the system. Once an OSS is unregistered, 
		/// it will no longer be recognized as a valid service provider.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization. Typically, this is the registered OSS that wishes to destroy its registration.
		#[pallet::call_index(4)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::destroy())]
		pub fn destroy(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<Oss<T>>::contains_key(&sender), Error::<T>::UnRegister);

			<Oss<T>>::remove(&sender);

			Self::deposit_event(Event::<T>::OssDestroy { acc: sender });

			Ok(())
		}

		#[pallet::call_index(5)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::destroy())]
		pub fn proxy_authorzie(origin: OriginFor<T>, auth_puk: sp_core::sr25519::Public, sig: BoundedVec<u8, ConstU32<64>>, payload: ProxyAuthPayload<T>) -> DispatchResult {
			let _ = ensure_signed(origin)?;

			let mut payload_encode = payload.encode();
			let mut b1 = "<Bytes>".to_string().as_bytes().to_vec();
			let mut b2 = "</Bytes>".to_string().as_bytes().to_vec();

			let mut origin: Vec<u8> = Default::default();
			origin.append(&mut b1);
			origin.append(&mut payload_encode);
			origin.append(&mut b2);

			let account = auth_puk.using_encoded(|entropy| {
				AccountOf::<T>::decode(&mut TrailingZeroInput::new(entropy))
					.expect("infinite input; no invalid input; qed")
			});

			let sig = 
				sp_core::sr25519::Signature::try_from(sig.as_slice()).or(Err(Error::<T>::MalformedSignature))?;

			ensure!(
				sp_io::crypto::sr25519_verify(&sig, &origin, &auth_puk),
				Error::<T>::VerifySigFailed
			);

			AuthorityList::<T>::try_mutate(&account, |list| -> DispatchResult {
				ensure!(!list.contains(&payload.oss), Error::<T>::Existed);

				list.try_push(payload.oss).map_err(|_| Error::<T>::BoundedVecError)?;

				Ok(())
			})?; 

			Ok(())
		}
	}
}

pub trait OssFindAuthor<AccountId> {
	fn is_authorized(owner: AccountId, operator: AccountId) -> bool;
}

impl<T: Config> OssFindAuthor<AccountOf<T>> for Pallet<T> {
	fn is_authorized(owner: AccountOf<T>, operator: AccountOf<T>) -> bool {
		let acc_list = <AuthorityList<T>>::get(&owner);
		acc_list.contains(&operator)
	}
}
