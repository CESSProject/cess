//! # Segemnt Book Module
//!
//! Contain operations related proof of storage.
//!
//! ### Terminology
//! 
//! * **uncid:** 		Necessary parameters for generating proof (unencrypted)
//! * **sealed_cid:** 	Necessary parameters for generating proof (encrypted)
//! * **segment_id:**	Allocated segment ID
//! * **is_ready:**		Used to know whether to submit a certificate
//! * **size_type:**	Segment size
//! * **peer_id:**		Miner's ID 
//! 
//! ### Interface
//!
//! ### Dispatchable Functions
//!
//! * `intent_submit` 		Pprovide miners with the necessary parameters to generate proof
//! * `intent_submit_po_st` Provide miners with the necessary parameters to generate proof
//! * `submit_to_vpa` 		Submit copy certificate of idle data segment
//! * `verify_in_vpa` 		Verify replication proof of idle data segments
//! * `submit_to_vpb` 		Submit space-time proof of idle data segments
//! * `verify_in_vpb` 		Verify the spatiotemporal proof of idle data segments
//! * `submit_to_vpc` 		Submit a copy certificate of the service data segment
//! * `verify_in_vpc` 		Verify the replication certificate of the service data segment
//! * `submit_to_vpd` 		Submit spatio-temporal proof of service data segment
//! * `verify_in_vpd` 		Verify the spatio-temporal proof of service data segments

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub use pallet::*;
mod benchmarking;
pub mod weights;
use sp_runtime::{
	RuntimeDebug,
	traits::{SaturatedConversion, CheckedDiv, CheckedAdd,},
};
mod types;
use types::*;

use sp_std::prelude::*;
use codec::{Encode, Decode};
use frame_support::{
	storage::bounded_vec::BoundedVec,
	dispatch::DispatchResult,
	PalletId,
	traits::{ReservableCurrency, Get, Randomness, FindAuthor},
};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
pub use weights::WeightInfo;
use pallet_file_bank::RandomFileList;
use sp_core::H256;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;



#[frame_support::pallet]
pub mod pallet {
	use super::*;
	// use frame_benchmarking::baseline::Config;
use frame_support::{
		ensure,
		
		traits::Get,
	};
	use frame_system::{ensure_signed, pallet_prelude::*};

	pub type BoundedString<T> = BoundedVec<u8, <T as Config>::StringLimit>;
	pub type BoundedList<T> = BoundedVec<BoundedVec<u8, <T as Config>::StringLimit>, <T as Config>::StringLimit>;
	pub const limit: u64 = 18446744073709551615;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		//the weights
		type WeightInfo: WeightInfo;
		#[pallet::constant]
		/// The pallet id
		type MyPalletId: Get<PalletId>;

		#[pallet::constant]
		type StringLimit: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type RandomLimit: Get<u32> + Clone + Eq + PartialEq;
		//one day block
		#[pallet::constant]
		type OneDay: Get<BlockNumberOf<Self>>;
		//one hours block
		#[pallet::constant]
		type OneHours: Get<BlockNumberOf<Self>>;
		// randomness for seeds.
		type MyRandomness: Randomness<Self::Hash, Self::BlockNumber>;
		//Find the consensus of the current block
		type FindAuthor: FindAuthor<Self::AccountId>;

		type RandomChallenge: RandomFileList;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		
	}

	/// Error for the segment-book pallet.
	#[pallet::error]
	pub enum Error<T> {
		//Vec to BoundedVec Error.
		BoundedVecError,
		//Error that the storage has reached the upper limit.
		StorageLimitReached,

		Overflow,
		//The miner submits a certificate, but there is no error in the challenge list
		NoChallenge,
	}

	//Information about storage challenges
	#[pallet::storage]
	#[pallet::getter(fn challenge_map)]
	pub type ChallengeMap<T: Config> = StorageMap<_, Twox64Concat, u64, BoundedVec<ChallengeInfo<T>, T::StringLimit>, ValueQuery>;

	//Relevant time nodes for storage challenges
	#[pallet::storage]
	#[pallet::getter(fn challenge_duration)]
	pub(super) type ChallengeDuration<T: Config> = StorageValue<_, BlockNumberOf<T>, ValueQuery>;

	//Store the certification information submitted by the miner and wait for the specified scheduling verification
	#[pallet::storage]
	#[pallet::getter(fn unverify_proof)]
	pub(super) type UnVerifyProof<T: Config> = StorageMap<_, Twox64Concat, AccountOf<T>, BoundedVec<ProveInfo<T>, T::StringLimit>, ValueQuery>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberOf<T>> for Pallet<T> {
		//Used to calculate whether it is implied to submit spatiotemporal proof
		//Cycle every 7.2 hours
		//When there is an uncommitted space-time certificate, the corresponding miner will be punished 
		//and the corresponding data segment will be removed
		fn on_initialize(now: BlockNumberOf<T>) -> Weight {
			let number: u128 = now.saturated_into();
			let deadline = Self::challenge_duration();
			log::info!("start --------------------------------- ");
			

			if now == deadline {
				//After the waiting time for the challenge reaches the deadline, 
				//the miners who fail to complete the challenge will be punished
				

			}
			if now > deadline {
				if Self::trigger_challenge() {
					Self::generation_challenge().expect("generation challenge failed");
					Self::record_challenge_time().expect("record time failed");
				}
			}


			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000_000)]
		pub fn submit_challenge_prove(
			origin: OriginFor<T>, 
			miner_id: u64, 
			file_id: Vec<u8>, 
			mu: Vec<u8>, 
			sigma: Vec<u8>
		) -> DispatchResult {
			let _sender = ensure_signed(origin)?;
			let acc = Self::get_current_scheduler();

			let challenge_list = Self::challenge_map(miner_id);

			for v in challenge_list.iter() {
				if v.file_id == file_id {
					Self::storage_prove(acc, miner_id.clone(), v.clone(), mu.clone(), sigma.clone())?;

					return Ok(())
				}
			}

			Err(Error::<T>::NoChallenge)?
		}

	}
		
	
		
}



impl<T: Config> Pallet<T> {
	fn storage_prove(
		acc: AccountOf<T>,
		miner_id: u64, 
		challenge_info: ChallengeInfo<T>,
		mu: Vec<u8>, 
		sigma: Vec<u8>
	) -> DispatchResult {

		let proveinfo = ProveInfo::<T>{
			miner_id: miner_id,
			challenge_info: challenge_info,
			mu: mu.clone().try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
			sigma: sigma.clone().try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
		};

		<UnVerifyProof<T>>::try_mutate(&acc, |o| -> DispatchResult {
			o.try_push(proveinfo).map_err(|_e| Error::<T>::StorageLimitReached)?;
			Ok(())
		})?;

		Ok(())
	}

	// fn clear_challenge_info(miner_id: Vec<u8>, file_id: Vec<u8>) -> DispatchResult {
	// 	<ChallengeMap<T>>
	// }

	fn get_current_scheduler() -> AccountOf<T> {
		let digest = <frame_system::Pallet<T>>::digest();
			let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
			let acc = T::FindAuthor::find_author(pre_runtime_digests).map(|a| {
				a
			});
			acc.unwrap()
	}

	fn record_challenge_time() -> DispatchResult {
		let mut now = <frame_system::Pallet<T>>::block_number();
		let one_hours = T::OneHours::get();
		<ChallengeDuration<T>>::try_mutate(|o| -> DispatchResult {	
			*o = now.checked_add(&one_hours).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	fn trigger_challenge() -> bool {

		let time_point = Self::random_time_number(20220509);
		//The chance to trigger a challenge is once a day
		let probability: u32 = T::OneDay::get().saturated_into();
		let range = limit / probability as u64;
		if (time_point > 2190502) && (time_point < (range + 2190502)) {
			return true;
		}
		false
	}

	fn generation_challenge() -> DispatchResult {
		let result = T::RandomChallenge::get_random_challenge_data()?;
		for (miner_id, file_id, block_list) in result {
			loop {
				let random = Self::generate_random_number(20220510);
				
				//Create a single challenge message in files
				let challenge_info = ChallengeInfo::<T>{
					file_id: file_id.try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
					block_list: block_list.try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
					random: random[0..47].to_vec().try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
				};
				//Push storage
				<ChallengeMap<T>>::try_mutate(&miner_id, |o| -> DispatchResult {
					o.try_push(challenge_info).map_err(|_e| Error::<T>::StorageLimitReached)?;
					Ok(())
				})?;
				break;
				
			}
			
		}

		Ok(())
	}

	// Generate a random number from a given seed.
	fn random_time_number(seed: u32) -> u64 {
		let (random_seed, _) = T::MyRandomness::random(&(T::MyPalletId::get(), seed).encode());
			let random_number = <u64>::decode(&mut random_seed.as_ref())
				.expect("secure hashes should always be bigger than u32; qed");
			random_number
	}

	fn generate_random_number(seed: u32) -> Vec<u8> {
		let (r_seed, _) = T::MyRandomness::random(&(T::MyPalletId::get(), seed).encode());
		let random_seed = <H256>::decode(&mut r_seed.as_ref())
		.expect("secure hashes should always be bigger than u32; qed");
		random_seed.as_bytes().to_vec()
	}

}


