#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
	dispatch::DispatchResult,
	pallet_prelude::*,
};
use frame_system::{
	ensure_root,
	pallet_prelude::OriginFor,
};

use pallet_tx_pause::{RuntimeCallNameOf, PalletNameOf, PalletCallNameOf, Pallet as TxPausePallet};
use sp_std::{marker::PhantomData, vec, vec::Vec};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	
	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_tx_pause::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Paused { full_name: RuntimeCallNameOf<T> },
	}

	// #[pallet::error]
	// pub enum Error<T> {

	// }

    #[pallet::storage]
    pub(super) type PausedPalletsHook<T: Config> = StorageMap<_, Blake2_128Concat, PalletNameOf<T>, bool>;

	#[derive(Debug, Clone, PartialEq, Eq, Decode, Encode, MaxEncodedLen, TypeInfo)]
	pub enum CessPallet {
		FileBank,
		Audit,
		StorageHandler,
		Sminer,
		Reservoir,
		Oss,
		EvmAccountMapping,
		TeeWorker,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn pause_pallet(origin: OriginFor<T>, full_name: RuntimeCallNameOf<T>) -> DispatchResult {
			let _ = ensure_root(origin.clone())?;
			let (pallet_name, tx_name) = full_name.clone();
			if tx_name == "hook".as_bytes().to_vec() {
				PausedPalletsHook::<T>::insert(&pallet_name, true);
			} else {
				TxPausePallet::<T>::pause(origin, full_name.clone())?;
			}

			Self::deposit_event(Event::<T>::Paused{full_name});
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn unpause_pallet(origin: OriginFor<T>, full_name: RuntimeCallNameOf<T>) -> DispatchResult {
			let _ = ensure_root(origin.clone())?;
			let (pallet_name, tx_name) = full_name.clone();
			if tx_name == "hook".as_bytes().to_vec() {
				PausedPalletsHook::<T>::remove(&pallet_name);
			} else {
				TxPausePallet::<T>::unpause(origin, full_name)?;
			}
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn pause_cess_for_pallet(origin: OriginFor<T>, pallet: CessPallet) -> DispatchResult {
			let _ = ensure_root(origin.clone())?;

			let call_of_pallet = Self::call_of_pallet(pallet.clone());
			let pallet_name: PalletNameOf<T> = match pallet {
				CessPallet::FileBank => {
					b"FileBank".to_vec().try_into().unwrap()
				},
				CessPallet::Audit => {
					b"Audit".to_vec().try_into().unwrap()
				},
				CessPallet::StorageHandler => {
					b"StorageHandler".to_vec().try_into().unwrap()
				},
				CessPallet::Sminer => {
					b"Sminer".to_vec().try_into().unwrap()
				},
				CessPallet::Reservoir => {
					b"Reservoir".to_vec().try_into().unwrap()
				},
				CessPallet::Oss => {
					b"Oss".to_vec().try_into().unwrap()
				},
				CessPallet::EvmAccountMapping => {
					b"EvmAccountMapping".to_vec().try_into().unwrap()
				},
				CessPallet::TeeWorker => {
					b"TeeWorker".to_vec().try_into().unwrap()
				},
			};

			for call in call_of_pallet {
				Self::pause_pallet(origin.clone(), (pallet_name.clone(), call))?;
			}
			
			Ok(())
		}
		
	}

	impl<T: Config> Pallet<T> {
		pub fn call_of_pallet(pallet_name: CessPallet) -> Vec<PalletCallNameOf<T>> {
			let res = match pallet_name {
				CessPallet::FileBank => {
					vec![
						b"hook".to_vec().try_into().unwrap(),
						b"upload_declaration".to_vec().try_into().unwrap(), 
						b"territory_file_delivery".to_vec().try_into().unwrap(), 
						b"transfer_report".to_vec().try_into().unwrap(), 
						b"calculate_report".to_vec().try_into().unwrap(),
						b"replace_idle_space".to_vec().try_into().unwrap(),
						b"delete_file".to_vec().try_into().unwrap(),
						b"cert_idle_space".to_vec().try_into().unwrap(),
						b"generate_restoral_order".to_vec().try_into().unwrap(),
						b"claim_restoral_order".to_vec().try_into().unwrap(),
						b"claim_restoral_noexist_order".to_vec().try_into().unwrap(),
					]
				},

				CessPallet::Audit => {
					vec![
						b"hook".to_vec().try_into().unwrap(),
						b"submit_idle_proof".to_vec().try_into().unwrap(),
						b"submit_service_proof".to_vec().try_into().unwrap(),
						b"submit_verify_idle_result".to_vec().try_into().unwrap(),
						b"submit_verify_service_result".to_vec().try_into().unwrap(),
					]
				},

				CessPallet::StorageHandler => {
					vec![
						b"hook".to_vec().try_into().unwrap(),
						b"mint_territory".to_vec().try_into().unwrap(),
						b"expanding_territory".to_vec().try_into().unwrap(),
						b"renewal_territory".to_vec().try_into().unwrap(),
						b"reactivate_territory".to_vec().try_into().unwrap(),
						b"territory_consignment".to_vec().try_into().unwrap(),
						b"buy_consignment".to_vec().try_into().unwrap(),
						b"exec_consignment".to_vec().try_into().unwrap(),
						b"cancel_consignment".to_vec().try_into().unwrap(),
						b"cancel_purchase_action".to_vec().try_into().unwrap(),
						b"territory_grants".to_vec().try_into().unwrap(),
						b"create_order".to_vec().try_into().unwrap(),
						b"territory_rename".to_vec().try_into().unwrap(),
						b"exec_order".to_vec().try_into().unwrap(),
					]
				},

				CessPallet::Sminer => {
					vec![
						b"hook".to_vec().try_into().unwrap(),
						b"regnstk".to_vec().try_into().unwrap(),
						b"increase_collateral".to_vec().try_into().unwrap(),
						b"update_beneficiary".to_vec().try_into().unwrap(),
						b"update_endpoint".to_vec().try_into().unwrap(),
						b"decrease_declaration_space".to_vec().try_into().unwrap(),
						b"receive_reward".to_vec().try_into().unwrap(),
						b"miner_exit_prep".to_vec().try_into().unwrap(),
						b"miner_withdraw".to_vec().try_into().unwrap(),
						b"faucet_top_up".to_vec().try_into().unwrap(),
						b"faucet".to_vec().try_into().unwrap(),
						b"register_pois_key".to_vec().try_into().unwrap(),
						b"regnstk_assign_staking".to_vec().try_into().unwrap(),
						b"increase_declaration_space".to_vec().try_into().unwrap(),
					]
				},

				CessPallet::Reservoir => {
					vec![
						b"hook".to_vec().try_into().unwrap(),
						b"filling".to_vec().try_into().unwrap(),
						b"store".to_vec().try_into().unwrap(),
						b"withdraw".to_vec().try_into().unwrap(),
						b"attend_event".to_vec().try_into().unwrap(),
					]
				},

				CessPallet::Oss => {
					vec![
						b"hook".to_vec().try_into().unwrap(),
						b"authorize".to_vec().try_into().unwrap(),
						b"cancel_authorize".to_vec().try_into().unwrap(),
						b"register".to_vec().try_into().unwrap(),
						b"update".to_vec().try_into().unwrap(),
						b"destroy".to_vec().try_into().unwrap(),
						b"proxy_authorzie".to_vec().try_into().unwrap(),
						b"evm_proxy_authorzie".to_vec().try_into().unwrap(),
					]
				},

				CessPallet::EvmAccountMapping => {
					vec![
						b"hook".to_vec().try_into().unwrap(),
						b"meta_call".to_vec().try_into().unwrap(),
					]
				},

				CessPallet::TeeWorker => {
					vec![
						b"hook".to_vec().try_into().unwrap(),
						b"refresh_tee_status".to_vec().try_into().unwrap(),
						b"register_worker".to_vec().try_into().unwrap(),
						b"register_worker_v2".to_vec().try_into().unwrap(),
						b"update_worker_endpoint".to_vec().try_into().unwrap(),
						b"apply_master_key".to_vec().try_into().unwrap(),
					]
				},
			};

			res
		}
	}
}

pub trait Pauseable {
	fn is_paused(pallet_name: Vec<u8>) -> bool;
}

impl<T: Config> Pauseable for Pallet<T> {
	fn is_paused(pallet_name: Vec<u8>) -> bool {
		let pallet_name = TryInto::<PalletNameOf<T>>::try_into(pallet_name);
		if let Ok(pallet_name) = pallet_name {
			PausedPalletsHook::<T>::get(&pallet_name).unwrap_or(false)
		} else {
			false
		}
	}
}