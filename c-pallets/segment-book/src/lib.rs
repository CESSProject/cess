//! # Segemnt Book Module
//!
//!  This file is the exclusive pallet of cess and the proof of podr2 adaptation
//!
//! ## OverView
//!
//!  The job of this segment Book pallet is to process the proof of miner's service file and filling
//! file,  and generate random challenges. Call some traits of Smith pallet to punish miners.
//!  Call the trail of file bank pallet to obtain random files or files with problems in handling
//! challenges.
//!
//! ### Terminology
//!
//! * **random_challenge:** The random time trigger initiates a challenge to the random documents.
//!   The miners need to complete the challenge within a limited time and submit the certificates of
//!   the corresponding documents.
//!
//! * **deadline:** 		Expiration time of challenge, stored in challengeduration
//! * **mu:**				Miner generated challenge related information
//! * **sigma:**			Miner generated challenge related information
//!
//! ### Interface
//!
//! ### Dispatchable Functions
//!
//! * `submit_challange_prove`   Miner submits challenge certificate.
//! * `verify_proof`             Consensus submission verification challenge proof results.
//!
//! ### Scenarios
//!
//! #### Punishment
//!
//!   When the verification result of the miner's certificate is false,
//!   or the miner fails to complete the challenge on time, the miner
//!   will be punished in both cases. Decide whether to reduce power
//!   or space according to the file type of punishment

#![cfg_attr(not(feature = "std"), no_std)]

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

pub use pallet::*;
mod benchmarking;
pub mod weights;
use sp_runtime::{
	traits::{CheckedAdd, SaturatedConversion},
	RuntimeDebug,
};
mod types;
use types::*;

use codec::{Decode, Encode};
use frame_support::{
	dispatch::DispatchResult,
	pallet_prelude::*,
	storage::bounded_vec::BoundedVec,
	traits::{FindAuthor, Randomness, ReservableCurrency},
	PalletId,
};
use frame_system::offchain::{AppCrypto, CreateSignedTransaction, SendSignedTransaction, Signer};
use pallet_file_bank::RandomFileList;
use pallet_file_map::ScheduleFind;
use pallet_sminer::MinerControl;
use scale_info::TypeInfo;
use sp_core::{crypto::KeyTypeId, H256};
use sp_std::{collections::btree_map::BTreeMap, prelude::*};
pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	// use frame_benchmarking::baseline::Config;
	use frame_support::traits::Get;
	use frame_system::{ensure_signed, pallet_prelude::*};

	pub type BoundedString<T> = BoundedVec<u8, <T as Config>::StringLimit>;
	pub type BoundedList<T> =
		BoundedVec<BoundedVec<u8, <T as Config>::StringLimit>, <T as Config>::StringLimit>;

	pub const LIMIT: u64 = 18446744073709551615;

	#[pallet::config]
	pub trait Config:
		frame_system::Config + sp_std::fmt::Debug + CreateSignedTransaction<Call<Self>>
	{
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
		type File: RandomFileList<Self::AccountId>;
		//Judge whether it is the trait of the consensus node
		type Scheduler: ScheduleFind<Self::AccountId>;
		//It is used to increase or decrease the miners' computing power, space, and execute
		// punishment
		type MinerControl: MinerControl;
		//Configuration to be used for offchain worker
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ChallengeProof { peer_id: u64, file_id: Vec<u8> },

		VerifyProof { peer_id: u64, file_id: Vec<u8> },

		OutstandingChallenges { peer_id: u64, file_id: Vec<u8> },
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
		//The user does not have permission to call this method
		NotQualified,
		//Error recording time
		RecordTimeError,

		OffchainSignedTxError,

		NoLocalAcctForSigning,

		LengthExceedsLimit,
	}

	//Information about storage challenges
	#[pallet::storage]
	#[pallet::getter(fn challenge_map)]
	pub type ChallengeMap<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u64,
		BoundedVec<ChallengeInfo<T>, T::StringLimit>,
		ValueQuery,
	>;

	//Relevant time nodes for storage challenges
	#[pallet::storage]
	#[pallet::getter(fn challenge_duration)]
	pub(super) type ChallengeDuration<T: Config> = StorageValue<_, BlockNumberOf<T>, ValueQuery>;

	//Store the certification information submitted by the miner and wait for the specified
	// scheduling verification
	#[pallet::storage]
	#[pallet::getter(fn unverify_proof)]
	pub(super) type UnVerifyProof<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountOf<T>,
		BoundedVec<ProveInfo<T>, T::StringLimit>,
		ValueQuery,
	>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberOf<T>> for Pallet<T> {
		//Used to calculate whether it is implied to submit spatiotemporal proof
		//Cycle every 7.2 hours
		//When there is an uncommitted space-time certificate, the corresponding miner will be
		// punished and the corresponding data segment will be removed
		fn on_initialize(now: BlockNumberOf<T>) -> Weight {
			let _number: u128 = now.saturated_into();
			let deadline = Self::challenge_duration();
			//The waiting time for the challenge has reached the deadline
			if now == deadline {
				//After the waiting time for the challenge reaches the deadline,
				//the miners who fail to complete the challenge will be punished
				for (miner_id, challenge_list) in <ChallengeMap<T>>::iter() {
					for v in challenge_list {
						if let Err(e) =
							Self::punish(miner_id, v.file_id.to_vec(), v.file_size, v.file_type)
						{
							log::info!("punish Err:{:?}", e);
						}
						log::info!(
							"challenge draw a blank, miner_id:{:?}, file_id: {:?}",
							miner_id.clone(),
							v.file_id.to_vec()
						);
						<ChallengeMap<T>>::remove(miner_id.clone());
						Self::deposit_event(Event::<T>::OutstandingChallenges {
							peer_id: miner_id,
							file_id: v.file_id.to_vec(),
						});
					}
				}
			}
			//The interval between challenges must be greater than one hour

			0
		}

		fn offchain_worker(now: T::BlockNumber) {
			let deadline = Self::challenge_duration();
			if now > deadline {
				//Determine whether to trigger a challenge
				if Self::trigger_challenge() {
					log::info!("offchain worker random challenge start");
					if let Err(e) = Self::generation_challenge() {
						log::info!("punish Err:{:?}", e);
					}
					log::info!("offchain worker random challenge end");
				}
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1000)]
		pub fn submit_challenge_prove(
			origin: OriginFor<T>,
			miner_id: u64,
			prove_info: Vec<ProveInfo<T>>,
		) -> DispatchResult {
			let _sender = ensure_signed(origin)?;
			let acc = Self::get_current_scheduler();
			if prove_info.len() > 100 {
				Err(Error::<T>::LengthExceedsLimit)?;
			}

			let challenge_list = Self::challenge_map(miner_id);
			let mut fileid_list: Vec<Vec<u8>> = Vec::new();
			for v in challenge_list.iter() {
				fileid_list.push(v.file_id.to_vec());
			}
			for prove in prove_info.iter() {
				if !fileid_list.contains(&prove.file_id) {
					Err(Error::<T>::NoChallenge)?;
				}
			}
			Self::storage_prove(acc, prove_info.clone())?;
			Self::clear_challenge_info(miner_id.clone(), prove_info.clone())?;

			Ok(())
		}

		#[pallet::weight(1000)]
		pub fn verify_proof(
			origin: OriginFor<T>,
			result_list: Vec<VerifyResult<T>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			if !T::Scheduler::contains_scheduler(sender.clone()) {
				Err(Error::<T>::ScheduleNonExistent)?;
			}
			if result_list.len() > 50 {
				Err(Error::<T>::LengthExceedsLimit)?;
			}

			let verify_list = Self::unverify_proof(&sender);
			//Clean up the corresponding data in the pool that is not verified by consensus,
			//and judge whether to punish according to the structure
			<UnVerifyProof<T>>::try_mutate(&sender, |o| -> DispatchResult {
				for result in result_list.iter() {
					for value in verify_list.iter() {
						if (value.miner_id == result.miner_id) &&
							(value.challenge_info.file_id == result.file_id)
						{
							o.retain(|x| (x.challenge_info.file_id != result.file_id.to_vec()));
							//If the result is false, a penalty will be imposed
							if !result.result {
								Self::punish(
									result.miner_id,
									result.file_id.clone().to_vec(),
									value.challenge_info.file_size,
									value.challenge_info.file_type,
								)?;
							}
							Self::deposit_event(Event::<T>::VerifyProof {
								peer_id: result.miner_id,
								file_id: result.file_id.to_vec(),
							});
							break
						}
					}
				}
				Ok(())
			})?;

			Ok(())
		}

		#[pallet::weight(1000)]
		pub fn save_challenge_info(
			origin: OriginFor<T>,
			miner_id: u64,
			challenge_info: Vec<ChallengeInfo<T>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			if !T::File::contains_member(sender) {
				Err(Error::<T>::NotQualified)?;
			}
			let mut convert: BoundedVec<ChallengeInfo<T>, T::StringLimit> = Default::default();
			for v in challenge_info {
				convert.try_push(v).map_err(|_e| Error::<T>::BoundedVecError)?;
			}
			ChallengeMap::<T>::insert(&miner_id, convert);

			Ok(())
		}

		#[pallet::weight(1000)]
		pub fn save_challenge_time(
			origin: OriginFor<T>,
			duration: BlockNumberOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			if !T::File::contains_member(sender) {
				Err(Error::<T>::NotQualified)?;
			}
			if let Err(e) = Self::record_challenge_time(duration) {
				log::info!("punish Err:{:?}", e);
				Err(Error::<T>::RecordTimeError)?;
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		//Storage proof method
		fn storage_prove(acc: AccountOf<T>, prove_list: Vec<ProveInfo<T>>) -> DispatchResult {
			<UnVerifyProof<T>>::try_mutate(&acc, |o| -> DispatchResult {
				for v in prove_list.iter() {
					o.try_push(v.clone()).map_err(|_e| Error::<T>::StorageLimitReached)?;
				}
				Ok(())
			})?;
			Ok(())
		}

		//Clean up the corresponding challenges in the miner's challenge pool
		fn clear_challenge_info(miner_id: u64, prove_list: Vec<ProveInfo<T>>) -> DispatchResult {
			<ChallengeMap<T>>::try_mutate(miner_id, |o| -> DispatchResult {
				for v in prove_list.iter() {
					o.retain(|x| x.file_id != *v.file_id);
					Self::deposit_event(Event::<T>::ChallengeProof {
						peer_id: v.miner_id,
						file_id: v.file_id.to_vec(),
					});
				}

				Ok(())
			})?;
			Ok(())
		}

		//Obtain the consensus of the current block
		fn get_current_scheduler() -> AccountOf<T> {
			let digest = <frame_system::Pallet<T>>::digest();
			let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
			let acc = T::FindAuthor::find_author(pre_runtime_digests).map(|a| a);
			T::Scheduler::get_controller_acc(acc.unwrap())
		}

		//Record challenge time
		fn record_challenge_time(duration: BlockNumberOf<T>) -> DispatchResult {
			let now = <frame_system::Pallet<T>>::block_number();
			<ChallengeDuration<T>>::try_mutate(|o| -> DispatchResult {
				*o = now.checked_add(&duration).ok_or(Error::<T>::Overflow)?;
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
				return true
			}
			false
		}

		//Ways to generate challenges
		fn generation_challenge() -> DispatchResult {
			let result = T::File::get_random_challenge_data()?;
			let mut x = 0;
			let mut new_challenge_map: BTreeMap<u64, Vec<ChallengeInfo<T>>> = BTreeMap::new();
			for (miner_id, file_id, block_list, file_size, file_type, segment_size) in result {
				x = x.checked_add(&1).ok_or(Error::<T>::Overflow)?;
				let random = Self::generate_random_number(
					x.checked_add(&20220510).ok_or(Error::<T>::Overflow)?,
					block_list.len() as u32,
				);
				//Create a single challenge message in files
				let challenge_info = ChallengeInfo::<T> {
					file_type,
					file_id: file_id.try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
					file_size,
					segment_size,
					block_list: Self::vec_to_bounded(block_list)?,
					random: Self::vec_to_bounded(random)?,
				};
				//Push Map

				if new_challenge_map.contains_key(&miner_id) {
					if let Some(e) = new_challenge_map.get_mut(&miner_id) {
						(*e).push(challenge_info);
					} else {
						log::error!("offchain worker read BTreeMap Err");
					}
				} else {
					let new_vec = vec![challenge_info];
					new_challenge_map.insert(miner_id, new_vec);
				}
			}
			Self::offchain_signed_tx(new_challenge_map)?;
			Ok(())
		}

		fn offchain_signed_tx(
			new_challenge_map: BTreeMap<u64, Vec<ChallengeInfo<T>>>,
		) -> Result<(), Error<T>> {
			let signer = Signer::<T, T::AuthorityId>::any_account();
			let mut max_len: u32 = 0;
			for (k, v) in new_challenge_map {
				if v.len() as u32 > max_len {
					max_len = v.len() as u32;
				}
				let result = signer.send_signed_transaction(|_account| Call::save_challenge_info {
					miner_id: k.clone(),
					challenge_info: v.clone(),
				});

				if let Some((acc, res)) = result {
					if res.is_err() {
						log::error!(
							"failure: offchain_signed_tx: tx sent: {:?} miner_id: {:?}",
							acc.id,
							k
						);
					}
					// Transaction is sent successfully
				}
			}

			let duration: BlockNumberOf<T> = max_len
				.checked_mul(3)
				.ok_or(Error::<T>::Overflow)?
				.checked_mul(120)
				.ok_or(Error::<T>::Overflow)?
				.checked_div(100)
				.ok_or(Error::<T>::Overflow)?
				.saturated_into();
			let result =
				signer.send_signed_transaction(|_account| Call::save_challenge_time { duration });

			if let Some((_acc, res)) = result {
				if res.is_err() {
					log::error!("failure: offchain_signed_tx: tx sent save_challenge_time");
				} else {
					return Ok(())
				}
				// Transaction is sent successfully
			}
			log::error!("No local account available");
			Err(<Error<T>>::NoLocalAcctForSigning)
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
					let (r_seed, _) =
						T::MyRandomness::random(&(T::MyPalletId::get(), seed).encode());
					let random_seed = <H256>::decode(&mut r_seed.as_ref())
						.expect("secure hashes should always be bigger than u32; qed");
					let random_vec = random_seed.as_bytes().to_vec();
					if random_vec.len() >= 20 {
						random_list.push(random_vec[0..19].to_vec());
						break
					}
				}
			}
			random_list
		}

		fn punish(
			miner_id: u64,
			file_id: Vec<u8>,
			file_size: u64,
			file_type: u8,
		) -> DispatchResult {
			if !T::MinerControl::miner_is_exist(miner_id) {
				return Ok(())
			}
			match file_type {
				1 => {
					T::MinerControl::sub_power(miner_id, file_size.into())?;
					T::File::add_invalid_file(miner_id, file_id.clone())?;
					T::File::delete_filler(miner_id, file_id)?;
					T::MinerControl::punish_miner(miner_id, file_size)?;
				},
				2 => {
					T::MinerControl::sub_space(miner_id, file_size.into())?;
					T::MinerControl::sub_power(miner_id, file_size.into())?;
					T::File::add_invalid_file(miner_id, file_id.clone())?;
					T::File::add_recovery_file(file_id.clone())?;
					T::MinerControl::punish_miner(miner_id, file_size)?;
				},
				_ => {
					Err(Error::<T>::FileTypeError)?;
				},
			}

			Ok(())
		}

		fn vec_to_bounded(param: Vec<Vec<u8>>) -> Result<BoundedList<T>, DispatchError> {
			let mut result: BoundedList<T> =
				Vec::new().try_into().map_err(|_| Error::<T>::BoundedVecError)?;

			for v in param {
				let string: BoundedVec<u8, T::StringLimit> =
					v.try_into().map_err(|_| Error::<T>::BoundedVecError)?;
				result.try_push(string).map_err(|_| Error::<T>::BoundedVecError)?;
			}

			Ok(result)
		}
	}
}
