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

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

use sp_runtime::{
	traits::{CheckedAdd, SaturatedConversion},
	RuntimeDebug, Permill,
	offchain::storage::{StorageValueRef, StorageRetrievalError},
};
mod types;
use types::*;

use codec::{Decode, Encode};
use frame_support::{
	transactional,
	dispatch::DispatchResult,
	pallet_prelude::*,
	storage::bounded_vec::BoundedVec,
	traits::{
		FindAuthor, Randomness, ReservableCurrency, EstimateNextSessionRotation,
		ValidatorSetWithIdentification, ValidatorSet, OneSessionHandler,
	},
	PalletId, WeakBoundedVec, BoundedSlice,
};
use sp_core::{
	crypto::KeyTypeId,
	offchain::OpaqueNetworkState,
};
use sp_runtime::app_crypto::RuntimeAppPublic;
use frame_system::offchain::{CreateSignedTransaction, SubmitTransaction};
use pallet_file_bank::RandomFileList;
use pallet_file_map::ScheduleFind;
use pallet_sminer::MinerControl;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};
pub mod weights;
pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
pub const SEGMENT_BOOK: KeyTypeId = KeyTypeId(*b"cess");
// type FailureRate = u32;

pub mod sr25519 {
	mod app_sr25519 {
		use crate::*;
		use sp_runtime::app_crypto::{app_crypto, sr25519};
		app_crypto!(sr25519, SEGMENT_BOOK);
	}

	sp_runtime::app_crypto::with_pair! {
		/// An i'm online keypair using sr25519 as its crypto.
		pub type AuthorityPair = app_sr25519::Pair;
	}

	/// An i'm online signature using sr25519 as its crypto.
	pub type AuthoritySignature = app_sr25519::Signature;

	/// An i'm online identifier using sr25519 as its crypto.
	pub type AuthorityId = app_sr25519::Public;
}
enum OffchainErr {
	UnexpectedError,
	Ineligible,
	GenerateInfoError,
	NetworkState,
	FailedSigning,
	Overflow,
	Working,
}

impl sp_std::fmt::Debug for OffchainErr {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			OffchainErr::UnexpectedError => write!(fmt, "Should not appear, Unexpected error."),
			OffchainErr::Ineligible => write!(fmt, "The current node does not have the qualification to execute offline working machines"),
			OffchainErr::GenerateInfoError => write!(fmt, "Failed to generate random file meta information"),
			OffchainErr::NetworkState => write!(fmt, "Failed to obtain the network status of the offline working machine"),
			OffchainErr::FailedSigning => write!(fmt, "Signing summary information failed"),
			OffchainErr::Overflow => write!(fmt, "Calculation data, boundary overflow"),
			OffchainErr::Working => write!(fmt, "The offline working machine is currently executing work"),
		}
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	// use frame_benchmarking::baseline::Config;
	use frame_support::{traits::Get};
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
		type MinerControl: MinerControl<Self::AccountId>;
		//Configuration to be used for offchain worker
		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;
		//Verifier of this round
		type ValidatorSet: ValidatorSetWithIdentification<Self::AccountId>;
		//Information for the next session
		type NextSessionRotation: EstimateNextSessionRotation<Self::BlockNumber>;
		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;
		#[pallet::constant]
		type LockTime: Get<BlockNumberOf<Self>>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ChallengeProof { miner: AccountOf<T>, file_id: Vec<u8> },

		VerifyProof { miner: AccountOf<T>, file_id: Vec<u8> },

		OutstandingChallenges { miner: AccountOf<T>, file_id: Vec<u8> },
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

		Locked,
	}

	//Information about storage challenges
	#[pallet::storage]
	#[pallet::getter(fn challenge_map)]
	pub type ChallengeMap<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountOf<T>,
		BoundedVec<ChallengeInfo<T>, T::StringLimit>,
		ValueQuery,
	>;

	//Relevant time nodes for storage challenges
	#[pallet::storage]
	#[pallet::getter(fn challenge_duration)]
	pub(super) type ChallengeDuration<T: Config> = StorageValue<_, BlockNumberOf<T>, ValueQuery>;

	//Relevant time nodes for storage challenges
	#[pallet::storage]
	#[pallet::getter(fn verify_duration)]
	pub(super) type VerifyDuration<T: Config> = StorageValue<_, BlockNumberOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn cur_authority_index)]
	pub(super) type CurAuthorityIndex<T: Config> = StorageValue<_, u16, ValueQuery>;

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

	#[pallet::storage]
	#[pallet::getter(fn keys)]
	pub(super) type Keys<T: Config> = StorageValue<_, WeakBoundedVec<T::AuthorityId, T::StringLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn lock)]
	pub(super) type Lock<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn failure_rate_map)]
	pub(super) type FailureNumMap<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn miner_total_proof)]
	pub(super) type MinerTotalProof<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn consecutive_fines)]
	pub(super) type ConsecutiveFines<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, u8, ValueQuery>;

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
			let challenge_deadline = Self::challenge_duration();
			let verify_deadline = Self::verify_duration();
			//The waiting time for the challenge has reached the deadline
			if now == challenge_deadline {
				//After the waiting time for the challenge reaches the deadline,
				//the miners who fail to complete the challenge will be punished
				for (acc, challenge_list) in <ChallengeMap<T>>::iter() {
					for v in challenge_list {
						let _ = Self::set_failure(acc.clone());
						if let Err(e) = Self::update_miner_file(
							acc.clone(),
							v.file_id.to_vec(),
							v.file_size,
							v.file_type,
						) {
							log::info!("punish Err:{:?}", e);
						}
						log::info!(
							"challenge draw a blank, miner_acc:{:?}, file_id: {:?}",
							acc.clone(),
							v.file_id.to_vec()
						);
						Self::deposit_event(Event::<T>::OutstandingChallenges {
							miner: acc.clone(),
							file_id: v.file_id.to_vec(),
						});
					}
					<ChallengeMap<T>>::remove(acc.clone());
				}
			}

			//Punish the scheduler who fails to verify the results for a long time
			if now == verify_deadline {
				let mut is_end = true;

				let mut verify_list: Vec<ProveInfo<T>> = Vec::new();
				for (acc, v_list) in <UnVerifyProof<T>>::iter() {
					if v_list.len() > 0 {
						is_end = false;
						verify_list.append(&mut v_list.to_vec());
						T::Scheduler::punish_scheduler(acc.clone());
						<UnVerifyProof<T>>::remove(&acc);
					}
				}
				if is_end {
					for (miner, total_proof) in <MinerTotalProof<T>>::iter() {
						if <FailureNumMap<T>>::contains_key(&miner) {
							if <ConsecutiveFines<T>>::contains_key(&miner) {
								<ConsecutiveFines<T>>::try_mutate(
									miner.clone(),
									|s_opt| -> DispatchResult {
										s_opt.checked_add(1).ok_or(Error::<T>::Overflow)?;
										Ok(())
									},
								).unwrap_or(());
							} else {
								<ConsecutiveFines<T>>::insert(&miner, 1);
							}
							Self::punish(
								miner.clone(),
								<FailureNumMap<T>>::get(&miner),
								total_proof,
								<ConsecutiveFines<T>>::get(&miner),
							).unwrap_or(());
						} else {
							<ConsecutiveFines<T>>::remove(&miner);
						}
					}

					Self::start_buffer_period_schedule().unwrap_or(());
					<FailureNumMap<T>>::remove_all(None);
					<MinerTotalProof<T>>::remove_all(None);
				} else {
					let result = Self::get_current_scheduler();
					match result {
						Ok(cur_acc) =>  {
							let new_deadline = match now.checked_add(&1200u32.saturated_into()).ok_or(Error::<T>::Overflow) {
								Ok(new_deadline) => new_deadline,
								Err(e) => {
									log::error!("over flow: {:?}", e);
									return 0
								},
							};
							<VerifyDuration<T>>::put(new_deadline);
							let _ = Self::storage_prove(cur_acc, verify_list);
						},
						Err(_e) => log::error!("get_current_scheduler err"),
					}
				}
			}
			0
		}

		fn offchain_worker(now: T::BlockNumber) {
			let deadline = Self::verify_duration();
			if sp_io::offchain::is_validator() {
				if now > deadline {
					//Determine whether to trigger a challenge
					if Self::trigger_challenge(now) {
						log::info!("offchain worker random challenge start");
						if let Err(e) = Self::offchain_work_start(now) {
							match e {
								OffchainErr::Working => log::info!("offchain working, Unable to perform a new round of work."),
								_ => log::info!("offchain worker generation challenge failed:{:?}", e),
							};
						}
						log::info!("offchain worker random challenge end");
					}
				}
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::submit_challenge_prove(prove_info.len() as u32))]
		pub fn submit_challenge_prove(
			origin: OriginFor<T>,
			prove_info: Vec<ProveInfo<T>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let acc = Self::get_current_scheduler()?;
			if prove_info.len() > 100 {
				Err(Error::<T>::LengthExceedsLimit)?;
			}

			let challenge_list = Self::challenge_map(sender.clone());
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
			Self::clear_challenge_info(sender.clone(), prove_info.clone())?;
			Ok(())
		}

		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::verify_proof(result_list.len() as u32))]
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
						if (value.miner_acc == result.miner_acc.clone())
							&& (value.challenge_info.file_id == result.file_id)
						{
							o.retain(|x| (x.challenge_info.file_id != result.file_id.to_vec()));
							//If the result is false, a penalty will be imposed
							if !result.result {
								Self::set_failure(result.miner_acc.clone()).unwrap_or(());
								Self::update_miner_file(
									result.miner_acc.clone(),
									result.file_id.clone().to_vec(),
									value.challenge_info.file_size,
									value.challenge_info.file_type,
								)?;
							}
							Self::deposit_event(Event::<T>::VerifyProof {
								miner: result.miner_acc.clone(),
								file_id: result.file_id.to_vec(),
							});
							break;
						}
					}
				}
				Ok(())
			})?;
			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
		pub fn save_challenge_info(
			origin: OriginFor<T>,
			_seg_digest: SegDigest<BlockNumberOf<T>>,
			_signature: <T::AuthorityId as RuntimeAppPublic>::Signature,
			miner_acc: AccountOf<T>,
			challenge_info: Vec<ChallengeInfo<T>>,
		) -> DispatchResult {
			ensure_none(origin)?;
			// let now = <frame_system::Pallet<T>>::block_number();
			let mut convert: BoundedVec<ChallengeInfo<T>, T::StringLimit> = Default::default();
			for v in challenge_info {
				convert.try_push(v).map_err(|_e| Error::<T>::BoundedVecError)?;
			}
			ChallengeMap::<T>::insert(&miner_acc, convert);
			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
		pub fn save_challenge_time(
			origin: OriginFor<T>,
			duration: BlockNumberOf<T>,
			_seg_digest: SegDigest<BlockNumberOf<T>>,
			_signature: <T::AuthorityId as RuntimeAppPublic>::Signature,
		) -> DispatchResult {
			ensure_none(origin)?;
			if let Err(e) = Self::record_challenge_time(duration.clone()) {
				log::info!("save challenge time Err:{:?}", e);
				Err(Error::<T>::RecordTimeError)?;
			}

			for (acc, challenge_list) in <ChallengeMap<T>>::iter() {
				<MinerTotalProof<T>>::insert(acc, challenge_list.len() as u32);
			}

			let max = Keys::<T>::get().len() as u16;
			let mut index = CurAuthorityIndex::<T>::get();
			if index >= max - 1 {
				index = 0;
			} else {
				index = index + 1;
			}
			CurAuthorityIndex::<T>::put(index);

			Ok(())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::save_challenge_info {
				seg_digest,
				signature,
				miner_acc: _,
				challenge_info: _,
			} = call {
				Self::check_unsign(seg_digest, signature)
			} else if let Call::save_challenge_time {
				duration: _,
				seg_digest,
				signature,
			} = call {
				Self::check_unsign(seg_digest, signature)
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn check_unsign(
			seg_digest: &SegDigest<BlockNumberOf<T>>,
			signature: &<T::AuthorityId as RuntimeAppPublic>::Signature,
		) -> TransactionValidity {
			let current_session = T::ValidatorSet::session_index();
			let keys = Keys::<T>::get();

				let index = CurAuthorityIndex::<T>::get();
				if index != seg_digest.validators_index {
					log::error!("invalid index");
					return InvalidTransaction::Stale.into();
				}
				//TODO!
				// Convert validatorsId => T::AuthorityId
				let authority_id = match keys.get(seg_digest.validators_index as usize) {
					Some(authority_id) => authority_id,
					None => return InvalidTransaction::Stale.into(),
				};

				let signature_valid = seg_digest.using_encoded(|encoded_seg_digest| {
					authority_id.verify(&encoded_seg_digest, &signature)
				});

				if !signature_valid {
					log::error!("bad signature.");
					return InvalidTransaction::BadProof.into()
				}

				log::info!("build valid transaction");
				ValidTransaction::with_tag_prefix("SegmentBook")
					.priority(T::UnsignedPriority::get())
					.and_provides((current_session, authority_id, signature))
					.longevity(
						TryInto::<u64>::try_into(
							T::NextSessionRotation::average_session_length() / 2u32.into(),
						)
						.unwrap_or(64_u64),
					)
					.propagate(true)
					.build()
		}

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

		fn set_failure(acc: AccountOf<T>) -> DispatchResult {
			if <FailureNumMap<T>>::contains_key(&acc) {
				<FailureNumMap<T>>::try_mutate(acc.clone(), |s_opt| -> DispatchResult {
					s_opt.checked_add(1).ok_or(Error::<T>::Overflow)?;
					Ok(())
				})?;
			} else {
				<FailureNumMap<T>>::insert(&acc, 1);
			}
			Ok(())
		}

		//Clean up the corresponding challenges in the miner's challenge pool
		fn clear_challenge_info(
			miner_acc: AccountOf<T>,
			prove_list: Vec<ProveInfo<T>>,
		) -> DispatchResult {
			<ChallengeMap<T>>::try_mutate(&miner_acc, |o| -> DispatchResult {
				for v in prove_list.iter() {
					o.retain(|x| x.file_id != *v.file_id);
					Self::deposit_event(Event::<T>::ChallengeProof {
						miner: v.miner_acc.clone(),
						file_id: v.file_id.to_vec(),
					});
				}

				Ok(())
			})?;
			Ok(())
		}

		//Obtain the consensus of the current block
		fn get_current_scheduler() -> Result<AccountOf<T>, DispatchError> {
			let digest = <frame_system::Pallet<T>>::digest();
			let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
			let acc = T::FindAuthor::find_author(pre_runtime_digests).map(|a| a);
			let acc = match acc {
				Some(acc) => T::Scheduler::get_controller_acc(acc),
				None => T::Scheduler::get_first_controller()?,
			};
			Ok(acc)
		}

		//Record challenge time
		fn record_challenge_time(duration: BlockNumberOf<T>) -> DispatchResult {
			let now = <frame_system::Pallet<T>>::block_number();
			let verify_deadline = now
				.checked_add(&duration)
				.ok_or(Error::<T>::Overflow)?
				.checked_add(&2000u32.saturated_into())
				.ok_or(Error::<T>::Overflow)?;
			<VerifyDuration<T>>::try_mutate(|o| -> DispatchResult {
				*o = verify_deadline;
				Ok(())
			})?;
			<ChallengeDuration<T>>::try_mutate(|o| -> DispatchResult {
				*o = now.checked_add(&duration).ok_or(Error::<T>::Overflow)?;
				Ok(())
			})?;
			Ok(())
		}

		//Trigger: whether to trigger the challenge
		fn trigger_challenge(now: BlockNumberOf<T>) -> bool {
			const START_FINAL_PERIOD: Permill = Permill::from_percent(80);

			let time_point = Self::random_time_number(20220509);
			//The chance to trigger a challenge is once a day
			let probability: u32 = T::OneDay::get().saturated_into();
			let range = LIMIT / probability as u64;
			if (time_point > 2190502) && (time_point < (range + 2190502)) {
				if let (Some(progress), _) =
				T::NextSessionRotation::estimate_current_session_progress(now) {
					if progress >= START_FINAL_PERIOD {
						log::error!("TooLate!");
						return false;
					}
				}
				return true;
			}
			false
		}

		//Ways to generate challenges
		fn generation_challenge() -> Result<BTreeMap<AccountOf<T>, Vec<ChallengeInfo<T>>>, DispatchError> {
			let result = T::File::get_random_challenge_data()?;
			let mut x = 0;
			let mut new_challenge_map: BTreeMap<AccountOf<T>, Vec<ChallengeInfo<T>>> =
				BTreeMap::new();
			for (miner_acc, file_id, block_list, file_size, file_type) in result {
				x = x.checked_add(&1).ok_or(Error::<T>::Overflow)?;
				let random = Self::generate_random_number(
					x.checked_add(&20220510).ok_or(Error::<T>::Overflow)?,
					block_list.len() as u32,
				);
				//Create a single challenge message in files
				let challenge_info = ChallengeInfo::<T> {
					file_size,
					file_type,
					file_id: file_id.try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
					block_list: block_list.try_into().map_err(|_e| Error::<T>::BoundedVecError)?,
					random: Self::vec_to_bounded(random)?,
				};
				//Push Map

				if new_challenge_map.contains_key(&miner_acc) {
					if let Some(e) = new_challenge_map.get_mut(&miner_acc) {
						(*e).push(challenge_info);
					} else {
						log::error!("offchain worker read BTreeMap Err");
					}
				} else {
					let new_vec = vec![challenge_info];
					new_challenge_map.insert(miner_acc, new_vec);
				}
			}

			Ok(new_challenge_map)
		}

		fn offchain_work_start(now: BlockNumberOf<T>) -> Result<(), OffchainErr> {
			let (authority_id, validators_index, validators_len) = Self::get_authority()?;

			if !Self::check_working(&now, &authority_id) {
				return Err(OffchainErr::Working);
			}

			let challenge_map = Self::generation_challenge().map_err(|e| {
				log::error!("generation challenge error:{:?}", e);
				OffchainErr::GenerateInfoError
			})?;

			Self::offchain_call_extrinsic(now, authority_id, challenge_map, validators_index, validators_len)?;

			Ok(())
		}

		fn check_working(now: &BlockNumberOf<T>, authority_id: &T::AuthorityId) -> bool {
			let key = &authority_id.encode();
			let storage = StorageValueRef::persistent(key);

			let res = storage.mutate(|status: Result<Option<BlockNumberOf<T>>, StorageRetrievalError>| {
				match status {
					// we are still waiting for inclusion.
					Ok(Some(last_block)) => {
						let lock_time = T::LockTime::get();
						if last_block + lock_time > *now {
							log::info!("last_block: {:?}, lock_time: {:?}, now: {:?}", last_block, lock_time, now);
							Err(OffchainErr::Working)
						} else {
							Ok(*now)
						}
					},
					// attempt to set new status
					_ => Ok(*now),
				}
			});

			if res.is_err() {
				log::error!("offchain work: {:?}", OffchainErr::Working);
				return false
			}

			true
		}

		fn get_authority() -> Result<(T::AuthorityId, u16, usize), OffchainErr> {
			let cur_index = <CurAuthorityIndex<T>>::get();
			let validators = Keys::<T>::get();
			//this round key to submit transationss
			let epicycle_key = match validators.get(cur_index as usize) {
				Some(id) => id,
				None => return Err(OffchainErr::UnexpectedError),
			};

			let mut local_keys = T::AuthorityId::all();

			if local_keys.len() == 0 {
				log::info!("no local_keys");
				return Err(OffchainErr::Ineligible);
			}

			local_keys.sort();

			let res = local_keys.binary_search(&epicycle_key);

			let authority_id = match res {
				Ok(index) => local_keys.get(index),
				Err(_e) => return Err(OffchainErr::Ineligible),
			};

			let authority_id = match authority_id {
				Some(id) => id,
				None => return Err(OffchainErr::Ineligible),
			};

			Ok((authority_id.clone(), cur_index, validators.len()))
		}

		fn offchain_call_extrinsic(
			now: BlockNumberOf<T>,
			authority_id: T::AuthorityId,
			challenge_map: BTreeMap<AccountOf<T>, Vec<ChallengeInfo<T>>>,
			validators_index: u16,
			validators_len: usize,
		) -> Result<(), OffchainErr> {
			let mut max_len: u32 = 0;
			for (key, value) in challenge_map {
				if value.len() as u32 > max_len {
					max_len = value.len() as u32;
				}
				let (signature, digest) = Self::offchain_sign_digest(now, &authority_id, validators_index, validators_len)?;
				let call = Call::save_challenge_info {
					seg_digest: digest.clone(),
					signature: signature.clone(),
					miner_acc: key.clone(),
					challenge_info: value.clone(),
				};

				let result = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());

				if result.is_err() {
					log::error!(
						"failure: offchain_signed_tx: tx miner_id: {:?}",
						key
					);
				}
			}

			let (signature, digest) = Self::offchain_sign_digest(now, &authority_id, validators_index, validators_len)?;
			let one_hour: u32 = T::OneHours::get().saturated_into();
			let duration: BlockNumberOf<T> = max_len
				.checked_mul(3)
				.ok_or(OffchainErr::Overflow)?
				.checked_mul(120)
				.ok_or(OffchainErr::Overflow)?
				.checked_div(100)
				.ok_or(OffchainErr::Overflow)?
				.checked_add(one_hour)
				.ok_or(OffchainErr::Overflow)?
				.saturated_into();

			let call = Call::save_challenge_time {
				duration,
				seg_digest: digest.clone(),
				signature: signature,
			};

			let result = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction( call.into());

			if result.is_err() {
				log::error!(
					"failure: offchain_signed_tx: save challenge time, digest is: {:?}",
					digest
				);
			}

			Ok(())
		}

		fn offchain_sign_digest(
			now: BlockNumberOf<T>,
			authority_id: &T::AuthorityId,
			validators_index: u16,
			validators_len: usize
		) -> Result< (<<T as pallet::Config>::AuthorityId as sp_runtime::RuntimeAppPublic>::Signature, SegDigest::<BlockNumberOf<T>>), OffchainErr> {

			let network_state =
				sp_io::offchain::network_state().map_err(|_| OffchainErr::NetworkState)?;

			let digest = SegDigest::<BlockNumberOf<T>>{
				validators_len: validators_len as u32,
				block_num: now,
				validators_index,
				network_state,
			};

			let signature = authority_id.sign(&digest.encode()).ok_or(OffchainErr::FailedSigning)?;

			Ok((signature, digest))
		}

		pub fn initialize_keys(keys: &[T::AuthorityId]) {
			if !keys.is_empty() {
				assert!(Keys::<T>::get().is_empty(), "Keys are already initialized!");
				let bounded_keys = <BoundedSlice<'_, _, T::StringLimit>>::try_from(keys)
					.expect("More than the maximum number of keys provided");
				Keys::<T>::put(bounded_keys);
			}
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
						break;
					}
				}
			}
			random_list
		}

		fn update_miner_file(
			acc: AccountOf<T>,
			file_id: Vec<u8>,
			file_size: u64,
			file_type: u8,
		) -> DispatchResult {
			if !T::MinerControl::miner_is_exist(acc.clone()) {
				return Ok(());
			}
			match file_type {
				1 => {
					T::MinerControl::sub_power(acc.clone(), file_size.into())?;
					T::File::delete_filler(acc.clone(), file_id.clone())?;
					T::File::add_invalid_file(acc.clone(), file_id.clone())?;
				},
				2 => {
					T::MinerControl::sub_space(&acc, file_size.into())?;
					T::File::add_recovery_file(file_id.clone())?;
					T::File::add_invalid_file(acc.clone(), file_id.clone())?;
				},
				_ => {
					Err(Error::<T>::FileTypeError)?;
				},
			}
			if !T::MinerControl::miner_is_exist(acc.clone()) {
				T::File::delete_miner_all_filler(acc.clone())?;
			}
			Ok(())
		}

		fn punish(
			acc: AccountOf<T>,
			failure_num: u32,
			total_proof: u32,
			consecutive_fines: u8,
		) -> DispatchResult {
			T::MinerControl::punish_miner(
				acc.clone(),
				failure_num,
				total_proof,
				consecutive_fines,
			)?;
			Ok(())
		}

		fn start_buffer_period_schedule() -> DispatchResult {
			T::MinerControl::start_buffer_period_schedule()?;
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

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::AuthorityId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::AuthorityId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::AuthorityId)>,
	{
		let keys = validators.map(|x| x.1).collect::<Vec<_>>();
		Self::initialize_keys(&keys);
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, _queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::AuthorityId)>,
	{
		// Tell the offchain worker to start making the next session's heartbeats.
		// Since we consider producing blocks as being online,
		// the heartbeat is deferred a bit to prevent spamming.

		// Remember who the authorities are for the new session.
		let keys = validators.map(|x| x.1).collect::<Vec<_>>();
		let bounded_keys = WeakBoundedVec::<_, T::StringLimit>::force_from(
			keys,
			Some(
				"Warning: The session has more keys than expected. \
  				A runtime configuration adjustment may be needed.",
			),
		);
		Keys::<T>::put(bounded_keys);
	}

	fn on_before_session_ending() {
		// ignore
	}

	fn on_disabled(_i: u32) {
		// ignore
	}
}
