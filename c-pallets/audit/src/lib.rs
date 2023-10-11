//! # Audit Module
//!
//!  This file is the exclusive pallet of cess and the proof of podr2 adaptation
//!
//! ## OverView
//!
//!  The job of this audit pallet is to process the proof of miner's service file and filling
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

mod types;
use types::*;

mod constants;
use constants::*;

// pub mod migrations;

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

pub mod weights;

use sp_runtime::{
	RuntimeDebug,
};

use codec::{Decode, Encode};
use cp_bloom_filter::BloomFilter;
use cp_cess_common::*;
use cp_enclave_verify::verify_rsa;
use cp_scheduler_credit::SchedulerCreditCounter;
use frame_support::{
	pallet_prelude::*, transactional,
	storage::bounded_vec::BoundedVec,
	traits::{
		StorageVersion, ReservableCurrency, Randomness, FindAuthor, 
		ValidatorSetWithIdentification, EstimateNextSessionRotation,
	},
	PalletId, WeakBoundedVec,
};
use sp_core::{
	crypto::KeyTypeId,
	offchain::OpaqueNetworkState,
};
use sp_runtime::{Saturating, app_crypto::RuntimeAppPublic, SaturatedConversion};
use frame_system::offchain::{CreateSignedTransaction};
use pallet_file_bank::RandomFileList;
use pallet_sminer::MinerControl;
use pallet_storage_handler::StorageHandle;
use pallet_tee_worker::TeeWorkerHandler;
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::{ 
		convert:: { TryFrom, TryInto },
		prelude::*,
	};
use cp_cess_common::*;
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

enum AuditErr {
	RandomErr,
	QElementErr,
	SpaceParamErr,
}

impl sp_std::fmt::Debug for AuditErr {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			AuditErr::RandomErr => write!(fmt, "Unexpected error in generating random numbers!"),
			AuditErr::QElementErr => write!(fmt, ""),
			AuditErr::SpaceParamErr => write!(fmt, ""),
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
		type OneDay: Get<BlockNumberOf<Self>>;
		// one hours block
		#[pallet::constant]
		type OneHours: Get<BlockNumberOf<Self>>;
		// randomness for seeds.
		type MyRandomness: Randomness<Option<Self::Hash>, Self::BlockNumber>;
		//Find the consensus of the current block
		type FindAuthor: FindAuthor<Self::AccountId>;
		//Random files used to obtain this batch of challenges
		type File: RandomFileList<Self::AccountId>;
		//Judge whether it is the trait of the consensus node
		type TeeWorkerHandler: TeeWorkerHandler<Self::AccountId>;
		//It is used to increase or decrease the miners' computing power, space, and execute
		// punishment
		type MinerControl: MinerControl<Self::AccountId, Self::BlockNumber>;

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

		#[pallet::constant]
		type ReassignCeiling: Get<u8> + Clone + Eq + PartialEq;

		type CreditCounter: SchedulerCreditCounter<Self::AccountId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		GenerateChallenge,

		SubmitIdleProof { miner: AccountOf<T> },

		SubmitServiceProof { miner: AccountOf<T> },

		SubmitIdleVerifyResult { tee: AccountOf<T>, miner: AccountOf<T>, result: bool },

		SubmitServiceVerifyResult { tee: AccountOf<T>, miner: AccountOf<T>, result: bool },

		VerifyProof { tee_worker: AccountOf<T>, miner: AccountOf<T> },
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

		Expired,

		VerifyTeeSigFailed,

		BloomFilterError,

		Submitted,

		Challenging,

		RandomErr,

		UnSubmitted,
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
	pub(super) type Keys<T: Config> =
		StorageValue<_, WeakBoundedVec<T::AuthorityId, T::SessionKeyMax>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn challenge_proposal)]
	pub(super) type ChallengeProposal<T: Config> =
		CountedStorageMap<_, Blake2_128Concat, [u8; 32], (u32, ChallengeInfo<T>)>;

	#[pallet::storage]
	#[pallet::getter(fn counted_idle_failed)]
	pub(super) type CountedIdleFailed<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn counted_service_failed)]
	pub(super) type CountedServiceFailed<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn counted_clear)]
	pub(super) type CountedClear<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, u8, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn challenge_era)]
	pub(super) type ChallengeEra<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn unverify_idle_proof)]
	pub(super) type UnverifyIdleProof<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountOf<T>,
		BoundedVec<IdleProveInfo<T>, T::VerifyMissionMax>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn unverify_service_proof)]
	pub(super) type UnverifyServiceProof<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountOf<T>,
		BoundedVec<ServiceProveInfo<T>, T::VerifyMissionMax>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn verify_result)]
	pub(super) type VerifyResult<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, (Option<bool>, Option<bool>)>;

	#[pallet::storage]
	#[pallet::getter(fn verify_reassign_count)]
	pub(super) type VerifyReassignCount<T: Config> = StorageValue<_, u8, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn chllenge_snap_shot)]
	pub(super) type ChallengeSnapShot<T: Config> = StorageMap<_, Blake2_128Concat, AccountOf<T>, ChallengeInfo<T>>;

	#[pallet::storage]
	#[pallet::getter(fn challenge_slip)]
	pub(super) type ChallengeSlip<T: Config> = StorageDoubleMap<_, Blake2_128Concat, BlockNumberOf<T>, Blake2_128Concat, AccountOf<T>, bool>;
	
	#[pallet::storage]
	#[pallet::getter(fn verify_slip)]
	pub(super) type VerifySlip<T: Config> = StorageDoubleMap<_, Blake2_128Concat, BlockNumberOf<T>, Blake2_128Concat, AccountOf<T>, bool>;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberOf<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberOf<T>) -> Weight {
			let weight: Weight = Weight::zero();
			weight
				.saturating_add(Self::generate_challenge(now))
				.saturating_add(Self::clear_challenge(now))
				.saturating_add(Self::clear_verify_mission(now))
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
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
					return Err(Error::<T>::Expired)?;
				}

				if challenge_info.prove_info.idle_prove.is_none() {
					let tee_acc = Self::random_select_tee_acc(0)?;

					let idle_prove_info = IdleProveInfo::<T> {
						tee_acc: tee_acc.clone(),
						idle_prove,
						verify_result: None,
					};

					challenge_info.prove_info.idle_prove = Some(idle_prove_info);
				} else {
					return Err(Error::<T>::Submitted)?;
				}

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::SubmitIdleProof { miner: sender });

			Ok(())
		}

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
					return Err(Error::<T>::Expired)?;
				}

				if challenge_info.prove_info.service_prove.is_none() {
					let tee_acc = Self::random_select_tee_acc(0)?;

					let service_prove_info = ServiceProveInfo::<T> {
						tee_acc: tee_acc.clone(),
						service_prove,
						verify_result: None,
					};

					challenge_info.prove_info.service_prove = Some(service_prove_info);
				} else {
					return Err(Error::<T>::Submitted)?;
				}

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::SubmitServiceProof { miner: sender });

			Ok(())
		}

		#[pallet::call_index(3)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::submit_verify_idle_result())]
		pub fn submit_verify_idle_result(
			origin: OriginFor<T>,
			total_prove_hash: BoundedVec<u8, T::IdleTotalHashLength>,
			front: u64,
			rear: u64,
			accumulator: Accumulator,
			idle_result: bool,
			signature: TeeRsaSignature,
			tee_acc: AccountOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			<ChallengeSnapShot<T>>::try_mutate(&sender, |challenge_info_opt| -> DispatchResult {
				let challenge_info = challenge_info_opt.as_mut().ok_or(Error::<T>::NoChallenge)?;

				let idle_prove = challenge_info.prove_info.idle_prove.as_mut().ok_or(Error::<T>::UnSubmitted)?;

				if tee_acc != idle_prove.tee_acc {
					return Err(Error::<T>::NonExistentMission)?;
				}

				if let Some(_) = idle_prove.verify_result {
					return Err(Error::<T>::Submitted)?;
				}

				let MinerSnapShot {
					idle_space,
					service_space,
					service_bloom_filter: _,
					space_proof_info,
					tee_signature: _,
				} = &challenge_info.miner_snapshot;

				let verify_idle_info = VerifyIdleResultInfo::<T>{
					miner: sender.clone(),
					miner_prove: total_prove_hash.clone(),
					front: space_proof_info.front,
					rear: space_proof_info.rear,
					accumulator: space_proof_info.accumulator,
					space_challenge_param: challenge_info.challenge_element.space_param,
					result: idle_result,
					tee_acc: tee_acc.clone(),
				};

				let tee_puk = T::TeeWorkerHandler::get_tee_publickey()?;
				let encoding = verify_idle_info.encode();
				let hashing = sp_io::hashing::sha2_256(&encoding);
				ensure!(verify_rsa(&tee_puk, &hashing, &signature), Error::<T>::VerifyTeeSigFailed);

				let idle_result = Self::check_idle_verify_param(idle_result, front, rear, &total_prove_hash, &accumulator, &challenge_info.miner_snapshot, &idle_prove.idle_prove);

				idle_prove.verify_result = Some(idle_result);
			
				if let Some(service_prove) = &challenge_info.prove_info.service_prove {
					if let Some(service_result) = service_prove.verify_result {
						if idle_result && service_result {
							let total_idle_space = T::StorageHandle::get_total_idle_space();
							let total_service_space = T::StorageHandle::get_total_service_space();
	
							T::MinerControl::calculate_miner_reward(
								&sender,
								total_idle_space,
								total_service_space,
								*idle_space,
								*service_space,
							)?;
						}
					}
				}

				if idle_result {
					<CountedIdleFailed<T>>::insert(&sender, u32::MIN);
				} else {
					let count = <CountedIdleFailed<T>>::get(&sender).checked_add(1).unwrap_or(IDLE_FAULT_TOLERANT as u32);
					if count >= IDLE_FAULT_TOLERANT as u32 {
						T::MinerControl::idle_punish(&sender, *idle_space, *service_space)?;
					}
					<CountedIdleFailed<T>>::insert(&sender, count);
				}

				let count = challenge_info.miner_snapshot.space_proof_info.rear
					.checked_sub(challenge_info.miner_snapshot.space_proof_info.front).ok_or(Error::<T>::Overflow)?;
				T::CreditCounter::record_proceed_block_size(&tee_acc, count)?;

				Self::deposit_event(Event::<T>::SubmitIdleVerifyResult { tee: tee_acc.clone(), miner: sender.clone(), result: idle_result });

				Ok(())
			})
		}

		#[pallet::call_index(4)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::submit_verify_service_result())]
		pub fn submit_verify_service_result(
			origin: OriginFor<T>,
			service_result: bool,
			signature: TeeRsaSignature,
			service_bloom_filter: BloomFilter,
			tee_acc: AccountOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			<ChallengeSnapShot<T>>::try_mutate(&sender, |challenge_info_opt| -> DispatchResult {
				let challenge_info = challenge_info_opt.as_mut().ok_or(Error::<T>::NoChallenge)?;

				let service_prove = challenge_info.prove_info.service_prove.as_mut().ok_or(Error::<T>::UnSubmitted)?;
				if tee_acc != service_prove.tee_acc {
					return Err(Error::<T>::NonExistentMission)?;
				}

				if let Some(_) = service_prove.verify_result {
					return Err(Error::<T>::Submitted)?;
				}
				
				let MinerSnapShot {
					idle_space,
					service_space,
					service_bloom_filter: s_service_bloom_filter,
					space_proof_info: _,
					tee_signature: _,
				} = challenge_info.miner_snapshot;

				let verify_service_info = VerifyServiceResultInfo::<T>{
					miner: sender.clone(),
					tee_acc: tee_acc.clone(),
					miner_prove: service_prove.service_prove.clone(),
					result: service_result,
					chal: QElement {
						random_index_list: challenge_info.challenge_element.service_param.random_index_list.clone(),
						random_list: challenge_info.challenge_element.service_param.random_list.clone(),
					},
					service_bloom_filter: s_service_bloom_filter,
				};

				let tee_puk = T::TeeWorkerHandler::get_tee_publickey()?;
				let encoding = verify_service_info.encode();
				let hashing = sp_io::hashing::sha2_256(&encoding);
				ensure!(verify_rsa(&tee_puk, &hashing, &signature), Error::<T>::VerifyTeeSigFailed);

				ensure!(
					service_bloom_filter == s_service_bloom_filter,
					Error::<T>::BloomFilterError,
				); 

				service_prove.verify_result = Some(service_result);
			
				if let Some(idle_prove) = &challenge_info.prove_info.idle_prove {
					if let Some(idle_result) = idle_prove.verify_result {
						if idle_result && service_result {
							let total_idle_space = T::StorageHandle::get_total_idle_space();
							let total_service_space = T::StorageHandle::get_total_service_space();
	
							T::MinerControl::calculate_miner_reward(
								&sender,
								total_idle_space,
								total_service_space,
								idle_space,
								service_space,
							)?;
						}
					}
				}

				if service_result {
					<CountedServiceFailed<T>>::insert(&sender, u32::MIN);
				} else {
					let count = <CountedServiceFailed<T>>::get(&sender).checked_add(1).unwrap_or(SERVICE_FAULT_TOLERANT as u32);
					if count >= SERVICE_FAULT_TOLERANT as u32 {
						T::MinerControl::service_punish(&sender, service_space, service_space)?;
					}
					<CountedServiceFailed<T>>::insert(&sender, count);
				}

				let count = challenge_info.miner_snapshot.service_space
							.checked_div(IDLE_SEG_SIZE).ok_or(Error::<T>::Overflow)?
							.checked_add(1).ok_or(Error::<T>::Overflow)?;
				T::CreditCounter::record_proceed_block_size(&tee_acc, count as u64)?;

				Self::deposit_event(Event::<T>::SubmitServiceVerifyResult { tee: tee_acc.clone(), miner: sender.clone(), result: service_result });

				Ok(())
			})
		}

		#[pallet::call_index(7)]
		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn update_counted_clear(origin: OriginFor<T>, miner: AccountOf<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;

			<CountedClear<T>>::insert(&miner, 0);

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn clear_challenge(now: BlockNumberOf<T>) -> Weight {
			let mut weight: Weight = Weight::zero();

			for (miner, _) in <ChallengeSlip<T>>::iter_prefix(&now) {
				if let Ok(challenge_info) = <ChallengeSnapShot<T>>::try_get(&miner) {
					weight = weight.saturating_add(T::DbWeight::get().reads(1));
					if challenge_info.prove_info.idle_prove.is_none() 
						|| challenge_info.prove_info.service_prove.is_none() {
							let count = <CountedClear<T>>::get(&miner).checked_add(1).unwrap_or(6);
							weight = weight.saturating_add(T::DbWeight::get().reads(1));
		
							let _ = T::MinerControl::clear_punish(
								&miner, 
								count, 
								challenge_info.miner_snapshot.idle_space, 
								challenge_info.miner_snapshot.service_space
							);
							weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
							//For Testing
							if count >= 6 {
								let result = T::MinerControl::force_miner_exit(&miner);
								weight = weight.saturating_add(T::DbWeight::get().reads_writes(5, 5));
								if result.is_err() {
									log::info!("force clear miner: {:?} failed", miner);
								}
								<CountedClear<T>>::remove(&miner);
								weight = weight.saturating_add(T::DbWeight::get().writes(1));
							} else {
								<CountedClear<T>>::insert(
									&miner, 
									count,
								);
								weight = weight.saturating_add(T::DbWeight::get().writes(1));
							}
						}
				}

				<ChallengeSlip<T>>::remove(&now, &miner);
				weight = weight.saturating_add(T::DbWeight::get().writes(1));
			}

			weight
		}

		fn clear_verify_mission(now: BlockNumberOf<T>) -> Weight {
			let mut weight: Weight = Weight::zero();

			for (miner, _) in <VerifySlip<T>>::iter_prefix(&now) {
				weight = weight.saturating_add(T::DbWeight::get().reads(1));
				let mut flag = false;
				if let Ok(challenge_info) = <ChallengeSnapShot<T>>::try_get(&miner) {
					weight = weight.saturating_add(T::DbWeight::get().reads(1));
					if challenge_info.prove_info.assign == 2 {
						flag = true;
					}

					if challenge_info.prove_info.idle_prove.is_none()
						&& challenge_info.prove_info.service_prove.is_none() 
					{
						flag = true;
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
							if let Ok(new_tee) = Self::random_select_tee_acc(0) {
								let idle_prove = challenge_info.prove_info.idle_prove.as_mut().unwrap();
								idle_prove.tee_acc = new_tee;
							}
						}

						if challenge_info.prove_info.service_prove.is_some() {
							if let Ok(new_tee) = Self::random_select_tee_acc(0) {
								let service_prove = challenge_info.prove_info.service_prove.as_mut().unwrap();
								service_prove.tee_acc = new_tee;
							}
						}

						let max_space = {
							if challenge_info.miner_snapshot.idle_space > challenge_info.miner_snapshot.service_space {
								challenge_info.miner_snapshot.idle_space
							} else {
								challenge_info.miner_snapshot.service_space
							}
						};

						let one_hour: u32 = T::OneHours::get().saturated_into();
						let verify_life: u32 = max_space
							.saturating_add(one_hour as u128)
							.saturating_div(IDLE_VERIFY_RATE) as u32;
						
						let new_slip = now.saturating_add(verify_life.saturated_into());

						<VerifySlip<T>>::remove(&now, &miner);
						<VerifySlip<T>>::insert(&new_slip, &miner, true);
						<ChallengeSnapShot<T>>::insert(&miner, challenge_info);
						weight = weight.saturating_add(T::DbWeight::get().writes(3));
					}
				}
			}
			
			weight
		}

		fn generate_challenge(now: BlockNumberOf<T>) -> Weight {
			let mut weight: Weight = Weight::zero();
			
			weight = weight.saturating_add(T::DbWeight::get().reads(1));
			let miner_list = match T::MinerControl::get_all_miner() {
				Ok(miner_list) => miner_list,
				Err(_) => return weight,
			};

			let miner_count = miner_list.len() as u64;
			if miner_count == 0 {
				return weight;
			}

			let factor = match Self::random_number(now.saturated_into()) {
				Ok(factor) => factor,
				Err(_) => return weight,
			};
			let index = factor % miner_count;
			let miner = &miner_list[index as usize];

			if <ChallengeSnapShot<T>>::contains_key(miner) {
				return weight;
			}

			weight = weight.saturating_add(T::DbWeight::get().reads(1));
			let miner_snapshot = match T::MinerControl::get_miner_snapshot(miner) {
				Ok(miner_snapshot) => miner_snapshot,
				Err(_) => return weight,
			};

			let (idle_space, service_space, service_bloom_filter, space_proof_info, tee_signature) = miner_snapshot;

			let service_param = match Self::generate_miner_qelement(now.saturated_into()) {
				Ok(service_param) => service_param,
				Err(e) => {log::info!("audit: {:?}", e); return weight},
			};
			let space_param = match Self::generate_miner_space_param(now.saturated_into()) {
				Ok(space_param) => space_param,
				Err(e) => {log::info!("audit: {:?}", e); return weight},
			};

			let idle_life: u32 = (idle_space
				.saturating_div(IDLE_PROVE_RATE)
				.saturating_add(50)
			) as u32;
			let idle_slip = now.saturating_add(idle_life.saturated_into());

			let service_life: u32 = (service_space
				.saturating_div(SERVICE_PROVE_RATE)
				.saturating_add(50)
			) as u32;
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
			let tee_length = T::TeeWorkerHandler::get_controller_list().len();
			let verify_life: u32 = (idle_space
				.saturating_add(service_space)
				.saturating_div(IDLE_VERIFY_RATE)
				.saturating_div(tee_length as u128)
			) as u32;
			let verify_slip = max_slip
				.saturating_add(verify_life.saturated_into())
				.saturating_add(one_hour);

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
				}
			};

			weight = weight.saturating_add(T::DbWeight::get().writes(3));
			<ChallengeSnapShot<T>>::insert(&miner, challenge_info);
			<ChallengeSlip<T>>::insert(&max_slip, &miner, true);
			<VerifySlip<T>>::insert(&verify_slip, &miner, true);

			weight
		}

		fn random_select_tee_acc(mask: u32) -> Result<AccountOf<T>, DispatchError> {
			let tee_list = T::TeeWorkerHandler::get_controller_list();
			ensure!(tee_list.len() > 0, Error::<T>::SystemError);
		
			let seed: u32 = <frame_system::Pallet<T>>::block_number().saturated_into();
			let index = Self::random_number(seed + mask).map_err(|_| Error::<T>::RandomErr)? as u32;
			let index: u32 = index % (tee_list.len() as u32);
			let tee_acc = &tee_list[index as usize];

			Ok(tee_acc.clone())
		}

		fn generate_miner_qelement(seed: u32) -> Result<QElement, AuditErr> {
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
					return Err(AuditErr::QElementErr);
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
					return Err(AuditErr::QElementErr);
				}
			}

			Ok(QElement{random_index_list, random_list})
		}

		fn generate_miner_space_param(seed: u32) -> Result<SpaceChallengeParam, AuditErr> {
			// generate idle challenge param
			let (_, n, d) = T::MinerControl::get_expenders().map_err(|_| AuditErr::SpaceParamErr)?;
			let mut space_challenge_param: SpaceChallengeParam = Default::default();
			let mut repeat_filter: Vec<u64> = Default::default();
			let seed_multi: u32 = 5;
			let mut seed: u32 = seed.checked_mul(seed_multi).ok_or(AuditErr::SpaceParamErr)?;
			for elem in &mut space_challenge_param {
				let mut counter: usize = 0;
				loop {
					let random = Self::random_number(seed.checked_add(1).ok_or(AuditErr::SpaceParamErr)?)? % n;
					counter = counter.checked_add(1).ok_or(AuditErr::SpaceParamErr)?;
					
					if counter > repeat_filter.len() * 3 {
						return Err(AuditErr::SpaceParamErr);
					}

					let random = n
						.checked_mul(d).ok_or(AuditErr::SpaceParamErr)?
						.checked_add(random).ok_or(AuditErr::SpaceParamErr)?;
					if repeat_filter.contains(&random) {
						continue
					}
					repeat_filter.push(random);
					*elem = random;
					seed = seed.checked_add(1).ok_or(AuditErr::SpaceParamErr)?;
					break;
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
			let random_number = <u64>::decode(&mut random_seed.as_ref())
				.map_err(|_| AuditErr::RandomErr)?;
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
				let random_seed = <H256>::decode(&mut r_seed.as_ref())
					.map_err(|_| AuditErr::RandomErr)?;
				let random_vec = random_seed.as_bytes().to_vec();
				if random_vec.len() >= 20 {
					let random = random_vec[0..20].try_into().map_err(|_| AuditErr::RandomErr)?;
					return Ok(random);
				}
			}
		}

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
