//! # Sminer Module
//!
//! Contain operations related storage miners.
//!
//! ### Terminology
//!
//! * **Collateral:** The Staking amount when registering storage miner.
//! * **Earnings:** Store the storage miner's earnings during mining.
//! * **Locked:** Store the locked amount of the storage miner during mining.
//!
//! ### Interface
//!
//! ### Dispatchable Functions
//!
//! * `regnstk` - Staking and register for storage miner.
//! * `redeem` - Redeem and exit for storage miner.
//! * `claim` - Claim the rewards from storage miner's earnings.

#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{
	transactional, ensure,
	storage::bounded_vec::BoundedVec, PalletId,
	traits::{
		Currency,
		ExistenceRequirement::KeepAlive,
		Get, ReservableCurrency,
		schedule::{self, v3::Anon as ScheduleAnon, DispatchTime, v3::Named as ScheduleNamed}, 
	},
	dispatch::DispatchResult,
	pallet_prelude::DispatchError,
};
use frame_system::pallet_prelude::BlockNumberFor;
use cp_cess_common::*;
use sp_runtime::traits::Zero;
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, CheckedAdd, CheckedSub, CheckedMul, Dispatchable, SaturatedConversion},
	RuntimeDebug, Perbill
};
use sp_staking::StakingInterface;
use sp_std::{convert::TryInto, prelude::*, marker::PhantomData};
use sp_core::ConstU32;
use pallet_tee_worker::TeeWorkerHandler;
use pallet_reservoir::ReservoirGate;
use cp_bloom_filter::BloomFilter;
use pallet_storage_handler::StorageHandle;
use pallet_cess_treasury::{RewardPool, TreasuryHandle};
use ces_types::{WorkerPublicKey, TeeSig};

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub mod types;
pub use types::*;

mod constants;
use constants::*;

mod functions;
mod helper;

pub mod weights;
pub use weights::WeightInfo;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type ProposalCallOf<T> = <T as pallet::Config>::SProposal;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::{StorageValue, ValueQuery, *},
		traits::{Get, QueryPreimage, StorePreimage}, Blake2_128Concat,
	};
	use frame_system::{ensure_signed, pallet_prelude::*};

	#[pallet::config]
	pub trait Config: pallet_timestamp::Config + frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		type TeeWorkerHandler: TeeWorkerHandler<Self::AccountId, BlockNumberFor<Self>>;
		/// The treasury's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type FaucetId: Get<PalletId>;

		#[pallet::constant]
		type ItemLimit: Get<u32>;

		#[pallet::constant]
		type OneDayBlock: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type StakingLockBlock: Get<BlockNumberFor<Self>>;

		#[pallet::constant]
		type MaximumRelease: Get<u128>;

		/// The WeightInfo.
		type WeightInfo: WeightInfo;

		type RewardPool: RewardPool<AccountOf<Self>, BalanceOf<Self>>;

		type CessTreasuryHandle: TreasuryHandle<AccountOf<Self>, BalanceOf<Self>>;

		type FScheduler: ScheduleNamed<BlockNumberFor<Self>, Self::SProposal, Self::SPalletsOrigin, Hasher = Self::Hashing>;

		type AScheduler: ScheduleAnon<BlockNumberFor<Self>, Self::SProposal, Self::SPalletsOrigin, Hasher = Self::Hashing>;
		/// Overarching type of all pallets origins.
		type SPalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;
		/// The SProposal.
		type SProposal: Parameter + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin> + From<Call<Self>>;

		type StorageHandle: StorageHandle<Self::AccountId>;

		type ReservoirGate: ReservoirGate<Self::AccountId, BalanceOf<Self>>;

		type Staking: StakingInterface;

		/// The preimage provider with which we look up call hashes to get the call.
		type Preimages: QueryPreimage<H = Self::Hashing> + StorePreimage;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new account was set.
		Registered {
			acc: AccountOf<T>,
		},
		RegisterPoisKey {
			miner: AccountOf<T>,
		},
		/// Users to withdraw faucet money
		DrawFaucetMoney(),
		/// User recharges faucet
		FaucetTopUpMoney {
			acc: AccountOf<T>,
		},
		/// Prompt time
		LessThan24Hours {
			last: BlockNumberFor<T>,
			now: BlockNumberFor<T>,
		},
		//The miners have been frozen
		AlreadyFrozen {
			acc: AccountOf<T>,
		},

		IncreaseCollateral {
			acc: AccountOf<T>,
			balance: BalanceOf<T>,
		},
		/// Some funds have been deposited. \[deposit\]
		Deposit {
			balance: BalanceOf<T>,
		},
		UpdateBeneficiary {
			acc: AccountOf<T>,
			new: AccountOf<T>,
		},
		UpdatePeerId {
			acc: AccountOf<T>,
			old: PeerId,
			new: PeerId,
		},
		Receive {
			acc: AccountOf<T>,
			reward: BalanceOf<T>,
		},
		MinerExitPrep{ 
			miner: AccountOf<T>,
		},
		Withdraw { 
			acc: AccountOf<T> 
		},
		IncreaseDeclarationSpace {
			miner: AccountOf<T>,
			space: u128,
		},
	}

	/// Error for the sminer pallet.
	#[pallet::error]
	pub enum Error<T> {
		NotMiner,
		/// An account already registered.
		AlreadyRegistered,
		/// An operation would lead to an overflow.
		Overflow,
		/// User does not exist.
		NotExisted,
		/// The duration is less than 24 hours.
		LessThan24Hours,
		/// Numerical conversion error.
		ConversionError,
		/// The account has been frozen
		AlreadyFrozen,
		/// The miner is not in a positive state and cannot perform this operation
		NotpositiveState,
		/// The capacity of Vec has reached the upper limit, making it impossible to register new miners. In theory, this error should not occur
		StorageLimitReached,
		/// Convert bounded vec error 
		BoundedVecError,
		/// The recording error of the faucet does not occur under normal circumstances
		DataNotExist,
		/// According to business logic, errors that should not occur
		Unexpected,
		/// Verify tee signature error
		VerifyTeeSigFailed,
		/// Count error in spatial proof related data
		CountError,
		/// The status of the miner cannot call the current transaction
		StateError,
		/// Bloom filter update error
		BloomElemPushError,
		/// Insufficient pledged amount
		CollateralNotUp,
		/// Not the pledgor
		NotStakingAcc,
		/// System method errors that should not occur
		BugInvalid,
		/// The pledge period is insufficient and needs to reach 180 days
		InsufficientStakingPeriod,
		/// The certified space exceeds the declared space
		ExceedingDeclarationSpace,
		/// Insufficient idle space for replacement
		InsufficientReplaceable,
		/// Conversion tee signature error
		MalformedSignature,
		/// Chain does not initialize whitelist
		NotSetWhite,
		/// Wrong signature source
		WrongOrigin,
		/// Exceeding the maximum release volume of the faucet in one day
		ExceedRelease,
		
	}

	/// The hashmap for info of storage miners.
	#[pallet::storage]
	#[pallet::getter(fn miner_items)]
	pub(super) type MinerItems<T: Config> = CountedStorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		MinerInfo<T>,
	>;

	/// Store all miner information
	#[pallet::storage]
	#[pallet::getter(fn miner_info)]
	pub(super) type AllMiner<T: Config> =
		StorageValue<_, BoundedVec<AccountOf<T>, T::ItemLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn reward_map)]
	pub(super) type RewardMap<T: Config> = StorageMap<
		_, 
		Blake2_128Concat, 
		AccountOf<T>, 
		Reward<T>,
	>;

	/// The hashmap for checking registered or not.
	#[pallet::storage]
	#[pallet::getter(fn faucet_record)]
	pub(super) type FaucetRecordMap<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, FaucetRecord<BlockNumberFor<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn miner_public_key)]
	pub(super) type MinerPublicKey<T: Config> = StorageMap<_, Blake2_128Concat, [u8; 32], AccountOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn expenders)]
	pub(super) type Expenders<T: Config> = StorageValue<_, (u64, u64, u64)>;

	#[pallet::storage]
	#[pallet::getter(fn miner_lock)]
	pub(super) type MinerLock<T: Config> = 
		StorageMap<_, Blake2_128Concat, AccountOf<T>, BlockNumberFor<T>>;
	
	#[pallet::storage]
	#[pallet::getter(fn restoral_target)]
	pub(super) type RestoralTarget<T: Config> = 
		StorageMap<_, Blake2_128Concat, AccountOf<T>, RestoralTargetInfo<AccountOf<T>, BlockNumberFor<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn staking_start_block)]
	pub(super) type StakingStartBlock<T: Config> = 
		StorageMap<_, Blake2_128Concat, AccountOf<T>, BlockNumberFor<T>>;

	
	#[pallet::storage]
	#[pallet::getter(fn pending_replacements)]
		pub(super) type PendingReplacements<T: Config> = 
			StorageMap<_, Blake2_128Concat, AccountOf<T>, u128, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn facuet_release_record)]
		pub(super) type FacuetReleaseRecord<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;
	
	#[pallet::storage]
	#[pallet::getter(fn return_staking_schedule)]
		pub(super) type ReturnStakingSchedule<T: Config> = 
			StorageMap<
				_,
				Blake2_128Concat,
				BlockNumberFor<T>,
				BoundedVec<(AccountOf<T>, AccountOf<T>, BalanceOf<T>), T::ItemLimit>,
				ValueQuery,
			>;
	
	#[pallet::storage]
	#[pallet::getter(fn complete_snap_shot)]
		pub(super) type CompleteSnapShot<T: Config> = 
			StorageMap<
				_,
				Blake2_128Concat,
				u32,
				CompleteInfo,
				ValueQuery,
			>;
	
	#[pallet::storage]
	#[pallet::getter(fn complete_miner_snap_shot)]
		pub(super) type CompleteMinerSnapShot<T: Config> = 
			StorageMap<
				_,
				Blake2_128Concat,
				AccountOf<T>,
				BoundedVec<MinerCompleteInfo<BlockNumberFor<T>>, ConstU32<{RELEASE_NUMBER as u32}>>,
				ValueQuery,
			>;

	#[pallet::storage]
	#[pallet::getter(fn facuet_whitelist)]
		pub(super) type FacuetWhitelist<T: Config> = StorageValue<_, AccountOf<T>>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config>{
		pub expenders: (u64, u64, u64),
		_marker: PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				// FOR TESTING
				expenders: (80, 1024*1024, 64),
				_marker: PhantomData,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			Expenders::<T>::put(self.expenders);
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			let mut weight: Weight = Weight::zero();
			let miner_list = ReturnStakingSchedule::<T>::get(&now);
			weight = weight.saturating_add(T::DbWeight::get().reads(1));
			for (miner, staking_acc, collaterals) in miner_list {
				let spec_acc = T::ReservoirGate::get_reservoir_acc();
				if spec_acc == staking_acc {
					if let Err(e) = T::ReservoirGate::redeem(&miner, collaterals, false) {
						log::error!("return staking [Reservoir]error: {:?}", e);
					}
					weight = weight.saturating_add(T::DbWeight::get().reads_writes(2, 2));
				}
				T::Currency::unreserve(&staking_acc, collaterals);
				weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
			}

			weight
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register as a Miner and Stake Collateral
		///
		/// This function allows an account to register as a Miner in the network and stake a certain amount of collateral to participate in the mining process. 
		/// The registration also includes specifying a beneficiary account, providing a unique peer ID for network identification, 
		/// and submitting a proof of space (PoS) key along with a TeeRSA signature for verification.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization. Typically, this is the account that wishes to register as a Miner.
		/// - `beneficiary`: The account that will receive the rewards for mining.
		/// - `peer_id`: A unique identifier for the Miner on the network.
		/// - `staking_val`: The amount of collateral that the Miner is staking to participate in mining.
		/// - `pois_key`: The PoS key provided by the Miner.
		/// - `tee_sig`: A TeeRSA signature for verifying the authenticity of the PoS key.
		#[pallet::call_index(0)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::regnstk())]
		pub fn regnstk(
			origin: OriginFor<T>,
			beneficiary: AccountOf<T>,
			peer_id: PeerId,
			staking_val: BalanceOf<T>,
			tib_count: u32,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!(<MinerItems<T>>::contains_key(&sender)), Error::<T>::AlreadyRegistered);
			let declaration_space = T_BYTE.checked_mul(tib_count as u128).ok_or(Error::<T>::Overflow)?;
			let base_limit: BalanceOf<T> = Self::calculate_limit_by_space(declaration_space)?
				.try_into().map_err(|_| Error::<T>::Overflow)?;
			ensure!(staking_val >= base_limit.try_into().map_err(|_| Error::<T>::Overflow)?, Error::<T>::CollateralNotUp);
			T::Currency::reserve(&sender, staking_val)?;

			let now = <frame_system::Pallet<T>>::block_number();
			<StakingStartBlock<T>>::insert(&sender, now);

			<MinerItems<T>>::insert(
				&sender,
				MinerInfo::<T> {
					beneficiary: beneficiary.clone(),
					staking_account: sender.clone(),
					peer_id: peer_id,
					collaterals: staking_val,
					debt: BalanceOf::<T>::zero(),
					state: Self::str_to_bound(STATE_NOT_READY)?,
					declaration_space: declaration_space,
					idle_space: u128::MIN,
					service_space: u128::MIN,
					lock_space: u128::MIN,
					space_proof_info: Option::None,			
					service_bloom_filter: Default::default(),
					tee_signature: Default::default(), // sp_core::sr25519::Signature([0u8; 64]),
				},
			);

			RewardMap::<T>::insert(
				&sender,
				Reward::<T>{
					total_reward: 0u32.saturated_into(),
					reward_issued: 0u32.saturated_into(),
					order_list: Default::default()
				},
			);

			Self::deposit_event(Event::<T>::Registered {
				acc: sender,
			});
			
			Ok(())
		}

		/// Increase Collateral and Update Miner's State
		///
		/// This function allows a registered Miner to increase their staked collateral. 
		/// The additional collateral is used to support the Miner's participation in the network and may affect the Miner's state.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization. Typically, this is the account that is a registered Miner.
		/// - `collaterals`: The amount of additional collateral the Miner wants to stake. It is specified as a `BalanceOf<T>` value, typically a token balance(Miner's TCESS).
		#[pallet::call_index(1)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::increase_collateral())]
		pub fn increase_collateral(
			origin: OriginFor<T>,
			miner: AccountOf<T>,
			#[pallet::compact] collaterals: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(MinerItems::<T>::contains_key(&miner), Error::<T>::NotMiner);

			let mut balance: BalanceOf<T> = 0u32.saturated_into();
			<MinerItems<T>>::try_mutate(&miner, |miner_info_opt| -> DispatchResult {
				let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
				let spec_acc = T::ReservoirGate::get_reservoir_acc();

				ensure!(miner_info.state.to_vec() != STATE_OFFLINE.as_bytes().to_vec(), Error::<T>::StateError);
				ensure!(miner_info.state.to_vec() != STATE_LOCK.as_bytes().to_vec(), Error::<T>::StateError);
				ensure!(miner_info.state.to_vec() != STATE_EXIT.as_bytes().to_vec(), Error::<T>::StateError);
				ensure!(miner_info.staking_account == sender || miner_info.staking_account == spec_acc, Error::<T>::NotStakingAcc);

				if miner_info.staking_account == spec_acc {
					ensure!(sender == miner, Error::<T>::NotStakingAcc);
					T::ReservoirGate::check_qualification(&sender, collaterals)?;
				}

				let mut remaining = collaterals;
				if miner_info.debt > BalanceOf::<T>::zero() {
					if miner_info.debt > collaterals {
						miner_info.debt = miner_info.debt.checked_sub(&collaterals).ok_or(Error::<T>::Overflow)?;
						remaining = BalanceOf::<T>::zero();
						if miner_info.staking_account == spec_acc {
							T::CessTreasuryHandle::send_to_pid(miner_info.staking_account.clone(), collaterals)?;
						} else {
							T::CessTreasuryHandle::send_to_pid(sender.clone(), collaterals)?;
						}
					} else {
						remaining = remaining.checked_sub(&miner_info.debt).ok_or(Error::<T>::Overflow)?;
						if miner_info.staking_account == spec_acc {
							T::CessTreasuryHandle::send_to_pid(miner_info.staking_account.clone(), miner_info.debt)?;
						} else {
							T::CessTreasuryHandle::send_to_pid(sender.clone(), miner_info.debt)?;
						}
						
						miner_info.debt = BalanceOf::<T>::zero();
					}
				}

				miner_info.collaterals =
					miner_info.collaterals.checked_add(&remaining).ok_or(Error::<T>::Overflow)?;

				balance = miner_info.collaterals;

				if miner_info.state == STATE_FROZEN.as_bytes().to_vec() {
					let limit = Self::calculate_limit_by_space(miner_info.declaration_space)?
						.try_into().map_err(|_| Error::<T>::Overflow)?;
					if miner_info.collaterals >= limit {
						miner_info.state = Self::str_to_bound(STATE_POSITIVE)?;
					}
				}

				let now = <frame_system::Pallet<T>>::block_number();
				<StakingStartBlock<T>>::insert(&miner, now);

				if miner_info.staking_account == spec_acc {
					T::ReservoirGate::staking(&miner, remaining, true)?;
				} else {
					T::Currency::reserve(&sender, remaining)?;
				}


				Ok(())
			})?;

			Self::deposit_event(Event::<T>::IncreaseCollateral { acc: sender, balance });
			Ok(())
		}

		/// Update Miner's Beneficiary Account
		///
		/// This function allows a registered Miner to update their beneficiary account. 
		/// The beneficiary account is the recipient of the Miner's rewards and earnings from the network.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization. Typically, this is the account that is a registered Miner.
		/// - `beneficiary`: The new beneficiary account to be associated with the Miner.
		#[pallet::call_index(2)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::update_beneficiary())]
		pub fn update_beneficiary(
			origin: OriginFor<T>,
			beneficiary: AccountOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);

			<MinerItems<T>>::try_mutate(&sender, |miner_info_opt| -> DispatchResult {
				let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
				miner_info.beneficiary = beneficiary.clone();
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::UpdateBeneficiary { acc: sender, new: beneficiary });
			Ok(())
		}

		/// Update Miner's Peer ID
		///
		/// This function allows a registered Miner to update their Peer ID, which is used for network communication and connectivity.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization. Typically, this is the account that is a registered Miner.
		/// - `peer_id`: The new Peer ID to be associated with the Miner for network communication.
		#[pallet::call_index(3)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::update_peer_id())]
		pub fn update_peer_id(origin: OriginFor<T>, peer_id: PeerId) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);

			let old = <MinerItems<T>>::try_mutate(&sender, |miner_info_opt| -> Result<PeerId, DispatchError> {
				let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
				let old = miner_info.peer_id.clone();
				miner_info.peer_id = peer_id.clone();
				Ok(old)
			})?;

			Self::deposit_event(Event::<T>::UpdatePeerId { acc: sender, old, new: peer_id.into() });
			Ok(())
		}


		/// Receive Miner's Reward
		///
		/// This function allows a registered Miner in a positive state to receive their available rewards from the reward pool. 
		/// It transfers the rewards to the Miner's beneficiary.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization. Typically, this is the account of a registered Miner.
		#[pallet::call_index(6)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::receive_reward(90))]
		pub fn receive_reward(
			origin: OriginFor<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			if let Ok(miner) = <MinerItems<T>>::try_get(&sender) {
				ensure!(
					miner.state == STATE_POSITIVE.as_bytes().to_vec() || miner.state == STATE_EXIT.as_bytes().to_vec(),
					Error::<T>::NotpositiveState
				);
				Self::calculate_miner_reward(&sender)?;
				Self::distribute_rewards(&sender, miner.beneficiary)?;
			}

			Ok(())
		}

		/// Prepare Miner Exit
		///
		/// This function allows a registered Miner to initiate the process of exiting the system gracefully. 
		/// It transitions the Miner from a positive state to a locked state, effectively preparing for the exit. 
		/// During the locked state, the Miner's resources will be locked for a specific period, 
		/// ensuring that they fulfill any existing challenges before they can exit completely.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller's authorization. Typically, this is the account of a registered Miner.
		#[pallet::call_index(7)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::miner_exit_prep())]
		pub fn miner_exit_prep(
			origin: OriginFor<T>,
			miner: AccountOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let now = <frame_system::Pallet<T>>::block_number();
			if let Ok(lock_time) = <MinerLock<T>>::try_get(&miner) {
				ensure!(now > lock_time, Error::<T>::StateError);
			}
			let staking_start_block = <StakingStartBlock<T>>::try_get(&miner).map_err(|_| Error::<T>::BugInvalid)?;
			let staking_lock_block = T::StakingLockBlock::get();
			ensure!(now > staking_start_block + staking_lock_block, Error::<T>::InsufficientStakingPeriod);

			<MinerItems<T>>::try_mutate(&sender, |miner_info_opt| -> DispatchResult {

				let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::NotExisted)?;
				if (&sender != &miner) && (&sender != &miner_info.staking_account) {
					Err(Error::<T>::NotStakingAcc)?;
				}
				ensure!(miner_info.state == STATE_POSITIVE.as_bytes().to_vec(), Error::<T>::StateError);
				ensure!(miner_info.lock_space == 0, Error::<T>::StateError);
				if miner_info.lock_space != 0 {
					Err(Error::<T>::StateError)?;
				}

				miner_info.state = Self::str_to_bound(STATE_LOCK)?;

				Ok(())
			})?;

			let now = <frame_system::Pallet<T>>::block_number();
			// TODO! Develop a lock-in period based on the maximum duration of the current challenge
			let lock_time = T::OneDayBlock::get()
				.checked_mul(&15u32.saturated_into()).ok_or(Error::<T>::Overflow)?
				.checked_add(&now).ok_or(Error::<T>::Overflow)?;

			<MinerLock<T>>::insert(&miner, lock_time);

			let task_id = miner.using_encoded(sp_io::hashing::blake2_256);			
			let call = T::Preimages::bound(ProposalCallOf::<T>::from(Call::miner_exit { miner: miner.clone() } ))?;
			T::FScheduler::schedule_named(
                task_id,
                DispatchTime::At(lock_time),
                Option::None,
                schedule::HARD_DEADLINE,
                frame_system::RawOrigin::Root.into(),
                call, 
        	).map_err(|_| Error::<T>::Unexpected)?;

			Self::deposit_event(Event::<T>::MinerExitPrep{ miner: miner });

			Ok(())
		}

		/// Miner Exit
		///
		/// This function allows a privileged entity, typically a root user, 
		/// to finalize the exit process for a Miner who has previously initiated the exit preparation using the `miner_exit_prep` function. 
		/// It transitions the Miner from a locked state to an exited state, effectively allowing the Miner to leave the system gracefully.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, ensuring the caller has root privileges.
		/// - `miner`: The account of the Miner who is exiting the system.
		#[pallet::call_index(8)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::miner_exit())]
		pub fn miner_exit(
			origin: OriginFor<T>,
			miner: AccountOf<T>,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			// judge lock state.
			let miner_info = <MinerItems<T>>::try_get(&miner).map_err(|_| Error::<T>::NotExisted)?;	
			ensure!(miner_info.state.to_vec() == STATE_LOCK.as_bytes().to_vec(), Error::<T>::StateError);
			// sub network total idle space.

			T::StorageHandle::sub_total_idle_space(miner_info.idle_space)?;

			Self::execute_exit(&miner)?;

			Self::create_restoral_target(&miner, miner_info.service_space + miner_info.lock_space)?;

			Ok(())
		}

		/// Miner Withdraw
		///
		/// This function allows a Miner to withdraw their assets and rights from the network after their exit has been finalized, 
		/// and any necessary cool-down period has passed.
		///
		/// Parameters:
		/// - `origin`: The origin from which the function is called, representing the Miner's account.
		#[pallet::call_index(9)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::miner_withdraw())]
		pub fn miner_withdraw(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let restoral_info = <RestoralTarget<T>>::try_get(&sender).map_err(|_| Error::<T>::StateError)?;
			let now = <frame_system::Pallet<T>>::block_number();

			if now < restoral_info.cooling_block && restoral_info.restored_space != restoral_info.service_space {
				Err(Error::<T>::StateError)?;
			}

			Self::withdraw(sender.clone())?;

			Self::deposit_event(Event::<T>::Withdraw {
				acc: sender,
			});

			Ok(())
		}
		
		/// Punish offline miners.
		///
		/// The dispatch origin of this call must be _root_.
		///
		/// Parameters:
		/// - `acc`: miner .
		/// The faucet top up.
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// Parameters:
		/// - `acc`: Top-up account .
		/// - `acc`: Top-up amount .
		#[pallet::call_index(13)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::faucet_top_up())]
		pub fn faucet_top_up(origin: OriginFor<T>, award: BalanceOf<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let reward_pot = T::FaucetId::get().into_account_truncating();
			<T as pallet::Config>::Currency::transfer(&sender, &reward_pot, award, KeepAlive)?;

			Self::deposit_event(Event::<T>::FaucetTopUpMoney { acc: sender.clone() });
			Ok(())
		}

		/// Users receive money through the faucet.
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// Parameters:
		/// - `acc`: Withdraw money account.
		/// FOR TEST
		#[pallet::call_index(14)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::faucet())]
		pub fn faucet(origin: OriginFor<T>, to: AccountOf<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			
			let white = <FacuetWhitelist<T>>::try_get().map_err(|_| Error::<T>::NotSetWhite)?;
			ensure!(sender == white, Error::<T>::WrongOrigin);

			let facuet_value: BalanceOf<T> = FAUCET_VALUE.try_into().map_err(|_e| Error::<T>::ConversionError)?;

			if !<FaucetRecordMap<T>>::contains_key(&to) {
				<FaucetRecordMap<T>>::insert(
					&to,
					FaucetRecord::<BlockNumberFor<T>> {
						last_claim_time: BlockNumberFor::<T>::from(0u32),
					},
				);

				let now = <frame_system::Pallet<T>>::block_number();
				let reward_pot = T::FaucetId::get().into_account_truncating();

				<T as pallet::Config>::Currency::transfer(
					&reward_pot,
					&to,
					facuet_value,
					KeepAlive,
				)?;
				<FaucetRecordMap<T>>::insert(
					&to,
					FaucetRecord::<BlockNumberFor<T>> { last_claim_time: now },
				);
			} else {
				let one_day: u32 = T::OneDayBlock::get().saturated_into();
				let faucet_record = FaucetRecordMap::<T>::try_get(&to).map_err(|e| {
					log::error!("faucet error is: {:?}", e);
					Error::<T>::DataNotExist
				})?;
				let now = <frame_system::Pallet<T>>::block_number();

				let mut flag: bool = true;
				if now >= one_day.into() {
					if !(faucet_record.last_claim_time
						<= now
							.checked_sub(&BlockNumberFor::<T>::from(one_day))
							.ok_or(Error::<T>::Overflow)?)
					{
						Self::deposit_event(Event::<T>::LessThan24Hours {
							last: faucet_record.last_claim_time,
							now,
						});
						flag = false;
					}
				} else {
					if !(faucet_record.last_claim_time <= BlockNumberFor::<T>::from(0u32)) {
						Self::deposit_event(Event::<T>::LessThan24Hours {
							last: faucet_record.last_claim_time,
							now,
						});
						flag = false;
					}
				}
				ensure!(flag, Error::<T>::LessThan24Hours);

				let reward_pot = T::FaucetId::get().into_account_truncating();
				<FacuetReleaseRecord<T>>::try_mutate(|release_amount| -> DispatchResult {
					let max: BalanceOf<T> = T::MaximumRelease::get().try_into().map_err(|_| Error::<T>::ConversionError)?;
					*release_amount = release_amount.checked_add(&facuet_value).ok_or(Error::<T>::Overflow)?;
					ensure!(*release_amount <= max, Error::<T>::ExceedRelease);

					Ok(())
				})?;

				<T as pallet::Config>::Currency::transfer(
					&reward_pot,
					&to,
					facuet_value,
					KeepAlive,
				)?;

				<FaucetRecordMap<T>>::insert(
					&to,
					FaucetRecord::<BlockNumberFor<T>> { last_claim_time: now },
				);
			}

			Self::deposit_event(Event::<T>::DrawFaucetMoney());
			Ok(())
		}

		// FOR TESTING
		#[pallet::call_index(15)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
		pub fn update_expender(
			origin: OriginFor<T>, 
			k: u64, 
			n: u64, 
			d: u64,
		) -> DispatchResult {
			let _ = ensure_root(origin)?;

			Expenders::<T>::put((k, n, d));

			Ok(())
		}

		#[pallet::call_index(16)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::register_pois_key())]
		pub fn register_pois_key(
			origin: OriginFor<T>, 
			pois_key: PoISKey,
			tee_sig_need_verify: BoundedVec<u8, ConstU32<64>>,
			tee_sig: BoundedVec<u8, ConstU32<64>>,
			tee_puk: WorkerPublicKey, 
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			// Because the next operation consumes system resources, make a judgment in advance.
			ensure!(<MinerItems<T>>::contains_key(&sender), Error::<T>::NotMiner);

			let space_proof_info = SpaceProofInfo::<AccountOf<T>> {
				miner: sender.clone(),
				front: u64::MIN,
				rear: u64::MIN,
				pois_key: pois_key.clone(),
				accumulator: pois_key.g,
			};

			let encoding = space_proof_info.encode();
			let tee_puk_encode = tee_puk.encode();
			let mut original = Vec::new();
			original.extend_from_slice(&encoding);
			original.extend_from_slice(&tee_puk_encode);
			let original_text = sp_io::hashing::sha2_256(&original);
			let master_puk = T::TeeWorkerHandler::get_master_publickey()?;

			let sig = 
				sp_core::sr25519::Signature::try_from(tee_sig_need_verify.as_slice()).or(Err(Error::<T>::MalformedSignature))?;

			ensure!(
				sp_io::crypto::sr25519_verify(&sig, &original_text, &master_puk),
				Error::<T>::VerifyTeeSigFailed
			);

			let now = <frame_system::Pallet<T>>::block_number();
			T::TeeWorkerHandler::update_work_block(now, &tee_puk)?;

			let sig = 
				sp_core::sr25519::Signature::try_from(tee_sig.as_slice()).or(Err(Error::<T>::MalformedSignature))?;

			MinerPublicKey::<T>::insert(&original_text, sender.clone());

			<MinerItems<T>>::try_mutate(&sender, |info_opt| -> DispatchResult {
				let miner_info = info_opt.as_mut().ok_or(Error::<T>::NotMiner)?;
				ensure!(STATE_NOT_READY.as_bytes().to_vec() == miner_info.state.to_vec(), Error::<T>::StateError);

				miner_info.space_proof_info = Some(space_proof_info);
				let base_limit: BalanceOf<T> = Self::calculate_limit_by_space(miner_info.declaration_space)?
					.try_into().map_err(|_| Error::<T>::Overflow)?;
				if miner_info.collaterals >= base_limit {
					miner_info.state = Self::str_to_bound(STATE_POSITIVE)?;
				} else {
					miner_info.state = Self::str_to_bound(STATE_FROZEN)?;
				}
				miner_info.tee_signature = sig;

				Ok(())
			})?;

			AllMiner::<T>::try_mutate(|all_miner| -> DispatchResult {
				all_miner
					.try_push(sender.clone())
					.map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::RegisterPoisKey {
				miner: sender,
			});

			Ok(())
		}

		#[pallet::call_index(17)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::regnstk_assign_staking())]
		pub fn regnstk_assign_staking(
			origin: OriginFor<T>,
			beneficiary: AccountOf<T>,
			peer_id: PeerId,
			staking_account: AccountOf<T>,
			tib_count: u32,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!(<MinerItems<T>>::contains_key(&sender)), Error::<T>::AlreadyRegistered);
			let declaration_space = T_BYTE.checked_mul(tib_count as u128).ok_or(Error::<T>::Overflow)?;

			let mut need_staking = BalanceOf::<T>::zero();
			let spec_acc = T::ReservoirGate::get_reservoir_acc();			
			if staking_account == spec_acc {
				need_staking = (tib_count as u128)
					.checked_mul(BASE_UNIT).ok_or(Error::<T>::Overflow)?
					.try_into().map_err(|_| Error::<T>::Overflow)?;
				T::ReservoirGate::check_qualification(&sender, need_staking)?;
				T::ReservoirGate::staking(&sender, need_staking, true)?;
				let now = <frame_system::Pallet<T>>::block_number();
				<StakingStartBlock<T>>::insert(&sender, now);
			}

			<MinerItems<T>>::insert(
				&sender,
				MinerInfo::<T> {
					beneficiary: beneficiary.clone(),
					staking_account: staking_account.clone(),
					peer_id: peer_id,
					collaterals: need_staking,
					debt: BalanceOf::<T>::zero(),
					state: Self::str_to_bound(STATE_NOT_READY)?,
					declaration_space: declaration_space,
					idle_space: u128::MIN,
					service_space: u128::MIN,
					lock_space: u128::MIN,
					space_proof_info: Option::None,	
					service_bloom_filter: Default::default(),
					tee_signature: Default::default(), // sp_core::sr25519::Signature([0u8; 64]),
				},
			);

			RewardMap::<T>::insert(
				&sender,
				Reward::<T> {
					total_reward: 0u32.saturated_into(),
					reward_issued: 0u32.saturated_into(),
					
					order_list: Default::default()
				},
			);

			Self::deposit_event(Event::<T>::Registered {
				acc: sender,
			});
			
			Ok(())
		}

		#[pallet::call_index(18)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::increase_declaration_space())]
		pub fn increase_declaration_space(origin: OriginFor<T>, tib_count: u32) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);
			let increase_space = T_BYTE.checked_mul(tib_count as u128).ok_or(Error::<T>::Overflow)?;

			<MinerItems<T>>::try_mutate(&sender, |miner_info_opt| -> DispatchResult {
				let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;

				ensure!(miner_info.state.to_vec() == STATE_POSITIVE.as_bytes().to_vec(), Error::<T>::StateError);
				miner_info.declaration_space = miner_info.declaration_space
					.checked_add(increase_space).ok_or(Error::<T>::Overflow)?;
				let base_limit: BalanceOf<T> = Self::calculate_limit_by_space(miner_info.declaration_space)?
					.try_into().map_err(|_| Error::<T>::Overflow)?;
				if base_limit > miner_info.collaterals {
					miner_info.state = Self::str_to_bound(STATE_FROZEN)?;
				}
				
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::IncreaseDeclarationSpace {
				miner: sender,
				space: increase_space,
			});

			Ok(())
		}

		// FOR TESTING
		#[pallet::call_index(19)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
		pub fn clear_miner_service(origin: OriginFor<T>, miner: AccountOf<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;

			<MinerItems<T>>::try_mutate(&miner, |miner_info_opt| -> DispatchResult {
				let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
				miner_info.service_space = 0;
				miner_info.service_bloom_filter = Default::default();

				Ok(())
			})?;

			Ok(())
		}

		#[pallet::call_index(20)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
		pub fn set_facuet_whitelist(origin: OriginFor<T>, acc: AccountOf<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;

			<FacuetWhitelist<T>>::put(acc);

			Ok(())
		}
	}
}

pub trait MinerControl<AccountId, BlockNumber> {
	fn add_miner_idle_space(acc: &AccountId, accumulator: Accumulator, check_front: u64, rear: u64, tee_sig: TeeSig) -> Result<u128, DispatchError>;
	// fn sub_miner_idle_space(acc: &AccountId, accumulator: Accumulator, rear: u64) -> DispatchResult;
	fn delete_idle_update_accu(acc: &AccountId, accumulator: Accumulator, front: u64, check_rear: u64, tee_sig: TeeSig) -> Result<u64, DispatchError>;
	fn delete_idle_update_space(acc: &AccountId, idle_space: u128) -> DispatchResult;
	fn add_miner_service_space(acc: &AccountId, power: u128) -> DispatchResult;
	fn sub_miner_service_space(acc: &AccountId, power: u128) -> DispatchResult;

	fn get_power(acc: &AccountId) -> Result<(u128, u128), DispatchError>;
	fn miner_is_exist(acc: AccountId) -> bool;
	fn get_miner_state(acc: &AccountId) -> Result<Vec<u8>, DispatchError>;
	fn get_all_miner() -> Result<Vec<AccountId>, DispatchError>;
	// Associated functions related to uploading files.
	fn insert_service_bloom(acc: &AccountId, hash_list: Vec<Box<[u8; 256]>>) -> DispatchResult;
	fn delete_service_bloom(acc: &AccountId, hash_list: Vec<Box<[u8; 256]>>) -> DispatchResult;
	fn lock_space(acc: &AccountId, space: u128) -> DispatchResult;
	fn unlock_space(acc: &AccountId, space: u128) -> DispatchResult;
	fn unlock_space_direct(acc: &AccountId, space: u128) -> DispatchResult;
	fn unlock_space_to_service(acc: &AccountId, space: u128) -> DispatchResult;

	fn get_miner_idle_space(acc: &AccountId) -> Result<u128, DispatchError>;
	fn get_miner_count() -> u32;
	fn record_snap_shot(
		miner: &AccountId,
		miner_idle_space: u128,
		miner_service_space: u128,
	) -> DispatchResult;

	fn clear_punish(miner: &AccountId, idle_space: u128, service_space: u128, count: u8) -> DispatchResult;
	fn idle_punish(miner: &AccountId, idle_space: u128, service_space: u128) -> DispatchResult;
	fn service_punish(miner: &AccountId, idle_space: u128, service_space: u128) -> DispatchResult;

	fn force_miner_exit(acc: &AccountId) -> DispatchResult;

	fn update_restoral_target(miner: &AccountId, service_space: u128) -> DispatchResult;
	fn restoral_target_is_exist(miner: &AccountId) -> bool;

	fn is_positive(miner: &AccountId) -> Result<bool, DispatchError>;
	fn is_lock(miner: &AccountId) -> Result<bool, DispatchError>;
	fn update_miner_state(miner: &AccountId, state: &str) -> DispatchResult;
	fn get_expenders() -> Result<(u64, u64, u64), DispatchError>;
	fn get_miner_snapshot(miner: &AccountId) -> Result<(u128, u128, BloomFilter, SpaceProofInfo<AccountId>, TeeSig), DispatchError>;

	fn increase_replace_space(miner: &AccountId, space: u128) -> DispatchResult;
	fn decrease_replace_space(miner: &AccountId, space: u128) -> DispatchResult;
}

impl<T: Config> MinerControl<<T as frame_system::Config>::AccountId, BlockNumberFor<T>> for Pallet<T> {
	fn add_miner_idle_space(
		acc: &<T as frame_system::Config>::AccountId, 
		accumulator: Accumulator,  
		check_front: u64,
		rear: u64, 
		tee_sig: TeeSig,
	) -> Result<u128, DispatchError> {
		let idle_space = Pallet::<T>::add_miner_idle_space(
			acc, 
			accumulator, 
			check_front,
			rear, 
			tee_sig
		)?;
		Ok(idle_space)
	}

	fn delete_idle_update_accu(
		acc: &AccountOf<T>, 
		accumulator: Accumulator, 
		front: u64,
		check_rear: u64,
		tee_sig: TeeSig,
	) -> Result<u64, DispatchError> {
		let count = Self::delete_idle_update_accu(
			acc, 
			accumulator, 
			front, 
			check_rear,
			tee_sig
		)?;

		Ok(count)
	}

	fn delete_idle_update_space(acc: &AccountOf<T>, idle_space: u128) -> DispatchResult {
		Self::delete_idle_update_space(acc, idle_space)
	}

	fn add_miner_service_space(acc: &<T as frame_system::Config>::AccountId, power: u128) -> DispatchResult {
		Pallet::<T>::add_miner_service_space(acc, power)?;
		Ok(())
	}

	fn sub_miner_service_space(acc: &<T as frame_system::Config>::AccountId, power: u128) -> DispatchResult {
		Pallet::<T>::sub_miner_service_space(acc, power)?;
		Ok(())
	}

	fn get_power(
		acc: &AccountOf<T>,
	) -> Result<(u128, u128), DispatchError> {
		if !<MinerItems<T>>::contains_key(acc) {
			Err(Error::<T>::NotMiner)?;
		}
		//There is a judgment on whether the primary key exists above
		let miner = <MinerItems<T>>::try_get(acc).map_err(|_| Error::<T>::NotMiner)?;
		Ok((miner.idle_space, miner.service_space))
	}

	fn get_miner_idle_space(acc: &AccountOf<T>) -> Result<u128, DispatchError> {
		let miner = <MinerItems<T>>::try_get(acc).map_err(|_e| Error::<T>::NotExisted)?;
		Ok(miner.idle_space)
	}

	fn miner_is_exist(acc: <T as frame_system::Config>::AccountId) -> bool {
		if <MinerItems<T>>::contains_key(&acc) {
			return true;
		}
		false
	}

	fn get_miner_state(acc: &AccountOf<T>) -> Result<Vec<u8>, DispatchError> {
		let miner = <MinerItems<T>>::try_get(acc).map_err(|_| Error::<T>::NotMiner)?;
		Ok(miner.state.to_vec())
	}

	fn get_all_miner() -> Result<Vec<AccountOf<T>>, DispatchError> {
		Ok(AllMiner::<T>::get().to_vec())
	}

	fn insert_service_bloom(acc: &AccountOf<T>, hash_list: Vec<Box<[u8; 256]>>) -> DispatchResult {
		Self::insert_service_bloom(acc, hash_list)
	}

	fn delete_service_bloom(acc: &AccountOf<T>, hash_list: Vec<Box<[u8; 256]>>) -> DispatchResult {
		Self::delete_service_bloom(acc, hash_list)
	}

	fn lock_space(acc: &AccountOf<T>, space: u128) -> DispatchResult {
		<MinerItems<T>>::try_mutate(acc, |miner_opt| -> DispatchResult {
			let miner = miner_opt.as_mut().ok_or(Error::<T>::NotExisted)?;
			miner.lock_space = miner.lock_space.checked_add(space).ok_or(Error::<T>::Overflow)?;
			miner.idle_space = miner.idle_space.checked_sub(space).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})
	}

	fn unlock_space(acc: &AccountOf<T>, space: u128) -> DispatchResult {
		<MinerItems<T>>::try_mutate(acc, |miner_opt| -> DispatchResult {
			if let Ok(miner) = miner_opt.as_mut().ok_or(Error::<T>::NotExisted) {
				miner.lock_space = miner.lock_space.checked_sub(space).ok_or(Error::<T>::Overflow)?;
				miner.idle_space = miner.idle_space.checked_add(space).ok_or(Error::<T>::Overflow)?;
			}
			Ok(())
		})
	}

	fn unlock_space_direct(acc: &AccountOf<T>, space: u128) -> DispatchResult {
		<MinerItems<T>>::try_mutate(acc, |miner_opt| -> DispatchResult {
			if let Ok(miner) = miner_opt.as_mut().ok_or(Error::<T>::NotExisted) {
				miner.lock_space = miner.lock_space.checked_sub(space).ok_or(Error::<T>::Overflow)?;
			}
			Ok(())
		})
	}

	fn unlock_space_to_service(acc: &AccountOf<T>, space: u128) -> DispatchResult {
		<MinerItems<T>>::try_mutate(acc, |miner_opt| -> DispatchResult {
			let miner = miner_opt.as_mut().ok_or(Error::<T>::NotExisted)?;
			miner.lock_space = miner.lock_space.checked_sub(space).ok_or(Error::<T>::Overflow)?;
			miner.service_space = miner.service_space.checked_add(space).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})
	}

	fn get_miner_count() -> u32 {
		<MinerItems<T>>::count()
	}
	
	fn record_snap_shot(
		miner: &AccountOf<T>,
		miner_idle_space: u128,
		miner_service_space: u128,
	) -> DispatchResult {
		let now = frame_system::Pallet::<T>::block_number();
		let era = T::Staking::current_era();
		let power = Self::calculate_power(miner_idle_space, miner_service_space);

		<CompleteMinerSnapShot<T>>::mutate(miner, |miner_info_list| -> DispatchResult {
			let snap_shot = MinerCompleteInfo::<BlockNumberFor<T>> {
				era_index: era,
				issued: false,
				finsh_block: now,
				power: power,
			};

			if miner_info_list.len() == RELEASE_NUMBER as usize {
				return Ok(())
			}

			<CompleteSnapShot<T>>::mutate(&era, |complete_info| -> DispatchResult {
				complete_info.miner_count = complete_info.miner_count.checked_add(1).ok_or(Error::<T>::Overflow)?;
				complete_info.total_power = complete_info.total_power.checked_add(power).ok_or(Error::<T>::Overflow)?;
	
				Ok(())
			})?;

			miner_info_list.try_push(snap_shot).map_err(|_| Error::<T>::Overflow)?;

			Ok(())
		})?;

		Ok(())
	}

	fn clear_punish(
		miner: &AccountOf<T>, 
		idle_space: u128, 
		service_space: u128,
		count: u8,
	) -> DispatchResult {
		Self::clear_punish(miner, idle_space, service_space, count)
	}

	fn idle_punish(
		miner: &AccountOf<T>, 
		idle_space: u128, 
		service_space: u128
	) -> DispatchResult {
		Self::idle_punish(miner, idle_space, service_space)
	}

	fn service_punish(
		miner: &AccountOf<T>, 
		idle_space: u128, 
		service_space: u128
	) -> DispatchResult {
		Self::service_punish(miner, idle_space, service_space)
	}

	fn is_positive(miner: &AccountOf<T>) -> Result<bool, DispatchError> {
		let state = Self::get_miner_state(miner)?;
		Ok(state == STATE_POSITIVE.as_bytes().to_vec())
	}

	fn is_lock(miner: &AccountOf<T>) -> Result<bool, DispatchError> {
		let state = Self::get_miner_state(miner)?;
		Ok(state == STATE_LOCK.as_bytes().to_vec())
	}

	fn update_miner_state(miner: &AccountOf<T>, state: &str) -> DispatchResult {
		let state = match state {
			STATE_POSITIVE | STATE_FROZEN | STATE_EXIT | STATE_LOCK => state.as_bytes().to_vec(),
			_ => Err(Error::<T>::Overflow)?,
		};

		<MinerItems<T>>::try_mutate(miner, |miner_opt| -> DispatchResult {
			let miner_info = miner_opt.as_mut().ok_or(Error::<T>::NotMiner)?;

			miner_info.state = state.try_into().map_err(|_| Error::<T>::BoundedVecError)?;

			Ok(())
		})
	}

	fn force_miner_exit(acc: &AccountOf<T>) -> DispatchResult {
		Self::force_miner_exit(acc)
	}

	fn get_expenders() -> Result<(u64, u64, u64), DispatchError> {
		let expenders = Expenders::<T>::try_get().map_err(|_| Error::<T>::Unexpected)?;

		Ok(expenders)
	}

	fn get_miner_snapshot(miner: &AccountOf<T>) -> Result<(u128, u128, BloomFilter, SpaceProofInfo<AccountOf<T>>, TeeSig), DispatchError> {
		if !<MinerItems<T>>::contains_key(miner) {
			Err(Error::<T>::NotMiner)?;
		}
		//There is a judgment on whether the primary key exists above
		let miner_info = <MinerItems<T>>::try_get(miner).map_err(|_| Error::<T>::NotMiner)?;
		let space_proof_info = miner_info.space_proof_info.ok_or(Error::<T>::Unexpected)?;
		Ok((miner_info.idle_space, miner_info.service_space, miner_info.service_bloom_filter, space_proof_info, miner_info.tee_signature))
	}

	fn update_restoral_target(miner: &AccountOf<T>, service_space: u128) -> DispatchResult {
		Self::update_restoral_target(miner, service_space)
	}

	fn restoral_target_is_exist(miner: &AccountOf<T>) -> bool {
		RestoralTarget::<T>::contains_key(miner)
	}

	fn increase_replace_space(miner: &AccountOf<T>, space: u128) -> DispatchResult {
		<PendingReplacements<T>>::try_mutate(&miner, |pending_space| -> DispatchResult {
			*pending_space = pending_space
                    .checked_add(space).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})
	}

	fn decrease_replace_space(miner: &AccountOf<T>, space: u128) -> DispatchResult {
		<PendingReplacements<T>>::try_mutate(&miner, |pending_space| -> DispatchResult {
			if *pending_space / IDLE_SEG_SIZE == 0 {
				Err(Error::<T>::InsufficientReplaceable)?;
			}

			*pending_space = pending_space
                    .checked_sub(space).ok_or(Error::<T>::Overflow)?;

			Ok(())
		})
	}
}
