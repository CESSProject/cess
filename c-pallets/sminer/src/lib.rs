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
		Get, Imbalance, OnUnbalanced, ReservableCurrency,
		schedule::{self, Anon as ScheduleAnon, DispatchTime, Named as ScheduleNamed}, 
	},
	dispatch::{Dispatchable, DispatchResult},
	pallet_prelude::DispatchError,
};
use cp_cess_common::*;
use sp_runtime::traits::Zero;
use codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, CheckedAdd, CheckedSub, CheckedDiv, SaturatedConversion},
	RuntimeDebug, Perbill,
};
use sp_std::{convert::TryInto, prelude::*};
use sp_core::ConstU32;
use cp_enclave_verify::verify_rsa;
use pallet_tee_worker::TeeWorkerHandler;
use cp_bloom_filter::BloomFilter;
use pallet_storage_handler::StorageHandle;
use pallet_sminer_reward::RewardPool;

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
type NegativeImbalanceOf<T> = <<T as pallet::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::{ValueQuery, *},
		traits::Get,
	};
	use frame_system::{ensure_signed, pallet_prelude::*};

	#[pallet::config]
	pub trait Config: pallet_timestamp::Config + frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		type TeeWorkerHandler: TeeWorkerHandler<Self::AccountId>;
		/// The treasury's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// The treasury's pallet id, used for deriving its sovereign account ID.
		#[pallet::constant]
		type FaucetId: Get<PalletId>;

		#[pallet::constant]
		type ItemLimit: Get<u32>;

		#[pallet::constant]
		type OneDayBlock: Get<BlockNumberOf<Self>>;
		/// The WeightInfo.
		type WeightInfo: WeightInfo;

		type RewardPool: RewardPool<BalanceOf<Self>>;

		type FScheduler: ScheduleNamed<Self::BlockNumber, Self::SProposal, Self::SPalletsOrigin>;

		type AScheduler: ScheduleAnon<Self::BlockNumber, Self::SProposal, Self::SPalletsOrigin>;
		/// Overarching type of all pallets origins.
		type SPalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;
		/// The SProposal.
		type SProposal: Parameter + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin> + From<Call<Self>>;

		type StorageHandle: StorageHandle<Self::AccountId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new account was set.
		Registered {
			acc: AccountOf<T>,
			staking_val: BalanceOf<T>,
		},
		/// Users to withdraw faucet money
		DrawFaucetMoney(),
		/// User recharges faucet
		FaucetTopUpMoney {
			acc: AccountOf<T>,
		},
		/// Prompt time
		LessThan24Hours {
			last: BlockNumberOf<T>,
			now: BlockNumberOf<T>,
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
		//The account has been frozen
		AlreadyFrozen,

		LockInNotOver,

		NotpositiveState,

		StorageLimitReached,

		BoundedVecError,

		DataNotExist,
		//haven't bought space at all
		NotPurchasedPackage,

		Unexpected,

		NoReward,

		VerifyTeeSigFailed,

		CountError,

		LowerOperationBlock,

		StateError,

		BloomElemPushError,
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
		StorageMap<_, Blake2_128Concat, T::AccountId, FaucetRecord<BlockNumberOf<T>>>;

	#[pallet::storage]
	#[pallet::getter(fn miner_public_key)]
	pub(super) type MinerPublicKey<T: Config> = StorageMap<_, Blake2_128Concat, [u8; 32], AccountOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn expenders)]
	pub(super) type Expenders<T: Config> = StorageValue<_, (u64, u64, u64)>;

	#[pallet::storage]
	#[pallet::getter(fn miner_lock)]
	pub(super) type MinerLock<T: Config> = 
		StorageMap<_, Blake2_128Concat, AccountOf<T>, BlockNumberOf<T>>;
	
	#[pallet::storage]
	#[pallet::getter(fn restoral_target)]
	pub(super) type RestoralTarget<T: Config> = 
		StorageMap< _, Blake2_128Concat, AccountOf<T>, RestoralTargetInfo<AccountOf<T>, BlockNumberOf<T>>>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::genesis_config]
	pub struct GenesisConfig{
		pub expenders: (u64, u64, u64),
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self {
				// FOR TESTING
				expenders: (8, 1024*1024, 64),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			Expenders::<T>::put(self.expenders);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Staking and register for storage miner.
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// Parameters:
		/// - `beneficiary`: The beneficiary related to signer account.
		/// - `ip`: The registered IP of storage miner.
		/// - `staking_val`: The number of staking.
		#[pallet::call_index(0)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::regnstk())]
		pub fn regnstk(
			origin: OriginFor<T>,
			beneficiary: AccountOf<T>,
			peer_id: PeerId,
			staking_val: BalanceOf<T>,
			pois_key: PoISKey,
			tee_sig: TeeRsaSignature,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!(<MinerItems<T>>::contains_key(&sender)), Error::<T>::AlreadyRegistered);
			T::Currency::reserve(&sender, staking_val)?;

			let space_proof_info = SpaceProofInfo::<AccountOf<T>> {
				miner: sender.clone(),
				front: u64::MIN,
				rear: u64::MIN,
				pois_key: pois_key.clone(),
				accumulator: pois_key.g,
			};

			let encoding = space_proof_info.encode();
			let original_text = sp_io::hashing::sha2_256(&encoding);
			let tee_puk = T::TeeWorkerHandler::get_tee_publickey()?;
			ensure!(verify_rsa(&tee_puk, &original_text, &tee_sig), Error::<T>::VerifyTeeSigFailed);

			MinerPublicKey::<T>::insert(&original_text, sender.clone());

			<MinerItems<T>>::insert(
				&sender,
				MinerInfo::<T> {
					beneficiary: beneficiary.clone(),
					peer_id: peer_id,
					collaterals: staking_val,
					debt: BalanceOf::<T>::zero(),
					state: Self::str_to_bound(STATE_POSITIVE)?,
					idle_space: u128::MIN,
					service_space: u128::MIN,
					lock_space: u128::MIN,
					space_proof_info,			
					service_bloom_filter: Default::default(),
					tee_signature: tee_sig,
				},
			);

			AllMiner::<T>::try_mutate(|all_miner| -> DispatchResult {
				all_miner
					.try_push(sender.clone())
					.map_err(|_e| Error::<T>::StorageLimitReached)?;
				Ok(())
			})?;

			RewardMap::<T>::insert(
				&sender,
				Reward::<T>{
					total_reward: 0u32.saturated_into(),
					reward_issued: 0u32.saturated_into(),
					currently_available_reward: 0u32.saturated_into(),
					order_list: Default::default()
				},
			);

			Self::deposit_event(Event::<T>::Registered {
				acc: sender,
				staking_val: staking_val,
			});
			
			Ok(())
		}

		/// Increase the miner collateral.
		///
		/// Parameters:
		/// - `collaterals`: Miner's TCESS.
		#[pallet::call_index(1)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::increase_collateral())]
		pub fn increase_collateral(
			origin: OriginFor<T>,
			#[pallet::compact] collaterals: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);

			let mut balance: BalanceOf<T> = 0u32.saturated_into();
			<MinerItems<T>>::try_mutate(&sender, |miner_info_opt| -> DispatchResult {
				let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;

				ensure!(miner_info.state.to_vec() != STATE_OFFLINE.as_bytes().to_vec(), Error::<T>::StateError);
				ensure!(miner_info.state.to_vec() != STATE_LOCK.as_bytes().to_vec(), Error::<T>::StateError);
				ensure!(miner_info.state.to_vec() != STATE_EXIT.as_bytes().to_vec(), Error::<T>::StateError);

				let mut remaining = collaterals;
				if miner_info.debt > BalanceOf::<T>::zero() {
					if miner_info.debt > collaterals {
						miner_info.debt = miner_info.debt.checked_sub(&collaterals).ok_or(Error::<T>::Overflow)?;
						remaining = BalanceOf::<T>::zero();
						T::RewardPool::add_reward(collaterals)?;
					} else {
						remaining = remaining.checked_sub(&miner_info.debt).ok_or(Error::<T>::Overflow)?;
						T::RewardPool::add_reward(miner_info.debt)?;
						miner_info.debt = BalanceOf::<T>::zero();
					}
				}

				miner_info.collaterals =
					miner_info.collaterals.checked_add(&remaining).ok_or(Error::<T>::Overflow)?;

				balance = miner_info.collaterals;

				if miner_info.state == STATE_FROZEN.as_bytes().to_vec() {
					let power = Self::calculate_power(miner_info.idle_space, miner_info.service_space);
					let limit = Self::check_collateral_limit(power)?;
					if miner_info.collaterals >= limit {
						miner_info.state = Self::str_to_bound(STATE_POSITIVE)?;
					}
				}

				T::Currency::reserve(&sender, remaining)?;

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::IncreaseCollateral { acc: sender, balance });
			Ok(())
		}

		/// update miner beneficiary.
		///
		/// Parameters:
		/// - `beneficiary`: The beneficiary related to signer account.
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

		/// update miner IP.
		///
		/// Parameters:
		/// - `ip`: The registered IP of storage miner.
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

		#[pallet::call_index(6)]
		#[transactional]
		#[pallet::weight(100_000_000_000)]
		pub fn receive_reward(
			origin: OriginFor<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			if let Ok(miner) = <MinerItems<T>>::try_get(&sender) {
				ensure!(
					miner.state == STATE_POSITIVE.as_bytes().to_vec(),
					Error::<T>::NotpositiveState
				);

				<RewardMap<T>>::try_mutate(&sender, |opt_reward| -> DispatchResult {
					let reward = opt_reward.as_mut().ok_or(Error::<T>::Unexpected)?;
					ensure!(reward.currently_available_reward != 0u32.saturated_into(), Error::<T>::NoReward);

					let reward_pot = T::PalletId::get().into_account_truncating();
					<T as pallet::Config>::Currency::transfer(&reward_pot, &miner.beneficiary, reward.currently_available_reward.clone(), KeepAlive)?;

					reward.reward_issued = reward.reward_issued
						.checked_add(&reward.currently_available_reward).ok_or(Error::<T>::Overflow)?;

					Self::deposit_event(Event::<T>::Receive {
						acc: sender.clone(),
						reward: reward.currently_available_reward,
					});

					reward.currently_available_reward = 0u32.saturated_into();

					Ok(())
				})?;
			}

			Ok(())
		}

		// The lock in time must be greater than the survival period of the challenge
		#[pallet::call_index(7)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::miner_exit_prep())]
		pub fn miner_exit_prep(
			origin: OriginFor<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			if let Ok(lock_time) = <MinerLock<T>>::try_get(&sender) {
				let now = <frame_system::Pallet<T>>::block_number();
				ensure!(now > lock_time, Error::<T>::StateError);
			}

			<MinerItems<T>>::try_mutate(&sender, |miner_opt| -> DispatchResult {
				let miner = miner_opt.as_mut().ok_or(Error::<T>::NotExisted)?;
				ensure!(miner.state == STATE_POSITIVE.as_bytes().to_vec(), Error::<T>::StateError);
				if miner.lock_space != 0 {
					Err(Error::<T>::StateError)?;
				}

				miner.state = Self::str_to_bound(STATE_LOCK)?;

				Ok(())
			})?;

			let now = <frame_system::Pallet<T>>::block_number();
			// TODO! Develop a lock-in period based on the maximum duration of the current challenge
			let lock_time = T::OneDayBlock::get().checked_add(&now).ok_or(Error::<T>::Overflow)?;

			<MinerLock<T>>::insert(&sender, lock_time);

			let task_id: Vec<u8> = sender.encode();
			T::FScheduler::schedule_named(
                task_id,
                DispatchTime::At(lock_time),
                Option::None,
                schedule::HARD_DEADLINE,
                frame_system::RawOrigin::Root.into(),
                Call::miner_exit{miner: sender.clone()}.into(), 
        	).map_err(|_| Error::<T>::Unexpected)?;

			Self::deposit_event(Event::<T>::MinerExitPrep{ miner: sender });

			Ok(())
		}

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

			Self::create_restoral_target(&miner, miner_info.service_space)?;

			Ok(())
		}

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

			Self::withdraw(&sender)?;

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
		#[pallet::call_index(14)]
		#[transactional]
		#[pallet::weight(35_000_000)]
		pub fn faucet(origin: OriginFor<T>, to: AccountOf<T>) -> DispatchResult {
			let _ = ensure_signed(origin)?;

			if !<FaucetRecordMap<T>>::contains_key(&to) {
				<FaucetRecordMap<T>>::insert(
					&to,
					FaucetRecord::<BlockNumberOf<T>> {
						last_claim_time: BlockNumberOf::<T>::from(0u32),
					},
				);

				let now = <frame_system::Pallet<T>>::block_number();
				let reward_pot = T::FaucetId::get().into_account_truncating();

				<T as pallet::Config>::Currency::transfer(
					&reward_pot,
					&to,
					FAUCET_VALUE.try_into().map_err(|_e| Error::<T>::ConversionError)?,
					KeepAlive,
				)?;
				<FaucetRecordMap<T>>::insert(
					&to,
					FaucetRecord::<BlockNumberOf<T>> { last_claim_time: now },
				);
			} else {
				let one_day: u32 = T::OneDayBlock::get().saturated_into();
				let faucet_record = FaucetRecordMap::<T>::try_get(&to).map_err(|e| {
					log::error!("faucet error is: {:?}", e);
					Error::<T>::DataNotExist
				})?;
				let now = <frame_system::Pallet<T>>::block_number();

				let mut flag: bool = true;
				if now >= BlockNumberOf::<T>::from(one_day) {
					if !(faucet_record.last_claim_time
						<= now
							.checked_sub(&BlockNumberOf::<T>::from(one_day))
							.ok_or(Error::<T>::Overflow)?)
					{
						Self::deposit_event(Event::<T>::LessThan24Hours {
							last: faucet_record.last_claim_time,
							now,
						});
						flag = false;
					}
				} else {
					if !(faucet_record.last_claim_time <= BlockNumberOf::<T>::from(0u32)) {
						Self::deposit_event(Event::<T>::LessThan24Hours {
							last: faucet_record.last_claim_time,
							now,
						});
						flag = false;
					}
				}
				ensure!(flag, Error::<T>::LessThan24Hours);

				let reward_pot = T::FaucetId::get().into_account_truncating();
				<T as pallet::Config>::Currency::transfer(
					&reward_pot,
					&to,
					FAUCET_VALUE.try_into().map_err(|_e| Error::<T>::ConversionError)?,
					KeepAlive,
				)?;
				<FaucetRecordMap<T>>::insert(
					&to,
					FaucetRecord::<BlockNumberOf<T>> { last_claim_time: now },
				);
			}

			Self::deposit_event(Event::<T>::DrawFaucetMoney());
			Ok(())
		}

		// FOR TESTING
		#[pallet::call_index(15)]
		#[transactional]
		#[pallet::weight(100_000)]
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
	}
}

impl<T: Config> OnUnbalanced<NegativeImbalanceOf<T>> for Pallet<T> {
	fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<T>) {
		let numeric_amount = amount.peek();

		// Must resolve into existing but better to be safe.
		let _ = T::Currency::resolve_creating(&T::PalletId::get().into_account_truncating(), amount);
		// The total issuance amount will not exceed u128::Max, so there is no overflow risk
		T::RewardPool::add_reward(numeric_amount).unwrap();

		Self::deposit_event(Event::Deposit { balance: numeric_amount });
	}
}

pub trait MinerControl<AccountId, BlockNumber> {
	fn add_miner_idle_space(acc: &AccountId, accumulator: Accumulator, rear: u64, tee_sig: TeeRsaSignature) -> Result<u128, DispatchError>;
	// fn sub_miner_idle_space(acc: &AccountId, accumulator: Accumulator, rear: u64) -> DispatchResult;
	fn delete_idle_update_accu(acc: &AccountId, accumulator: Accumulator, front: u64, tee_sig: TeeRsaSignature) -> Result<u64, DispatchError>;
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
	fn unlock_space_to_service(acc: &AccountId, space: u128) -> DispatchResult;

	fn get_miner_idle_space(acc: &AccountId) -> Result<u128, DispatchError>;
	fn get_miner_count() -> u32;
	fn calculate_miner_reward(
		miner: &AccountId, 
		total_idle_space: u128,
		total_service_space: u128,
		miner_idle_space: u128,
		miner_service_space: u128,
	) -> DispatchResult;

	fn clear_punish(miner: &AccountId, level: u8, idle_space: u128, service_space: u128) -> DispatchResult;
	fn idle_punish(miner: &AccountId, idle_space: u128, service_space: u128) -> DispatchResult;
	fn service_punish(miner: &AccountId, idle_space: u128, service_space: u128) -> DispatchResult;

	fn force_miner_exit(acc: &AccountId) -> DispatchResult;

	fn update_restoral_target(miner: &AccountId, service_space: u128) -> DispatchResult;
	fn restoral_target_is_exist(miner: &AccountId) -> bool;

	fn is_positive(miner: &AccountId) -> Result<bool, DispatchError>;
	fn is_lock(miner: &AccountId) -> Result<bool, DispatchError>;
	fn update_miner_state(miner: &AccountId, state: &str) -> DispatchResult;
	fn get_expenders() -> Result<(u64, u64, u64), DispatchError>;
	fn get_miner_snapshot(miner: &AccountId) -> Result<(u128, u128, BloomFilter, SpaceProofInfo<AccountId>, TeeRsaSignature), DispatchError>;
}

impl<T: Config> MinerControl<<T as frame_system::Config>::AccountId, BlockNumberOf<T>> for Pallet<T> {
	fn add_miner_idle_space(
		acc: &<T as frame_system::Config>::AccountId, 
		accumulator: Accumulator,  
		rear: u64, 
		tee_sig: TeeRsaSignature
	) -> Result<u128, DispatchError> {
		let idle_space = Pallet::<T>::add_miner_idle_space(
			acc, 
			accumulator, 
			rear, 
			tee_sig
		)?;
		Ok(idle_space)
	}

	fn delete_idle_update_accu(
		acc: &AccountOf<T>, 
		accumulator: Accumulator, 
		front: u64, 
		tee_sig: TeeRsaSignature
	) -> Result<u64, DispatchError> {
		let count = Self::delete_idle_update_accu(
			acc, 
			accumulator, 
			front, 
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

	fn calculate_miner_reward(
		miner: &AccountOf<T>, 
		total_idle_space: u128,
		total_service_space: u128,
		miner_idle_space: u128,
		miner_service_space: u128,
	) -> DispatchResult {
		Self::calculate_miner_reward(
			miner, 
			total_idle_space, 
			total_service_space, 
			miner_idle_space, 
			miner_service_space
		)
	}

	fn clear_punish(
		miner: &AccountOf<T>, 
		level: u8, 
		idle_space: u128, 
		service_space: u128
	) -> DispatchResult {
		Self::clear_punish(miner, level, idle_space, service_space)
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

	fn get_miner_snapshot(miner: &AccountOf<T>) -> Result<(u128, u128, BloomFilter, SpaceProofInfo<AccountOf<T>>, TeeRsaSignature), DispatchError> {
		if !<MinerItems<T>>::contains_key(miner) {
			Err(Error::<T>::NotMiner)?;
		}
		//There is a judgment on whether the primary key exists above
		let miner_info = <MinerItems<T>>::try_get(miner).map_err(|_| Error::<T>::NotMiner)?;
		Ok((miner_info.idle_space, miner_info.service_space, miner_info.service_bloom_filter, miner_info.space_proof_info, miner_info.tee_signature))
	}

	fn update_restoral_target(miner: &AccountOf<T>, service_space: u128) -> DispatchResult {
		Self::update_restoral_target(miner, service_space)
	}

	fn restoral_target_is_exist(miner: &AccountOf<T>) -> bool {
		RestoralTarget::<T>::contains_key(miner)
	}
}
