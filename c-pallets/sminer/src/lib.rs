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

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::{
	transactional,
	storage::bounded_vec::BoundedVec,
	traits::{
		schedule::{Anon as ScheduleAnon, Named as ScheduleNamed},
		Currency,
		ExistenceRequirement::KeepAlive,
		Get, Imbalance, OnUnbalanced, ReservableCurrency,
	},
};
use cp_cess_common::*;

use sp_runtime::traits::Zero;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod types;
use types::*;

mod constants;
use constants::*;

use codec::{Decode, Encode};
use frame_support::{
	ensure,
	dispatch::{DispatchResult, Dispatchable},
	pallet_prelude::DispatchError,
	PalletId,
};
use frame_system::{self as system};
pub use pallet::*;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, CheckedAdd, CheckedSub, SaturatedConversion},
	RuntimeDebug, Perbill,
};
use sp_std::{convert::TryInto, prelude::*};
use sp_core::ConstU32;
use cp_enclave_verify::verify_rsa;
use pallet_tee_worker::TeeWorkerHandler;

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

		#[pallet::constant]
		type ItemLimit: Get<u32>;
		#[pallet::constant]
		type MultipleFines: Get<u8>;
		#[pallet::constant]
		type DepositBufferPeriod: Get<u32>;
		#[pallet::constant]
		type OneDayBlock: Get<BlockNumberOf<Self>>;
		#[pallet::constant]
		type LockInPeriod: Get<u8>;
		#[pallet::constant]
		type MaxAward: Get<u128>;
		#[pallet::constant]
		type ChallengeMinerMax: Get<u32>;
		/// The Scheduler.
		type SScheduler: ScheduleNamed<Self::BlockNumber, Self::SProposal, Self::SPalletsOrigin>;

		type AScheduler: ScheduleAnon<Self::BlockNumber, Self::SProposal, Self::SPalletsOrigin>;
		/// Overarching type of all pallets origins.
		type SPalletsOrigin: From<system::RawOrigin<Self::AccountId>>;
		/// The SProposal.
		type SProposal: Parameter + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin> + From<Call<Self>>;
		/// The WeightInfo.
		type WeightInfo: WeightInfo;
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
		UpdataBeneficiary {
			acc: AccountOf<T>,
			new: AccountOf<T>,
		},
		UpdataIp {
			acc: AccountOf<T>,
			old: PeerId,
			new: PeerId,
		},
		Receive {
			acc: AccountOf<T>,
			reward: BalanceOf<T>,
		}
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
	}

	#[pallet::storage]
	#[pallet::getter(fn miner_lock_in)]
	pub(super) type MinerLockIn<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, BlockNumberOf<T>>;

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
	#[pallet::getter(fn currency_reward)]
	pub(super) type CurrencyReward<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn miner_public_key)]
	pub(super) type MinerPublicKey<T: Config> = StorageMap<_, Blake2_128Concat, Podr2Key, AccountOf<T>>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

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
			puk: Podr2Key,
			tee_sig: TeeRsaSignature,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!(<MinerItems<T>>::contains_key(&sender)), Error::<T>::AlreadyRegistered);
			T::Currency::reserve(&sender, staking_val)?;

			let tee_puk = T::TeeWorkerHandler::get_tee_publickey()?;
			ensure!(verify_rsa(&tee_puk, &puk, &tee_sig), Error::<T>::VerifyTeeSigFailed);

			MinerPublicKey::<T>::insert(&puk, sender.clone());

			<MinerItems<T>>::insert(
				&sender,
				MinerInfo::<T> {
					beneficiary: beneficiary.clone(),
					peer_id: peer_id,
					collaterals: staking_val,
					debt: BalanceOf::<T>::zero(),
					state: Self::vec_to_bound::<u8>(STATE_POSITIVE.as_bytes().to_vec())?,
					idle_space: u128::MIN,
					service_space: u128::MIN,
					lock_space: u128::MIN,
					puk,
					accumulator: [0u8; 256],
					last_operation_block: u32::MIN.saturated_into(),
					front: u64::MIN,
					rear: u64::MIN,
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

				let mut remaining = collaterals;
				if miner_info.debt > BalanceOf::<T>::zero() {
					if miner_info.debt > collaterals {
						miner_info.debt = miner_info.debt.checked_sub(&collaterals).ok_or(Error::<T>::Overflow)?;
						remaining = BalanceOf::<T>::zero();
					} else {
						remaining = remaining.checked_sub(&miner_info.debt).ok_or(Error::<T>::Overflow)?;
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
						miner_info.state = Self::vec_to_bound(STATE_POSITIVE.as_bytes().to_vec())?;
					}
				}

				T::Currency::reserve(&sender, remaining)?;

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::IncreaseCollateral { acc: sender, balance });
			Ok(())
		}

		/// updata miner beneficiary.
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

			Self::deposit_event(Event::<T>::UpdataBeneficiary { acc: sender, new: beneficiary });
			Ok(())
		}

		/// updata miner IP.
		///
		/// Parameters:
		/// - `ip`: The registered IP of storage miner.
		#[pallet::call_index(3)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::update_ip())]
		pub fn update_peer_id(origin: OriginFor<T>, peer_id: PeerId) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);

			let old = <MinerItems<T>>::try_mutate(&sender, |miner_info_opt| -> Result<PeerId, DispatchError> {
				let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
				let old = miner_info.peer_id.clone();
				miner_info.peer_id = peer_id.clone();
				Ok(old)
			})?;

			Self::deposit_event(Event::<T>::UpdataIp { acc: sender, old, new: peer_id.into() });
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
					<T as pallet::Config>::Currency::transfer(&reward_pot, &sender, reward.currently_available_reward.clone(), KeepAlive)?;

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
		#[pallet::weight(100_000)]
		pub fn faucet_top_up(origin: OriginFor<T>, award: BalanceOf<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let reward_pot = T::PalletId::get().into_account_truncating();
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
		#[pallet::weight(100_000)]
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
				let reward_pot = T::PalletId::get().into_account_truncating();

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

				let reward_pot = T::PalletId::get().into_account_truncating();
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
	}
}

impl<T: Config> Pallet<T> {
	/// Add computing power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn add_miner_idle_space(acc: &AccountOf<T>, accumulator: Accumulator, last_operation_block: u32, rear: u64) -> Result<u128, DispatchError> {
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> Result<u128, DispatchError> {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::NotMiner)?;

			let last_operation_block: BlockNumberOf<T> = last_operation_block.saturated_into();

			ensure!(miner_info.last_operation_block < last_operation_block, Error::<T>::LowerOperationBlock);
			// check state 
			ensure!(miner_info.state.to_vec() == STATE_POSITIVE.as_bytes().to_vec(), Error::<T>::NotpositiveState);

			ensure!(miner_info.rear < rear, Error::<T>::CountError);

			miner_info.last_operation_block = last_operation_block;

			let count = rear.checked_sub(miner_info.rear).ok_or(Error::<T>::Overflow)?;
			let idle_space = FRAGMENT_SIZE.checked_mul(count as u128).ok_or(Error::<T>::Overflow)?;

			miner_info.rear = rear;

			miner_info.accumulator = accumulator;

			miner_info.idle_space =
				miner_info.idle_space.checked_add(idle_space).ok_or(Error::<T>::Overflow)?;

			Ok(idle_space)
		})
	}

	pub fn delete_idle_update_accu(acc: &AccountOf<T>, accumulator: Accumulator, last_operation_block: u32, front: u64) -> Result<u64, DispatchError> {
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> Result<u64, DispatchError> {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::NotMiner)?;

			let last_operation_block: BlockNumberOf<T> = last_operation_block.saturated_into();

			ensure!(miner_info.last_operation_block < last_operation_block, Error::<T>::LowerOperationBlock);

			ensure!(miner_info.front < front, Error::<T>::CountError);

			let count = front - miner_info.front;

			miner_info.last_operation_block = last_operation_block;

			miner_info.front = front;

			miner_info.accumulator = accumulator;

			Ok(count)
		})
	}

	fn delete_idle_update_space(acc: &AccountOf<T>, idle_space: u128) -> DispatchResult {
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> DispatchResult {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::NotMiner)?;

			miner_info.idle_space = miner_info.idle_space.checked_sub(idle_space).ok_or(Error::<T>::Overflow)?;

			Ok(())
		})
	}
	/// Sub computing power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn sub_miner_idle_space(acc: &AccountOf<T>, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerItems<T>>::contains_key(acc) {
			return Ok(());
		}

		let state = Self::check_state(acc)?; //read 1
		if state == STATE_EXIT.as_bytes().to_vec() {
			return Ok(());
		}
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> DispatchResult {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
			miner_info.idle_space =
				miner_info.idle_space.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?; //read 1 write 1

		Ok(())
	}

	/// Add space calculation power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn add_miner_service_space(acc: &AccountOf<T>, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerItems<T>>::contains_key(acc) {
			return Ok(());
		}

		let state = Self::check_state(acc)?;
		if state == STATE_EXIT.as_bytes().to_vec() {
			return Ok(());
		}
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> DispatchResult {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
			miner_info.service_space =
				miner_info.service_space.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	/// Sub space calculation power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn sub_miner_service_space(acc: &AccountOf<T>, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerItems<T>>::contains_key(acc) {
			return Ok(());
		}

		let state = Self::check_state(acc)?;
		if state == STATE_EXIT.as_bytes().to_vec() {
			return Ok(());
		}
		MinerItems::<T>::try_mutate(acc, |miner_info_opt| -> DispatchResult {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
			miner_info.service_space =
				miner_info.service_space.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	pub fn calculate_power(idle_space: u128, service_space: u128) -> u128 {
		let service_power = SERVICE_MUTI.mul_floor(service_space);

        let idle_power = IDLE_MUTI.mul_floor(idle_space);

		let power: u128 = idle_power + service_power;

		power
	}

	pub fn calculate_miner_reward(
		miner: &AccountOf<T>,
		total_reward: u128,
		total_idle_space: u128,
		total_service_space: u128,
		miner_idle_space: u128,
		miner_service_space: u128,
	) -> DispatchResult {
		let total_power = Self::calculate_power(total_idle_space, total_service_space);
		let miner_power = Self::calculate_power(miner_idle_space, miner_service_space);

		let miner_prop = Perbill::from_rational(miner_power, total_power);
		let this_round_reward = miner_prop.mul_floor(total_reward);
		let each_share = EACH_SHARE_MUTI.mul_floor(this_round_reward);
		let each_share = each_share.checked_div(RELEASE_NUMBER.into()).ok_or(Error::<T>::Overflow)?;
		let issued: BalanceOf<T> = ISSUE_MUTI.mul_floor(this_round_reward).try_into().map_err(|_| Error::<T>::Overflow)?;

		let order = RewardOrder::<BalanceOf<T>>{
			order_reward: this_round_reward.try_into().map_err(|_| Error::<T>::Overflow)?,
			each_share: each_share.try_into().map_err(|_| Error::<T>::Overflow)?,
			award_count: 1,
			has_issued: true,
		};
		// calculate available reward
		RewardMap::<T>::try_mutate(miner, |opt_reward_info| -> DispatchResult {
			let reward_info = opt_reward_info.as_mut().ok_or(Error::<T>::Unexpected)?;
			// traverse the order list
			for order_temp in reward_info.order_list.iter_mut() {
				// skip if the order has been issued for 180 times
				if order_temp.award_count == RELEASE_NUMBER {
					continue;
				}
				reward_info.currently_available_reward = reward_info.currently_available_reward
					.checked_add(&order_temp.each_share).ok_or(Error::<T>::Overflow)?;

				order_temp.award_count += 1;
			}

			if reward_info.order_list.len() == RELEASE_NUMBER as usize {
				reward_info.order_list.remove(0);
			}

			reward_info.currently_available_reward = reward_info.currently_available_reward
				.checked_add(&issued).ok_or(Error::<T>::Overflow)?
				.checked_add(&order.each_share).ok_or(Error::<T>::Overflow)?;
			reward_info.total_reward = reward_info.total_reward
				.checked_add(&order.order_reward).ok_or(Error::<T>::Overflow)?;
			reward_info.order_list.try_push(order.clone()).map_err(|_| Error::<T>::BoundedVecError)?;

			Ok(())
		})?;

		<CurrencyReward<T>>::mutate(|v| -> DispatchResult {
			*v = v.checked_sub(&order.order_reward).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		
		Ok(())
	}

	pub fn deposit_punish(miner: &AccountOf<T>, punish_amount: BalanceOf<T>) -> DispatchResult {
		<MinerItems<T>>::try_mutate(miner, |miner_info_opt| -> DispatchResult {
			let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::NotMiner)?;
			
			let reward_pot = T::PalletId::get().into_account_truncating();

			if miner_info.collaterals > punish_amount {
				T::Currency::unreserve(miner, punish_amount);
				T::Currency::transfer(miner, &reward_pot, punish_amount, KeepAlive)?;
				<CurrencyReward<T>>::mutate(|reward| {
					*reward = *reward + punish_amount;
				});
				miner_info.collaterals = miner_info.collaterals.checked_sub(&punish_amount).ok_or(Error::<T>::Overflow)?;
			} else {
				T::Currency::unreserve(miner, miner_info.collaterals);
				T::Currency::transfer(miner, &reward_pot, miner_info.collaterals, KeepAlive)?;
				<CurrencyReward<T>>::mutate(|reward| {
					*reward = *reward + miner_info.collaterals;
				});
				miner_info.collaterals = BalanceOf::<T>::zero();
				miner_info.debt = punish_amount.checked_sub(&miner_info.collaterals).ok_or(Error::<T>::Overflow)?;
			}

			let power = Self::calculate_power(miner_info.idle_space, miner_info.service_space);
			let limit = Self::check_collateral_limit(power)?;

			if miner_info.collaterals < limit {
				miner_info.state = STATE_FROZEN.as_bytes().to_vec().try_into().map_err(|_| Error::<T>::BoundedVecError)?;
			}

			Ok(())
		})?;

		Ok(())
	}

	pub fn idle_punish(miner: &AccountOf<T>, idle_space: u128, service_space: u128) -> DispatchResult {
		let power = Self::calculate_power(idle_space, service_space);
		let limit = Self::check_collateral_limit(power)?;

		let punish_amount = IDLE_PUNI_MUTI.mul_floor(limit);

		Self::deposit_punish(miner, punish_amount)?;

		Ok(())
	}

	pub fn service_punish(miner: &AccountOf<T>, idle_space: u128, service_space: u128) -> DispatchResult {
		let power = Self::calculate_power(idle_space, service_space);
		let limit = Self::check_collateral_limit(power)?;

		let punish_amount = SERVICE_PUNI_MUTI.mul_floor(limit);

		Self::deposit_punish(miner, punish_amount)?;

		Ok(())
	}

	pub fn clear_punish(miner: &AccountOf<T>, level: u8, idle_space: u128, service_space: u128) -> DispatchResult {
		let power = Self::calculate_power(idle_space, service_space);
		let limit = Self::check_collateral_limit(power)?;

		let punish_amount = match level {
			1 => Perbill::from_percent(30).mul_floor(limit),
			2 => Perbill::from_percent(60).mul_floor(limit),
			3 => limit,
			_ => return Err(Error::<T>::Unexpected)?,
		};

		Self::deposit_punish(miner, punish_amount)?;

		Ok(())
	}

	fn check_collateral_limit(power: u128) -> Result<BalanceOf<T>, Error<T>> {
		let limit = 1 + power.checked_div(T_BYTE).ok_or(Error::<T>::Overflow)?;
		let limit = BASE_LIMIT.checked_mul(limit).ok_or(Error::<T>::Overflow)?;
		let limit: BalanceOf<T> = limit.try_into().map_err(|_| Error::<T>::Overflow)?;

		Ok(limit)
	}

	fn check_state(acc: &AccountOf<T>) -> Result<Vec<u8>, Error<T>> {
		Ok(<MinerItems<T>>::try_get(acc).map_err(|_e| Error::<T>::NotMiner)?.state.to_vec())
	}

	fn vec_to_bound<P>(param: Vec<P>) -> Result<BoundedVec<P, T::ItemLimit>, DispatchError> {
		let result: BoundedVec<P, T::ItemLimit> =
			param.try_into().map_err(|_e| Error::<T>::StorageLimitReached)?;

		Ok(result)
	}

	// Note: that it is necessary to determine whether the state meets the exit conditions before use.
	fn force_miner_exit(acc: &AccountOf<T>) -> DispatchResult {
		if let Ok(reward_info) = <RewardMap<T>>::try_get(acc).map_err(|_| Error::<T>::NotExisted) {
			let reward = reward_info.total_reward
				.checked_sub(&reward_info.reward_issued).ok_or(Error::<T>::Overflow)?;
			<CurrencyReward<T>>::mutate(|v| {
				*v = *v + reward;
			});
		}
		let mut miner_list = AllMiner::<T>::get();
		miner_list.retain(|s| s != acc);
		AllMiner::<T>::put(miner_list);

		<RewardMap<T>>::remove(acc);

		<MinerItems<T>>::try_mutate(acc, |miner_opt| -> DispatchResult {
			let miner = miner_opt.as_mut().ok_or(Error::<T>::Unexpected)?;
			miner.state = STATE_OFFLINE.as_bytes().to_vec().try_into().map_err(|_| Error::<T>::BoundedVecError)?;
			MinerPublicKey::<T>::remove(miner.puk);
			Ok(())
		})?;

		Ok(())
	}

	// Note: that it is necessary to determine whether the state meets the exit conditions before use.
	fn execute_exit(acc: &AccountOf<T>) -> DispatchResult {
		// T::Currency::unreserve(acc, miner.collaterals);
		if let Ok(reward_info) = <RewardMap<T>>::try_get(acc).map_err(|_| Error::<T>::NotExisted) {
			let reward = reward_info.total_reward
				.checked_sub(&reward_info.reward_issued).ok_or(Error::<T>::Overflow)?;
			<CurrencyReward<T>>::mutate(|v| {
				*v = *v + reward;
			});
		}

		let mut miner_list = AllMiner::<T>::get();
		miner_list.retain(|s| s != acc);
		AllMiner::<T>::put(miner_list);

		<RewardMap<T>>::remove(acc);
		<MinerItems<T>>::try_mutate(acc, |miner_opt| -> DispatchResult {
			let miner_info = miner_opt.as_mut().ok_or(Error::<T>::NotMiner)?;
			miner_info.state = Self::vec_to_bound::<u8>(STATE_EXIT.as_bytes().to_vec())?;

			Ok(())
		})
	}
	// Note: that it is necessary to determine whether the state meets the exit conditions before use.
	fn withdraw(acc: &AccountOf<T>) -> DispatchResult {
		let miner_info = <MinerItems<T>>::try_get(acc).map_err(|_| Error::<T>::NotMiner)?;
		T::Currency::unreserve(acc, miner_info.collaterals);
		MinerPublicKey::<T>::remove(miner_info.puk);
		<MinerItems<T>>::remove(acc);

		Ok(())
	}
}

impl<T: Config> OnUnbalanced<NegativeImbalanceOf<T>> for Pallet<T> {
	fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<T>) {
		let numeric_amount = amount.peek();

		// Must resolve into existing but better to be safe.
		let _ = T::Currency::resolve_creating(&T::PalletId::get().into_account_truncating(), amount);
		<CurrencyReward<T>>::mutate(|v| {
			*v = *v + numeric_amount;
		});

		Self::deposit_event(Event::Deposit { balance: numeric_amount });
	}
}

pub trait MinerControl<AccountId> {
	fn add_miner_idle_space(acc: &AccountId, accumulator: Accumulator, last_operation_block: u32, rear: u64) -> Result<u128, DispatchError>;
	// fn sub_miner_idle_space(acc: &AccountId, accumulator: Accumulator, rear: u64) -> DispatchResult;
	fn delete_idle_update_accu(acc: &AccountId, accumulator: Accumulator, last_operation_block: u32, front: u64) -> Result<u64, DispatchError>;
	fn delete_idle_update_space(acc: &AccountId, idle_space: u128) -> DispatchResult;
	fn add_miner_service_space(acc: &AccountId, power: u128) -> DispatchResult;
	fn sub_miner_service_space(acc: &AccountId, power: u128) -> DispatchResult;
	fn get_power(acc: &AccountId) -> Result<(u128, u128), DispatchError>;
	fn miner_is_exist(acc: AccountId) -> bool;
	fn get_miner_state(acc: &AccountId) -> Result<Vec<u8>, DispatchError>;
	fn get_all_miner() -> Result<Vec<AccountId>, DispatchError>;
	fn lock_space(acc: &AccountId, space: u128) -> DispatchResult;
	fn unlock_space(acc: &AccountId, space: u128) -> DispatchResult;
	fn unlock_space_to_service(acc: &AccountId, space: u128) -> DispatchResult;
	fn get_miner_idle_space(acc: &AccountId) -> Result<u128, DispatchError>;
	fn get_miner_count() -> u32;
	fn get_reward() -> u128; 
	fn calculate_miner_reward(
		miner: &AccountId, 
		total_reward: u128,
		total_idle_space: u128,
		total_service_space: u128,
		miner_idle_space: u128,
		miner_service_space: u128,
	) -> DispatchResult;
	fn clear_punish(miner: &AccountId, level: u8, idle_space: u128, service_space: u128) -> DispatchResult;
	fn idle_punish(miner: &AccountId, idle_space: u128, service_space: u128) -> DispatchResult;
	fn service_punish(miner: &AccountId, idle_space: u128, service_space: u128) -> DispatchResult;

	fn execute_exit(acc: &AccountId) -> DispatchResult;
	fn withdraw(acc: &AccountId) -> DispatchResult;
	fn force_miner_exit(acc: &AccountId) -> DispatchResult; 

	fn is_positive(miner: &AccountId) -> Result<bool, DispatchError>;
	fn is_lock(miner: &AccountId) -> Result<bool, DispatchError>;
	fn update_miner_state(miner: &AccountId, state: &str) -> DispatchResult;
}

impl<T: Config> MinerControl<<T as frame_system::Config>::AccountId> for Pallet<T> {
	fn add_miner_idle_space(acc: &<T as frame_system::Config>::AccountId, accumulator: Accumulator,  last_operation_block: u32, rear: u64) -> Result<u128, DispatchError> {
		let idle_space = Pallet::<T>::add_miner_idle_space(acc, accumulator, last_operation_block, rear)?;
		Ok(idle_space)
	}

	fn delete_idle_update_accu(acc: &AccountOf<T>, accumulator: Accumulator, last_operation_block: u32, front: u64) -> Result<u64, DispatchError> {
		let count = Self::delete_idle_update_accu(acc, accumulator, last_operation_block, front)?;
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

	fn get_reward() -> u128 {
		<CurrencyReward<T>>::get().saturated_into()
	}

	fn calculate_miner_reward(
		miner: &AccountOf<T>, 
		total_reward: u128,
		total_idle_space: u128,
		total_service_space: u128,
		miner_idle_space: u128,
		miner_service_space: u128,
	) -> DispatchResult {
		Self::calculate_miner_reward(
			miner, 
			total_reward, 
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

	fn execute_exit(acc: &AccountOf<T>) -> DispatchResult {
		Self::execute_exit(acc)
	}

	fn force_miner_exit(acc: &AccountOf<T>) -> DispatchResult {
		Self::force_miner_exit(acc)
	}

	fn withdraw(acc: &AccountOf<T>) -> DispatchResult {
		Self::withdraw(acc)
	}
}
