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

mod types;
use types::*;

mod constants;
use constants::*;

// pub mod migrations;

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

use sp_runtime::{
	traits::{CheckedAdd, SaturatedConversion},
	RuntimeDebug, Permill,
	offchain::storage::{StorageValueRef, StorageRetrievalError},
};


use codec::{Decode, Encode};
use frame_support::{
	transactional,
	dispatch::DispatchResult,
	pallet_prelude::*,
	storage::bounded_vec::BoundedVec,
	traits::{
		FindAuthor, Randomness, ReservableCurrency, EstimateNextSessionRotation,
		ValidatorSetWithIdentification, ValidatorSet, OneSessionHandler, StorageVersion,
	},
	PalletId, WeakBoundedVec, BoundedSlice,
};
use sp_core::{
	crypto::KeyTypeId,
	offchain::OpaqueNetworkState,
};
use sp_runtime::{Saturating, app_crypto::RuntimeAppPublic};
use frame_system::offchain::{CreateSignedTransaction, SubmitTransaction};
use pallet_file_bank::RandomFileList;
use pallet_tee_worker::ScheduleFind;
use pallet_sminer::MinerControl;
use pallet_storage_handler::StorageHandle;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::{ 
		convert:: { TryFrom, TryInto },
		collections::btree_map::BTreeMap, 
		prelude::*
	};
use cp_cess_common::*;
pub mod weights;
pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

pub const AUDIT: KeyTypeId = KeyTypeId(*b"cess");
// type FailureRate = u32;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

pub mod sr25519 {
	mod app_sr25519 {
		use crate::*;
		use sp_runtime::app_crypto::{app_crypto, sr25519};
		app_crypto!(sr25519, AUDIT);
	}

	sp_runtime::app_crypto::with_pair! {
		pub type AuthorityPair = app_sr25519::Pair;
	}

	pub type AuthoritySignature = app_sr25519::Signature;

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
	SubmitTransactionFailed,
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
			OffchainErr::SubmitTransactionFailed => write!(fmt, "Failed to submit transaction."),
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
	///18446744073709551615
	pub const LIMIT: u64 = u64::MAX;

	#[pallet::config]
	pub trait Config:
		frame_system::Config + sp_std::fmt::Debug + CreateSignedTransaction<Call<Self>>
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
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
		type ChallengeMinerMax: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type VerifyMissionMax: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type SigmaMax: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type SubmitValidationLimit: Get<u32> + Clone + Eq + PartialEq;
		//one day block
		#[pallet::constant]
		type OneDay: Get<BlockNumberOf<Self>>;
		//one hours block
		#[pallet::constant]
		type OneHours: Get<BlockNumberOf<Self>>;
		// randomness for seeds.
		type MyRandomness: Randomness<Option<Self::Hash>, Self::BlockNumber>;
		//Find the consensus of the current block
		type FindAuthor: FindAuthor<Self::AccountId>;
		//Random files used to obtain this batch of challenges
		type File: RandomFileList<Self::AccountId>;
		//Judge whether it is the trait of the consensus node
		type Scheduler: ScheduleFind<Self::AccountId>;
		//It is used to increase or decrease the miners' computing power, space, and execute
		// punishment
		type MinerControl: MinerControl<Self::AccountId>;

		type StorageHandle: StorageHandle<Self::AccountId>;
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
		GenerateChallenge,

		SubmitProof { miner: AccountOf<T> },

		VerifyProof { tee_worker: AccountOf<T>, miner: AccountOf<T> },

		OutstandingChallenges { miner: AccountOf<T>, file_id: Hash },
	}

	/// Error for the audit pallet.
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

		SystemError,

		NonExistentMission,

		UnexpectedError,
	}



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

	#[pallet::storage]
	#[pallet::getter(fn keys)]
	pub(super) type Keys<T: Config> = StorageValue<_, WeakBoundedVec<T::AuthorityId, T::StringLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn challenge_proposal)]
	pub(super) type ChallengeProposal<T: Config> = CountedStorageMap<_, Blake2_128Concat, [u8; 32], (u32, ChallengeInfo<T>)>;

	#[pallet::storage]
	#[pallet::getter(fn challenge_snap_shot)]
	pub(super) type ChallengeSnapShot<T: Config> = StorageValue<_, ChallengeInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn unverify_proof)]
	pub(super) type UnverifyProof<T: Config> = StorageMap<_, Blake2_128Concat, AccountOf<T>, BoundedVec<ProveInfo<T>, T::VerifyMissionMax>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn counted_idle_failed)]
	pub(super) type CountedIdleFailed<T: Config> = StorageMap<_, Blake2_128Concat, AccountOf<T>, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn counted_service_failed)]
	pub(super) type CountedServiceFailed<T: Config> = StorageMap<_, Blake2_128Concat, AccountOf<T>, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn counted_clear)]
	pub(super) type CountedClear<T: Config> = StorageMap<_, Blake2_128Concat, AccountOf<T>, u8, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn lock)]
	pub(super) type Lock<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn test_option)]
	pub(super) type TestOption<T: Config> = 
		StorageValue<_, Option<T::AccountId>>;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberOf<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberOf<T>) -> Weight {
			let weight: Weight = Weight::from_ref_time(0);
			weight
				.saturating_add(Self::clear_challenge(now))
				.saturating_add(Self::clear_verify_mission(now))
		}

		fn offchain_worker(now: T::BlockNumber) {
			let deadline = Self::verify_duration();
			if sp_io::offchain::is_validator() {
				if now > deadline {
					//Determine whether to trigger a challenge
					// if Self::trigger_challenge(now) {
						log::info!("offchain worker random challenge start");
						if let Err(e) = Self::offchain_work_start(now) {
							match e {
								OffchainErr::Working => log::info!("offchain working, Unable to perform a new round of work."),
								_ => log::info!("offchain worker generation challenge failed:{:?}", e),
							};
						}
						log::info!("offchain worker random challenge end");
					// }
				}
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[transactional]
		#[pallet::weight(0)]
		pub fn save_challenge_info(
			origin: OriginFor<T>,
			challenge_info: ChallengeInfo<T>,
			_key: T::AuthorityId,
			_seg_digest: SegDigest<BlockNumberOf<T>>,
			_signature: <T::AuthorityId as RuntimeAppPublic>::Signature,
		) -> DispatchResult {
			ensure_none(origin)?;

			let encode_info: Vec<u8> = challenge_info.encode();

			let hash = sp_io::hashing::sha2_256(&encode_info);

			let count: u32 = Keys::<T>::get().len() as u32;
			let limit = count
				.checked_mul(2).ok_or(Error::<T>::Overflow)?
				.checked_div(3).ok_or(Error::<T>::Overflow)?;

			if ChallengeProposal::<T>::contains_key(&hash) {
				let proposal = ChallengeProposal::<T>::get(&hash).unwrap();
				if proposal.0 + 1 >= limit {
					let cur_blcok = <ChallengeDuration<T>>::get();
					let now = <frame_system::Pallet<T>>::block_number();
					if now > cur_blcok {
						let duration = now.checked_add(&proposal.1.net_snap_shot.life).ok_or(Error::<T>::Overflow)?;
						<ChallengeSnapShot<T>>::put(proposal.1);
						<ChallengeDuration<T>>::put(duration);
						let _ = ChallengeProposal::<T>::clear(ChallengeProposal::<T>::count(), None);
					}

					Self::deposit_event(Event::<T>::GenerateChallenge);
				}
			} else {
				if ChallengeProposal::<T>::count() > count {
					// Proposal Generally Less
					let _ = ChallengeProposal::<T>::clear(ChallengeProposal::<T>::count(), None);
				} else {
					ChallengeProposal::<T>::insert(
						&hash,
						(1, challenge_info),
					);
				}
			}

			Ok(())
		}

		#[pallet::call_index(1)]
		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn submit_prove(
			origin: OriginFor<T>,
			idle_prove: BoundedVec<u8, T::SigmaMax>,
			service_prove: BoundedVec<u8, T::SigmaMax>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let challenge_info = <ChallengeSnapShot<T>>::get().ok_or(Error::<T>::NoChallenge)?;

			let miner_snapshot = <ChallengeSnapShot<T>>::try_mutate(|challenge_opt| -> Result<MinerSnapShot<AccountOf<T>>, DispatchError> {
				let challenge_info = challenge_opt.as_mut().ok_or(Error::<T>::NoChallenge)?;

				for (index, miner_snapshot) in challenge_info.miner_snapshot_list.clone().iter().enumerate() {
					if miner_snapshot.miner == sender {
						challenge_info.miner_snapshot_list.remove(index);
						return Ok(miner_snapshot.clone());
					}
				}

				Err(Error::<T>::NoChallenge)?
			})?;

			let tee_list = T::Scheduler::get_controller_list();
			ensure!(tee_list.len() > 0, Error::<T>::SystemError);

			let seed: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();
			let index = Self::random_number(seed) as u32;
			let index: u32 = index % (tee_list.len() as u32);
			let tee_acc = &tee_list[index as usize];

			let prove_info = ProveInfo::<T> {
				snap_shot: miner_snapshot,
				idle_prove,
				service_prove,
			};

			<CountedClear<T>>::insert(&sender, u8::MIN);

			UnverifyProof::<T>::mutate(tee_acc, |unverify_list| -> DispatchResult {
				unverify_list.try_push(prove_info).map_err(|_| Error::<T>::Overflow)?;

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::SubmitProof { miner: sender });

			Ok(())
		}

		#[pallet::call_index(2)]
		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn submit_verify_result(
			origin: OriginFor<T>,
			miner: AccountOf<T>,
			idle_result: bool,
			service_result: bool,
			tee_signature: NodeSignature,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
	
			// TODO! Podr2Key verify
			UnverifyProof::<T>::mutate(&sender, |unverify_list| -> DispatchResult {
				let last_count = unverify_list.len();

				for (index, miner_info) in unverify_list.iter().enumerate() {
					if miner_info.snap_shot.miner == miner {
						let snap_shot = <ChallengeSnapShot<T>>::try_get().map_err(|_| Error::<T>::UnexpectedError)?;

						if idle_result && service_result {
							T::MinerControl::calculate_miner_reward(
								&miner,
								snap_shot.net_snap_shot.total_reward,
								snap_shot.net_snap_shot.total_idle_space,
								snap_shot.net_snap_shot.total_service_space,
								miner_info.snap_shot.idle_space,
								miner_info.snap_shot.service_space,
							)?;
						}

						if idle_result {
							<CountedIdleFailed<T>>::insert(&miner, u32::MIN);
						} else {
							let count = <CountedIdleFailed<T>>::get(&miner) + 1;
							if count >= IDLE_FAULT_TOLERANT as u32 {
								T::MinerControl::idle_punish(&miner, miner_info.snap_shot.idle_space, miner_info.snap_shot.service_space)?;
							}
							<CountedIdleFailed<T>>::insert(&miner, count);
						}

						if service_result {
							<CountedServiceFailed<T>>::insert(&miner, u32::MIN);
						} else {
							let count = <CountedServiceFailed<T>>::get(&miner) + 1;
							if count >= SERVICE_FAULT_TOLERANT as u32 {
								T::MinerControl::service_punish(&miner, miner_info.snap_shot.idle_space, miner_info.snap_shot.service_space)?;
							}
							<CountedServiceFailed<T>>::insert(&miner, count);
						}

						unverify_list.remove(index);

						return Ok(())
					}
				}

				Err(Error::<T>::NonExistentMission)?
			})?;


			Self::deposit_event(Event::<T>::VerifyProof { tee_worker: sender, miner, });
	
			Ok(())
		}
	}

	

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::save_challenge_info {
				challenge_info: _,
				key,
				seg_digest,
				signature,			
			} = call {
				Self::check_unsign(key.clone(), &seg_digest, &signature)
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}

	impl<T: Config> Pallet<T> {
		fn clear_challenge(now: BlockNumberOf<T>) -> Weight {
			let mut weight: Weight = Weight::from_ref_time(0);
			let duration = <ChallengeDuration<T>>::get();
			weight = weight.saturating_add(T::DbWeight::get().reads(1));
			if now == duration {
				let snap_shot = match <ChallengeSnapShot<T>>::get() {
					Some(snap_shot) => snap_shot,
					None => return weight,
				};
				weight = weight.saturating_add(T::DbWeight::get().reads(1));
				for miner_snapshot in snap_shot.miner_snapshot_list.iter() {

					let count = <CountedClear<T>>::get(&miner_snapshot.miner) + 1;
					weight = weight.saturating_add(T::DbWeight::get().reads(1));

					let _ = T::MinerControl::clear_punish(
						&miner_snapshot.miner, 
						count, 
						miner_snapshot.idle_space, 
						miner_snapshot.service_space
					);
					weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

					if count >= 3 {
						let result = T::File::force_miner_exit(&miner_snapshot.miner);
						if result.is_err() {
							log::info!("force clear miner: {:?} failed", miner_snapshot.miner);
						}
						<CountedClear<T>>::remove(&miner_snapshot.miner);
					} else {
						<CountedClear<T>>::insert(
							&miner_snapshot.miner, 
							count,
						);
					}
				}

				weight = weight.saturating_add(T::DbWeight::get().writes(1));
			}

			weight
		}

		fn clear_verify_mission(now: BlockNumberOf<T>) -> Weight {
			let mut weight: Weight = Weight::from_ref_time(0);
			let duration = <VerifyDuration<T>>::get();
			if now == duration {
				let mut seed: u32 = 0;
				// Used to calculate the new validation period.
				let mut mission_count: u32 = 0;
				let tee_list = T::Scheduler::get_controller_list();

				for (acc, unverify_list) in UnverifyProof::<T>::iter() {
					seed += 1;
					weight = weight.saturating_add(T::DbWeight::get().reads(1));
					if unverify_list.len() > 0 {
						// Count the number of verification tasks that need to be performed.
						mission_count = mission_count.saturating_add(unverify_list.len() as u32);

						let index = Self::random_number(seed) as u32;
						let mut index: u32 = index % (tee_list.len() as u32);
						let mut tee_acc = &tee_list[index as usize];
	
						if &acc == tee_acc {
							index += 1;
							index = index % (tee_list.len() as u32);
							tee_acc = &tee_list[index as usize];
						}
	
						let result = UnverifyProof::<T>::mutate(tee_acc, |tar_unverify_list| -> DispatchResult {
							tar_unverify_list.try_append(&mut unverify_list.to_vec()).map_err(|_| Error::<T>::Overflow)?;
							Ok(())
						});
						weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

						if result.is_err() {
							let new_block: BlockNumberOf<T> = now.saturating_add(5u32.saturated_into());
							<VerifyDuration<T>>::put(new_block);
							weight = weight.saturating_add(T::DbWeight::get().writes(1));
							return weight;
						}

						UnverifyProof::<T>::remove(acc);
					}
				}

				//todo! duration reasonable time
				let duration: BlockNumberOf<T> = mission_count.saturating_mul(10u32).saturated_into();
				let new_block: BlockNumberOf<T> = now.saturating_add(duration);
				<VerifyDuration<T>>::put(new_block);
				<ChallengeSnapShot<T>>::kill();
			}

			weight
		}

		fn check_unsign(
			key: T::AuthorityId,
			seg_digest: &SegDigest<BlockNumberOf<T>>,
			signature: &<T::AuthorityId as RuntimeAppPublic>::Signature,
		) -> TransactionValidity {
			let current_session = T::ValidatorSet::session_index();
			let keys = Keys::<T>::get();

			if !keys.contains(&key) {
				return InvalidTransaction::Stale.into();
			} 

			let signature_valid = seg_digest.using_encoded(|encoded_seg_digest| {
				key.verify(&encoded_seg_digest, &signature)
			});

			if !signature_valid {
				log::error!("bad signature.");
				return InvalidTransaction::BadProof.into()
			}

			log::info!("build valid transaction");
			ValidTransaction::with_tag_prefix("Audit")
				.priority(T::UnsignedPriority::get())
				.and_provides((current_session, key, signature))
				.longevity(
					TryInto::<u64>::try_into(
						T::NextSessionRotation::average_session_length() / 2u32.into(),
					)
					.unwrap_or(64_u64),
				)
				.propagate(true)
				.build()
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
		fn _trigger_challenge(now: BlockNumberOf<T>) -> bool {
			const START_FINAL_PERIOD: Permill = Permill::from_percent(80);

			let time_point = Self::random_number(20220509);
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

		fn offchain_work_start(now: BlockNumberOf<T>) -> Result<(), OffchainErr> {
			log::info!("get loacl authority...");
			let (authority_id, validators_len) = Self::get_authority()?;
			log::info!("get loacl authority success!");
			if !Self::check_working(&now, &authority_id) {
				return Err(OffchainErr::Working);
			}
			log::info!("get challenge data...");
			let challenge_info = Self::generation_challenge(now).map_err(|e| {
				log::error!("generation challenge error:{:?}", e);
				OffchainErr::GenerateInfoError
			})?;
			log::info!("get challenge success!");
			log::info!("submit challenge to chain...");
			Self::offchain_call_extrinsic(now, authority_id, challenge_info)?;
			log::info!("submit challenge to chain!");

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

		fn get_authority() -> Result<(T::AuthorityId, usize), OffchainErr> {
			let validators = Keys::<T>::get();

			let mut local_keys = T::AuthorityId::all();

			if local_keys.len() == 0 {
				log::info!("no local_keys");
				return Err(OffchainErr::Ineligible);
			}

			local_keys.sort();
			// Find eligible keys locally.
			for key in validators.iter() {
				let res = local_keys.binary_search(key);

				let authority_id = match res {
					Ok(index) => local_keys.get(index),
					Err(_e) => continue,
				};
	
				if let Some(authority_id) = authority_id {
					return Ok((authority_id.clone(), validators.len()));
				}
			}

			Err(OffchainErr::Ineligible)
		}

		fn generation_challenge(now: BlockNumberOf<T>) 
			-> Result<ChallengeInfo<T>, OffchainErr> 
		{
			let miner_count = T::MinerControl::get_miner_count();

			let need_miner_count = miner_count / 10;

			let index_list = Self::random_select_miner(need_miner_count, miner_count);

			let allminer = T::MinerControl::get_all_miner().map_err(|_| OffchainErr::GenerateInfoError)?;

			let mut miner_list: BoundedVec<MinerSnapShot<AccountOf<T>>, T::ChallengeMinerMax> = Default::default();
			let mut total_idle_space: u128 = u128::MIN;
			let mut total_service_space: u128 = u128::MIN;
			let total_reward: u128 = T::MinerControl::get_reward();
			for index in index_list {
				
				let miner = allminer[index as usize].clone();
				let state = T::MinerControl::get_miner_state(&miner).map_err(|_| OffchainErr::GenerateInfoError)?;
				if state == "lock".as_bytes().to_vec() {
					continue;
				}

				let (idle_space, service_space) = T::MinerControl::get_power(&miner).map_err(|_| OffchainErr::GenerateInfoError)?;

				total_idle_space = total_idle_space.checked_add(idle_space).ok_or(OffchainErr::Overflow)?;
				total_service_space = total_service_space.checked_add(service_space).ok_or(OffchainErr::Overflow)?;
				let miner_snapshot = MinerSnapShot::<AccountOf<T>> {
					miner,
					idle_space,
					service_space,
				};
				let result = miner_list.try_push(miner_snapshot);
				if let Err(_e) = result {
					break;
				};
			}

			let random = Self::generate_challenge_random(now.saturated_into());

			let snap_shot = NetSnapShot::<BlockNumberOf<T>>{
				start: now,
				life: now,
				total_reward,
				total_idle_space,
				total_service_space,
				random,
			};

			Ok( ChallengeInfo::<T>{ net_snap_shot: snap_shot, miner_snapshot_list: miner_list } )
		}

		fn random_select_miner(need: u32, length: u32) -> Vec<u32> {
			let mut miner_index_list: Vec<u32> = Default::default();
			let mut seed: u32 = 20230417;
			while (miner_index_list.len() as u32) < need {
				seed += 1;
				let index = Self::random_number(seed);
				let index: u32 = (index % length as u64) as u32;

				if !miner_index_list.contains(&index) {
					miner_index_list.push(index);
				}
			}

			miner_index_list
		}

		fn offchain_call_extrinsic(
			now: BlockNumberOf<T>,
			authority_id: T::AuthorityId,
			challenge_info: ChallengeInfo<T>,
		) -> Result<(), OffchainErr> {

			let (signature, digest) = Self::offchain_sign_digest(now, &authority_id)?;

			let call = Call::save_challenge_info {
							challenge_info,
							seg_digest: digest,
							signature: signature,
							key: authority_id,
						};
		
			let result = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());

			if let Err(e) = result {
				log::error!("{:?}", e);
				return Err(OffchainErr::SubmitTransactionFailed);
			}

			Ok(())
		}

		fn offchain_sign_digest(
			now: BlockNumberOf<T>,
			authority_id: &T::AuthorityId,
		) -> Result< (<<T as pallet::Config>::AuthorityId as sp_runtime::RuntimeAppPublic>::Signature, SegDigest::<BlockNumberOf<T>>), OffchainErr> {

			let network_state =
				sp_io::offchain::network_state().map_err(|_| OffchainErr::NetworkState)?;

			let author_len = Keys::<T>::get().len();

			let digest = SegDigest::<BlockNumberOf<T>>{
				validators_len: author_len as u32,
				block_num: now,
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
		pub fn random_number(seed: u32) -> u64 {
			let (random_seed, _) = T::MyRandomness::random(&(T::MyPalletId::get(), seed).encode());
			let random_seed = match random_seed {
				Some(v) => v,
				None => Default::default(),
			};
			let random_number = <u64>::decode(&mut random_seed.as_ref())
				.expect("secure hashes should always be bigger than u32; qed");
			random_number
		}

		//The number of pieces generated is vec
		fn generate_challenge_random(seed: u32) -> [u8; 20] {
			let mut increase = seed;
			loop {
				increase += 1;
				let (r_seed, _) =
					T::MyRandomness::random(&(T::MyPalletId::get(), increase).encode());
				let r_seed = match r_seed {
					Some(v) => v,
					None => Default::default(),
				};
				let random_seed = <H256>::decode(&mut r_seed.as_ref())
					.expect("secure hashes should always be bigger than u32; qed");
				let random_vec = random_seed.as_bytes().to_vec();
				if random_vec.len() >= 20 {
					return random_vec[0..20].try_into().unwrap();
				}
			}
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
