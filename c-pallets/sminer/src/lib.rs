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

use frame_support::traits::{
	Get, 
	Currency, 
	ReservableCurrency, 
	LockIdentifier, 
	OnUnbalanced,
	Imbalance,
	schedule::{
		Named as ScheduleNamed, 
		DispatchTime
	}, 
	ExistenceRequirement::AllowDeath
};
use frame_support::storage::bounded_vec::BoundedVec;
mod benchmarking;
pub mod weights;
mod types;
use types::*;
pub use pallet::*;
use sp_runtime::{
	RuntimeDebug,
	traits::{AccountIdConversion, StaticLookup, SaturatedConversion},
};
use sp_std::prelude::*;
use codec::{Encode, Decode};
use scale_info::TypeInfo;
use sp_std::convert::TryInto;
use frame_system::{self as system};
use frame_support::{dispatch::{Dispatchable, DispatchResult}, PalletId};
pub use weights::WeightInfo;
use sp_runtime::traits::CheckedAdd;
use sp_runtime::traits::CheckedSub;
use frame_support::pallet_prelude::DispatchError;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type NegativeImbalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::NegativeImbalance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
const M_BYTE: u128 = 1_048_576;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		ensure,
		pallet_prelude::*,
		traits::Get,
	};
	use frame_system::{ensure_root, ensure_signed, pallet_prelude::*};

	const DEMOCRACY_IDA: LockIdentifier = *b"msminerA";
	const DEMOCRACY_IDB: LockIdentifier = *b"msminerB";
	const DEMOCRACY_IDC: LockIdentifier = *b"msminerC";
	
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
		/// The Scheduler.
		type SScheduler: ScheduleNamed<Self::BlockNumber, Self::SProposal, Self::SPalletsOrigin>;
		/// Overarching type of all pallets origins.
		type SPalletsOrigin: From<system::RawOrigin<Self::AccountId>>;
		/// The SProposal.
		type SProposal: Parameter + Dispatchable<Origin=Self::Origin> + From<Call<Self>>;
		/// The WeightInfo.
		type WeightInfo: WeightInfo;

	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new account was set.
		Registered{acc: AccountOf<T>, staking_val: BalanceOf<T>},
		/// An account was redeemed.
		Redeemed{acc: AccountOf<T>, deposit: BalanceOf<T>},
		/// An account was claimed.
		Claimed{acc: AccountOf<T>, deposit: BalanceOf<T>},
		/// Storage space is triggered periodically.
		TimingStorageSpace(),
		/// Scheduled Task Execution
		TimedTask(),
		/// Users to withdraw faucet money
		DrawFaucetMoney(),
		/// User recharges faucet
		FaucetTopUpMoney{acc: AccountOf<T>},
		/// Prompt time
		LessThan24Hours{last: BlockNumberOf<T>, now: BlockNumberOf<T>},
		//The miners have been frozen
		AlreadyFrozen{acc: AccountOf<T>},
		//Miner exit event
		MinerExit{acc: AccountOf<T>},

		MinerClaim{acc: AccountOf<T>},

		IncreaseCollateral{acc: AccountOf<T>, balance: BalanceOf<T>},
		/// Some funds have been deposited. \[deposit\]
		Deposit{balance: BalanceOf<T>},
	}

	/// Error for the sminer pallet.
	#[pallet::error]
	pub enum Error<T> {
		NotMiner,
		/// An account doesn't registered.
		UnregisteredAccountId,
		/// An account has locked balances.
		LockedNotEmpty,
		/// An account already registered.
		AlreadyRegistered,
		/// An account's earnings is empty.
		EarningsIsEmpty,
		/// An operation would lead to an overflow.
		Overflow,
		/// No owner.
		// NotOwner,
		/// User does not exist.
		NotExisted,	
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

		CollingNotOver,

		NotpositiveState,
	}

	#[pallet::storage]
	#[pallet::getter(fn miner_cooling)]
	pub(super) type MinerColling<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BlockNumberOf<T>>;

	/// The hashmap for info of storage miners.
	#[pallet::storage]
	#[pallet::getter(fn miner_items)]
	pub(super) type MinerItems<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Mr<T::AccountId, BalanceOf<T>, BoundedVec<u8, T::ItemLimit>> >;

	/// The hashmap for index of storage miners, it's unique to whole system.
	#[pallet::storage]
	#[pallet::getter(fn peer_index)]
	pub(super) type PeerIndex<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// The total power of all storage miners.
	#[pallet::storage]
	#[pallet::getter(fn total_power)]
	pub(super) type TotalPower<T: Config> = StorageValue<_, u128, ValueQuery>;

	/// The total storage space to fill of all storage miners.
	#[pallet::storage]
	#[pallet::getter(fn total_space)]
	pub(super) type TotalSpace<T: Config> = StorageValue<_, u128, ValueQuery>;
	/// Store all miner information
	#[pallet::storage]
	#[pallet::getter(fn miner_info)]
	pub(super) type AllMiner<T: Config> = StorageValue<_, BoundedVec<MinerInfo<BoundedVec<u8, T::ItemLimit>>, T::ItemLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn purchased_space)]
	pub(super) type PurchasedSpace<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn available_space)]
	pub(super) type AvailableSpace<T: Config> = StorageValue<_, u128, ValueQuery>;

	/// Store all miner table information 
	#[pallet::storage]
	#[pallet::getter(fn miner_table)]
	pub(super) type MinerTable<T: Config> = StorageMap<_, Twox64Concat, u64, TableInfo<T::AccountId, BalanceOf<T>>>;

	/// Store all miner details information 
	#[pallet::storage]
	#[pallet::getter(fn miner_details)]
	pub(super) type MinerDetails<T: Config> = StorageMap<_, Twox64Concat, u64, MinerDetailInfo<T::AccountId, BalanceOf<T>, BoundedVec<u8, T::ItemLimit>>>;

	/// Store all miner stat information 
	#[pallet::storage]
	#[pallet::getter(fn miner_stat_value)]
	pub(super) type MinerStatValue<T: Config> = StorageValue<_, MinerStatInfo<BalanceOf<T>>>; 

	/// The hashmap for info of storage miners.
	#[pallet::storage]
	#[pallet::getter(fn calculate_reward_order)]
	pub(super) type CalculateRewardOrderMap<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<CalculateRewardOrder<T>, T::ItemLimit>, ValueQuery>;

	/// The hashmap for checking registered or not.
	#[pallet::storage]
	#[pallet::getter(fn reward_claim)]
	pub(super) type RewardClaimMap<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, RewardClaim<T::AccountId, BalanceOf<T>>>;

	/// The hashmap for checking registered or not.
	#[pallet::storage]
	#[pallet::getter(fn faucet_record)]
	pub(super) type FaucetRecordMap<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, FaucetRecord<BlockNumberOf<T>>>;

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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::regnstk())]
		pub fn regnstk(
				origin: OriginFor<T>, 
				beneficiary: AccountOf<T>,
				ip: Vec<u8>, 
				staking_val: BalanceOf<T>,
				public_key: Vec<u8>,
			) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!(<MinerItems<T>>::contains_key(&sender)), Error::<T>::AlreadyRegistered);
			T::Currency::reserve(&sender, staking_val.clone())?;
			let value = BalanceOf::<T>::from(0 as u32);
			let cur_idx = PeerIndex::<T>::get();
			let peerid = cur_idx.checked_add(1).ok_or(Error::<T>::Overflow)?;
			<MinerItems<T>>::insert(
				&sender,
				Mr::<T::AccountId, BalanceOf<T>, BoundedVec<u8, T::ItemLimit>> {
					peerid: peerid,
					beneficiary: beneficiary.clone(),
					ip: Self::vec_to_bound::<u8>(ip.clone())?,
					collaterals: staking_val.clone(),
					earnings: value.clone(),
					locked: value.clone(),
					state: Self::vec_to_bound::<u8>("positive".as_bytes().to_vec())?,
					power: 0,
					space: 0,
					public_key: Self::vec_to_bound::<u8>(public_key)?,
				}
			);

			<PeerIndex<T>>::put(peerid);

			let bounded_ip = Self::vec_to_bound::<u8>(ip)?;
			let add_minerinfo = MinerInfo::<BoundedVec<u8, T::ItemLimit>> {
				peerid: peerid,
				ip: bounded_ip.clone(),
				power: 0 as u128,
				space: 0 as u128,
			};
			AllMiner::<T>::try_mutate(|s| -> DispatchResult {
				s.try_push(add_minerinfo).expect("Maximum length exceeded");
				Ok(())
			})?;

			<MinerTable<T>>::insert(
				peerid,
				TableInfo::<T::AccountId, BalanceOf<T>> {
					address: sender.clone(),
					beneficiary: beneficiary.clone(),
					total_storage: 0u128,
					average_daily_data_traffic_in: 0u64,
					average_daily_data_traffic_out: 0u64,
					mining_reward: BalanceOf::<T>::from(0u32),
				}
			);

			<MinerDetails<T>>::insert(
				peerid,
				MinerDetailInfo::<T::AccountId, BalanceOf<T>, BoundedVec<u8, T::ItemLimit>> {
					address: sender.clone(),
					beneficiary,
					power: 0u128,
					space: 0u128,
					ip: bounded_ip,
					total_reward: BalanceOf::<T>::from(0u32),
					total_rewards_currently_available: BalanceOf::<T>::from(0u32),
					totald_not_receive: BalanceOf::<T>::from(0u32),
				}
			);

			MinerStatValue::<T>::try_mutate(|s_opt| -> DispatchResult {
				let s = s_opt.as_mut().unwrap();
				s.total_miners = s.total_miners.checked_add(1).ok_or(Error::<T>::Overflow)?;
				s.active_miners = s.active_miners.checked_add(1).ok_or(Error::<T>::Overflow)?;
				s.staking = s.staking.checked_add(&staking_val).ok_or(Error::<T>::Overflow)?;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::Registered{acc: sender.clone(), staking_val: staking_val.clone()});
			Ok(())
		}

		#[pallet::weight(1_000_000)]
		pub fn increase_collateral(origin: OriginFor<T>, #[pallet::compact] collaterals: BalanceOf<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			
			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);
			T::Currency::reserve(&sender, collaterals)?;
			let mut balance: BalanceOf<T> = 0u32.saturated_into();
			<MinerItems<T>>::try_mutate(&sender, |s_opt| -> DispatchResult {
				let s = s_opt.as_mut().unwrap();
				s.collaterals = s.collaterals.checked_add(&collaterals).ok_or(Error::<T>::Overflow)?;
				balance = s.collaterals;
				if s.state == "frozen".as_bytes().to_vec() || s.state == "e_frozen".as_bytes().to_vec() {
					let limit = Self::check_collateral_limit(s.peerid)?;
					if s.collaterals > limit {
						if s.state.to_vec() == "frozen".as_bytes().to_vec() {
							s.state = Self::vec_to_bound("positive".as_bytes().to_vec())?;
						} else {
							s.state = Self::vec_to_bound("exit".as_bytes().to_vec())?;
						}
					}
				}

				Ok(())
			})?;

			MinerStatValue::<T>::try_mutate(|s_opt| -> DispatchResult {
				let s = s_opt.as_mut().unwrap();
				s.staking = s.staking.checked_add(&collaterals).ok_or(Error::<T>::Overflow)?;
				Ok(())
			})?;
			Self::deposit_event(Event::<T>::IncreaseCollateral{acc: sender, balance: balance});

			Ok(())
		}

		//Miner exit method, Irreversible process.
		#[pallet::weight(1_000_000)]
		pub fn exit_miner(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);
			let state = Self::check_state(sender.clone());
			if state != "positive".as_bytes().to_vec() {
				Err(Error::<T>::NotpositiveState)?;
			}

			MinerItems::<T>::try_mutate(&sender, |s_opt| -> DispatchResult {
				let s = s_opt.as_mut().unwrap();
				
				Self::sub_space(s.peerid, s.space)?;
				Self::sub_power(s.peerid, s.power)?;
				
				s.state = Self::vec_to_bound("exit".as_bytes().to_vec())?;
				Ok(())
			})?; 

			let now = <frame_system::Pallet<T>>::block_number();
			MinerColling::<T>::insert(&sender, now);

			Self::deposit_event(Event::<T>::MinerExit{acc: sender});
			Ok(())

		}

		//Method for miners to redeem deposit
		#[pallet::weight(2_000_000)]
		pub fn withdraw(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(MinerItems::<T>::contains_key(&sender), Error::<T>::NotMiner);
			let state = Self::check_state(sender.clone());
			if state != "exit".as_bytes().to_vec() {
				Err(Error::<T>::NotExisted)?;
			}
			let now: u128 = <frame_system::Pallet<T>>::block_number().saturated_into();
			let colling_line: u128 = MinerColling::<T>::get(&sender).unwrap().saturated_into();
			if colling_line + 1200 > now {
				Err(Error::<T>::CollingNotOver)?;
			}

			let collaterals = MinerItems::<T>::get(&sender).unwrap().collaterals;
			T::Currency::unreserve(&sender, collaterals);
			Self::delete_miner_info(sender.clone())?;
			MinerColling::<T>::remove(&sender);

			Self::deposit_event(Event::<T>::MinerClaim{acc: sender});
			Ok(())
		}
		
		/// Miner information initialization.
		/// The dispatch origin of this call must be _root_.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::initi())]
		pub fn initi(origin: OriginFor<T>) -> DispatchResult {
			//sudo call
			let _ = ensure_root(origin)?;
			let value = BalanceOf::<T>::from(0 as u32);
			let mst = MinerStatInfo::<BalanceOf<T>> {
				total_miners: 0u64,
				active_miners: 0u64,
				staking: value,
				miner_reward: value,
				sum_files: 0u128,
			};
			<MinerStatValue<T>>::put(mst);
			Ok(())
		}

		

		// 	storage_info_vec.append(&mut info1);

		// 	<StorageInfoVec<T>>::put(storage_info_vec);
		// 	Self::deposit_event(Event::<T>::TimingStorageSpace());
		// 	Ok(())
		// }
		/// Add reward orders.
		///
		#[pallet::weight(<T as pallet::Config>::WeightInfo::timed_increase_rewards())]
		pub fn timed_increase_rewards(origin: OriginFor<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;
			let total_power = <TotalPower<T>>::get();
			ensure!(total_power != 0, Error::<T>::DivideByZero);
			// let reward_pot = T::PalletId::get().into_account();

			for (_, detail) in <MinerDetails<T>>::iter() {
				if detail.power == 0 {
					continue;
				}
				let tmp1:u128 = 750_000_000_000_000_000_u128.checked_mul(detail.power).ok_or(Error::<T>::Overflow)?;
				let tmp2:u128 = tmp1.checked_div(total_power).ok_or(Error::<T>::Overflow)?;
				let _ = Self::add_reward_order1(detail.address,tmp2);

				//Give 20% reward to users in advance
				// let reward_20_percent: BalanceOf<T> = (tmp2.checked_mul(2).ok_or(Error::<T>::Overflow)?.checked_div(10).ok_or(Error::<T>::Overflow)?)
				// .try_into()
				// .map_err(|_e| Error::<T>::ConversionError)?;

				// <T as pallet::Config>::Currency::transfer(&reward_pot, &detail.beneficiary, reward_20_percent, AllowDeath)?;
			}
			let reward3:BalanceOf<T> = 750000000000000000_u128.try_into().map_err(|_e| Error::<T>::ConversionError)?;

			MinerStatValue::<T>::try_mutate(|s_opt| -> DispatchResult {
				let s = s_opt.as_mut().unwrap();
				s.miner_reward = s.miner_reward.checked_add(&reward3).ok_or(Error::<T>::Overflow)?;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::TimedTask());
			Ok(())
		}
		/// Added timed tasks for reward orders.
		///
		/// The dispatch origin of this call must be _root_.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::timing_task_increase_power_rewards())]
		pub fn timing_task_increase_power_rewards(origin: OriginFor<T>, when: BlockNumberOf<T>, cycle: BlockNumberOf<T>, degree: u32) -> DispatchResult {
			let _ = ensure_root(origin)?;

			if T::SScheduler::schedule_named(
				(DEMOCRACY_IDA).encode(),
				DispatchTime::At(when),
				Some(( cycle, degree)),
				60,
				frame_system::RawOrigin::Root.into(),
				Call::timed_increase_rewards{}.into(),
			).is_err() {
				frame_support::print("LOGIC ERROR: timed_increase_rewards/schedule_named failed");
			}

			// Self::deposit_event(Event::<T>::Add(sender.clone()));
			Ok(())
		}
		/// Delete reward orders.
		///
		/// The dispatch origin of this call must be _root_.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::del_reward_order())]
		pub fn del_reward_order(origin: OriginFor<T>,acc: AccountOf<T>, order_num: u128) -> DispatchResult {
			let _ = ensure_root(origin)?;

			ensure!(CalculateRewardOrderMap::<T>::contains_key(&acc), Error::<T>::NotExisted);
			let mut order_vec = CalculateRewardOrderMap::<T>::get(&acc);
			order_vec.remove(order_num.try_into().unwrap());
			<CalculateRewardOrderMap<T>>::insert(
				acc,
				order_vec,
			);

			// Self::deposit_event(Event::<T>::Del(sender.clone()));
			Ok(())
		}

		// #[pallet::weight(50_000_000)]
		// pub fn user_receive_award(origin: OriginFor<T>, award: BalanceOf<T>) -> DispatchResult {
		// 	let sender = ensure_signed(origin)?;
			
		// 	let acc = Self::get_acc(sender);

		// 	ensure!(RewardClaimMap::<T>::contains_key(&sender), Error::<T>::NotExisted);
			
		// 	let reward_pot = T::PalletId::get().into_account();

		// 	let reward_claim1 = RewardClaimMap::<T>::get(&sender).unwrap();
			
		// 	ensure!((reward_claim1.have_to_receive + award) <= reward_claim1.total_rewards_currently_available, Error::<T>::BeyondClaim);
			
		// 	<T as pallet::Config>::Currency::transfer(&reward_pot, &acc, award, AllowDeath)?;

		// 	RewardClaimMap::<T>::mutate(&sender, |reward_claim_opt| {
		// 		let reward_claim = reward_claim_opt.as_mut().unwrap();
		// 		reward_claim.have_to_receive = reward_claim.have_to_receive + award;
		// 	});

		// 	Self::deposit_event(Event::<T>::DrawMoney(sender.clone()));
		// 	Ok(())
		// }

		/// Users receive rewards for scheduled tasks.
		///
		#[pallet::weight(<T as pallet::Config>::WeightInfo::timed_user_receive_award1())]
		pub fn timed_user_receive_award1(origin: OriginFor<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;
			for (sender, info) in <RewardClaimMap<T>>::iter() {
				
				let acc = info.beneficiary;
				let state = <MinerItems<T>>::get(&sender).unwrap().state;
				if state == "frozen".as_bytes().to_vec() || state == "e_frozen".as_bytes().to_vec() {
					Self::deposit_event(Event::<T>::AlreadyFrozen{acc: acc.clone()});
					continue;
				}
				
				let reward_pot = T::PalletId::get().into_account();

				let reward_claim1 = RewardClaimMap::<T>::get(&sender).unwrap();
				
				let award = reward_claim1.current_availability;
				let total = reward_claim1.total_reward;

				ensure!(reward_claim1.have_to_receive.checked_add(&award).ok_or(Error::<T>::Overflow)? <= reward_claim1.total_reward, Error::<T>::BeyondClaim);
				
				<T as pallet::Config>::Currency::transfer(&reward_pot, &acc, award, AllowDeath)?;

				RewardClaimMap::<T>::try_mutate(&sender, |reward_claim_opt| -> DispatchResult {
					let reward_claim = reward_claim_opt.as_mut().unwrap();
					let have_to_receive = reward_claim.have_to_receive.checked_add(&award).ok_or(Error::<T>::Overflow)?;
					reward_claim.have_to_receive = have_to_receive;
					reward_claim.current_availability = 0u32.into();
					reward_claim.total_not_receive = total.checked_sub(&award).ok_or(Error::<T>::Overflow)?;
					Ok(())
				})?;
				let peerid = Self::get_peerid(&sender);
				MinerDetails::<T>::try_mutate(peerid, |miner_detail_opt| -> DispatchResult {
					let miner_detail = miner_detail_opt.as_mut().unwrap();
					let total_not_receive = RewardClaimMap::<T>::get(&sender).unwrap().total_not_receive;
					miner_detail.totald_not_receive = total_not_receive;
					Ok(())
				})?;

				if Self::check_exist_miner_reward(sender.clone()) {
					Self::clean_reward_map(sender)
				}
			}
			Ok(())
		}
		/// Users receive rewards for scheduled tasks.
		///
		/// The dispatch origin of this call must be _root_.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::timing_user_receive_award())]
		pub fn timing_user_receive_award(origin: OriginFor<T>, when: BlockNumberOf<T>, cycle: BlockNumberOf<T>, degree: u32) -> DispatchResult {
			let _ = ensure_root(origin)?;

			if T::SScheduler::schedule_named(
				(DEMOCRACY_IDC).encode(),
				DispatchTime::At(when),
				Some(( cycle, degree)),
				63,
				frame_system::RawOrigin::Root.into(),
				Call::timed_user_receive_award1{}.into(),
			).is_err() {
				frame_support::print("LOGIC ERROR: timed_user_receive_award1/schedule_named failed");
			}

			// Self::deposit_event(Event::<T>::Add(sender.clone()));
			Ok(())
		}
		/// Update the user reward table for scheduled tasks.
		///
		#[pallet::weight(<T as pallet::Config>::WeightInfo::timed_task_award_table())]
		pub fn timed_task_award_table(origin: OriginFor<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;
			for (_acc, order_vec) in <CalculateRewardOrderMap<T>>::iter() {
				let mut total:u128 = 0;

				let now = <frame_system::Pallet<T>>::block_number();
				let mut avail:BalanceOf<T> = 0u128.try_into().map_err(|_e| Error::<T>::ConversionError)?;

				for i in &order_vec.to_vec(){
					total = total.checked_add(i.calculate_reward).ok_or(Error::<T>::Overflow)?;
					if i.deadline > now {
						// let tmp1 = TryInto::<u128>::try_into(now.checked_sub(&i.start_t).ok_or(Error::<T>::Overflow)?).ok().unwrap();
						// let day:u128 = tmp/28800+1;
						// // test 5 minutes
						// let day:u128 = tmp/100+1;
						// test 6 hours
						// let day:u128 = tmp1.checked_div(7200).ok_or(Error::<T>::Overflow)?.checked_add(1).ok_or(Error::<T>::Overflow)?;
						let tmp2:BalanceOf<T> = (i.calculate_reward*8/10/180).try_into().map_err(|_e| Error::<T>::ConversionError)?;
						avail = avail.checked_add(&tmp2).ok_or(Error::<T>::Overflow)?;
					} 
					// else {
					// 	let tmp1:BalanceOf<T> = (i.calculate_reward*8/10).try_into().map_err(|_e| Error::<T>::ConversionError)?;
					// 	avail = avail.checked_add(&tmp1).ok_or(Error::<T>::Overflow)?;
					// 	// Call::del_order(_acc,i);
					// }
				}

				let mut order_clone = order_vec.clone();
				order_clone.retain(|item| {
					if item.deadline > now {
						true
					} else {
						false
					}
				});

				let total_20_percent:BalanceOf<T> = total
						.checked_mul(2).ok_or(Error::<T>::Overflow)?
						.checked_div(10).ok_or(Error::<T>::Overflow)?
						.try_into().map_err(|_e| Error::<T>::ConversionError)?;

				let currently_available:BalanceOf<T> = avail;

				let reward2:BalanceOf<T> = total.try_into().map_err(|_e| Error::<T>::ConversionError)?;

				let miner = MinerItems::<T>::get(&_acc).unwrap();
				let peerid = miner.peerid;
				MinerTable::<T>::try_mutate(peerid, |s_opt| -> DispatchResult {
					let s = s_opt.as_mut().unwrap();
					s.mining_reward = reward2;
					Ok(())
				})?;

				if !<RewardClaimMap<T>>::contains_key(&_acc) {
					<RewardClaimMap<T>>::insert(
						&_acc, 
						RewardClaim::<T::AccountId, BalanceOf<T>> {
							beneficiary: miner.beneficiary,
							total_reward: reward2,
							have_to_receive: 0u32.into(),
							current_availability: currently_available.checked_add(&total_20_percent).ok_or(Error::<T>::Overflow)?,
							total_not_receive: reward2,
						}
					);

					
					let peerid = MinerItems::<T>::get(&_acc).unwrap().peerid;
					if <MinerDetails<T>>::contains_key(peerid) {
						MinerDetails::<T>::try_mutate(peerid, |miner_detail_opt| -> DispatchResult {
							let miner_detail = miner_detail_opt.as_mut().unwrap();
							miner_detail.total_reward = reward2;
							miner_detail.total_rewards_currently_available = currently_available.checked_add(&total_20_percent).ok_or(Error::<T>::Overflow)?;
							miner_detail.totald_not_receive = reward2;
							Ok(())
						})?;
					}
					
				} else {
					RewardClaimMap::<T>::try_mutate(&_acc, |reward_claim_opt| -> DispatchResult {
						//Convert balance to U128 for multiplication and division
						let reward_claim = reward_claim_opt.as_mut().unwrap();
						let diff = reward2.checked_sub(&reward_claim.total_reward).ok_or(Error::<T>::Overflow)?;
						let diff128 = TryInto::<u128>::try_into(diff).ok().unwrap();
						let diff_20_percent: BalanceOf<T> = diff128
							.checked_mul(2).ok_or(Error::<T>::Overflow)?
							.checked_div(10).ok_or(Error::<T>::Overflow)?
							.try_into()
							.map_err(|_e| Error::<T>::ConversionError)?;
						//Before switching back to balance
						//Plus 20% of the new share
						reward_claim.total_reward = reward2;
						reward_claim.current_availability = reward_claim.current_availability
							.checked_add(&currently_available).ok_or(Error::<T>::Overflow)?
							.checked_add(&diff_20_percent).ok_or(Error::<T>::Overflow)?;
						reward_claim.total_not_receive = reward2
							.checked_sub(&reward_claim.have_to_receive).ok_or(Error::<T>::Overflow)?;
						Ok(())
					})?;

					let peerid = MinerItems::<T>::get(&_acc).unwrap().peerid;
					if <MinerDetails<T>>::contains_key(peerid) {
						MinerDetails::<T>::try_mutate(peerid, |miner_detail_opt| -> DispatchResult {
							let miner_detail = miner_detail_opt.as_mut().unwrap();
							miner_detail.total_reward = reward2;

							let reward_claim_map = RewardClaimMap::<T>::get(&_acc).unwrap();
							miner_detail.total_rewards_currently_available = reward_claim_map.have_to_receive
								.checked_add(&reward_claim_map.current_availability).ok_or(Error::<T>::Overflow)?;

							let total_not_receive = RewardClaimMap::<T>::get(&_acc).unwrap().total_not_receive;
							miner_detail.totald_not_receive = total_not_receive;
							Ok(())
						})?;
					}
				}

				<CalculateRewardOrderMap<T>>::insert(_acc, order_clone);
			}

			Self::deposit_event(Event::<T>::TimedTask());
			Ok(())
		}
		/// Update the user reward table for scheduled tasks.
		///
		/// The dispatch origin of this call must be _root_.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::timing_task_award_table())]
		pub fn timing_task_award_table(origin: OriginFor<T>, when: BlockNumberOf<T>, cycle: BlockNumberOf<T>, degree: u32) -> DispatchResult {
			let _ = ensure_root(origin)?;

			if T::SScheduler::schedule_named(
				(DEMOCRACY_IDB).encode(),
				DispatchTime::At(when),
				Some(( cycle, degree)),
				61,
				frame_system::RawOrigin::Root.into(),
				Call::timed_task_award_table{}.into(),
			).is_err() {
				frame_support::print("LOGIC ERROR: timed_task_receive_award/schedule_named failed");
			}

			// Self::deposit_event(Event::<T>::Add(sender.clone()));
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
		#[pallet::weight(<T as pallet::Config>::WeightInfo::faucet_top_up())]
		pub fn faucet_top_up(origin: OriginFor<T>, award: BalanceOf<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let reward_pot = T::PalletId::get().into_account();
				
			<T as pallet::Config>::Currency::transfer(&sender, &reward_pot, award, AllowDeath)?;

			Self::deposit_event(Event::<T>::FaucetTopUpMoney{acc: sender.clone()});
			Ok(())
		}
		/// Users receive money through the faucet.
		///
		/// The dispatch origin of this call must be _Signed_.
		///
		/// Parameters:
		/// - `acc`: Withdraw money account.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::faucet())]
		pub fn faucet(origin: OriginFor<T>, to: AccountOf<T>) -> DispatchResult {
			let _ = ensure_signed(origin)?;

			if !<FaucetRecordMap<T>>::contains_key(&to) {
				<FaucetRecordMap<T>>::insert(
					&to,
					FaucetRecord::<BlockNumberOf<T>> {
						last_claim_time: BlockNumberOf::<T>::from(0u32),
					}
				);

				let now = <frame_system::Pallet<T>>::block_number();
				let reward_pot = T::PalletId::get().into_account();
				<T as pallet::Config>::Currency::transfer(&reward_pot, &to, 10000000000000000u128.try_into().map_err(|_e| Error::<T>::ConversionError)?, AllowDeath)?;
				<FaucetRecordMap<T>>::insert(
					&to,
					FaucetRecord::<BlockNumberOf<T>> {
						last_claim_time: now,
					}
				);
			} else {
				let faucet_record = FaucetRecordMap::<T>::get(&to).unwrap();
				let now = <frame_system::Pallet<T>>::block_number();

				let mut flag: bool = true;
				if now >= BlockNumberOf::<T>::from(28800u32) {
					if !(faucet_record.last_claim_time <= now.checked_sub(&BlockNumberOf::<T>::from(28800u32)).ok_or(Error::<T>::Overflow)?) {
						Self::deposit_event(Event::<T>::LessThan24Hours{last: faucet_record.last_claim_time, now: now});
						flag = false;
					}
				} else {
					if !(faucet_record.last_claim_time <= BlockNumberOf::<T>::from(0u32)) {
						Self::deposit_event(Event::<T>::LessThan24Hours{last: faucet_record.last_claim_time, now: now});
						flag = false;
					}
				}
				ensure!(flag , Error::<T>::LessThan24Hours);
				
				let reward_pot = T::PalletId::get().into_account();
				<T as pallet::Config>::Currency::transfer(&reward_pot, &to, 10000000000000000u128
					.try_into()
					.map_err(|_e| Error::<T>::ConversionError)?, AllowDeath)?;
				<FaucetRecordMap<T>>::insert(
					&to,
					FaucetRecord::<BlockNumberOf<T>> {
						last_claim_time: now,
					}
				);
			}
			Self::deposit_event(Event::<T>::DrawFaucetMoney());
			Ok(())
		}

		/// Test method for increasing computational power.
		///
		/// The dispatch origin of this call must be _root_.
		///
		/// Parameters:
		/// - `peerid`: The miners' peerid.
		/// - `increment`: Increased computational power.
		#[pallet::weight(50_000_000)]
		pub fn add_power_test(origin: OriginFor<T>, peerid: u64, increment: u128) -> DispatchResult {
			let _ = ensure_root(origin)?;
			let _ = Self::add_power(peerid,increment);
			Self::deposit_event(Event::<T>::DrawFaucetMoney());
			Ok(())
		}

	}
}

impl<T: Config> Pallet<T> {
	/// Use aid to get to peerid.
	///
	/// Parameters:
	/// - `aid`: aid.
	pub fn get_peerid(aid: &AccountOf<T>) -> u64 {
		if !<MinerItems<T>>::contains_key(&aid) {
			frame_support::print("UnregisteredAccountId");
		}
		let peerid = MinerItems::<T>::get(&aid).unwrap().peerid;
		peerid
	}
	/// Add computing power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn add_power(peerid: u64, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerDetails<T>>::contains_key(peerid) {
			return Ok(());
		}

		let acc = Self::get_acc(peerid)?;
		let state = Self::check_state(acc.clone());
		if state == "exit".as_bytes().to_vec() {
			return Ok(())
		}
		Self::add_available_space(increment.clone())?;
		MinerItems::<T>::try_mutate(&acc, |s_opt| -> DispatchResult{
			let s = s_opt.as_mut().unwrap();
			s.power = s.power.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		TotalPower::<T>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		MinerTable::<T>::try_mutate(peerid, |s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.total_storage = s.total_storage.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		MinerDetails::<T>::try_mutate(peerid, |s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.power = s.power.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		let mut allminer = AllMiner::<T>::get();
		let mut k = 0;
		for i in allminer.clone().iter() {
			if i.peerid == peerid {
				let newminer = MinerInfo::<BoundedVec<u8, T::ItemLimit>> {
					peerid: i.peerid,
					ip: i.ip.clone(),
					power: i.power.checked_add(increment).ok_or(Error::<T>::Overflow)?,
					space: i.space,
				};
				allminer.remove(k);
				allminer.try_push(newminer).expect("Maximum length exceeded");
			}
			k = k.checked_add(1).ok_or(Error::<T>::Overflow)?;
		}
		AllMiner::<T>::put(allminer);
		Ok(())
	}
	/// Sub computing power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn sub_power(peerid: u64, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerDetails<T>>::contains_key(peerid) {
			return Ok(());
		}

		let acc = Self::get_acc(peerid)?;
		let state = Self::check_state(acc.clone());
		if state == "exit".as_bytes().to_vec() {
			return Ok(())
		}
		Self::sub_available_space(increment.clone())?;
		MinerItems::<T>::try_mutate(&acc, |s_opt| -> DispatchResult{
			let s = s_opt.as_mut().unwrap();
			s.power = s.power.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		MinerTable::<T>::try_mutate(peerid, |s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.total_storage = s.total_storage.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		MinerDetails::<T>::try_mutate(peerid, |s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.power = s.power.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		TotalPower::<T>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		let mut allminer = AllMiner::<T>::get();
		let mut k = 0;
		for i in allminer.clone().iter() {
			if i.peerid == peerid {
				let newminer = MinerInfo::<BoundedVec<u8, T::ItemLimit>> {
					peerid: i.peerid,
					ip: i.ip.clone(),
					power: i.power.checked_sub(increment).ok_or(Error::<T>::Overflow)?,
					space: i.space,
				};
				allminer.remove(k);
				allminer.try_push(newminer).expect("Maximum length exceeded");
			}
			k = k.checked_add(1).ok_or(Error::<T>::Overflow)?;
		}
		AllMiner::<T>::put(allminer);
		Ok(())
	}

	/// Add space calculation power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn add_space(peerid: u64, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerDetails<T>>::contains_key(peerid) {
			return Ok(());
		}

		let acc = Self::get_acc(peerid)?;
		let state = Self::check_state(acc.clone());
		if state == "exit".as_bytes().to_vec() {
			return Ok(())
		}
		MinerItems::<T>::try_mutate(&acc, |s_opt| -> DispatchResult{
			let s = s_opt.as_mut().unwrap();
			s.space = s.space.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		MinerDetails::<T>::try_mutate(peerid, |s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.space = s.space.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		TotalSpace::<T>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_add(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		// MinerStatValue::<T>::try_mutate(|s_opt| -> DispatchResult {
		// 	let s = s_opt.as_mut().unwrap();
		// 	s.sum_files = s.sum_files.checked_add(1).ok_or(Error::<T>::Overflow)?;
		// 	Ok(())
		// })?;

		let mut allminer = AllMiner::<T>::get();
		let mut k = 0;
		for i in allminer.clone().iter() {
			if i.peerid == peerid {
 				let newminer = MinerInfo::<BoundedVec<u8, T::ItemLimit>> {
					peerid: i.peerid,
					ip: i.ip.clone(),
					power: i.power,
					space: i.space.checked_add(increment).ok_or(Error::<T>::Overflow)?,
				};
				allminer.remove(k);
				allminer.try_push(newminer).expect("Maximum length exceeded");
			}
			k = k.checked_add(1).ok_or(Error::<T>::Overflow)?;
		}
		AllMiner::<T>::put(allminer);
		Ok(())
	}
	/// Sub space calculation power to corresponding miners.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	/// - `increment`: computing power.
	pub fn sub_space(peerid: u64, increment: u128) -> DispatchResult {
		//check exist
		if !<MinerDetails<T>>::contains_key(peerid) {
			return Ok(());
		}

		let acc = Self::get_acc(peerid)?;
		let state = Self::check_state(acc.clone());
		if state == "exit".as_bytes().to_vec() {
			return Ok(())
		}
		MinerItems::<T>::try_mutate(&acc, |s_opt| -> DispatchResult{
			let s = s_opt.as_mut().unwrap();
			s.space = s.space.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		MinerDetails::<T>::mutate(peerid, |s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.space = s.space.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		TotalSpace::<T>::mutate(|s| -> DispatchResult {
			*s = s.checked_sub(increment).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		// MinerStatValue::<T>::mutate(|s_opt| -> DispatchResult {
		// 	let s = s_opt.as_mut().unwrap();
		// 	s.sum_files = s.sum_files.checked_sub(1).ok_or(Error::<T>::Overflow)?;
		// 	Ok(())
		// })?;

		let mut allminer = AllMiner::<T>::get();
		let mut k = 0;
		for i in allminer.clone().iter() {
			if i.peerid == peerid {
				let newminer = MinerInfo::<BoundedVec<u8, T::ItemLimit>> {
					peerid: i.peerid,
					ip: i.ip.clone(),
					power: i.power,
					space: i.space.checked_sub(increment).ok_or(Error::<T>::Overflow)?,
				};
				allminer.remove(k);
				allminer.try_push(newminer).expect("Maximum length exceeded");
			}
			k = k.checked_add(1).ok_or(Error::<T>::Overflow)?;
		}
		AllMiner::<T>::put(allminer);
		Ok(())
	}
	/// According to aid to punish.
	///
	/// Parameters:
	/// - `aid`: aid.
	pub fn punish(aid: AccountOf<T>, file_size: u128) -> DispatchResult {
		if !<MinerItems<T>>::contains_key(&aid) {
			return Ok(());
		}
		let mr = MinerItems::<T>::get(&aid).unwrap();
		let acc = T::PalletId::get().into_account();
		let growth: u128 =  file_size
		    .checked_div(1_048_576).ok_or(Error::<T>::Overflow)?
		    .checked_div(2).ok_or(Error::<T>::Overflow)?;
		let punish_amount: BalanceOf<T> = 400_000_000_000_000u128
			.checked_add(growth).ok_or(Error::<T>::Overflow)?
			.try_into().map_err(|_e| Error::<T>::ConversionError)?;

		if mr.collaterals < punish_amount {
			T::Currency::unreserve(&aid ,mr.collaterals);
			Self::delete_miner_info(aid.clone())?;
			return Ok(())	
		}
		T::Currency::unreserve(&aid, punish_amount);
		MinerItems::<T>::try_mutate(&aid, |s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.collaterals = s.collaterals.checked_sub(&punish_amount).ok_or(Error::<T>::Overflow)?;
			let limit = Self::check_collateral_limit(s.peerid)?;
			if mr.collaterals < limit && s.state != "frozen".as_bytes().to_vec() && s.state != "e_frozen".as_bytes().to_vec() {
				if s.state.to_vec() == "positive".as_bytes().to_vec() {
					s.state = Self::vec_to_bound::<u8>("frozen".as_bytes().to_vec())?;
				} else if s.state.to_vec() == "exit".as_bytes().to_vec() {
					s.state = Self::vec_to_bound::<u8>("e_frozen".as_bytes().to_vec())?;
				}	
			}
			Ok(())
		})?;
		MinerStatValue::<T>::try_mutate(|s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.staking = s.staking.checked_sub(&punish_amount).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		T::Currency::transfer(&aid, &acc, punish_amount, AllowDeath)?;

		

		Ok(())
	}

	fn modify_miner_statvalue(acc: AccountOf<T>) -> DispatchResult {
		let staking_val = <MinerItems<T>>::get(&acc).unwrap().collaterals;

		MinerStatValue::<T>::try_mutate(|s_opt| -> DispatchResult {
			let s = s_opt.as_mut().unwrap();
			s.total_miners = s.total_miners.checked_sub(1).ok_or(Error::<T>::Overflow)?;
			s.active_miners = s.active_miners.checked_sub(1).ok_or(Error::<T>::Overflow)?;
			s.staking = s.staking.checked_sub(&staking_val).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Ok(())
	}

	fn delete_miner_info(acc: AccountOf<T>) -> DispatchResult {
		let peerid = Self::get_peerid(&acc);
		let miner = <MinerDetails<T>>::get(peerid).unwrap();
		TotalPower::<T>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_sub(miner.power).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		TotalSpace::<T>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_sub(miner.space).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		AvailableSpace::<T>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_sub(miner.power).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;

		Self::modify_miner_statvalue(acc.clone())?;

		let mut miner_list = AllMiner::<T>::get();
		miner_list.retain(|s| if s.peerid == peerid {false} else {true});
		AllMiner::<T>::put(miner_list);

		<MinerItems<T>>::remove(&acc);
		<MinerDetails<T>>::remove(peerid);
		Ok(())
	}

	//Check whether the rewards for exited miners have been paid out. 
	//true is Distribution completed. false is Unfinished
	fn check_exist_miner_reward(acc: AccountOf<T>) -> bool {
		if !<MinerItems<T>>::contains_key(&acc) {
			let order_vec = <CalculateRewardOrderMap<T>>::get(&acc);
			if order_vec.len() == 0 {
				let reward_map = <RewardClaimMap<T>>::get(&acc).unwrap();
				if reward_map.have_to_receive == reward_map.total_reward {
					return true
				}
			}
		}
		
		false
	}

	fn clean_reward_map(acc: AccountOf<T>) {
		<CalculateRewardOrderMap<T>>::remove(&acc);
		<RewardClaimMap<T>>::remove(&acc);
	}
	/// Add reward orders for corresponding accounts.
	///
	/// Parameters:
	/// - `acc`: Rewards account.
	/// - `calculate_reward`: Calculate the reward.
	pub fn add_reward_order1(acc: AccountOf<T>, calculate_reward: u128) -> DispatchResult {
		let now = <frame_system::Pallet<T>>::block_number();
		// With block timing, 180 days =5184000 blocks
		// let deadline = now + T::BlockNumber::from(5184000u32);
		// // test 5 minutes
		// let deadline = now + T::BlockNumber::from(18000u32);
		// test 6 hours
		// test 1 hours
		let deadline = now.checked_add(&T::BlockNumber::from(216000u32)).ok_or(Error::<T>::Overflow)?;

		if !<CalculateRewardOrderMap<T>>::contains_key(&acc) {
			let order: CalculateRewardOrder<T> = CalculateRewardOrder::<T>{
				calculate_reward:calculate_reward,
				start_t: now,
				deadline: deadline,
			};
			let mut order_vec:Vec<CalculateRewardOrder<T>> = Vec::new();
			order_vec.push(order);
			let bounded_order_vec = Self::vec_to_bound::<CalculateRewardOrder<T>>(order_vec)?;
			<CalculateRewardOrderMap<T>>::insert(
				acc,
				bounded_order_vec,
			);
		} else {
			let order1: CalculateRewardOrder<T> = CalculateRewardOrder::<T>{
				calculate_reward:calculate_reward,
				start_t: now,
				deadline: deadline,
			};
			let mut order_vec = CalculateRewardOrderMap::<T>::get(&acc);
			order_vec.try_push(order1).expect("Maximum length exceeded");
			<CalculateRewardOrderMap<T>>::insert(
				acc,
				order_vec,
			);
		}
		Ok(())
	}
	/// Get an account based on peerID.
	///
	/// Parameters:
	/// - `peerid`: peerid.
	pub fn get_acc(peerid: u64) -> Result<AccountOf<T>, DispatchError> {
		if !<MinerDetails<T>>::contains_key(peerid) {
			Err(Error::<T>::NotExisted)?;
		}
		let acc = MinerDetails::<T>::get(peerid).unwrap();
		Ok(acc.address)
	}

	//Get the available space on the current chain.
	pub fn get_space() -> Result<u128, DispatchError> {
		let purchased_space = <PurchasedSpace<T>>::get();
		let total_space = <AvailableSpace<T>>::get();
		//If the total space on the current chain is less than the purchased space, 0 will be returned.
		if total_space < purchased_space {
			return Ok(0);
		}
		//Calculate available space.
		let value = total_space.checked_sub(purchased_space).ok_or(Error::<T>::Overflow)?;
		return Ok(value)
	}

	pub fn add_purchased_space(size: u128) -> DispatchResult{
		<PurchasedSpace<T>>::try_mutate(|s| -> DispatchResult {
			let available_space = <AvailableSpace<T>>::get();
			if *s + size > available_space {
				Err(<Error<T>>::InsufficientAvailableSpace)?;
			}
			*s = s.checked_add(size).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		Ok(())
	}
	pub fn sub_purchased_space(size: u128) -> DispatchResult {
		<PurchasedSpace<T>>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_sub(size).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		Ok(())
	}
	pub fn add_available_space(size: u128) -> DispatchResult {
		<AvailableSpace<T>>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_add(size).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		Ok(())
	}
	pub fn sub_available_space(size: u128) -> DispatchResult {
		<AvailableSpace<T>>::try_mutate(|s| -> DispatchResult {
			*s = s.checked_sub(size).ok_or(Error::<T>::Overflow)?;
			Ok(())
		})?;
		Ok(())
	}

	fn check_collateral_limit(peer_id: u64) -> Result<BalanceOf<T>, Error<T>> {
		let power = <MinerDetails<T>>::get(peer_id).unwrap().power;
		let mut current_power_num:u128 = 1;
		current_power_num += power.checked_div(1024 * 1024 * M_BYTE).ok_or(Error::<T>::Overflow)?;
		//2000TCESS/TB(space)
		let limit: BalanceOf<T> = (current_power_num.checked_mul(2_000_000_000_000_000u128).ok_or(Error::<T>::Overflow)?)
		.try_into()
		.map_err(|_e| Error::<T>::ConversionError)?;

		Ok(limit)
	}

	fn check_state(acc: AccountOf<T>) -> Vec<u8> {
		<MinerItems<T>>::get(&acc).unwrap().state.to_vec()
	}

	fn vec_to_bound<P>(param: Vec<P>) -> Result<BoundedVec<P, T::ItemLimit>, DispatchError> {
		let result: BoundedVec<P, T::ItemLimit> = param.try_into().expect("too long");
		Ok(result)
	}

	// fn veclist_to_bounde(param: Vec<Vec<u8>>) -> Result<StringList<T>, DispatchError> {
	// 	let mut result: StringList<T> = Vec::new().try_into().expect("...");

	// 	for v in param {
	// 		let string: BoundedVec<u8, T::StringLimit> = v.try_into().expect("keywords too long");
	// 		result.try_push(string).expect("keywords too long");
	// 	}

	// 	Ok(result)
	// }
}

impl<T: Config> OnUnbalanced<NegativeImbalanceOf<T>> for Pallet<T> {
	fn on_nonzero_unbalanced(amount: NegativeImbalanceOf<T>) {
		let numeric_amount = amount.peek();

		// Must resolve into existing but better to be safe.
		let _ = T::Currency::resolve_creating(&T::PalletId::get().into_account(), amount);

		Self::deposit_event(Event::Deposit{balance: numeric_amount});
	}
}

pub trait MinerControl {
	fn add_power(peer_id: u64, power: u128) -> DispatchResult;
	fn sub_power(peer_id: u64, power: u128) -> DispatchResult;
	fn add_space(peer_id: u64, power: u128) -> DispatchResult;
	fn sub_space(peer_id: u64, power: u128) -> DispatchResult;
	fn get_power_and_space(peer_id: u64) -> Result<(u128, u128), DispatchError>;
	fn punish_miner(peer_id: u64, file_size: u64) -> DispatchResult;
	fn miner_is_exist(peer_id: u64) -> bool;
}

impl<T: Config> MinerControl for Pallet<T> {
	fn add_power(peer_id: u64, power: u128) -> DispatchResult {
		Pallet::<T>::add_power(peer_id, power)?;
		Ok(())
	}

	fn sub_power(peer_id: u64, power: u128) -> DispatchResult {
		Pallet::<T>::sub_power(peer_id, power)?;
		Ok(())
	}

	fn add_space(peer_id: u64, power: u128) -> DispatchResult {
		Pallet::<T>::add_space(peer_id, power)?;
		Ok(())
	}

	fn sub_space(peer_id: u64, power: u128) -> DispatchResult {
		Pallet::<T>::sub_space(peer_id, power)?;
		Ok(())
	}

	fn punish_miner(peer_id: u64, file_size: u64) -> DispatchResult {
		let acc = Pallet::<T>::get_acc(peer_id)?;
		Pallet::<T>::punish(acc, file_size.into())?;
		Ok(())
	}

	fn get_power_and_space(peer_id: u64) -> Result<(u128, u128), DispatchError> {
		if !<MinerDetails<T>>::contains_key(peer_id) {
			Err(Error::<T>::NotMiner)?;
		}
		let miner = <MinerDetails<T>>::get(peer_id).unwrap();
		Ok((miner.power, miner.space))
	}

	fn miner_is_exist(peer_id: u64) -> bool {
		if <MinerDetails<T>>::contains_key(peer_id) {
			return true;
		}
		false
	}
}