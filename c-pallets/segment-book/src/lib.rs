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

mod impls;
use impls::*;

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
		FindAuthor, Randomness, ReservableCurrency, EstimateNextSessionRotation, Currency,
		ValidatorSetWithIdentification, ValidatorSet, OneSessionHandler,
	},
	PalletId, WeakBoundedVec, BoundedSlice,
};
use serde_json::Value;
use sp_core::{
	crypto::KeyTypeId,
	offchain::OpaqueNetworkState,
};
use sp_application_crypto::{
	ecdsa::{Signature as SgxSignature, Public}, 
};
use sp_io::hashing::sha2_256;
use sp_runtime::app_crypto::RuntimeAppPublic;
use frame_system::offchain::{CreateSignedTransaction, SubmitTransaction};
use pallet_file_bank::RandomFileList;
use pallet_file_map::ScheduleFind;
use pallet_sminer::MinerControl;
use scale_info::TypeInfo;
use cp_bloom_filter::BloomFilter;
use sp_core::H256;
use sp_std::{collections::btree_map::BTreeMap, prelude::*};
pub mod weights;
pub use weights::WeightInfo;
use cp_cess_common::Hash as H68;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

type Hash = H68;
pub const SEGMENT_BOOK: KeyTypeId = KeyTypeId(*b"cess");
// type FailureRate = u32;

pub mod sr25519 {
	mod app_sr25519 {
		use crate::*;
		use sp_runtime::app_crypto::{app_crypto, sr25519};
		app_crypto!(sr25519, SEGMENT_BOOK);
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
	SendTransaction,
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
			OffchainErr::SendTransaction => write!(fmt, "Failed to send transaction"),
		}
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use cp_cess_common::SLICE_DEFAULT_BYTE;
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
		type ChallengeMaximum: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type RandomLimit: Get<u32> + Clone + Eq + PartialEq;

		#[pallet::constant]
		type SubmitProofLimit: Get<u32> + Clone + Eq + PartialEq;

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
		type MinerControl: MinerControl<Self::AccountId, <<Self as pallet::Config>::Currency as Currency<Self::AccountId>>::Balance>;
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
		ChallengeStart { total_power: u128, reward: BalanceOf<T> },

		SubmitReport { miner: AccountOf<T> },

		ForceClearMiner { miner: AccountOf<T> },

		PunishMiner { miner: AccountOf<T> },
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

		VerifyFailed,

		ParseError,

		BloomVerifyError,
	}

	#[pallet::storage]
	#[pallet::getter(fn challenge_snapshot)]
	pub type ChallengeSnapshot<T: Config> = StorageValue<
		_,
		NetworkSnapshot<T>,
	>;

	//Information about storage challenges
	#[pallet::storage]
	#[pallet::getter(fn miner_snapshot_map)]
	pub type MinerSnapshotMap<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountOf<T>,
		MinerSnapshot<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn miner_clear_coount)]
	pub type MinerClearCount<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		AccountOf<T>,
		u8,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn cur_authority_index)]
	pub(super) type CurAuthorityIndex<T: Config> = StorageValue<_, u16, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn keys)]
	pub(super) type Keys<T: Config> = StorageValue<_, WeakBoundedVec<T::AuthorityId, T::StringLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn lock)]
	pub(super) type Lock<T: Config> = StorageValue<_, bool, ValueQuery>;

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

			let mut weight: Weight = 0;
			//The waiting time for the challenge has reached the deadline
			let netsnapshot = match <ChallengeSnapshot<T>>::get() {
				Some(snapshot) => snapshot,
				None => return weight,
			};

			if now == netsnapshot.deadline {
				// clear start
				for (miner_acc, _snapshot) in <MinerSnapshotMap<T>>::iter() {
					// Get the number of consecutive penalties
					let count = <MinerClearCount<T>>::get(&miner_acc) + 1;
					<MinerClearCount<T>>::insert(&miner_acc, count);
					weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
					// clear punishment
					match T::MinerControl::miner_clear_punish(miner_acc.clone(), count) {
						Ok(weight_temp) => {
							Self::deposit_event(Event::<T>::PunishMiner { miner: miner_acc.clone() });
							weight = weight.saturating_add(weight_temp);
						},
						Err(e) => {
							log::warn!("clear punish miner failed: {:?}", e)
						},
					};
					// The current number of consecutive punishments reaches 3, 
					// and the miner will be kicked out
					if count >= 3 {
						let weight_temp = T::File::clear_miner_file(miner_acc.clone());
						weight = weight.saturating_add(weight_temp);

						match T::MinerControl::force_clear_miner(miner_acc.clone()) {
							Ok(weight_temp) => {
								Self::deposit_event(Event::<T>::ForceClearMiner { miner: miner_acc.clone() });
								<MinerClearCount<T>>::remove(miner_acc.clone());
								weight = weight.saturating_add(weight_temp);
							},
							Err(e) => {
								log::warn!("force clear miner failed: {:?}", e)
							},
						};
					}

					<MinerSnapshotMap<T>>::remove(miner_acc);
				}
			}

			weight
		}

		fn offchain_worker(now: T::BlockNumber) {
			let deadline = match <ChallengeSnapshot<T>>::get() {
				Some(snapshot) => snapshot.deadline,
				None => 0u32.saturated_into(),
			};
			if sp_io::offchain::is_validator() {
				if now > deadline {
					//Determine whether to trigger a challenge
					// if Self::trigger_challenge(now) {
						log::info!("offchain worker challenge start");
						if let Err(e) = Self::offchain_work_start(now) {
							match e {
								OffchainErr::Working => log::info!("offchain working, Unable to perform a new round of work."),
								// If the execution of the offline working machine fails, 
								// it will be executed again, up to 5 times.
								OffchainErr::SendTransaction => {
									log::info!("offchain send transcation failed: {:?}", e);
									let mut count = 0;
									while let Err(e) = Self::offchain_work_start(now) {
										match e {
											OffchainErr::SendTransaction => {
												if count > 5 {
													break;
												}
													count = count + 1;
											}
											_ => break,
										};
									}
								},

								_ => log::info!("offchain execution failed: {:?}", e),
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
		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn re_challenge(origin: OriginFor<T>) -> DispatchResult {
			let _sender = ensure_signed(origin)?;

			<ChallengeSnapshot<T>>::kill();

			Ok(())
		}

		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn update_block(origin: OriginFor<T>, block: BlockNumberOf<T>) -> DispatchResult {
			let _sender = ensure_signed(origin)?;

			<ChallengeSnapshot<T>>::try_mutate(|s_opt| -> DispatchResult {
				let s = s_opt.as_mut().unwrap();
				s.deadline = block;
				Ok(())
			})?;

			Ok(())
		}

		#[transactional]
		#[pallet::weight(100_000_000)]
		pub fn submit_challenge_result(origin: OriginFor<T>, report: ChallengeReport<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let net_snapshot = match <ChallengeSnapshot<T>>::get() {
				Some(snapshot) => snapshot,
				None => return Err(Error::<T>::VerifyFailed)?,
			};
			// 1. Whether miners need challenges this round
			let miner_snapshot = <MinerSnapshotMap<T>>::try_get(&sender).map_err(|_| Error::<T>::NoChallenge)?;
			// 2. Verify signatrue
			let pk = T::MinerControl::get_public(&sender)?;
			let result = sp_io::crypto::ecdsa_verify_prehashed(&report.signature, &sha2_256(&report.message), &pk);
			ensure!(result, Error::<T>::VerifyFailed);
			// 3. Parse message
			let report_message = report.parse()?;
			// 4. Compare bloomfilter
			ensure!(report_message.idle_filter == miner_snapshot.idle_filter, Error::<T>::BloomVerifyError);
			ensure!(report_message.service_filter == miner_snapshot.service_filter, Error::<T>::BloomVerifyError);
			ensure!(report_message.autonomy_filter == miner_snapshot.autonomy_filter, Error::<T>::BloomVerifyError);

			ensure!(report_message.random == net_snapshot.random, Error::<T>::VerifyFailed);
			// 5. Processing failed data segments
			let failed_idle_count = report_message.failed_idle_file.len();
			for file_hash in report_message.failed_idle_file {
				T::File::challenge_clear_idle(sender.clone(), file_hash)?;
			}

			let failed_service_count = report_message.failed_service_file.len();
			for shard_id in report_message.failed_service_file {
				T::File::challenge_clear_service(sender.clone(), shard_id)?;
			}

			let failed_autonomy_count = report_message.failed_autonomy_file.len();
			for file_hash in report_message.failed_autonomy_file {
				T::File::challenge_clear_autonomy(sender.clone(), file_hash)?;
			}
			// If there are failure segments, punish the miners.
			if failed_idle_count != 0 || failed_service_count != 0 || failed_autonomy_count != 0 {
				T::MinerControl::miner_slice_punish(sender.clone(), failed_idle_count as u64, failed_service_count as u64, failed_autonomy_count as u64)?;
			}
			// 6. Calculate reward
			let network_snapshot = <ChallengeSnapshot<T>>::get().ok_or(Error::<T>::NoChallenge)?;
			T::MinerControl::calculate_reward(network_snapshot.reward, sender.clone(), network_snapshot.total_power, miner_snapshot.power)?;

			<MinerSnapshotMap<T>>::remove(&sender);
			<MinerClearCount<T>>::remove(&sender);

			Self::deposit_event(Event::<T>::SubmitReport { miner: sender });

			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
		pub fn save_challenge_info(
			origin: OriginFor<T>,
			_seg_digest: SegDigest<BlockNumberOf<T>>,
			_signature: <T::AuthorityId as RuntimeAppPublic>::Signature,
			miner_acc: AccountOf<T>,
			challenge_info: MinerSnapshot<T>,
		) -> DispatchResult {
			ensure_none(origin)?;
			// let now = <frame_system::Pallet<T>>::block_number();
			MinerSnapshotMap::<T>::insert(&miner_acc, challenge_info);

			Ok(())
		}

		#[transactional]
		#[pallet::weight(0)]
		pub fn save_challenge_time(
			origin: OriginFor<T>,
			snapshot: NetworkSnapshot<T>,
			_seg_digest: SegDigest<BlockNumberOf<T>>,
			_signature: <T::AuthorityId as RuntimeAppPublic>::Signature,
		) -> DispatchResult {
			ensure_none(origin)?;

			ChallengeSnapshot::<T>::put(snapshot.clone());
			
			let max = Keys::<T>::get().len() as u16;
			let mut index = CurAuthorityIndex::<T>::get();
			if index >= max - 1 {
				index = 0;
			} else {
				index = index + 1;
			}
			CurAuthorityIndex::<T>::put(index);

			Self::deposit_event(Event::<T>::ChallengeStart { total_power: snapshot.total_power, reward: snapshot.reward});

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
				snapshot: _,
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

		//Obtain the consensus of the current block
		pub fn get_current_scheduler() -> Result<AccountOf<T>, DispatchError> {
			let digest = <frame_system::Pallet<T>>::digest();
			let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
			let acc = T::FindAuthor::find_author(pre_runtime_digests).map(|a| a);
			let acc = match acc {
				Some(acc) => T::Scheduler::get_controller_acc(acc),
				None => T::Scheduler::get_first_controller()?,
			};
			Ok(acc)
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
		fn generation_challenge() -> Result<BTreeMap<AccountOf<T>, MinerSnapshot<T>>, DispatchError> {
			log::info!("-----------------genearion challenge start---------------------");
			let result = T::MinerControl::all_miner_snapshot();

			let mut new_challenge_map: BTreeMap<AccountOf<T>, MinerSnapshot<T>> = BTreeMap::new();
			for (miner_acc, bloom, power) in result {
				let miner_snapshot = MinerSnapshot::<T> {
					power: power,
					miner_acc: miner_acc.clone(),
					idle_filter: bloom.idle_filter,
					service_filter: bloom.service_filter,
					autonomy_filter: bloom.autonomy_filter,
				};

				new_challenge_map.insert(miner_acc, miner_snapshot);
			}

			Ok(new_challenge_map)
		}

		fn offchain_work_start(now: BlockNumberOf<T>) -> Result<(), OffchainErr> {
			log::info!("get loacl authority...");
			let (authority_id, validators_index, validators_len) = Self::get_authority()?;
			log::info!("get loacl authority success!");
			if !Self::check_working(&now, &authority_id) {
				return Err(OffchainErr::Working);
			}
			log::info!("get challenge data...");
			let challenge_map = Self::generation_challenge().map_err(|e| {
				log::error!("generation challenge error:{:?}", e);
				OffchainErr::GenerateInfoError
			})?;
			log::info!("get challenge success!");
			log::info!("submit challenge to chain...");
			Self::offchain_call_extrinsic(now, authority_id, challenge_map, validators_index, validators_len)?;
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
			challenge_map: BTreeMap<AccountOf<T>, MinerSnapshot<T>>,
			validators_index: u16,
			validators_len: usize,
		) -> Result<(), OffchainErr> {
			let mut max_power: u128 = 0;
			for (miner_acc, snapshot) in challenge_map {
				if snapshot.power  > max_power {
					max_power = snapshot.power;
				}
				let (signature, digest) = Self::offchain_sign_digest(now, &authority_id, validators_index, validators_len)?;
				let call = Call::save_challenge_info {
					seg_digest: digest.clone(),
					signature: signature.clone(),
					miner_acc: miner_acc.clone(),
					challenge_info: snapshot.clone(),
				};

				let result = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());

				if result.is_err() {
					log::error!(
						"failure: offchain_signed_tx: tx miner_id"
					);
					Self::offchain_abnormal_exit(OffchainErr::SendTransaction, &authority_id)?;
				}
			}

			let one_hour: u32 = T::OneHours::get().saturated_into();
			let duration: BlockNumberOf<T> = max_power
				.checked_div(SLICE_DEFAULT_BYTE)
				.ok_or(OffchainErr::Overflow)?
				.checked_add(one_hour.into())
				.ok_or(OffchainErr::Overflow)?
				.saturated_into();

			let total_power = T::MinerControl::get_total_power();

			let random_time_number = Self::generate_random_number(20221215);

			let start_block = <frame_system::Pallet<T>>::block_number();

			let reward = T::MinerControl::get_current_reward();
			
			let deadline = start_block.checked_add(&duration).ok_or(OffchainErr::Overflow)?;
			// Record the current network snapshot
			let net_shot = NetworkSnapshot::<T> {
				total_power: total_power,
				reward: reward,
				random: random_time_number,
				start: start_block,
				deadline: deadline,
			};

			let (signature, digest) = Self::offchain_sign_digest(now, &authority_id, validators_index, validators_len)?;

			let call = Call::save_challenge_time {
				snapshot: net_shot,
				seg_digest: digest.clone(),
				signature: signature,
			};

			let result = SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into());

			if result.is_err() {
				log::error!(
					"failure: offchain_signed_tx: save challenge time, digest is: {:?}",
					digest
				);
				Self::offchain_abnormal_exit(OffchainErr::SendTransaction, &authority_id)?;
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
		// It is used to handle the abnormal end of the working machine under the chain
		// Local storage will be cleared
		fn offchain_abnormal_exit(err: OffchainErr, authority_id: &T::AuthorityId) -> Result<(), OffchainErr> {
			let key = authority_id.encode();
			let mut storage = StorageValueRef::persistent(&key);
			storage.clear();

			Err(err)
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
			let random_seed = match random_seed {
				Some(v) => v,
				None => Default::default(),
			};
			let random_number = <u64>::decode(&mut random_seed.as_ref())
				.expect("secure hashes should always be bigger than u32; qed");
			random_number
		}

		//The number of pieces generated is VEC
		fn generate_random_number(seed: u32) -> [u8; 20] {
			loop {
				let (r_seed, _) =
					T::MyRandomness::random(&(T::MyPalletId::get(), seed).encode());
				let r_seed = match r_seed {
					Some(v) => v,
					None => Default::default(),
				};
				let random_seed = <H256>::decode(&mut r_seed.as_ref())
					.expect("secure hashes should always be bigger than u32; qed");
				let random_vec = random_seed.as_bytes().to_vec();
				if random_vec.len() >= 20 {
					let temp: &[u8] = &random_vec;
					let result: [u8; 20] = (temp[0 .. 20]).try_into().expect("random try_from error!");
					// .try_into() {
					// 	Ok(random) => random,
					// 	Err(e) => {
					// 		log::info!("random slice try_into err: {:?}, \n slice: {:?}", e, random_vec);
					// 		continue;
					// 	},
					// };
					return result;
				}
			}
		}

		pub fn vec_to_bounded(param: Vec<Vec<u8>>) -> Result<BoundedList<T>, DispatchError> {
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
