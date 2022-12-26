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
		schedule::{Anon as ScheduleAnon, DispatchTime, Named as ScheduleNamed},
		Currency,
		ExistenceRequirement::AllowDeath,
		Get, Imbalance, LockIdentifier, OnUnbalanced, ReservableCurrency,
	},
};
use cp_cess_common::{IpAddress, M_BYTE, Mrenclave};
use cp_bloom_filter::BloomFilter;
use cp_crypto::{IasCert, IasSig, QuoteBody};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

mod types;
use types::*;

mod constants;
use constants::*;

mod impls;
use impls::*;

use codec::{Decode, Encode};
use frame_support::{
	ensure,
	dispatch::{DispatchResult, Dispatchable},
	pallet_prelude::DispatchError,
	PalletId,
	traits::schedule,
	weights::Weight,
};
use frame_system::{self as system};

pub use pallet::*;

use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, CheckedAdd, CheckedSub, CheckedMul, SaturatedConversion, Zero},
	RuntimeDebug, Perbill
};
use sp_std::{convert::TryInto, prelude::*};
pub mod weights;
pub use weights::WeightInfo;
use sp_core::crypto::KeyTypeId;
use sp_application_crypto::{
	RuntimePublic,
	ecdsa::{Signature, Public},
};

// use sp_core::ecdsa::{Pair as EPair, Public, Signature};
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
		traits::Get, Blake2_128Concat,
	};
	use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};
	use system::pallet_prelude::OriginFor;

	// const DEMOCRACY_IDD: LockIdentifier = *b"msminerD";

	#[pallet::config]
	pub trait Config: pallet_timestamp::Config + frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;
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
		type MrenclaveNumber: Get<u32>;

		#[pallet::constant]
		type OrderLimit: Get<u32>;

		#[pallet::constant]
		type MaxAward: Get<u128>;
		/// The Scheduler.
		type SScheduler: ScheduleNamed<Self::BlockNumber, Self::SProposal, Self::SPalletsOrigin>;

		type AScheduler: ScheduleAnon<Self::BlockNumber, Self::SProposal, Self::SPalletsOrigin>;
		/// Overarching type of all pallets origins.
		type SPalletsOrigin: From<system::RawOrigin<Self::AccountId>>;
		/// The SProposal.
		type SProposal: Parameter + Dispatchable<Origin = Self::Origin> + From<Call<Self>>;
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
		/// An account was redeemed.
		Redeemed {
			acc: AccountOf<T>,
			deposit: BalanceOf<T>,
		},
		/// An account was claimed.
		Claimed {
			acc: AccountOf<T>,
			deposit: BalanceOf<T>,
		},
		/// Storage space is triggered periodically.
		TimingStorageSpace(),
		/// Scheduled Task Execution
		TimedTask(),
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
		//Miner exit event
		MinerExit {
			acc: AccountOf<T>,
		},

		MinerClaim {
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
			old: IpAddress,
			new: IpAddress,
		},
		StartOfBufferPeriod {
			when: BlockNumberOf<T>,
		},
		EndOfBufferPeriod {
			when: BlockNumberOf<T>,
		},
	}

	/// Error for the sminer pallet.
	#[pallet::error]
	pub enum Error<T> {
		NotMiner,
		/// An account has locked balances.
		LockedNotEmpty,
		/// An account already registered.
		AlreadyRegistered,
		/// An account's earnings is empty.
		EarningsIsEmpty,
		/// An operation would lead to an overflow.
		Overflow,
		/// User does none exist.
		NonExisted,
		/// Lack of permissions.
		LackOfPermissions,
		/// Beyond the requirements.
		BeyondClaim,
		/// The duration is less than 24 hours.
		LessThan24Hours,
		/// Numerical conversion error.
		ConversionError,
		/// You can't divide by zero
		DivideByZero,

		InsufficientAvailableSpace,
		//The account has been frozen
		AlreadyFrozen,

		LockInNotOver,

		NotpositiveState,

		StorageLimitReached,

		BoundedVecError,

		DataNotExist,
		//haven't bought space at all
		NotPurchasedPackage,

		InvalidCert,

		BloomElemPushError,

		BloomElemPopError,

		Unexpected,

		NoReward,
	}

	/// The hashmap for info of storage miners.
	#[pallet::storage]
	#[pallet::getter(fn miner_items)]
	pub(super) type MinerItems<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		MinerInfo<T::AccountId, BalanceOf<T>, BoundedVec<u8, T::ItemLimit>>,
	>;

	/// The total power of all storage miners.
	#[pallet::storage]
	#[pallet::getter(fn total_idle_space)]
	pub(super) type TotalIdleSpace<T: Config> = StorageValue<_, u128, ValueQuery>;

	/// The total storage space to fill of all storage miners.
	#[pallet::storage]
	#[pallet::getter(fn total_service_space)]
	pub(super) type TotalServiceSpace<T: Config> = StorageValue<_, u128, ValueQuery>;

	//Total autonomous space of miners in the whole network
	#[pallet::storage]
	#[pallet::getter(fn total_autonomy_space)]
	pub(super) type TotalAutonomySpace<T: Config> = StorageValue<_, u128, ValueQuery>;

	/// Store all miner information
	#[pallet::storage]
	#[pallet::getter(fn miner_info)]
	pub(super) type AllMiner<T: Config> =
		StorageValue<_, BoundedVec<AccountOf<T>, T::ItemLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn purchased_space)]
	pub(super) type PurchasedSpace<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn buffer_period)]
	pub(super) type BufferPeriod<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BlockNumberOf<T>,
		BoundedVec<AccountOf<T>, T::ItemLimit>,
		ValueQuery,
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
	#[pallet::getter(fn reward_map)]
	pub(super) type RewardMap<T: Config> = StorageMap<
		_, 
		Blake2_128Concat, 
		AccountOf<T>, 
		Reward<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn mrenclave_codes)]
	pub(super) type MrenclaveCodes<T: Config> = StorageValue<_, BoundedVec<Mrenclave, T::MrenclaveNumber>, ValueQuery>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// #[transactional]
		// #[pallet::weight(<T as pallet::Config>::WeightInfo::regnstk())]
		// pub fn test_cert(
		// 	origin: OriginFor<T>,
		// 	ias_sig: IasSig,
		// 	ias_cert: IasCert,
		// 	msg: Vec<u8>,
		// ) -> DispatchResult {
		// 	let _ = ensure_signed(origin)?;

		// 	ensure!(cp_crypto::verify_miner_report(&ias_sig, &ias_cert, &msg), Error::<T>::NotMiner);

		// 	Ok(())
		// }


		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::regnstk())]
		pub fn test_sig(
			origin: OriginFor<T>,
			sig: [u8; 65],
			msg: [u8; 32],
			puk: [u8; 33],
		) -> DispatchResult {
			let _sender = ensure_signed(origin)?;
			log::info!("sig: {:?} \n, msg: {:?} \n, puk: {:?}", sig.clone(), msg, puk.clone());

			let secp_sig: Signature = Signature::from_raw(sig);
			let secp_puk: Public = Public::from_raw(puk);
			// let r_puk = secp_sig.recover(&msg);
			// log::info!("result r_puk: {:?}", r_puk.as_ref());
			let puk_result = secp_puk.verify(&msg, &secp_sig);
			let sp_io_result = sp_io::crypto::ecdsa_verify_prehashed(&secp_sig, &msg, &secp_puk);
			log::info!("result sp_io_result: {:?}", sp_io_result);
			log::info!("result puk_result: {:?}", puk_result);
			Ok(())
		}

		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::regnstk())]
		pub fn test_recover(
			origin: OriginFor<T>,
			sig: [u8; 65],
			msg: [u8; 32],
			puk: [u8; 33],
		) -> DispatchResult {
			let _sender = ensure_signed(origin)?;
			log::info!("sig: {:?} \n, msg: {:?} \n, puk: {:?}", sig.clone(), msg, puk.clone());

			let r_puk = sp_io::crypto::secp256k1_ecdsa_recover_compressed(&sig, &msg).map_err(|_| Error::<T>::Overflow)?;
			
			log::info!("r_puk: {:?}", r_puk);
			if r_puk == puk {
				log::info!("success!!");
			}

			let secp_sig: Signature = Signature::from_raw(sig);
			let secp_puk: Public = Public::from_raw(puk);
			// let r_puk = secp_sig.recover(&msg);
			// log::info!("result r_puk: {:?}", r_puk.as_ref());
			
			let puk_result = secp_puk.verify(&msg, &secp_sig);
			log::info!("result puk_result: {:?}", puk_result);
			Ok(())
		}
		/// Staking and register for storage miner.
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// Parameters:
		/// - `beneficiary`: The beneficiary related to signer account.
		/// - `ip`: The registered IP of storage miner.
		/// - `staking_val`: The number of staking.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::regnstk())]
		pub fn regnstk(
			origin: OriginFor<T>,
			beneficiary: AccountOf<T>,
			ip: IpAddress,
			staking_val: BalanceOf<T>,
			ias_cert: IasCert,
			ias_sig: IasSig,
			quote_body: QuoteBody,
			quote_sig: Signature,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!(<MinerItems<T>>::contains_key(&sender)), Error::<T>::AlreadyRegistered);
			T::Currency::reserve(&sender, staking_val.clone())?;

			let mernclave_codes = MrenclaveCodes::<T>::get();

			let puk = cp_crypto::verify_miner_cert(
				&ias_sig,
				&ias_cert,
				&quote_sig,
				&quote_body,
				&mernclave_codes,
			).ok_or(Error::<T>::InvalidCert)?;

			<MinerItems<T>>::insert(
				&sender,
				MinerInfo::<AccountOf<T>, BalanceOf<T>, BoundedVec<u8, T::ItemLimit>>::new(
					beneficiary.clone(), 
					staking_val.clone(),
					ip.clone(), 
					puk.clone(), 
					ias_cert.clone(),
				),
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
				acc: sender.clone(),
				staking_val: staking_val.clone(),
			});
			Ok(())
		}

		#[transactional]
		#[pallet::weight(100_000_000_000)]
		pub fn receive_reward(
			origin: OriginFor<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			if let Ok(miner) = <MinerItems<T>>::try_get(&sender) {
				ensure!(
					miner.state == STATE_POSITIVE.as_bytes().to_vec() || miner.state == STATE_EXIT.as_bytes().to_vec(), 
					Error::<T>::NotpositiveState
				);

				<RewardMap<T>>::try_mutate(&sender, |opt_reward| -> DispatchResult {
					let reward = opt_reward.as_mut().ok_or(Error::<T>::Unexpected)?;
					ensure!(reward.currently_available_reward != 0u32.saturated_into(), Error::<T>::NoReward);

					let reward_pot = T::PalletId::get().into_account();
					<T as pallet::Config>::Currency::transfer(&reward_pot, &sender, reward.currently_available_reward, AllowDeath)?;

					reward.reward_issued = reward.reward_issued
						.checked_add(&reward.currently_available_reward).ok_or(Error::<T>::Overflow)?;

					reward.currently_available_reward = 0u32.saturated_into();

					Ok(())
				})?;
			} 
			// else {
			// 	<RewardMap<T>>::try_mutate(&sender, |opt_reward| -> DispatchResult {
			// 		let reward = opt_reward.as_mut().ok_or(Error::<T>::Unexpected)?;
			// 		ensure!(reward.currently_available_reward != 0u32.saturated_into(), Error::<T>::NoReward);

			// 		let reward_pot = T::PalletId::get().into_account();
			// 		<T as pallet::Config>::Currency::transfer(&reward_pot, &sender, reward.currently_available_reward, AllowDeath)?;

			// 		reward.reward_issued = reward.reward_issued
			// 			.checked_add(&reward.currently_available_reward).ok_or(Error::<T>::Overflow)?;

			// 		reward.currently_available_reward = 0u32.saturated_into();

			// 		Ok(())
			// 	})?;
			// }

			Ok(())
		}

		/// Increase the miner collateral.
		///
		/// Parameters:
		/// - `collaterals`: Miner's TCESS.
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::increase_collateral())]
		pub fn increase_collateral(
			origin: OriginFor<T>,
			#[pallet::compact] collaterals: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);

			T::Currency::reserve(&sender, collaterals)?;
			let mut balance: BalanceOf<T> = Default::default();

			<MinerItems<T>>::try_mutate(&sender, |miner_info_opt| -> DispatchResult {
				let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;

				miner_info.collaterals =
					miner_info.collaterals.checked_add(&collaterals).ok_or(Error::<T>::Overflow)?;

				balance = miner_info.collaterals;

				let power = miner_info.calculate_power();

				let limit = Self::check_collateral_limit(power)?;

				match sp_std::str::from_utf8(miner_info.state.to_vec().as_slice()).map_err(|_| Error::<T>::Overflow)? {
					STATE_FROZEN => {
						if miner_info.collaterals > limit {
							miner_info.state = STATE_POSITIVE
								.as_bytes().to_vec().try_into().map_err(|_| Error::<T>::BoundedVecError)?;
						}
					},
					STATE_EXIT_FROZEN => {
						if miner_info.collaterals > limit {
							miner_info.state = STATE_EXIT
								.as_bytes().to_vec().try_into().map_err(|_| Error::<T>::BoundedVecError)?;
						}
					},
					STATE_DEBT => {
						let reward_pot = T::PalletId::get().into_account();
						if miner_info.collaterals > miner_info.debt {
							T::Currency::unreserve(&sender, miner_info.debt);
							<T as pallet::Config>::Currency::transfer(&sender, &reward_pot, miner_info.debt, AllowDeath)?;

							miner_info.collaterals = miner_info.collaterals.checked_sub(&miner_info.debt).ok_or(Error::<T>::Overflow)?;
							miner_info.debt = Zero::zero();
							if miner_info.collaterals > limit {
								miner_info.state = STATE_POSITIVE
									.as_bytes().to_vec().try_into().map_err(|_| Error::<T>::BoundedVecError)?;
							} else {
								miner_info.state = STATE_FROZEN
									.as_bytes().to_vec().try_into().map_err(|_| Error::<T>::BoundedVecError)?;
							}
						} else {
							T::Currency::unreserve(&sender, miner_info.collaterals);
							<T as pallet::Config>::Currency::transfer(&sender, &reward_pot, miner_info.collaterals, AllowDeath)?;

							miner_info.debt = miner_info.debt.checked_sub(&miner_info.collaterals).ok_or(Error::<T>::Overflow)?;
							miner_info.collaterals = Zero::zero();
						}
					},
					STATE_EXIT => {
						Err(Error::<T>::NonExisted)?;
					},
					_ => Err(Error::<T>::NonExisted)?,
				};

				Ok(())
			})?;

			Self::deposit_event(Event::<T>::IncreaseCollateral { acc: sender, balance });
			Ok(())
		}

		/// updata miner beneficiary.
		///
		/// Parameters:
		/// - `beneficiary`: The beneficiary related to signer account.
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
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::update_ip())]
		pub fn update_ip(origin: OriginFor<T>, ip: IpAddress) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);

			let old = <MinerItems<T>>::try_mutate(&sender, |miner_info_opt| -> Result<IpAddress, DispatchError> {
				let miner_info = miner_info_opt.as_mut().ok_or(Error::<T>::ConversionError)?;
				let old = miner_info.ip.clone();
				miner_info.ip = ip.clone();
				Ok(old)
			})?;

			Self::deposit_event(Event::<T>::UpdataIp { acc: sender, old, new: ip });
			Ok(())
		}

		//Method for miners to redeem deposit
		// #[transactional]
		// #[pallet::weight(<T as pallet::Config>::WeightInfo::withdraw())]
		// pub fn withdraw(origin: OriginFor<T>) -> DispatchResult {
		// 	let sender = ensure_signed(origin)?;
		// 	ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);

		// 	let state = Self::check_state(&sender)?;
		// 	if state != STATE_EXIT.as_bytes().to_vec() {
		// 		Err(Error::<T>::NonExisted)?;
		// 	}
		// 	let now_block: u128 = <frame_system::Pallet<T>>::block_number().saturated_into();
		// 	let lock_in_strat: u128 = MinerLockIn::<T>::try_get(&sender)
		// 		.map_err(|_e| Error::<T>::LockInNotOver)?
		// 		.saturated_into();
		// 	let mut lock_in_period: u128 = T::OneDayBlock::get().saturated_into();
		// 	let day = T::LockInPeriod::get();
		// 	lock_in_period = lock_in_period * day as u128;
		// 	// let mut lock_in_period: u128 = 50;
		// 	if lock_in_strat + lock_in_period > now_block {
		// 		Err(Error::<T>::LockInNotOver)?;
		// 	}
		// 	let collaterals = MinerItems::<T>::try_get(&sender)
		// 		.map_err(|_e| Error::<T>::NotMiner)?
		// 		.collaterals;
		// 	T::Currency::unreserve(&sender, collaterals);
		// 	Self::delete_miner_info(&sender)?;
		// 	MinerLockIn::<T>::remove(&sender);

		// 	Self::deposit_event(Event::<T>::MinerClaim { acc: sender });
		// 	Ok(())
		// }
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
		#[transactional]
		#[pallet::weight(100_000)]
		pub fn faucet_top_up(origin: OriginFor<T>, award: BalanceOf<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let reward_pot = T::PalletId::get().into_account();
			<T as pallet::Config>::Currency::transfer(&sender, &reward_pot, award, AllowDeath)?;

			Self::deposit_event(Event::<T>::FaucetTopUpMoney { acc: sender.clone() });
			Ok(())
		}

		/// Users receive money through the faucet.
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// Parameters:
		/// - `acc`: Withdraw money account.
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
				let reward_pot = T::PalletId::get().into_account();

				<T as pallet::Config>::Currency::transfer(
					&reward_pot,
					&to,
					FAUCET_VALUE.try_into().map_err(|_e| Error::<T>::ConversionError)?,
					AllowDeath,
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

				let reward_pot = T::PalletId::get().into_account();
				<T as pallet::Config>::Currency::transfer(
					&reward_pot,
					&to,
					FAUCET_VALUE.try_into().map_err(|_e| Error::<T>::ConversionError)?,
					AllowDeath,
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
	/// - `increment`: computing power.
	pub fn add_total_idle_space(increment: u128) -> DispatchResult {
		TotalIdleSpace::<T>::try_mutate(|total_space| -> DispatchResult {
			*total_space = total_space.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}
	/// Sub computing power to corresponding miners.
	///
	/// Parameters:
	/// - `increment`: idle space.
	pub fn sub_total_idle_space(increment: u128) -> DispatchResult {
		TotalIdleSpace::<T>::try_mutate(|total_space| -> DispatchResult {
			*total_space = total_space.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?; //read 1 write 1

		Ok(())
	}

	/// Add space calculation power to corresponding miners.
	///
	/// Parameters:
	/// - `increment`: service space.
	pub fn add_total_service_space(increment: u128) -> DispatchResult {
		TotalServiceSpace::<T>::try_mutate(|total_space| -> DispatchResult {
			*total_space = total_space.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}
	/// Sub space calculation power to corresponding miners.
	///
	/// Parameters:
	/// - `increment`: service space.
	pub fn sub_total_service_space(increment: u128) -> DispatchResult {
		TotalServiceSpace::<T>::mutate(|total_space| -> DispatchResult {
			*total_space = total_space.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	pub fn add_total_autonomy_space(increment: u128) -> DispatchResult {
		TotalAutonomySpace::<T>::mutate(|total_space| -> DispatchResult {
			*total_space = total_space.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		Ok(())
	}

	pub fn sub_total_autonomy_space(increment: u128) -> DispatchResult {
		TotalAutonomySpace::<T>::mutate(|total_space| -> DispatchResult {
			*total_space = total_space.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		Ok(())
	}

	pub fn add_miner_idle_space(acc: AccountOf<T>, increment: u128) -> DispatchResult {
		<MinerItems<T>>::try_mutate(&acc, |opt_m_info| -> DispatchResult{
			let m_info = opt_m_info.as_mut().ok_or(Error::<T>::NonExisted)?;
			m_info.idle_space = m_info.idle_space.checked_add(increment).ok_or(Error::<T>::Overflow)?;

			Ok(())
		})?;

		Ok(())
	}

	pub fn sub_miner_idle_space(acc: AccountOf<T>, increment: u128) -> DispatchResult {
		<MinerItems<T>>::try_mutate(&acc, |opt_m_info| -> DispatchResult{
			let m_info = opt_m_info.as_mut().ok_or(Error::<T>::NonExisted)?;
			m_info.idle_space = m_info.idle_space.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	pub fn add_miner_service_space(acc: AccountOf<T>, increment: u128) -> DispatchResult {
		<MinerItems<T>>::try_mutate(&acc, |opt_m_info| -> DispatchResult{
			let m_info = opt_m_info.as_mut().ok_or(Error::<T>::NonExisted)?;
			m_info.service_space = m_info.service_space.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	pub fn sub_miner_service_space(acc: AccountOf<T>, increment: u128) -> DispatchResult {
		<MinerItems<T>>::try_mutate(&acc, |opt_m_info| -> DispatchResult{
			let m_info = opt_m_info.as_mut().ok_or(Error::<T>::NonExisted)?;
			m_info.service_space = m_info.service_space.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	pub fn add_miner_autonomy_space(acc: AccountOf<T>, increment: u128) -> DispatchResult {
		<MinerItems<T>>::try_mutate(&acc, |opt_m_info| -> DispatchResult{
			let m_info = opt_m_info.as_mut().ok_or(Error::<T>::NonExisted)?;
			m_info.autonomy_space = m_info.autonomy_space.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	pub fn sub_miner_autonomy_space(acc: AccountOf<T>, increment: u128) -> DispatchResult {
		<MinerItems<T>>::try_mutate(&acc, |opt_m_info| -> DispatchResult{
			let m_info = opt_m_info.as_mut().ok_or(Error::<T>::NonExisted)?;
			m_info.autonomy_space = m_info.autonomy_space.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	pub fn insert_idle_bloom(acc: &AccountOf<T>, hash: [u8; 256]) -> DispatchResult {
		<MinerItems<T>>::try_mutate(&acc, |opt_m_info| -> DispatchResult{
			let m_info = opt_m_info.as_mut().ok_or(Error::<T>::NonExisted)?;
			m_info.bloom_filter.idle_filter.insert(hash).map_err(|_| Error::<T>::BloomElemPushError)?;
			Ok(())
		})?;

		Ok(())
	}

	pub fn insert_service_bloom(acc: &AccountOf<T>, hash: [u8; 256]) -> DispatchResult {
		<MinerItems<T>>::try_mutate(&acc, |opt_m_info| -> DispatchResult{
			let m_info = opt_m_info.as_mut().ok_or(Error::<T>::NonExisted)?;
			m_info.bloom_filter.service_filter.insert(hash).map_err(|_| Error::<T>::BloomElemPushError)?;
			Ok(())
		})?;

		Ok(())
	}

	pub fn insert_autonomy_bloom(acc: &AccountOf<T>, hash: [u8; 256]) -> DispatchResult {
		<MinerItems<T>>::try_mutate(&acc, |opt_m_info| -> DispatchResult{
			let m_info = opt_m_info.as_mut().ok_or(Error::<T>::NonExisted)?;
			m_info.bloom_filter.autonomy_filter.insert(hash).map_err(|_| Error::<T>::BloomElemPushError)?;
			Ok(())
		})?;

		Ok(())
	}

	fn remove_idle_bloom(acc: &AccountOf<T>, hash: [u8; 256]) -> DispatchResult {
		<MinerItems<T>>::try_mutate(&acc, |opt_m_info| -> DispatchResult{
			let m_info = opt_m_info.as_mut().ok_or(Error::<T>::NonExisted)?;
			m_info.bloom_filter.idle_filter.delete(hash).map_err(|_| Error::<T>::BloomElemPushError)?;
			Ok(())
		})?;

		Ok(())
	}

	fn remove_service_bloom(acc: &AccountOf<T>, hash: [u8; 256]) -> DispatchResult {
		<MinerItems<T>>::try_mutate(&acc, |opt_m_info| -> DispatchResult{
			let m_info = opt_m_info.as_mut().ok_or(Error::<T>::NonExisted)?;
			m_info.bloom_filter.service_filter.delete(hash).map_err(|_| Error::<T>::BloomElemPushError)?;
			Ok(())
		})?;

		Ok(())
	}

	fn remove_autonomy_bloom(acc: &AccountOf<T>, hash: [u8; 256]) -> DispatchResult {
		<MinerItems<T>>::try_mutate(&acc, |opt_m_info| -> DispatchResult{
			let m_info = opt_m_info.as_mut().ok_or(Error::<T>::NonExisted)?;
			m_info.bloom_filter.autonomy_filter.delete(hash).map_err(|_| Error::<T>::BloomElemPushError)?;
			Ok(())
		})?;

		Ok(())
	}

	fn insert_slice_update_bloom(acc: &AccountOf<T>, file_hash: [u8; 256], filler_hash: [u8; 256]) -> DispatchResult {
		<MinerItems<T>>::try_mutate(&acc, |opt_m_info| -> DispatchResult{
			let m_info = opt_m_info.as_mut().ok_or(Error::<T>::NonExisted)?;
			m_info.bloom_filter.service_filter.insert(file_hash).map_err(|_| Error::<T>::BloomElemPushError)?;
			m_info.bloom_filter.idle_filter.delete(filler_hash).map_err(|_| Error::<T>::BloomElemPushError)?;
			Ok(())
		})?;

		Ok(())
	}

	fn miner_punish_collaterals(acc: &AccountOf<T>, fine: BalanceOf<T>) -> DispatchResult {
		let reward_pot = T::PalletId::get().into_account();

		<MinerItems<T>>::try_mutate(acc, |opt_miner| -> DispatchResult {
			let miner = opt_miner.as_mut().ok_or(Error::<T>::NonExisted)?;
			// Judge whether the remaining deposit is sufficient
			// If the deposit is insufficient, it will enter into the debt state
			if miner.collaterals < fine {	
				let remaining_fine = fine.checked_sub(&miner.collaterals).ok_or(Error::<T>::Overflow)?;

				T::Currency::unreserve(acc, miner.collaterals.clone());
				<T as pallet::Config>::Currency::transfer(acc, &reward_pot, miner.collaterals, AllowDeath)?;

				miner.collaterals = 0u32.saturated_into();
				//Recording debt
				miner.debt = miner.debt.checked_add(&remaining_fine).ok_or(Error::<T>::Overflow)?;
				miner.state = STATE_DEBT.as_bytes().to_vec().try_into().map_err(|_| Error::<T>::ConversionError)?;
			} else {
				let power = miner.calculate_power();
				let limit = Self::check_collateral_limit(power)?;
				miner.collaterals = miner.collaterals.checked_sub(&fine).ok_or(Error::<T>::Overflow)?;

				T::Currency::unreserve(acc, fine.clone());
				<T as pallet::Config>::Currency::transfer(acc, &reward_pot, fine, AllowDeath)?;
				// Judge whether to freeze the miners
				if miner.collaterals < limit {
					miner.state = STATE_FROZEN.as_bytes().to_vec().try_into().map_err(|_| Error::<T>::ConversionError)?;
				}
			}
			
			Ok(())
		})
	}

	fn miner_clear_punish(acc: AccountOf<T>, count: u8) -> Result<Weight, DispatchError> {
		let mut weight: Weight = 0;
		let miner = <MinerItems<T>>::try_get(&acc).map_err(|_| Error::<T>::NonExisted)?;
		weight = weight.saturating_add(T::DbWeight::get().reads(1 as Weight));
		let power = miner.calculate_power();
		let limit = Self::check_collateral_limit(power)?;
		let fine = match count {
			1 => Perbill::from_percent(50).mul_floor(limit),
			2 => Perbill::from_percent(80).mul_floor(limit),
			3 => limit,
			_ => {
				log::info!("Exceptions, please resolve: fn miner_clear_punish()");
				return Ok(weight)
			},
		};

		Self::miner_punish_collaterals(&acc, fine)?;
		weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

		Ok(weight)
	}

	fn miner_slice_punish(acc: AccountOf<T>, idle_count: u128, service_count: u128, autonomy_count: u128) -> DispatchResult {
		let idle_fine = IDLE_MUTI.mul_floor(FINE_DEFAULT * idle_count);
		let service_fine = SERVICE_MUTI.mul_floor(FINE_DEFAULT * service_count);
		let autonomy_fine = AUTONOMY_MUTI.mul_floor(FINE_DEFAULT * autonomy_count);

		let fine = idle_fine
			.checked_add(service_fine)
			.ok_or(Error::<T>::Overflow)?
			.checked_add(autonomy_fine)
			.ok_or(Error::<T>::Overflow)?;
		
		if fine == 0 {
			return Ok(())
		}
		
		let fine: BalanceOf<T> = fine.try_into().map_err(|_| Error::<T>::Overflow)?;
		Self::miner_punish_collaterals(&acc, fine)?;

		Ok(())
	}

	fn calculate_reward(reward: BalanceOf<T>, miner: AccountOf<T>, total_power: u128, power: u128) -> DispatchResult {
		// calculate this round reward
		let miner_prop = Perbill::from_rational(power, total_power);
		let this_round_reward = miner_prop.mul_floor(reward);
		let each_share = Perbill::from_percent(80).mul_floor(this_round_reward);
		let issued = Perbill::from_percent(20).mul_floor(this_round_reward);
		let order = RewardOrder::<BalanceOf<T>>{
			order_reward: this_round_reward,
			each_share: each_share,
			award_count: 1,
			has_issued: true,
		};
		// calculate available reward
		RewardMap::<T>::try_mutate(&miner, |opt_reward_info| -> DispatchResult {
			let reward_info = opt_reward_info.as_mut().ok_or(Error::<T>::Unexpected)?;

			for order_temp in reward_info.order_list.iter_mut() {
				if order_temp.award_count == 180 {
					continue;
				}
				reward_info.currently_available_reward = reward_info.currently_available_reward
					.checked_add(&order_temp.each_share).ok_or(Error::<T>::Overflow)?;

				order_temp.award_count = order_temp.award_count + 1;
			}

			if reward_info.order_list.len() == ORDER_LIST_DEFAULT as usize {
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

	fn delete_miner_info(acc: &AccountOf<T>) -> DispatchResult {
		//There is a judgment on whether the primary key exists above
		let miner = MinerItems::<T>::try_get(&acc).map_err(|_e| Error::<T>::NotMiner)?;

		let mut miner_list = AllMiner::<T>::get();
		miner_list.retain(|s| *s != acc.clone());
		AllMiner::<T>::put(miner_list);

		<MinerItems<T>>::remove(acc);

		Ok(())
	}

	pub fn add_purchased_space(size: u128) -> DispatchResult {
		<PurchasedSpace<T>>::try_mutate(|purchased_space| -> DispatchResult {
			let total_space = <TotalIdleSpace<T>>::get().checked_add(<TotalServiceSpace<T>>::get()).ok_or(Error::<T>::Overflow)?;
			if *purchased_space + size > total_space {
				Err(<Error<T>>::InsufficientAvailableSpace)?;
			}
			*purchased_space = purchased_space.checked_add(size).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	pub fn sub_purchased_space(size: u128) -> DispatchResult {
		<PurchasedSpace<T>>::try_mutate(|purchased_space| -> DispatchResult {
			*purchased_space = purchased_space.checked_sub(size).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	fn check_collateral_limit(power: u128) -> Result<BalanceOf<T>, Error<T>> {
		let mut current_power_num: u128 = 1;
		current_power_num += power.checked_div(1024 * 1024 * M_BYTE).ok_or(Error::<T>::Overflow)?;
		//2000TCESS/TB(space)
		let limit: BalanceOf<T> = (current_power_num
			.checked_mul(2_000_000_000_000_000u128)
			.ok_or(Error::<T>::Overflow)?)
		.try_into()
		.map_err(|_e| Error::<T>::ConversionError)?;

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
}

impl<T: Config> OnUnbalanced<NegativeImbalanceOf<T>> for Pallet<T> {
	fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<T>) {
		let numeric_amount = amount.peek();

		// Must resolve into existing but better to be safe.
		let _ = T::Currency::resolve_creating(&T::PalletId::get().into_account(), amount);
		<CurrencyReward<T>>::mutate(|v| {
			*v = *v + numeric_amount;
		});

		Self::deposit_event(Event::Deposit { balance: numeric_amount });
	}
}

pub trait MinerControl<AccountId, Balance> {
	type Acc;
	type Balance;
	fn add_total_idle_space(space: u128) -> DispatchResult;
	fn add_miner_idle_space(acc: AccountId, space: u128) -> DispatchResult;
	fn sub_total_idle_space(space: u128) -> DispatchResult;
	fn sub_miner_idle_space(acc: AccountId, space: u128) -> DispatchResult;
	fn add_idle_space(acc: AccountId, space: u128) -> DispatchResult;
	fn sub_idle_space(acc: AccountId, space: u128) -> DispatchResult;

	fn add_total_service_space(space: u128) -> DispatchResult;
	fn add_miner_service_space(acc: AccountId, space: u128) -> DispatchResult;
	fn sub_total_service_space(space: u128) -> DispatchResult;
	fn sub_miner_service_space(acc: AccountId, space: u128) -> DispatchResult;
	fn add_service_space(acc: AccountId, space: u128) -> DispatchResult;
	fn sub_service_space(acc: AccountId, space: u128) -> DispatchResult;

	fn add_total_autonomy_space(space: u128) -> DispatchResult;
	fn add_miner_autonomy_space(acc: AccountId, space: u128) -> DispatchResult;
	fn sub_total_autonomy_space(space: u128) -> DispatchResult;
	fn sub_miner_autonomy_space(acc: AccountId, space: u128) -> DispatchResult;
	fn add_autonomy_space(acc: AccountId, space: u128) -> DispatchResult;
	fn sub_autonomy_space(acc: AccountId, space: u128) -> DispatchResult;

	fn get_public(acc: &AccountId) -> Result<Public, DispatchError>;

	fn insert_idle_bloom(miner: &AccountId, hash: [u8; 256]) -> DispatchResult;
	fn insert_service_bloom(miner: &AccountId, hash: [u8; 256]) -> DispatchResult;
	fn insert_autonomy_bloom(miner: &AccountId, hash: [u8; 256]) -> DispatchResult;

	fn remove_idle_bloom(miner: &AccountId, hash: [u8; 256]) -> DispatchResult;
	fn remove_service_bloom(miner: &AccountId, hash: [u8; 256]) -> DispatchResult;
	fn remove_autonomy_bloom(miner: &AccountId, hash: [u8; 256]) -> DispatchResult;
	//One less read/write operation
	fn insert_slice_update_bloom(miner: &AccountId, file_hash: [u8; 256], filler_hash: [u8; 256]) -> DispatchResult;

	fn add_purchased_space(space: u128) -> DispatchResult;
	fn sub_purchased_space(space: u128) -> DispatchResult;
	//For random challenges, When the challenge arises, it is necessary to record the bron filter of all network miners
	fn all_miner_snapshot() -> Vec<(AccountId, BloomCollect, u128)>;
	fn get_total_power() -> u128;
	// for random challenge punish
	fn miner_slice_punish(acc: AccountId, idle_count: u64, service_count: u64, autonomy_count: u64) -> DispatchResult;
	fn miner_clear_punish(acc: AccountId, count: u8) -> Result<Weight, DispatchError>;
	fn force_clear_miner(acc: AccountId) -> Result<Weight, DispatchError>;

	fn get_current_reward() -> Balance;
	fn calculate_reward(reward: Balance, miner: AccountId, total_power: u128, power: u128) -> DispatchResult;

}

impl<T: Config> MinerControl<
	<T as frame_system::Config>::AccountId, 
	BalanceOf<T>
> for Pallet<T> {
	type Acc = <T as frame_system::Config>::AccountId;
	type Balance = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	fn add_total_idle_space(space: u128) -> DispatchResult {
		Self::add_total_idle_space(space)
	}

	fn add_miner_idle_space(acc: Self::Acc, space: u128) -> DispatchResult {
		Self::add_miner_idle_space(acc, space)
	}

	fn sub_total_idle_space(space: u128) -> DispatchResult {
		Self::sub_total_idle_space(space)
	}

	fn sub_miner_idle_space(acc: Self::Acc, space: u128) -> DispatchResult {
		Self::sub_miner_idle_space(acc, space)
	}

	fn add_idle_space(acc: Self::Acc, space: u128) -> DispatchResult {
		Self::add_total_idle_space(space)?;
		Self::add_miner_idle_space(acc, space)
	}

	fn sub_idle_space(acc: Self::Acc, space: u128) -> DispatchResult {
		Self::sub_total_idle_space(space)?;
		Self::sub_miner_idle_space(acc, space)
	}

	fn add_total_service_space(space: u128) -> DispatchResult {
		Self::add_total_service_space(space)
	}

	fn add_miner_service_space(acc: Self::Acc, space: u128) -> DispatchResult {
		Self::add_miner_service_space(acc, space)
	}

	fn sub_total_service_space(space: u128) -> DispatchResult {
		Self::sub_total_service_space(space)
	}

	fn sub_miner_service_space(acc: Self::Acc, space: u128) -> DispatchResult {
		Self::sub_miner_service_space(acc, space)
	}

	fn add_service_space(acc: Self::Acc, space: u128) -> DispatchResult {
		Self::add_total_service_space(space)?;
		Self::add_miner_service_space(acc, space)
	}

	fn sub_service_space(acc: Self::Acc, space: u128) -> DispatchResult {
		Self::sub_total_service_space(space)?;
		Self::sub_miner_service_space(acc, space)
	}

	fn add_total_autonomy_space(space: u128) -> DispatchResult {
		Self::add_total_autonomy_space(space)
	}

	fn add_miner_autonomy_space(acc: Self::Acc, space: u128) -> DispatchResult {
		Self::add_miner_autonomy_space(acc, space)
	}

	fn sub_total_autonomy_space(space: u128) -> DispatchResult {
		Self::sub_total_autonomy_space(space)
	}

	fn sub_miner_autonomy_space(acc: Self::Acc, space: u128) -> DispatchResult {
		Self::sub_miner_autonomy_space(acc, space)
	}

	fn add_autonomy_space(acc: Self::Acc, space: u128) -> DispatchResult {
		Self::add_total_autonomy_space(space)?;
		Self::add_miner_autonomy_space(acc, space)
	}

	fn sub_autonomy_space(acc: Self::Acc, space: u128) -> DispatchResult {
		Self::sub_total_autonomy_space(space)?;
		Self::sub_miner_autonomy_space(acc, space)
	}

	fn get_public(acc: &Self::Acc) -> Result<Public, DispatchError> {
		let miner = MinerItems::<T>::try_get(acc).map_err(|_| Error::<T>::NonExisted)?;
		Ok(miner.puk)
	}

	fn insert_idle_bloom(miner: &Self::Acc, hash: [u8; 256]) -> DispatchResult {
		Self::insert_idle_bloom(miner, hash)
	}

	fn insert_service_bloom(miner: &Self::Acc, hash: [u8; 256]) -> DispatchResult {
		Self::insert_service_bloom(miner, hash)
	}

	fn insert_autonomy_bloom(miner: &Self::Acc, hash: [u8; 256]) -> DispatchResult {
		Self::insert_autonomy_bloom(miner, hash)
	}

	fn remove_idle_bloom(miner: &Self::Acc, hash: [u8; 256]) -> DispatchResult{ 
		Self::remove_idle_bloom(miner, hash)
	}

	fn remove_service_bloom(miner: &Self::Acc, hash: [u8; 256]) -> DispatchResult {
		Self::remove_service_bloom(miner, hash)
	}

	fn remove_autonomy_bloom(miner: &Self::Acc, hash: [u8; 256]) -> DispatchResult {
		Self::remove_autonomy_bloom(miner, hash)
	}

	fn insert_slice_update_bloom(miner: &Self::Acc, file_hash: [u8; 256], filler_hash: [u8; 256]) -> DispatchResult {
		Self::insert_slice_update_bloom(miner, file_hash, filler_hash)
	}

	fn add_purchased_space(space: u128) -> DispatchResult {
		Self::add_purchased_space(space)
	}

	fn sub_purchased_space(space: u128) -> DispatchResult {
		Self::sub_purchased_space(space)
	}

	fn all_miner_snapshot() -> Vec<(Self::Acc, BloomCollect, u128)> {
		let mut result: Vec<(Self::Acc, BloomCollect, u128)> = Default::default();
		for (miner_acc, miner_info) in MinerItems::<T>::iter() {
			let power = miner_info.calculate_power();
			// If the calculation power of the miner is 0, 
			// it is unnecessary to carry out this challenge.
			if power == 0 {
				continue;
			}
			let bloom = BloomCollect {
				idle_filter: miner_info.bloom_filter.idle_filter,
				service_filter: miner_info.bloom_filter.service_filter,
				autonomy_filter: miner_info.bloom_filter.autonomy_filter,
			};
			result.push((miner_acc, bloom, power));
		}
		result
	}

	fn get_total_power() -> u128 {
		let idle_space = <TotalIdleSpace<T>>::get();
		let service_space = <TotalServiceSpace<T>>::get();
		let autonomy_space = <TotalAutonomySpace<T>>::get();

		let autonomy_power = AUTONOMY_MUTI.mul_floor(autonomy_space);
        let service_power = SERVICE_MUTI.mul_floor(service_space);
        let idle_power = IDLE_MUTI.mul_floor(idle_space);

        let power: u128 = autonomy_power + service_power + idle_power;
		
		power
	}

	fn miner_slice_punish(acc: Self::Acc, idle_count: u64, service_count: u64, autonomy_count: u64) -> DispatchResult {
		Self::miner_slice_punish(acc, idle_count.into(), service_count.into(), autonomy_count.into())
	}

	fn get_current_reward() -> Self::Balance {
		<CurrencyReward<T>>::get()
	}

	fn calculate_reward(reward: Self::Balance, miner: Self::Acc, total_power: u128, power: u128) -> DispatchResult {
		Self::calculate_reward(reward, miner, total_power, power)
	}

	fn miner_clear_punish(acc: Self::Acc, count: u8) -> Result<Weight, DispatchError> {
		Self::miner_clear_punish(acc, count)
	}

	fn force_clear_miner(acc: Self::Acc) -> Result<Weight, DispatchError> {
		let mut weight: Weight = 0;
		let miner = <MinerItems<T>>::try_get(&acc).map_err(|_| Error::<T>::NonExisted)?;
		weight = weight.saturating_add(T::DbWeight::get().reads(1 as Weight));

		let reward_pot = T::PalletId::get().into_account();
		T::Currency::unreserve(&acc, miner.collaterals.clone());
		<T as pallet::Config>::Currency::transfer(&acc, &reward_pot, miner.collaterals, AllowDeath)?;

		Self::sub_total_idle_space(miner.idle_space)?;
		Self::sub_total_service_space(miner.service_space)?;
		Self::sub_total_autonomy_space(miner.autonomy_space)?;
		weight = weight.saturating_add(T::DbWeight::get().reads_writes(3, 3));

		if let Ok(reward_info) = <RewardMap<T>>::try_get(&acc).map_err(|_| Error::<T>::NonExisted) {
			let reward = reward_info.total_reward
				.checked_sub(&reward_info.reward_issued).ok_or(Error::<T>::Overflow)?;
			<T as pallet::Config>::Currency::transfer(&acc, &reward_pot, reward, AllowDeath)?;
		}

		let mut miner_list = AllMiner::<T>::get();
		miner_list.retain(|s| *s != acc.clone());
		AllMiner::<T>::put(miner_list);

		<RewardMap<T>>::remove(&acc);
		<MinerItems<T>>::remove(&acc);
		weight = weight.saturating_add(T::DbWeight::get().reads(2 as Weight));

		Ok(weight)
	}
}