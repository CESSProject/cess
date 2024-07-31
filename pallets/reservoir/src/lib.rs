#![cfg_attr(not(feature = "std"), no_std)]

mod types;
use types::*;

mod impls;

pub use pallet::*;

use codec::{Decode, Encode};
use sp_runtime::traits::{
    Zero, CheckedAdd, CheckedSub, AccountIdConversion,

};
use frame_system::{
    pallet_prelude::{OriginFor, *},
};
use frame_support::{
    PalletId,
    pallet_prelude::*,
    traits::{
        Get, Currency, ReservableCurrency,
        ExistenceRequirement::KeepAlive,
    },
};

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_system::ensure_signed;

    #[pallet::config]
	pub trait Config: frame_system::Config + sp_std::fmt::Debug {
        /// The overarching event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;
        /// The pallet id
        #[pallet::constant]
		type PalletId: Get<PalletId>;
        /// Maximum length of event id
        #[pallet::constant]
        type IdLength: Get<u32>;
        /// ongoing event cap
        #[pallet::constant]
        type EventLimit: Get<u32>;
    }

    #[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
        /// Successful events that inject working capital into the reservoir.
        Filling { amount: BalanceOf<T> },
        /// Activity event when the user successfully deposits tokens into the reservoir.
        Store { amount: BalanceOf<T> },
        /// A successful event when a user redeems stored tokens from the reservoir
        Withdraw { amount: BalanceOf<T> },
        /// An event in which Root users get back the active funds in the reservoir.
        RootWithdraw { amount: BalanceOf<T> },
        /// Host a event succuess event.
        CreateEvent { id: BoundedVec<u8, T::IdLength> },
        /// Attending an event success event.
        AttendEvent { id: BoundedVec<u8, T::IdLength> },
        /// Information reported when polling and processing expiration events.
        EventExpired { id: BoundedVec<u8, T::IdLength> },     
    }

    #[pallet::error]
	pub enum Error<T> {
        /// Data overflow.
        Overflow,
        /// The account does not exist in the loan list.
        NonExistent,
        /// Insufficient borrowed or stored balance in reservoir.
        Insufficient,
        /// Some operations that should not have errors.
		BugInvalid,
        /// The length of the event id exceeds the limit.
		LengthExceedsLimit,
        /// Vec to BoundedVec Error.
		BoundedVecError,
        /// Event id does not exist.
        IdNonExistent,
        /// The number of people in the event has reached the upper limit.
        UpperLimit,
        /// The current user already has a borrowing bill.
        Borrowed,
    }

    #[pallet::storage]
	#[pallet::getter(fn reservoir)]
    pub(super) type Reservoir<T: Config> = StorageValue<_, ReservoirInfo<BalanceOf<T>>, ValueQuery>;

    #[pallet::storage]
	#[pallet::getter(fn borrow_list)]
    pub(super) type BorrowList<T: Config> = StorageMap<_, Twox64Concat, AccountOf<T>, BorrowInfo<T>>;

    #[pallet::storage]
    #[pallet::getter(fn user_passbook)]
    pub(super) type UserPassbook<T: Config> = StorageMap<_, Twox64Concat, AccountOf<T>, UserHold<BalanceOf<T>>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn events)]
    pub(super) type Events<T: Config> = StorageMap<_, Twox64Concat, BoundedVec<u8, T::IdLength>, EventInfo<T>>;

    #[pallet::storage]
    #[pallet::getter(fn event_expired_records)]
    pub(super) type EventExpiredRecords<T: Config> = StorageMap<_, Twox64Concat, BlockNumberFor<T>, BoundedVec<BoundedVec<u8, T::IdLength>, T::EventLimit>, ValueQuery>;
    
    #[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
            let mut weight = Weight::zero();

            let list = <EventExpiredRecords<T>>::get(&now);
            weight = weight.saturating_add(T::DbWeight::get().reads(1));
            for id in list.iter() {
                Events::<T>::remove(id);
                weight = weight.saturating_add(T::DbWeight::get().writes(1));
                Self::deposit_event( Event::<T>::EventExpired { id: id.clone() });
            }

            weight
        }
    }

    #[pallet::call]
	impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
		#[pallet::weight(Weight::zero())]
        pub fn filling(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let reservoir = T::PalletId::get().into_account_truncating();
            T::Currency::transfer(&sender, &reservoir, amount, KeepAlive)?;

            Reservoir::<T>::try_mutate(|reservoir| -> DispatchResult {
                reservoir.free_balance = reservoir.free_balance.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

                Ok(())
            })?;

            Self::deposit_event(Event::<T>::Filling { amount: amount });

            return Ok(())
        }

        #[pallet::call_index(1)]
		#[pallet::weight(Weight::zero())]
        pub fn store(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let reservoir = T::PalletId::get().into_account_truncating();
            T::Currency::transfer(&sender, &reservoir, amount, KeepAlive)?;

            <UserPassbook<T>>::try_mutate(&sender, |user_hold| -> DispatchResult {
                user_hold.free =  user_hold.free.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

                Ok(())
            })?;

            Reservoir::<T>::try_mutate(|reservoir| -> DispatchResult {
                reservoir.store_balance = reservoir.store_balance.checked_add(&amount).ok_or(Error::<T>::Overflow)?;

                Ok(())
            })?;

            Self::deposit_event(Event::<T>::Store { amount: amount });

            return Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(Weight::zero())]
        pub fn withdraw(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            <UserPassbook<T>>::try_mutate(&sender, |user_hold| -> DispatchResult {
                user_hold.free =  user_hold.free.checked_sub(&amount).ok_or(Error::<T>::Overflow)?;

                Ok(())
            })?;

            Reservoir::<T>::try_mutate(|reservoir| -> DispatchResult {
                reservoir.store_balance = reservoir.store_balance.checked_sub(&amount).ok_or(Error::<T>::Overflow)?;

                Ok(())
            })?;

            let reservoir = T::PalletId::get().into_account_truncating();
            T::Currency::transfer(&reservoir, &sender, amount, KeepAlive)?;

            Self::deposit_event(Event::<T>::Withdraw { amount: amount });

            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(Weight::zero())]
        pub fn event_withdraw(origin: OriginFor<T>, amount: BalanceOf<T>, target: AccountOf<T>) -> DispatchResult {
            ensure_root(origin)?;

            let reservoir = T::PalletId::get().into_account_truncating();
            T::Currency::transfer(&reservoir, &target, amount, KeepAlive)?;

            Reservoir::<T>::try_mutate(|reservoir| -> DispatchResult {
                reservoir.free_balance = reservoir.free_balance.checked_sub(&amount).ok_or(Error::<T>::Overflow)?;

                Ok(())
            })?;

            Self::deposit_event(Event::<T>::RootWithdraw { amount: amount });

            Ok(())
        }
        // When calling this method to create an activity, 
        // please ensure that the free_balance of the reservoir is sufficient.
        #[pallet::call_index(4)]
        #[pallet::weight(Weight::zero())]
        pub fn create_event(
            origin: OriginFor<T>, 
            id: BoundedVec<u8, T::IdLength>,
            quota: u32, 
            deadline: BlockNumberFor<T>, 
            unit_amount: BalanceOf<T>, 
            borrow_period: BlockNumberFor<T>, 
            use_type: UseType,
        ) -> DispatchResult {
            ensure_root(origin)?;

            ensure!(!<Events<T>>::contains_key(&id), Error::<T>::LengthExceedsLimit);

            let event_info = EventInfo::<T>{
                quota,
                deadline,
                unit_amount,
                borrow_period,
                use_type,
            };

            <Events<T>>::insert(&id, event_info);

            let now = <frame_system::Pallet<T>>::block_number();
            let expired = now.checked_add(&deadline).ok_or(Error::<T>::Overflow)?;
            EventExpiredRecords::<T>::try_mutate(&expired, |list| -> DispatchResult {
                list.try_push(id.clone()).map_err(|_| Error::<T>::BoundedVecError)?;

                Ok(())
            })?;

            Self::deposit_event( Event::<T>::CreateEvent { id: id });

            Ok(())
        }

        #[pallet::call_index(5)]
        #[pallet::weight(Weight::zero())]
        pub fn attend_event(origin: OriginFor<T>, id: BoundedVec<u8, T::IdLength>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            
            ensure!(Events::<T>::contains_key(&id), Error::<T>::IdNonExistent);
            if let Ok(borrow_info) = <BorrowList<T>>::try_get(&sender) {
                let now = <frame_system::Pallet<T>>::block_number();
                if now >= borrow_info.deadline && borrow_info.staking == BalanceOf::<T>::zero() {
                    Reservoir::<T>::try_mutate(|reservoir_info| -> DispatchResult {
                        reservoir_info.borrow_balance = reservoir_info.borrow_balance.checked_sub(&borrow_info.free).ok_or(Error::<T>::Overflow)?;
                        reservoir_info.free_balance = reservoir_info.free_balance.checked_add(&borrow_info.free).ok_or(Error::<T>::Overflow)?;

                        Ok(())
                    })?;
                    <BorrowList<T>>::remove(&sender);
                } else {
                    return Err(Error::<T>::Borrowed)?;
                }
            }

            <Events<T>>::try_mutate(&id, |event_opt| -> DispatchResult {
                let event = event_opt.as_mut().ok_or(Error::<T>::IdNonExistent)?;

                if event.quota == 0 {
                    return Err(Error::<T>::UpperLimit)?;
                }

                event.quota = event.quota.checked_sub(1).ok_or(Error::<T>::Overflow)?;

                let reservoir = T::PalletId::get().into_account_truncating();
                let now = <frame_system::Pallet<T>>::block_number();
                let deadline = now.checked_add(&event.borrow_period).ok_or(Error::<T>::Overflow)?;

                let borrow_info = BorrowInfo::<T>{
                    free: event.unit_amount,
                    lender: reservoir,
                    staking: BalanceOf::<T>::zero(),
                    deadline: deadline,
                };
                BorrowList::<T>::insert(&sender, borrow_info);

                Reservoir::<T>::try_mutate(|reservoir_info| -> DispatchResult {
                    reservoir_info.borrow_balance = reservoir_info.borrow_balance.checked_add(&event.unit_amount).ok_or(Error::<T>::Overflow)?;
                    reservoir_info.free_balance = reservoir_info.free_balance.checked_sub(&event.unit_amount).ok_or(Error::<T>::Overflow)?;

                    Ok(())
                })?;

                Ok(())
            })?;

            Self::deposit_event( Event::<T>::AttendEvent { id: id });

            Ok(())
        }
    }
}

pub trait ReservoirGate<AccountId, Balance> {
    fn check_qualification(acc: &AccountId, amount: Balance) -> DispatchResult;
    fn staking(acc: &AccountId, amount: Balance, flag: bool) -> DispatchResult;
    fn redeem(acc: &AccountId, amount: Balance, flag: bool) -> DispatchResult;
    fn punish(acc: &AccountId, amount: Balance, flag:bool) -> DispatchResult;
    fn get_reservoir_acc() -> AccountId;
}

impl<T: Config> ReservoirGate<AccountOf<T>, BalanceOf<T>> for Pallet<T> {
    fn check_qualification(acc: &AccountOf<T>, amount: BalanceOf<T>) -> DispatchResult {
        let mut total_free: BalanceOf<T> = BalanceOf::<T>::zero();
        if let Ok(borrow_info) = <BorrowList<T>>::try_get(acc) {
            total_free = total_free.checked_add(&borrow_info.free).ok_or(Error::<T>::Overflow)?;
        }

        let user_passbook = <UserPassbook<T>>::get(acc);
        total_free = total_free.checked_add(&user_passbook.free).ok_or(Error::<T>::Overflow)?;

        ensure!(total_free >= amount, Error::<T>::Insufficient);

        Ok(())
    }

    fn staking(acc: &AccountOf<T>, amount: BalanceOf<T>, flag: bool) -> DispatchResult {
        let mut need_staking = amount;
        if let Ok(mut borrow_info) = <BorrowList<T>>::try_get(acc) {
            let now = <frame_system::Pallet<T>>::block_number();
            if now < borrow_info.deadline {
                if borrow_info.free >= need_staking {
                    borrow_info.free = borrow_info.free.checked_sub(&need_staking).ok_or(Error::<T>::Overflow)?;
                    borrow_info.staking = borrow_info.staking.checked_add(&need_staking).ok_or(Error::<T>::Overflow)?;
                    need_staking = BalanceOf::<T>::zero();
                } else {
                    need_staking = need_staking.checked_sub(&borrow_info.free).ok_or(Error::<T>::Overflow)?;
                    borrow_info.staking = borrow_info.staking.checked_add(&borrow_info.free).ok_or(Error::<T>::Overflow)?;
                    borrow_info.free = BalanceOf::<T>::zero();
                }
    
                <BorrowList<T>>::insert(acc, borrow_info);
            } else {
                BorrowList::<T>::remove(acc);
            }
        }

        if need_staking != 0u32.into() {
            <UserPassbook<T>>::try_mutate(acc, |user_hold| -> DispatchResult {
                user_hold.free =  user_hold.free.checked_sub(&need_staking).ok_or(Error::<T>::Overflow)?;
                user_hold.staking = user_hold.staking.checked_add(&need_staking).ok_or(Error::<T>::Overflow)?;

                Ok(())
            })?;
        }

        let reservoir = T::PalletId::get().into_account_truncating();

        if flag {
            T::Currency::reserve(&reservoir, amount)?;
        }
       

        Ok(())
    }

    fn redeem(acc: &AccountOf<T>, amount: BalanceOf<T>, flag: bool) -> DispatchResult {
        let mut need_amount = amount;
        <UserPassbook<T>>::try_mutate(acc, |user_hold| -> DispatchResult {
            if user_hold.staking >= need_amount {
                user_hold.staking = user_hold.staking.checked_sub(&need_amount).ok_or(Error::<T>::Overflow)?;
                user_hold.free =  user_hold.free.checked_add(&need_amount).ok_or(Error::<T>::Overflow)?;
                need_amount = BalanceOf::<T>::zero();
            } else {
                need_amount = need_amount.checked_sub(&user_hold.staking).ok_or(Error::<T>::Overflow)?;
                user_hold.free =  user_hold.free.checked_add(&user_hold.staking).ok_or(Error::<T>::Overflow)?;
                user_hold.staking = BalanceOf::<T>::zero();
            }

            Ok(())
        })?;

        if need_amount != 0u32.into() {
            if let Ok(mut borrow_info) = <BorrowList<T>>::try_get(acc) {
                if borrow_info.staking >= need_amount {
                    borrow_info.staking = borrow_info.staking.checked_sub(&need_amount).ok_or(Error::<T>::Overflow)?;
                    borrow_info.free = borrow_info.free.checked_add(&need_amount).ok_or(Error::<T>::Overflow)?;
                    <BorrowList<T>>::insert(acc, borrow_info);
                } else {
                    return Err(Error::<T>::BugInvalid)?;
                }
            } else {
                return Err(Error::<T>::BugInvalid)?;
            }
        }

        let reservoir = T::PalletId::get().into_account_truncating();

        if flag {
            T::Currency::unreserve(&reservoir, amount);
        }


        Ok(())
    }

    fn punish(acc: &AccountOf<T>, amount: BalanceOf<T>, flag:bool) -> DispatchResult {
        let mut need_amount = amount;
        <UserPassbook<T>>::try_mutate(acc, |user_hold| -> DispatchResult {
            if user_hold.staking >= need_amount {
                user_hold.staking = user_hold.staking.checked_sub(&need_amount).ok_or(Error::<T>::Overflow)?;
                need_amount = BalanceOf::<T>::zero();
            } else {
                need_amount = need_amount.checked_sub(&user_hold.staking).ok_or(Error::<T>::Overflow)?;
                user_hold.staking = BalanceOf::<T>::zero();
            }

            Ok(())
        })?;

        if need_amount != 0u32.into() {
            if let Ok(mut borrow_info) = <BorrowList<T>>::try_get(acc) {
                if borrow_info.staking >= need_amount {
                    borrow_info.staking = borrow_info.staking.checked_sub(&need_amount).ok_or(Error::<T>::Overflow)?;
                    <BorrowList<T>>::insert(acc, borrow_info);
                } else {
                    return Err(Error::<T>::BugInvalid)?;
                }
            } else {
                return Err(Error::<T>::BugInvalid)?;
            }
        }

        let reservoir = T::PalletId::get().into_account_truncating();

        if flag {
            T::Currency::unreserve(&reservoir, amount);
        }

        Ok(())
    }

    fn get_reservoir_acc() -> AccountOf<T> {
        T::PalletId::get().into_account_truncating()
    }
}