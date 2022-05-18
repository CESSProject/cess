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
	traits::{SaturatedConversion, CheckedAdd,},
};
mod types;
use types::*;

use sp_std::prelude::*;
use codec::{Encode, Decode};
use frame_support::{
	storage::bounded_vec::BoundedVec,
	dispatch::DispatchResult,
	PalletId,
	traits::{ReservableCurrency, Randomness, FindAuthor},
};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
pub use weights::WeightInfo;
use pallet_file_bank::RandomFileList;
use sp_core::H256;
use pallet_file_map::ScheduleFind;
use pallet_sminer::MinerControl;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;



#[frame_support::pallet]
pub mod pallet {
	use super::*;
	// use frame_benchmarking::baseline::Config;
use frame_support::{
		traits::Get,
	};
	use frame_system::{ensure_signed, pallet_prelude::*};

	pub type BoundedString<T> = BoundedVec<u8, <T as Config>::StringLimit>;
	pub type BoundedList<T> = BoundedVec<BoundedVec<u8, <T as Config>::StringLimit>, <T as Config>::StringLimit>;
	
	pub const LIMIT: u64 = 18446744073709551615;

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
		//Random files used to obtain this batch of challenges
		type File: RandomFileList;
		//Judge whether it is the trait of the consensus node
		type Scheduler: ScheduleFind<Self::AccountId>;
		//It is used to increase or decrease the miners' computing power, space, and execute punishment
		type MinerControl: MinerControl;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ChallengeProof{peer_id: u64},

		VerifyProof{peer_id: u64},
	}

	/// Error for the segment-book pallet.
	#[pallet::error]
	pub enum Error<T> {
		//Vec to BoundedVec Error.
		BoundedVecError,
		//Error that the storage has reached the upper LIMIT.
		StorageLimitReached,

		Overflow,
		//The miner submits a certificate, but there is no error in the challenge list
		NoChallenge,
		//Not a consensus node or not registered
		ScheduleNonExistent,
		//The certificate does not exist or the certificate is not verified by this dispatcher
		NonProof,
		//filetype error
		FileTypeError,
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
			let _number: u128 = now.saturated_into();
			let deadline = Self::challenge_duration();	
			//The waiting time for the challenge has reached the deadline
			if now == deadline {
				//After the waiting time for the challenge reaches the deadline, 
				//the miners who fail to complete the challenge will be punished
				for (miner_id, challenge_list) in <ChallengeMap<T>>::iter() {
					for v in challenge_list {
						if let Err(e) = Self::punish(miner_id, v.file_id.to_vec(), v.file_size, v.file_type) {
							log::info!("punish Err:{:?}", e);
						}
						<ChallengeMap<T>>::remove(miner_id);
					}
				}
			}
			//The interval between challenges must be greater than one hour
			if now > deadline {
				//Determine whether to trigger a challenge
				// if Self::trigger_challenge() {
					if let Err(e) = Self::generation_challenge() {
						log::info!("punish Err:{:?}", e);
					}
					if let Err(e) = Self::record_challenge_time() {
						log::info!("punish Err:{:?}", e);
					}
				// }
			}
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn submit_challenge_prove(
			origin: OriginFor<T>, 
			miner_id: u64, 
			file_id: Vec<u8>, 
			mu: Vec<Vec<u8>>, 
			sigma: Vec<u8>
		) -> DispatchResult {
			let _sender = ensure_signed(origin)?;
			let acc = Self::get_current_scheduler();

			let challenge_list = Self::challenge_map(miner_id);

			for v in challenge_list.iter() {
				if v.file_id == file_id {
					Self::storage_prove(acc, miner_id.clone(), v.clone(), mu.clone(), sigma.clone())?;
					Self::clear_challenge_info(miner_id.clone(), file_id.clone())?;
					Self::deposit_event(Event::<T>::ChallengeProof{peer_id: miner_id});
					return Ok(())
				}
			}
			
			Err(Error::<T>::NoChallenge)?
		}

		#[pallet::weight(9_700)]
		pub fn verify_proof(
			origin: OriginFor<T>, 
			miner_id: u64, 
			file_id: Vec<u8>, 
			result: bool
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			if !T::Scheduler::contains_scheduler(sender.clone()) {
				Err(Error::<T>::ScheduleNonExistent)?;
			}

			let verify_list = Self::unverify_proof(&sender);
			//Clean up the corresponding data in the pool that is not verified by consensus, 
			//and judge whether to punish according to the structure
			for value in verify_list.iter() {
				if (value.miner_id == miner_id) && (value.challenge_info.file_id == file_id) {
					Self::clear_verify_proof(sender.clone(), miner_id.clone(), file_id.clone())?;
					//If the result is false, a penalty will be imposed
					if !result {
						Self::punish(miner_id, file_id, value.challenge_info.file_size, value.challenge_info.file_type)?;
					}
					Self::deposit_event(Event::<T>::VerifyProof{peer_id: miner_id});
					return Ok(())
				}
			}
			
			
			Err(Error::<T>::NonProof)?
		}

	}

	impl<T: Config> Pallet<T> {
		//Storage proof method
		fn storage_prove(
			acc: AccountOf<T>,
			miner_id: u64, 
			challenge_info: ChallengeInfo<T>,
			mu: Vec<Vec<u8>>, 
			sigma: Vec<u8>
		) -> DispatchResult {
			let proveinfo = ProveInfo::<T>{
				miner_id: miner_id,
				challenge_info: challenge_info,
				mu: Self::vec_to_bounded(mu.clone())?,
				sigma: sigma.clone().try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
			};
	
			<UnVerifyProof<T>>::try_mutate(&acc, |o| -> DispatchResult {
				o.try_push(proveinfo).map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;
			Ok(())
		}
	
		//Clean up the verified certificate corresponding to the consensus
		fn clear_verify_proof(acc: AccountOf<T>, miner_id: u64, file_id: Vec<u8>) -> DispatchResult {
			<UnVerifyProof<T>>::try_mutate(&acc, |o| -> DispatchResult {
				o.retain(|x| x.miner_id != miner_id && x.challenge_info.file_id != file_id);
				Ok(())
			})?;
			Ok(())
		}
	
		//Clean up the corresponding challenges in the miner's challenge pool
		fn clear_challenge_info(miner_id: u64, file_id: Vec<u8>) -> DispatchResult {
			<ChallengeMap<T>>::try_mutate(miner_id, |o| -> DispatchResult {
				o.retain(|x| x.file_id != file_id);
				Ok(())
			})?;
			Ok(())
		}
	
		//Obtain the consensus of the current block
		fn get_current_scheduler() -> AccountOf<T> {
			let digest = <frame_system::Pallet<T>>::digest();
			let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
			let acc = T::FindAuthor::find_author(pre_runtime_digests).map(|a| {
				a
			});
			T::Scheduler::get_controller_acc(acc.unwrap())
		}
	
		//Record challenge time
		fn record_challenge_time() -> DispatchResult {
			let now = <frame_system::Pallet<T>>::block_number();
			// let one_hours = T::OneHours::get();
			let test: BlockNumberOf<T> = 600u32.try_into().map_err(|_e| Error::<T>::Overflow)?;
			<ChallengeDuration<T>>::try_mutate(|o| -> DispatchResult {	
				*o = now.checked_add(&test).ok_or(Error::<T>::Overflow)?;
				Ok(())
			})?;
	
			Ok(())
		}
	
		//Trigger: whether to trigger the challenge
		fn trigger_challenge() -> bool {
	
			let time_point = Self::random_time_number(20220509);
			//The chance to trigger a challenge is once a day
			let probability: u32 = T::OneDay::get().saturated_into();
			let range = LIMIT / probability as u64;
			if (time_point > 2190502) && (time_point < (range + 2190502)) {
				return true;
			}
			false
		}
	
		//Ways to generate challenges
		fn generation_challenge() -> DispatchResult {
			let result = T::File::get_random_challenge_data()?;
			let mut x = 0;
			for (miner_id, file_id, block_list, file_size, file_type, segment_size) in result {
					x = x + 1;
					let random = Self::generate_random_number(20220510 + x, block_list.len() as u32);
					//Create a single challenge message in files
					let challenge_info = ChallengeInfo::<T>{
						file_type: file_type,
						file_id: file_id.try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
						file_size: file_size,
						segment_size: segment_size,
						block_list: block_list.try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
						random: Self::vec_to_bounded(random)?,
					};
					//Push storage
					<ChallengeMap<T>>::try_mutate(&miner_id, |o| -> DispatchResult {
						o.try_push(challenge_info).map_err(|_e| Error::<T>::StorageLimitReached)?;
						Ok(())
					})?;
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
	
		//The number of pieces generated is VEC
		fn generate_random_number(seed: u32, length: u32) -> Vec<Vec<u8>> {
			let mut random_list: Vec<Vec<u8>> = Vec::new();
			for _ in 0..length {
				loop {
					let (r_seed, _) = T::MyRandomness::random(&(T::MyPalletId::get(), seed).encode());
					let random_seed = <H256>::decode(&mut r_seed.as_ref())
					.expect("secure hashes should always be bigger than u32; qed");
					let random_vec = random_seed.as_bytes().to_vec();
					if random_vec.len() >= 20 {
						random_list.push(random_vec[0..19].to_vec());
						break;
					}
				}
			}
			random_list	
		}

		fn punish(miner_id: u64, file_id: Vec<u8>, file_size: u64, file_type: u8) -> DispatchResult {
			if !T::MinerControl::miner_is_exist(miner_id) {
				return Ok(());
			}
			match file_type {
				1 =>  {
					T::MinerControl::sub_power(miner_id, file_size.into())?;
					T::File::add_invalid_file(miner_id, file_id.clone())?;
					T::File::delete_filler(miner_id, file_id)?;
					T::MinerControl::punish_miner(miner_id, file_size)?;
				}
				2 => {
					T::MinerControl::sub_space(miner_id, file_size.into())?;
					T::File::add_invalid_file(miner_id, file_id.clone())?;
					T::File::add_recovery_file(file_id.clone())?;
					T::MinerControl::punish_miner(miner_id, file_size)?;
				}
				_ => {
					Err(Error::<T>::FileTypeError)?;
				}
			}

			Ok(())
		}
	
		fn vec_to_bounded(param: Vec<Vec<u8>>) -> Result<BoundedList<T>, DispatchError> {
			let mut result: BoundedList<T> = Vec::new().try_into().map_err(|_| Error::<T>::BoundedVecError)?;
	
			for v in param {
				let string: BoundedVec<u8, T::StringLimit> = v.try_into().map_err(|_| Error::<T>::BoundedVecError)?;
				result.try_push(string).map_err(|_| Error::<T>::BoundedVecError)?;
			}
	
			Ok(result)
		}
	}
}






