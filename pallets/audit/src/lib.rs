//! # Audit Module
//!
//!  This file is the exclusive pallet of cess and the proof of podr2 adaptation
//!
//! ## OverView
//!
//! The job of this audit pallet is to process the proof of miner's service file and filling
//! file, and generate random challenges. Call some traits of Sminer pallet to punish miners.
//! Call the trail of file bank pallet to obtain random files or files with problems in handling
//! challenges.
//!
//! ### Terminology
//!
//! * **random_challenge:** The random time trigger initiates a challenge to the random documents.
//!   The miners need to complete the challenge within a limited time and submit the certificates of
//!   the corresponding documents.
//!
//! * **deadline:** 		Expiration time of challenge, stored in challenge duration
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

mod types;
use types::*;

mod constants;
use constants::*;

// pub mod migrations;

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

pub mod weights;

use sp_runtime::RuntimeDebug;

use codec::{Decode, Encode};
use cp_bloom_filter::BloomFilter;
use cp_cess_common::*;
use cp_scheduler_credit::SchedulerCreditCounter;
use frame_support::{
	pallet_prelude::*,
	storage::bounded_vec::BoundedVec,
	traits::{
		EstimateNextSessionRotation, FindAuthor, Randomness, ReservableCurrency, StorageVersion,
		ValidatorSetWithIdentification,
	},
	transactional, PalletId,
};
use frame_system::offchain::CreateSignedTransaction;
use pallet_sminer::MinerControl;
use pallet_storage_handler::StorageHandle;
use pallet_tee_worker::TeeWorkerHandler;
use scale_info::TypeInfo;
use sp_core::{crypto::KeyTypeId, offchain::OpaqueNetworkState, H256};
use sp_runtime::{app_crypto::RuntimeAppPublic, SaturatedConversion, Saturating};
use sp_std::{
	convert::{TryFrom, TryInto},
	prelude::*,
};
use ces_types::{TeeSig, WorkerPublicKey};
pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;

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

enum AuditErr {
	RandomErr,
	QElementErr,
	SpaceParamErr,
}

impl sp_std::fmt::Debug for AuditErr {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			AuditErr::RandomErr => write!(fmt, "Unexpected error in generating random numbers!"),
			AuditErr::QElementErr => write!(fmt, "qelement generation failed"),
			AuditErr::SpaceParamErr => write!(fmt, "Spatial parameter generation failed!"),
		}
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	// use frame_benchmarking::baseline::Config;
	use frame_support::traits::Get;
	use frame_system::{
		ensure_signed,
		pallet_prelude::{OriginFor, *},
	};

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
		type SessionKeyMax: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type ChallengeMinerMax: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type VerifyMissionMax: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type SigmaMax: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type IdleTotalHashLength: Get<u32> + Clone + Eq + PartialEq;
		// one day block
		#[pallet::constant]
		type OneDay: Get<BlockNumberFor<Self>>;
		// one hours block
		#[pallet::constant]
		type OneHours: Get<BlockNumberFor<Self>>;
		// randomness for seeds.
		type MyRandomness: Randomness<Option<Self::Hash>, BlockNumberFor<Self>>;
		//Find the consensus of the current block
		type FindAuthor: FindAuthor<Self::AccountId>;
		//Judge whether it is the trait of the consensus node
		type TeeWorkerHandler: TeeWorkerHandler<Self::AccountId, BlockNumberFor<Self>>;
		//It is used to increase or decrease the miners' computing power, space, and execute
		// punishment
		type MinerControl: MinerControl<Self::AccountId, BlockNumberFor<Self>>;

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
		type NextSessionRotation: EstimateNextSessionRotation<BlockNumberFor<Self>>;
		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;

		#[pallet::constant]
		type LockTime: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type ReassignCeiling: Get<u8> + Clone + Eq + PartialEq;

		type CreditCounter: SchedulerCreditCounter<Self::AccountId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		GenerateChallenge { miner: AccountOf<T> },

		SubmitIdleProof { miner: AccountOf<T> },

		SubmitServiceProof { miner: AccountOf<T> },

		SubmitIdleVerifyResult { tee: WorkerPublicKey, miner: AccountOf<T>, result: bool },

		SubmitServiceVerifyResult { tee: WorkerPublicKey, miner: AccountOf<T>, result: bool },

		VerifyProof { tee_worker: WorkerPublicKey, miner: AccountOf<T> },
	}

	/// Error for the audit pallet.
	#[pallet::error]
	pub enum Error<T> {
		/// Vec to BoundedVec Error
		BoundedVecError,
		/// Error indicating that the storage has reached its limit
		StorageLimitReached,
		/// Data overflow
		Overflow,
		/// The miner submits a certificate, but there is no error in the challenge list
		NoChallenge,
		/// Not a consensus node or not registered
		ScheduleNonExistent,
		/// filetype error
		FileTypeError,
		/// The user does not have permission to call this method
		NotQualified,
		/// Error recording time
		RecordTimeError,
		/// Offchain worker: Error Signing the transaction
		OffchainSignedTxError,
		/// There is no local account that can be used for signing
		NoLocalAcctForSigning,
		/// Length exceeds limit
		LengthExceedsLimit,
		/// An error that will not occur by design will be prompted after an error occurs during the random number generation process
		SystemError,
		/// The verification task does not exist
		NonExistentMission,
		/// Unexpected Error
		UnexpectedError,
		/// Challenge has expired
		Expired,
		/// Verification of tee signature failed
		VerifyTeeSigFailed,
		/// Bloom filter validation failed
		BloomFilterError,
		/// The certificate has been submitted and cannot be submitted again
		Submitted,
		/// Random number generation failed
		RandomErr,
		/// No proof submitted
		UnSubmitted,
		/// The tee does not have permission
		TeeNoPermission,
		/// Signature format conversion failed
		MalformedSignature,
	}

	#[pallet::storage]
	#[pallet::getter(fn counted_service_failed)]
	pub(super) type CountedServiceFailed<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn counted_clear)]
	pub(super) type CountedClear<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, u8, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn challenge_snap_shot)]
	pub(super) type ChallengeSnapShot<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, ChallengeInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn challenge_slip)]
	pub(super) type ChallengeSlip<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		BlockNumberFor<T>,
		Blake2_128Concat,
		AccountOf<T>,
		bool,
	>;

	#[pallet::storage]
	#[pallet::getter(fn verify_slip)]
	pub(super) type VerifySlip<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		BlockNumberFor<T>,
		Blake2_128Concat,
		AccountOf<T>,
		bool,
	>;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			let weight: Weight = Weight::zero();
			weight
				.saturating_add(Self::generate_challenge(now))
				.saturating_add(Self::clear_challenge(now))
				.saturating_add(Self::clear_verify_mission(now))
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit an idle proof for challenge verification.
		///
		/// This function allows an account, identified by the `origin`, to submit an idle proof as
		/// part of a challenge verification process.
		///
		/// # Parameters
		///
		/// - `origin`: The origin of the request, typically a user or account.
		/// - `idle_prove`: A bounded vector of bytes representing the idle proof to be submitted.
		///   It should match the configured total hash length for idle proofs.
		#[pallet::call_index(1)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::submit_idle_proof())]
		pub fn submit_idle_proof(
			origin: OriginFor<T>,
			idle_prove: BoundedVec<u8, T::IdleTotalHashLength>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			<ChallengeSnapShot<T>>::try_mutate(&sender, |challenge_info_opt| -> DispatchResult {
				let challenge_info = challenge_info_opt.as_mut().ok_or(Error::<T>::NoChallenge)?;
				let now = <frame_system::Pallet<T>>::block_number();
				if now > challenge_info.challenge_element.idle_slip {
					return Err(Error::<T>::Expired)?
				}

				if challenge_info.prove_info.idle_prove.is_none() {
					let tee_puk = Self::random_select_tee_acc(0)?;

					let idle_prove_info = IdleProveInfo::<T> {
						tee_puk: tee_puk.clone(),
						idle_prove,
						verify_result: None,
					};

					challenge_info.prove_info.idle_prove = Some(idle_prove_info);
					if challenge_info.prove_info.service_prove.is_some() {
						<CountedClear<T>>::insert(&sender, u8::MIN);
					}
				} else {
					return Err(Error::<T>::Submitted)?
				}

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::SubmitIdleProof { miner: sender });

			Ok(())
		}

		/// Submit a service proof as part of a challenge in the pallet.
		///
		/// This function is a part of the pallet's public interface and allows an authorized
		/// account (identified by the `origin`) to submit a service proof as a response to a
		/// challenge.
		///
		/// # Parameters
		///
		/// - `origin`: The origin of the transaction, representing the caller.
		/// - `service_prove`: A bounded vector of bytes (`BoundedVec`) containing the service proof
		///   data.
		#[pallet::call_index(2)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::submit_service_proof())]
		pub fn submit_service_proof(
			origin: OriginFor<T>,
			service_prove: BoundedVec<u8, T::SigmaMax>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			<ChallengeSnapShot<T>>::try_mutate(&sender, |challenge_info_opt| -> DispatchResult {
				let challenge_info = challenge_info_opt.as_mut().ok_or(Error::<T>::NoChallenge)?;
				let now = <frame_system::Pallet<T>>::block_number();
				if now > challenge_info.challenge_element.service_slip {
					return Err(Error::<T>::Expired)?
				}

				if challenge_info.prove_info.service_prove.is_none() {
					let tee_puk = Self::random_select_tee_acc(0)?;

					let service_prove_info = ServiceProveInfo::<T> {
						tee_puk: tee_puk.clone(),
						service_prove,
						verify_result: None,
					};

					challenge_info.prove_info.service_prove = Some(service_prove_info);
					if challenge_info.prove_info.idle_prove.is_some() {
						<CountedClear<T>>::insert(&sender, u8::MIN);
					}
				} else {
					return Err(Error::<T>::Submitted)?
				}

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::SubmitServiceProof { miner: sender });

			Ok(())
		}

		/// Submit a verification result for idle space proofs in response to a challenge.
		///
		/// This function is a part of the pallet's public interface and allows an authorized user
		/// (identified by the `origin`) to submit the verification result for idle space proofs
		/// as part of a challenge response.
		///
		/// # Parameters
		///
		/// - `origin`: The origin of the transaction, representing the caller.
		/// - `total_prove_hash`: A bounded vector of bytes (`BoundedVec`) containing the total
		///   proof hash.
		/// - `front`: The front index of the proof range.
		/// - `rear`: The rear index of the proof range.
		/// - `accumulator`: The accumulator data.
		/// - `idle_result`: A boolean indicating the verification result for idle space.
		/// - `signature`: A TEERsaSignature for the verification.
		/// - `tee_acc`: The TEERsaSignature worker account associated with the proof.
		#[pallet::call_index(3)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::submit_verify_idle_result_reward())]
		pub fn submit_verify_idle_result(
			origin: OriginFor<T>,
			total_prove_hash: BoundedVec<u8, T::IdleTotalHashLength>,
			front: u64,
			rear: u64,
			accumulator: Accumulator,
			idle_result: bool,
			signature: BoundedVec<u8, ConstU32<64>>,
			tee_puk: WorkerPublicKey,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			<ChallengeSnapShot<T>>::try_mutate(&sender, |challenge_info_opt| -> DispatchResult {
				let challenge_info = challenge_info_opt.as_mut().ok_or(Error::<T>::NoChallenge)?;

				let idle_prove =
					challenge_info.prove_info.idle_prove.as_mut().ok_or(Error::<T>::UnSubmitted)?;

				if tee_puk != idle_prove.tee_puk {
					return Err(Error::<T>::NonExistentMission)?
				}

				if let Some(_) = idle_prove.verify_result {
					return Err(Error::<T>::Submitted)?
				}

				let MinerSnapShot {
					idle_space,
					service_space,
					service_bloom_filter: _,
					space_proof_info,
					tee_signature: _,
				} = &challenge_info.miner_snapshot;

				ensure!(
					T::TeeWorkerHandler::can_verify(&tee_puk),
					Error::<T>::TeeNoPermission
				);
				let verify_idle_info = VerifyIdleResultInfo::<T> {
					miner: sender.clone(),
					miner_prove: total_prove_hash.clone(),
					front: space_proof_info.front,
					rear: space_proof_info.rear,
					accumulator: space_proof_info.accumulator,
					space_challenge_param: challenge_info.challenge_element.space_param,
					result: idle_result,
					tee_puk: tee_puk.clone(),
				};
				
				let encoding = verify_idle_info.encode();
				let hashing = sp_io::hashing::sha2_256(&encoding);
				let sig = 
					sp_core::sr25519::Signature::try_from(signature.as_slice()).or(Err(Error::<T>::MalformedSignature))?;
					
				ensure!(
					T::TeeWorkerHandler::verify_master_sig(&sig, hashing),
					Error::<T>::VerifyTeeSigFailed
				);

				let now = <frame_system::Pallet<T>>::block_number();
				T::TeeWorkerHandler::update_work_block(now, &tee_puk)?;

				let idle_result = Self::check_idle_verify_param(
					idle_result,
					front,
					rear,
					&total_prove_hash,
					&accumulator,
					&challenge_info.miner_snapshot,
					&idle_prove.idle_prove,
				);

				idle_prove.verify_result = Some(idle_result);

				if let Some(service_prove) = &challenge_info.prove_info.service_prove {
					if let Some(service_result) = service_prove.verify_result {
						if idle_result && service_result {
							T::MinerControl::record_snap_shot(
								&sender,
								*idle_space,
								*service_space,
							)?;
						}
					}
				}

				let count = challenge_info
					.miner_snapshot
					.space_proof_info
					.rear
					.checked_sub(challenge_info.miner_snapshot.space_proof_info.front)
					.ok_or(Error::<T>::Overflow)?;
				
				let space = IDLE_SEG_SIZE.checked_mul(count as u128).ok_or(Error::<T>::Overflow)?;
				let bond_stash = T::TeeWorkerHandler::get_stash(&tee_puk)?;
				T::CreditCounter::increase_point_for_idle_verify(&bond_stash, space)?;

				Self::deposit_event(Event::<T>::SubmitIdleVerifyResult {
					tee: tee_puk.clone(),
					miner: sender.clone(),
					result: idle_result,
				});

				Ok(())
			})
		}

		/// Submit a verification result for service proofs in response to a challenge.
		///
		/// This function is a part of the pallet's public interface and allows an authorized user
		/// (identified by the `origin`) to submit the verification result for service proofs
		/// as part of a challenge response.
		///
		/// # Parameters
		///
		/// - `origin`: The origin of the transaction, representing the caller.
		/// - `service_result`: A boolean indicating the verification result for service proofs.
		/// - `signature`: A TEERsaSignature for the verification.
		/// - `service_bloom_filter`: A BloomFilter representing the service's data.
		/// - `tee_acc`: The TEERsaSignature worker account associated with the proof.
		#[pallet::call_index(4)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::submit_verify_service_result_reward())]
		pub fn submit_verify_service_result(
			origin: OriginFor<T>,
			service_result: bool,
			signature: BoundedVec<u8, ConstU32<64>>,
			service_bloom_filter: BloomFilter,
			tee_puk: WorkerPublicKey,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			<ChallengeSnapShot<T>>::try_mutate(&sender, |challenge_info_opt| -> DispatchResult {
				let challenge_info = challenge_info_opt.as_mut().ok_or(Error::<T>::NoChallenge)?;

				let service_prove = challenge_info
					.prove_info
					.service_prove
					.as_mut()
					.ok_or(Error::<T>::UnSubmitted)?;
				if tee_puk != service_prove.tee_puk {
					return Err(Error::<T>::NonExistentMission)?
				}

				if let Some(_) = service_prove.verify_result {
					return Err(Error::<T>::Submitted)?
				}

				let MinerSnapShot {
					idle_space,
					service_space,
					service_bloom_filter: s_service_bloom_filter,
					space_proof_info: _,
					tee_signature: _,
				} = challenge_info.miner_snapshot;

				ensure!(
					T::TeeWorkerHandler::can_verify(&tee_puk),
					Error::<T>::TeeNoPermission
				);
				let verify_service_info = VerifyServiceResultInfo::<T> {
					miner: sender.clone(),
					tee_puk: tee_puk.clone(),
					miner_prove: service_prove.service_prove.clone(),
					result: service_result,
					chal: QElement {
						random_index_list: challenge_info
							.challenge_element
							.service_param
							.random_index_list
							.clone(),
						random_list: challenge_info
							.challenge_element
							.service_param
							.random_list
							.clone(),
					},
					service_bloom_filter: s_service_bloom_filter,
				};

				let encoding = verify_service_info.encode();
				let hashing = sp_io::hashing::sha2_256(&encoding);
				let sig = 
					sp_core::sr25519::Signature::try_from(signature.as_slice()).or(Err(Error::<T>::MalformedSignature))?;

				ensure!(
					T::TeeWorkerHandler::verify_master_sig(&sig, hashing),
					Error::<T>::VerifyTeeSigFailed
				);

				ensure!(
					service_bloom_filter == s_service_bloom_filter,
					Error::<T>::BloomFilterError,
				);

				let now = <frame_system::Pallet<T>>::block_number();
				T::TeeWorkerHandler::update_work_block(now, &tee_puk)?;

				service_prove.verify_result = Some(service_result);

				if let Some(idle_prove) = &challenge_info.prove_info.idle_prove {
					if let Some(idle_result) = idle_prove.verify_result {
						if idle_result && service_result {
							T::MinerControl::record_snap_shot(
								&sender,
								idle_space,
								service_space,
							)?;
						}
					}
				}

				if service_result {
					<CountedServiceFailed<T>>::insert(&sender, u32::MIN);
				} else {
					let count = <CountedServiceFailed<T>>::get(&sender)
						.checked_add(1)
						.unwrap_or(SERVICE_FAULT_TOLERANT as u32);
					if count >= SERVICE_FAULT_TOLERANT as u32 {
						T::MinerControl::service_punish(&sender, service_space, service_space)?;
					}
					<CountedServiceFailed<T>>::insert(&sender, count);
				}

				let bond_stash = T::TeeWorkerHandler::get_stash(&tee_puk)?;
				T::CreditCounter::increase_point_for_idle_verify(&bond_stash, challenge_info.miner_snapshot.service_space)?;

				Self::deposit_event(Event::<T>::SubmitServiceVerifyResult {
					tee: tee_puk.clone(),
					miner: sender.clone(),
					result: service_result,
				});

				Ok(())
			})
		}
		// FOR TEST
		/// Update and reset the counted clear value for a specific miner.
		///
		/// This function is designed for administrative purposes and can only be called by a root
		/// user (administrator) to reset the `CountedClear` value for a specific miner to zero.
		///
		/// # Parameters
		///
		/// - `origin`: The origin of the transaction, representing the caller (administrator).
		/// - `miner`: The account of the miner whose `CountedClear` value will be reset to zero.
		#[pallet::call_index(7)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
		pub fn update_counted_clear(origin: OriginFor<T>, miner: AccountOf<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;

			<CountedClear<T>>::insert(&miner, 0);

			Ok(())
		}
		// FOR TEST
		#[pallet::call_index(9)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
		pub fn test_update_clear_slip(
			origin: OriginFor<T>,
			old: BlockNumberFor<T>,
			new: BlockNumberFor<T>,
			miner: AccountOf<T>,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			ChallengeSlip::<T>::remove(&old, &miner);
			ChallengeSlip::<T>::insert(&new, &miner, true);

			Ok(())
		}
		// FOR TEST
		#[pallet::call_index(10)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
		pub fn test_update_verify_slip(
			origin: OriginFor<T>,
			old: BlockNumberFor<T>,
			new: BlockNumberFor<T>,
			miner: AccountOf<T>,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			VerifySlip::<T>::remove(&old, &miner);
			VerifySlip::<T>::insert(&new, &miner, true);

			Ok(())
		}
		// FOR TEST
		#[pallet::call_index(11)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
		pub fn point_miner_challenge(
			origin: OriginFor<T>,
			miner: AccountOf<T>,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			let miner = &miner;
			let now = <frame_system::Pallet<T>>::block_number();
			if <ChallengeSnapShot<T>>::contains_key(miner) {
				return Ok(())
			}

			let miner_snapshot = match T::MinerControl::get_miner_snapshot(miner) {
				Ok(miner_snapshot) => miner_snapshot,
				Err(_) => return Ok(()),
			};

			let (idle_space, service_space, service_bloom_filter, space_proof_info, tee_signature) =
				miner_snapshot;

			if idle_space + service_space == 0 {
				return Ok(())
			}

			let service_param = match Self::generate_miner_qelement(now.saturated_into()) {
				Ok(service_param) => service_param,
				Err(e) => {
					log::info!("audit: {:?}", e);
					return Ok(())
				},
			};
			let space_param = match Self::generate_miner_space_param(now.saturated_into()) {
				Ok(space_param) => space_param,
				Err(e) => {
					log::info!("audit: {:?}", e);
					return Ok(())
				},
			};

			let idle_life: u32 =
				(idle_space.saturating_div(IDLE_PROVE_RATE).saturating_add(50)) as u32;
			let idle_slip = now.saturating_add(idle_life.saturated_into());

			let service_life: u32 =
				(service_space.saturating_div(SERVICE_PROVE_RATE).saturating_add(50)) as u32;
			let service_slip = now.saturating_add(service_life.saturated_into());

			let max_slip = {
				if idle_slip > service_slip {
					idle_slip
				} else {
					service_slip
				}
			};

			let one_hour = T::OneHours::get();
			let tee_length = T::TeeWorkerHandler::get_pubkey_list().len();
			if tee_length == 0 {
				return Ok(());
			}
			let verify_life: u32 = (idle_space
				.saturating_add(service_space)
				.saturating_div(IDLE_VERIFY_RATE)
				.saturating_div(tee_length as u128)) as u32;
			let verify_slip =
				max_slip.saturating_add(verify_life.saturated_into()).saturating_add(one_hour);

			let challenge_info = ChallengeInfo::<T> {
				miner_snapshot: MinerSnapShot::<T> {
					idle_space,
					service_space,
					service_bloom_filter,
					space_proof_info,
					tee_signature,
				},
				challenge_element: ChallengeElement::<T> {
					start: now,
					idle_slip,
					service_slip,
					verify_slip,
					space_param,
					service_param,
				},
				prove_info: ProveInfo::<T> {
					assign: u8::MIN,
					idle_prove: None,
					service_prove: None,
				},
			};

			<ChallengeSnapShot<T>>::insert(&miner, challenge_info);
			<ChallengeSlip<T>>::insert(&max_slip, &miner, true);
			<VerifySlip<T>>::insert(&verify_slip, &miner, true);

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Clear challenge data and perform associated actions for the given block number.
		///
		/// This function is used to clear challenge data and perform various operations for a
		/// specific block number. It iterates over challenges in the `ChallengeSlip` storage item
		/// and checks the associated `ChallengeSnapShot`.
		///
		/// # Parameters
		///
		/// - `now`: The block number for which challenge data is to be cleared and processed.
		///
		/// # Returns
		///
		/// The total weight consumed by the operation.
		fn clear_challenge(now: BlockNumberFor<T>) -> Weight {
			let mut weight: Weight = Weight::zero();

			for (miner, _) in <ChallengeSlip<T>>::iter_prefix(&now) {
				if let Ok(challenge_info) = <ChallengeSnapShot<T>>::try_get(&miner) {
					weight = weight.saturating_add(T::DbWeight::get().reads(1));
					if challenge_info.prove_info.service_prove.is_none() {
						let count = <CountedClear<T>>::get(&miner).checked_add(1).unwrap_or(3);
						weight = weight.saturating_add(T::DbWeight::get().reads(1));

						if count > 1 {
							let _ = T::MinerControl::clear_punish(
								&miner,
								challenge_info.miner_snapshot.idle_space,
								challenge_info.miner_snapshot.service_space,
								count,
							);
							weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
						}

						if count >= 100 {
							let result = T::MinerControl::force_miner_exit(&miner);
							weight = weight.saturating_add(T::DbWeight::get().reads_writes(5, 5));
							if result.is_err() {
								log::info!("force clear miner: {:?} failed", miner);
							}
							<CountedClear<T>>::remove(&miner);
							weight = weight.saturating_add(T::DbWeight::get().writes(1));
						} else {
							<CountedClear<T>>::insert(&miner, count);
							weight = weight.saturating_add(T::DbWeight::get().writes(1));
						}
					}
				}

				<ChallengeSlip<T>>::remove(&now, &miner);
				weight = weight.saturating_add(T::DbWeight::get().writes(1));
			}

			weight
		}

		/// Clear Verify Missions and Update Challenge Information
		///
		/// This function iterates through Verify Missions for a given block number `now`,
		/// processes each mission, and updates challenge information accordingly. Verify
		/// Missions represent task miners must complete within a certain time frame. If
		/// a miner fails to meet the requirements, their mission may be cleared and
		/// challenge information updated.
		///
		/// Parameters:
		/// - `now`: The block number for which verification missions are processed.
		///
		/// Returns:
		/// - A `Weight` value representing the computational cost of the operation.
		fn clear_verify_mission(now: BlockNumberFor<T>) -> Weight {
			let mut weight: Weight = Weight::zero();

			for (miner, _) in <VerifySlip<T>>::iter_prefix(&now) {
				weight = weight.saturating_add(T::DbWeight::get().reads(1));
				let mut flag = false;
				if let Ok(challenge_info) = <ChallengeSnapShot<T>>::try_get(&miner) {
					weight = weight.saturating_add(T::DbWeight::get().reads(1));
					if let Some(idle_prove) = &challenge_info.prove_info.idle_prove {
						if idle_prove.verify_result.is_some() {
							flag = true;
						}
					} else {
						flag = true;
					}

					if let Some(service_prove) = &challenge_info.prove_info.service_prove {
						if service_prove.verify_result.is_none() {
							flag = false;
						}
					}

					if challenge_info.prove_info.assign >= 2 {
						flag = true;
					}

					if challenge_info.prove_info.idle_prove.is_none() &&
						challenge_info.prove_info.service_prove.is_none()
					{
						flag = true
					}
				}

				if flag {
					<VerifySlip<T>>::remove(&now, &miner);
					<ChallengeSnapShot<T>>::remove(&miner);
					weight = weight.saturating_add(T::DbWeight::get().writes(2));
				} else {
					if let Ok(mut challenge_info) = <ChallengeSnapShot<T>>::try_get(&miner) {
						challenge_info.prove_info.assign += 1;
						if challenge_info.prove_info.idle_prove.is_some() {
							if let Ok(tee_puk) = Self::random_select_tee_acc(0) {
								let idle_prove =
									challenge_info.prove_info.idle_prove.as_mut().unwrap();
								idle_prove.tee_puk = tee_puk;
							}
						}

						if challenge_info.prove_info.service_prove.is_some() {
							if let Ok(tee_puk) = Self::random_select_tee_acc(0) {
								let service_prove =
									challenge_info.prove_info.service_prove.as_mut().unwrap();
								service_prove.tee_puk = tee_puk;
							}
						}

						let max_space = {
							if challenge_info.miner_snapshot.idle_space >
								challenge_info.miner_snapshot.service_space
							{
								challenge_info.miner_snapshot.idle_space
							} else {
								challenge_info.miner_snapshot.service_space
							}
						};

						let one_hour: u32 = T::OneHours::get().saturated_into();
						let verify_life: u32 = max_space
							.saturating_div(IDLE_VERIFY_RATE)
							.saturating_add(one_hour as u128) as u32;

						let new_slip = now.saturating_add(verify_life.saturated_into());
						challenge_info.challenge_element.verify_slip = new_slip;

						<VerifySlip<T>>::remove(&now, &miner);
						<VerifySlip<T>>::insert(&new_slip, &miner, true);
						<ChallengeSnapShot<T>>::insert(&miner, challenge_info);
						weight = weight.saturating_add(T::DbWeight::get().writes(3));
					}
				}
			}

			weight
		}

		/// Generate and Initiate Challenge for a Miner
		///
		/// This function generates and initiates a challenge for a miner based on various
		/// parameters and configuration settings. It involves selecting a miner, creating
		/// a challenge with specific timing characteristics, and initializing challenge-related
		/// data structures.
		///
		/// Parameters:
		/// - `now`: The block number representing the current state of the blockchain.
		///
		/// Returns:
		/// - A `Weight` value representing the computational cost of the operation.
		pub(crate) fn generate_challenge(now: BlockNumberFor<T>) -> Weight {
			let mut weight: Weight = Weight::zero();

			let one_day = T::OneDay::get();
			if now < one_day.saturating_mul(3u32.saturated_into()) {
				return weight;
			}

			if now % 4u32.saturated_into() != 0u32.saturated_into() {
				return weight;
			}

			weight = weight.saturating_add(T::DbWeight::get().reads(1));
			let miner_list = match T::MinerControl::get_all_miner() {
				Ok(miner_list) => miner_list,
				Err(_) => return weight,
			};

			let miner_count = miner_list.len() as u64;
			if miner_count == 0 {
				return weight
			}

			let factor = match Self::random_number(now.saturated_into()) {
				Ok(factor) => factor,
				Err(_) => return weight,
			};
			let index = factor % miner_count;
			let miner = &miner_list[index as usize];
			log::info!("generate challenge index: {:?}", index);

			if <ChallengeSnapShot<T>>::contains_key(miner) {
				log::info!("audit: Select invalid miner");
				return weight
			}

			weight = weight.saturating_add(T::DbWeight::get().reads(1));
			let miner_snapshot = match T::MinerControl::get_miner_snapshot(miner) {
				Ok(miner_snapshot) => miner_snapshot,
				Err(e) => {
					log::info!("audit: {:?}", e);
					return weight
				},
			};

			let (idle_space, service_space, service_bloom_filter, space_proof_info, tee_signature) =
				miner_snapshot;

			if idle_space + service_space == 0 {
				log::info!("audit: Select invalid miner");
				return weight
			}

			let service_param = match Self::generate_miner_qelement(now.saturated_into()) {
				Ok(service_param) => service_param,
				Err(e) => {
					log::info!("audit: {:?}", e);
					return weight
				},
			};
			let space_param = match Self::generate_miner_space_param(now.saturated_into()) {
				Ok(space_param) => space_param,
				Err(e) => {
					log::info!("audit: {:?}", e);
					return weight
				},
			};

			let idle_life: u32 =
				(idle_space.saturating_div(IDLE_PROVE_RATE).saturating_add(50)) as u32;
			let idle_slip = now.saturating_add(idle_life.saturated_into());

			let service_life: u32 =
				(service_space.saturating_div(SERVICE_PROVE_RATE).saturating_add(50)) as u32;
			let service_slip = now.saturating_add(service_life.saturated_into());

			let max_slip = {
				if idle_slip > service_slip {
					idle_slip
				} else {
					service_slip
				}
			};

			let one_hour = T::OneHours::get();
			weight = weight.saturating_add(T::DbWeight::get().reads(1));
			let tee_length = T::TeeWorkerHandler::get_pubkey_list().len();
			if tee_length == 0 {
				return weight;
			}
			let verify_life: u32 = (idle_space
				.saturating_add(service_space)
				.saturating_div(IDLE_VERIFY_RATE)
				.saturating_div(tee_length as u128)) as u32;
			let verify_slip =
				max_slip.saturating_add(verify_life.saturated_into()).saturating_add(one_hour);

			let challenge_info = ChallengeInfo::<T> {
				miner_snapshot: MinerSnapShot::<T> {
					idle_space,
					service_space,
					service_bloom_filter,
					space_proof_info,
					tee_signature,
				},
				challenge_element: ChallengeElement::<T> {
					start: now,
					idle_slip,
					service_slip,
					verify_slip,
					space_param,
					service_param,
				},
				prove_info: ProveInfo::<T> {
					assign: u8::MIN,
					idle_prove: None,
					service_prove: None,
				},
			};

			weight = weight.saturating_add(T::DbWeight::get().writes(3));
			<ChallengeSnapShot<T>>::insert(&miner, challenge_info);
			<ChallengeSlip<T>>::insert(&max_slip, &miner, true);
			<VerifySlip<T>>::insert(&verify_slip, &miner, true);

			Self::deposit_event(Event::<T>::GenerateChallenge {
				miner: miner.clone(),
			});

			weight
		}

		/// Randomly Select Trusted Execution Environment (TEE) Account
		///
		/// This function selects a Trusted Execution Environment (TEE) account from a list
		/// of available TEE accounts. The selection is made based on a random index generated
		/// from a seed derived from the current block number. An optional bit mask can be
		/// applied to the seed to influence the randomness.
		///
		/// Parameters:
		/// - `mask`: An optional bit mask to apply to the randomization process. It can be used to
		///   influence the selection by XOR-ing with the block-based seed.
		///
		/// Returns:
		/// - A `Result` containing the selected TEE account if successful, or a `DispatchError` if
		///   there are no available TEE accounts or if random number generation fails.
		fn random_select_tee_acc(mask: u32) -> Result<WorkerPublicKey, DispatchError> {
			let tee_list = T::TeeWorkerHandler::get_pubkey_list();
			ensure!(tee_list.len() > 0, Error::<T>::SystemError);

			let seed: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();
			let index = Self::random_number(seed + mask).map_err(|_| Error::<T>::RandomErr)? as u32;
			let index: u32 = index % (tee_list.len() as u32);
			let tee_puk = &tee_list[index as usize];

			Ok(tee_puk.clone())
		}

		/// Generate Miner QElement
		///
		/// This function generates a `QElement`, which contains a list of random indices
		/// (`random_index_list`) and corresponding random values (`random_list`). These
		/// values are used for auditing purposes.
		///
		/// The generation process involves selecting a number of random indices (random_index_list)
		/// and corresponding random values (random_list). These selections are based on a seed,
		/// and the generated elements are used in auditing processes.
		///
		/// Parameters:
		/// - `seed`: An initial seed value used to generate random indices and values.
		///
		/// Returns:
		/// - A `Result` containing a `QElement` with populated random indices and values if
		///   successful, or an `AuditErr` error in case of potential issues during the generation
		///   process.
		pub(crate) fn generate_miner_qelement(seed: u32) -> Result<QElement, AuditErr> {
			let mut random_index_list: BoundedVec<u32, ConstU32<1024>> = Default::default();
			let mut random_list: BoundedVec<[u8; 20], ConstU32<1024>> = Default::default();

			let need_count = CHUNK_COUNT * 46 / 1000;
			let mut seed = seed;
			let mut counter: u32 = 0;
			while random_index_list.len() < need_count as usize {
				seed = seed.checked_add(1).ok_or(AuditErr::QElementErr)?;
				counter = counter.checked_add(1).ok_or(AuditErr::QElementErr)?;
				let random_index = (Self::random_number(seed)? % CHUNK_COUNT as u64) as u32;
				if !random_index_list.contains(&random_index) {
					random_index_list.try_push(random_index).map_err(|_| AuditErr::QElementErr)?;
				}
				if counter > need_count * 3 {
					return Err(AuditErr::QElementErr)
				}
			}
			let mut counter: u32 = 0;
			while random_list.len() < random_index_list.len() {
				seed = seed.checked_add(1).ok_or(AuditErr::QElementErr)?;
				counter = counter.checked_add(1).ok_or(AuditErr::QElementErr)?;
				let random_number = Self::generate_challenge_random(seed)?;
				if !random_list.contains(&random_number) {
					random_list.try_push(random_number).map_err(|_| AuditErr::QElementErr)?;
				}
				if counter > need_count * 3 {
					return Err(AuditErr::QElementErr)
				}
			}
			Ok(QElement { random_index_list, random_list })
		}

		/// Generate Miner Space Challenge Parameters
		///
		/// This function generates space challenge parameters (`SpaceChallengeParam`) for a miner.
		/// These parameters are used in auditing processes. The parameters consist of a list
		/// of elements, each representing a specific space challenge.
		///
		/// The generation process involves generating unique random values for the space
		/// challenges.
		///
		/// Parameters:
		/// - `seed`: An initial seed value used to introduce variation in random value generation.
		///
		/// Returns:
		/// - A `Result` containing a `SpaceChallengeParam` with populated space challenge elements
		///   if successful, or an `AuditErr` error in case of potential issues during the
		///   generation process.
		pub(crate) fn generate_miner_space_param(seed: u32) -> Result<SpaceChallengeParam, AuditErr> {
			// generate idle challenge param
			let (_, n, d) =
				T::MinerControl::get_expenders().map_err(|_| AuditErr::SpaceParamErr)?;
			let mut space_challenge_param: SpaceChallengeParam = Default::default();
			let mut repeat_filter: Vec<u64> = Default::default();
			let seed_multi: u32 = 5;
			let mut seed: u32 = seed.checked_mul(seed_multi).ok_or(AuditErr::SpaceParamErr)?;
			let limit = space_challenge_param.len();
			for elem in &mut space_challenge_param {
				let mut counter: usize = 0;
				loop {
					let random =
						Self::random_number(seed.checked_add(1).ok_or(AuditErr::SpaceParamErr)?)? %
							n;
					counter = counter.checked_add(1).ok_or(AuditErr::SpaceParamErr)?;

					if counter > limit * 3 {
						return Err(AuditErr::SpaceParamErr)
					}

					let random = n
						.checked_mul(d)
						.ok_or(AuditErr::SpaceParamErr)?
						.checked_add(random)
						.ok_or(AuditErr::SpaceParamErr)?;
					if repeat_filter.contains(&random) {
						continue
					}
					repeat_filter.push(random);
					*elem = random;
					seed = seed.checked_add(1).ok_or(AuditErr::SpaceParamErr)?;
					break
				}
			}
			Ok(space_challenge_param)
		}

		// Generate a random number from a given seed.
		fn random_number(seed: u32) -> Result<u64, AuditErr> {
			let (random_seed, _) = T::MyRandomness::random(&(T::MyPalletId::get(), seed).encode());
			let random_seed = match random_seed {
				Some(v) => v,
				None => Default::default(),
			};
			let random_number =
				<u64>::decode(&mut random_seed.as_ref()).map_err(|_| AuditErr::RandomErr)?;
			Ok(random_number)
		}

		//The number of pieces generated is vec
		fn generate_challenge_random(seed: u32) -> Result<[u8; 20], AuditErr> {
			let mut increase = seed;
			loop {
				increase += 1;
				let (r_seed, _) =
					T::MyRandomness::random(&(T::MyPalletId::get(), increase).encode());
				let r_seed = match r_seed {
					Some(v) => v,
					None => Default::default(),
				};
				let random_seed =
					<H256>::decode(&mut r_seed.as_ref()).map_err(|_| AuditErr::RandomErr)?;
				let random_vec = random_seed.as_bytes().to_vec();
				if random_vec.len() >= 20 {
					let random = random_vec[0..20].try_into().map_err(|_| AuditErr::RandomErr)?;
					return Ok(random)
				}
			}
		}

		/// Check Idle Verify Parameters
		///
		/// This function checks the integrity of idle verification parameters by comparing them to
		/// expected values. The parameters are used in the idle verification process, and ensuring
		/// their correctness is crucial for audit procedures.
		///
		/// The function compares the following parameters:
		///
		/// - `accumulator`: The current accumulator, which should match the one stored in
		///   `miner_info`.
		/// - `rear`: The rear value, which should match the `rear` value in `miner_info`.
		/// - `front`: The front value, which should match the `front` value in `miner_info`.
		/// - `total_prove_hash`: The total proof hash, which should match `m_total_prove_hash`.
		///
		/// If any of the comparisons result in a mismatch, the `idle_result` flag is set to
		/// `false`, indicating that the parameters are not correct.
		///
		/// Parameters:
		/// - `idle_result`: A boolean flag that tracks the correctness of the idle verification
		///   parameters. It is initially set to `true` and is updated to `false` if any of the
		///   checks fail.
		/// - `front`: The front value used in idle verification.
		/// - `rear`: The rear value used in idle verification.
		/// - `total_prove_hash`: The total proof hash generated during idle verification.
		/// - `accumulator`: The current accumulator, which should match the one stored in
		///   `miner_info`.
		/// - `miner_info`: Information about the miner's space proof and related data.
		/// - `m_total_prove_hash`: The total proof hash value expected to match the generated
		///   `total_prove_hash`.
		///
		/// Returns:
		/// - A boolean flag (`idle_result`) indicating whether the idle verification parameters are
		///   correct.
		fn check_idle_verify_param(
			mut idle_result: bool,
			front: u64,
			rear: u64,
			total_prove_hash: &BoundedVec<u8, T::IdleTotalHashLength>,
			accumulator: &Accumulator,
			miner_info: &MinerSnapShot<T>,
			m_total_prove_hash: &BoundedVec<u8, T::IdleTotalHashLength>,
		) -> bool {
			if accumulator != &miner_info.space_proof_info.accumulator {
				idle_result = false
			}

			if rear != miner_info.space_proof_info.rear {
				idle_result = false
			}

			if front != miner_info.space_proof_info.front {
				idle_result = false
			}

			if total_prove_hash != m_total_prove_hash {
				idle_result = false
			}

			idle_result
		}
	}
}
