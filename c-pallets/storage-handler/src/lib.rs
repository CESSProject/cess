#![cfg_attr(not(feature = "std"), no_std)]

use frame_system::{
    ensure_root, ensure_signed,
    pallet_prelude::*,
};
use frame_support::{
    Blake2_128Concat, PalletId, weights::Weight, ensure, transactional,
    storage::bounded_vec::BoundedVec,
    traits::{
        StorageVersion, Currency, ReservableCurrency, Randomness,
    },
    pallet_prelude::*,
};
use sp_runtime::{
	traits::{
        CheckedAdd, CheckedMul, CheckedDiv, CheckedSub,
		SaturatedConversion,
	},
	RuntimeDebug,
};
use sp_std::{convert::TryInto, prelude::*, str};
/// for types 
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use cp_cess_common::*;
use pallet_cess_treasury::{TreasuryHandle};
use sp_core::H256;

pub mod weights;
use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

mod types;
use types::*;

pub use pallet::*;

pub const SPACE_NORMAL: &str = "normal";
pub const SPACE_FROZEN: &str = "frozen";
pub const SPACE_DEAD: &str = "dead";

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);


#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
	pub trait Config: frame_system::Config + sp_std::fmt::Debug {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

        type WeightInfo: WeightInfo;

        #[pallet::constant]
		type OneDay: Get<BlockNumberOf<Self>>;

        #[pallet::constant]
		type OneHours: Get<BlockNumberOf<Self>>;
		/// pallet address.
		#[pallet::constant]
		type RewardPalletId: Get<PalletId>;

        #[pallet::constant]
        type StateStringMax: Get<u32> + Clone + Eq + PartialEq;
        
		#[pallet::constant]
		type FrozenDays: Get<BlockNumberOf<Self>> + Clone + Eq + PartialEq;

        type CessTreasuryHandle: TreasuryHandle<AccountOf<Self>, BalanceOf<Self>>;

        type MyRandomness: Randomness<Option<Self::Hash>, Self::BlockNumber>;
    }

    #[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
		//User buy package event
		BuySpace { acc: AccountOf<T>, storage_capacity: u128, spend: BalanceOf<T> },
		//Expansion Space
		ExpansionSpace { acc: AccountOf<T>, expansion_space: u128, fee: BalanceOf<T> },
		//Package upgrade
		RenewalSpace { acc: AccountOf<T>, renewal_days: u32, fee: BalanceOf<T> },
        //Expired storage space
		LeaseExpired { acc: AccountOf<T>, size: u128 },
		//Storage space expiring within 24 hours
		LeaseExpireIn24Hours { acc: AccountOf<T>, size: u128 },

        CreatePayOrder { order_hash: BoundedVec<u8, ConstU32<32>> },

        PaidOrder { order_hash: BoundedVec<u8, ConstU32<32>> },
    }

    #[pallet::error]
	pub enum Error<T> {
        BugInvalid,

        BoundedVecError,
        
        InsufficientAvailableSpace,
        // Balance not enough
        InsufficientBalance,

        InsufficientStorage,

        Overflow,

        WrongOperation,

        PurchasedSpace,

        NotPurchasedSpace,
        // storage space frozen
        LeaseFreeze,

        LeaseExpired,

        OrderExpired,

        RandomErr,

        NoOrder,

        ParamError,
    }

	#[pallet::storage]
	#[pallet::getter(fn user_owned_space)]
	pub(super) type UserOwnedSpace<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountOf<T>, OwnedSpaceDetails<T>>;
 
	#[pallet::storage]
	#[pallet::getter(fn unit_price)]
    pub(super) type UnitPrice<T: Config> = StorageValue<_, BalanceOf<T>>;

    /// The total power of all storage miners.
	#[pallet::storage]
	#[pallet::getter(fn total_power)]
	pub(super) type TotalIdleSpace<T: Config> = StorageValue<_, u128, ValueQuery>;

	/// The total storage space to fill of all storage miners.
	#[pallet::storage]
	#[pallet::getter(fn total_space)]
	pub(super) type TotalServiceSpace<T: Config> = StorageValue<_, u128, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn purchased_space)]
	pub(super) type PurchasedSpace<T: Config> = StorageValue<_, u128, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn pay_order)]
    pub(super) type PayOrder<T: Config> = StorageMap<_, Blake2_128Concat, BoundedVec<u8, ConstU32<32>>, OrderInfo<T>>;

    #[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(PhantomData<T>);

    #[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		// price / gib / 30days
		pub price: BalanceOf<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				price: 30u32.saturated_into(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			UnitPrice::<T>::put(self.price);
		}
	}

    #[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Buy Space
        ///
        /// This function allows a user to purchase storage space in the network by specifying the desired capacity in gibibytes (GiB). 
        /// The user's account is debited with the corresponding payment for the purchased space.
        ///
        /// Parameters:
        /// - `origin`: The origin from which the function is called, representing the user's account.
        /// - `gib_count`: The amount of storage space to purchase, specified in gibibytes (GiB).
		#[pallet::call_index(0)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::buy_space())]
		pub fn buy_space(origin: OriginFor<T>, gib_count: u32) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!<UserOwnedSpace<T>>::contains_key(&sender), Error::<T>::PurchasedSpace);

			let space = G_BYTE.checked_mul(gib_count as u128).ok_or(Error::<T>::Overflow)?;
			let unit_price = <UnitPrice<T>>::try_get()
				.map_err(|_e| Error::<T>::BugInvalid)?;

			Self::add_user_purchased_space(sender.clone(), space, 30)?;
			Self::add_purchased_space(space)?;
			let price: BalanceOf<T> = unit_price
				.checked_mul(&gib_count.saturated_into())
				.ok_or(Error::<T>::Overflow)?;
            
			ensure!(
				<T as pallet::Config>::Currency::can_slash(&sender, price.clone()),
				Error::<T>::InsufficientBalance
			);

            T::CessTreasuryHandle::send_to_sid(sender.clone(), price)?;

			Self::deposit_event(Event::<T>::BuySpace { acc: sender, storage_capacity: space, spend: price });
			Ok(())
		}
		
        /// Expansion of Purchased Space
        ///
        /// This function allows a user who has previously purchased storage space to expand their purchased space by adding more storage capacity. 
        /// The user specifies the desired capacity in gibibytes (GiB) for expansion.
        ///
        /// Parameters:
        /// - `origin`: The origin from which the function is called, representing the user's account.
        /// - `gib_count`: The amount of additional storage space to purchase for expansion, specified in gibibytes (GiB).
        #[pallet::call_index(1)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::expansion_space())]
		pub fn expansion_space(origin: OriginFor<T>, gib_count: u32) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let cur_owned_space = <UserOwnedSpace<T>>::try_get(&sender)
				.map_err(|_e| Error::<T>::NotPurchasedSpace)?;
			let now = <frame_system::Pallet<T>>::block_number();
			ensure!(now < cur_owned_space.deadline, Error::<T>::LeaseExpired);
			ensure!(
				cur_owned_space.state.to_vec() != SPACE_FROZEN.as_bytes().to_vec(),
				Error::<T>::LeaseFreeze
			);
			// The unit price recorded in UnitPrice is the unit price of one month.
			// Here, the daily unit price is calculated.
			let day_unit_price = <UnitPrice<T>>::try_get()
				.map_err(|_e| Error::<T>::BugInvalid)?
				.checked_div(&30u32.saturated_into()).ok_or(Error::<T>::Overflow)?;
			let space = G_BYTE.checked_mul(gib_count as u128).ok_or(Error::<T>::Overflow)?;
			//Calculate remaining days.
			let block_oneday: BlockNumberOf<T> = <T as pallet::Config>::OneDay::get();
			let diff_block = cur_owned_space.deadline.checked_sub(&now).ok_or(Error::<T>::Overflow)?;
			let mut remain_day: u32 = diff_block
				.checked_div(&block_oneday)
				.ok_or(Error::<T>::Overflow)?
				.saturated_into();
			if diff_block % block_oneday != 0u32.saturated_into() {
				remain_day = remain_day
					.checked_add(1)
					.ok_or(Error::<T>::Overflow)?
					.saturated_into();
			}
			//Calculate the final price difference to be made up.
			let price: BalanceOf<T> = day_unit_price
				.checked_mul(&gib_count.saturated_into())
				.ok_or(Error::<T>::Overflow)?
				.checked_mul(&remain_day.saturated_into())
				.ok_or(Error::<T>::Overflow)?
				.try_into()
				.map_err(|_e| Error::<T>::Overflow)?;
			//Judge whether the balance is sufficient
			ensure!(
				<T as pallet::Config>::Currency::can_slash(&sender, price.clone()),
				Error::<T>::InsufficientBalance
			);

			Self::add_purchased_space(
				space,
			)?;

			Self::expension_puchased_package(sender.clone(), space)?;

            T::CessTreasuryHandle::send_to_sid(sender.clone(), price.clone())?;

			Self::deposit_event(Event::<T>::ExpansionSpace {
				acc: sender,
				expansion_space: space,
				fee: price,
			});
			Ok(())
		}

        /// Renewal of Purchased Space Lease
        ///
        /// This function allows a user who has purchased storage space to renew their lease for additional days by paying a renewal fee. The user specifies the number of days they wish to extend the lease.
        ///
        /// Parameters:
        /// - `origin`: The origin from which the function is called, representing the user's account.
        /// - `days`: The number of days for which the user wishes to renew the space lease.
		#[pallet::call_index(2)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::renewal_space())]
		pub fn renewal_space(origin: OriginFor<T>, days: u32) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let cur_owned_space = <UserOwnedSpace<T>>::try_get(&sender)
				.map_err(|_e| Error::<T>::NotPurchasedSpace)?;

            ensure!(
                cur_owned_space.state.to_vec() != SPACE_DEAD.as_bytes().to_vec(), 
                Error::<T>::LeaseExpired,
            );

			let days_unit_price = <UnitPrice<T>>::try_get()
				.map_err(|_e| Error::<T>::BugInvalid)?
				.checked_div(&30u32.saturated_into())
				.ok_or(Error::<T>::Overflow)?;
			let gib_count = cur_owned_space.total_space.checked_div(G_BYTE).ok_or(Error::<T>::Overflow)?;
			let price: BalanceOf<T> = days_unit_price
				.checked_mul(&gib_count.saturated_into())
				.ok_or(Error::<T>::Overflow)?
				.checked_mul(&days.saturated_into())
				.ok_or(Error::<T>::Overflow)?
				.try_into()
				.map_err(|_e| Error::<T>::Overflow)?;
            
			ensure!(
				<T as pallet::Config>::Currency::can_slash(&sender, price.clone()),
				Error::<T>::InsufficientBalance
			);

			T::CessTreasuryHandle::send_to_sid(sender.clone(), price.clone())?;

			Self::update_puchased_package(sender.clone(), days)?;
			Self::deposit_event(Event::<T>::RenewalSpace {
				acc: sender,
				renewal_days: days,
				fee: price,
			});
			Ok(())
		}
        // FOR TEST
		#[pallet::call_index(4)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
		pub fn update_price(origin: OriginFor<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;
			let default_price: BalanceOf<T> = 30_000_000_000_000u128.try_into().map_err(|_| Error::<T>::Overflow)?;
			UnitPrice::<T>::put(default_price);

			Ok(())
		}
        // FOR TEST
        #[pallet::call_index(5)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
        pub fn update_user_life(origin: OriginFor<T>, user: AccountOf<T>, deadline: BlockNumberOf<T>) -> DispatchResult {
            let _ = ensure_root(origin)?;

            <UserOwnedSpace<T>>::try_mutate(&user, |space_opt| -> DispatchResult {
                let space_info = space_opt.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;

                space_info.deadline = deadline;

                Ok(())
            })
        }

        #[pallet::call_index(6)]
        #[transactional]
        #[pallet::weight(Weight::zero())]
        pub fn create_order(
            origin: OriginFor<T>, 
            target_acc: AccountOf<T>, 
            order_type: OrderType, 
            gib_count: u32, 
            days: u32,
            // minute
            expired: u32,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;

            let expired: BlockNumberOf<T> = (expired
                .checked_mul(6).ok_or(Error::<T>::Overflow)?).saturated_into();
            ensure!(expired < T::OneHours::get(), Error::<T>::ParamError);

            let price = match order_type {
                OrderType::Buy => {
                    ensure!(!<UserOwnedSpace<T>>::contains_key(&target_acc), Error::<T>::PurchasedSpace);
                    let price = Self::calculate_price(gib_count, days)?;
                    price
                },
                OrderType::Expansion => {
                    let user_owned_space = <UserOwnedSpace<T>>::try_get(&target_acc).map_err(|_| Error::<T>::NotPurchasedSpace)?;
                    let remain_day = Self::calculate_remain_day(user_owned_space.deadline)?;
                    let price = Self::calculate_price(gib_count, remain_day.saturated_into())?;
                    price
                },
                OrderType::Renewal => {
                    let user_owned_space = <UserOwnedSpace<T>>::try_get(&target_acc).map_err(|_| Error::<T>::NotPurchasedSpace)?;
                    let gib_count = user_owned_space.total_space.checked_div(G_BYTE).ok_or(Error::<T>::Overflow)?;
                    let price = Self::calculate_price(gib_count as u32, days)?;
                    price
                },
            };

            let now = <frame_system::Pallet<T>>::block_number();
            let expired = now.checked_add(&expired.saturated_into()).ok_or(Error::<T>::Overflow)?;
            let pay_order = OrderInfo::<T> {
                pay: price,
                gib_count: gib_count,
                days,
                expired,
                target_acc: target_acc,
                order_type,
            };

            let (seed, _) =
					T::MyRandomness::random(&(T::RewardPalletId::get(), now).encode());
			let seed = match seed {
				Some(v) => v,
				None => Default::default(),
			};
			let random_hash =
				<H256>::decode(&mut seed.as_ref()).map_err(|_| Error::<T>::RandomErr)?;

            let random_hash: BoundedVec<u8, sp_core::ConstU32<32>> = random_hash.as_bytes().to_vec().try_into().map_err(|_| Error::<T>::BoundedVecError)?;
            <PayOrder<T>>::insert(&random_hash, pay_order);

            Self::deposit_event(Event::<T>::CreatePayOrder { order_hash: random_hash });

            Ok(())
        }

        #[pallet::call_index(7)]
        #[pallet::weight(Weight::zero())]
        pub fn exec_order(origin: OriginFor<T>, order_id: BoundedVec<u8, ConstU32<32>>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let order = <PayOrder<T>>::try_get(&order_id).map_err(|_| Error::<T>::NoOrder)?;
            let now = <frame_system::Pallet<T>>::block_number();
            ensure!(order.expired > now, Error::<T>::OrderExpired);
            match order.order_type {
                OrderType::Buy => {
                    ensure!(!<UserOwnedSpace<T>>::contains_key(&order.target_acc), Error::<T>::PurchasedSpace);
                    let space = G_BYTE.checked_mul(order.gib_count as u128).ok_or(Error::<T>::Overflow)?;
                    Self::add_user_purchased_space(order.target_acc, space, order.days)?;
			        Self::add_purchased_space(space)?;
                },
                OrderType::Expansion => {
                    ensure!(<UserOwnedSpace<T>>::contains_key(&order.target_acc), Error::<T>::NotPurchasedSpace);
                    let space = G_BYTE.checked_mul(order.gib_count as u128).ok_or(Error::<T>::Overflow)?;
                    Self::add_purchased_space(space)?;
                    Self::expension_puchased_package(order.target_acc, space)?;
                },
                OrderType::Renewal => {
                    ensure!(<UserOwnedSpace<T>>::contains_key(&order.target_acc), Error::<T>::NotPurchasedSpace);
                    Self::update_puchased_package(order.target_acc, order.days)?;
                },
            };

            T::CessTreasuryHandle::send_to_sid(sender, order.pay)?;
            Self::deposit_event(Event::<T>::PaidOrder { order_hash: order_id });

            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn calculate_price(gib_count: u32, days: u32) -> Result<BalanceOf<T>, DispatchError> {
        let unit_price: u128 = <UnitPrice<T>>::get().unwrap().try_into().map_err(|_| Error::<T>::Overflow)?;
        let gib_count: u128 = gib_count.into();
        let days: u128 = days.into();
        let price = gib_count
            .checked_mul(days).ok_or(Error::<T>::Overflow)?
            .checked_mul(unit_price).ok_or(Error::<T>::Overflow)?;
        
        let price: BalanceOf<T> = price.try_into().map_err(|_| Error::<T>::Overflow)?;
        Ok(price)
    }

    fn calculate_remain_day(deadline: BlockNumberOf<T>) -> Result<BlockNumberOf<T>, DispatchError>{
        let now = <frame_system::Pallet<T>>::block_number();
        //Calculate remaining days.
        let block_oneday: BlockNumberOf<T> = <T as pallet::Config>::OneDay::get();
        let diff_block = deadline.checked_sub(&now).ok_or(Error::<T>::Overflow)?;
        let mut remain_day: u32 = diff_block
            .checked_div(&block_oneday)
            .ok_or(Error::<T>::Overflow)?
            .saturated_into();
        if diff_block % block_oneday != 0u32.saturated_into() {
            remain_day = remain_day
                .checked_add(1)
                .ok_or(Error::<T>::Overflow)?
                .saturated_into();
        }

        Ok(remain_day.into())
    }
    /// helper: update_puchased_package.
    ///
    /// How to update the corresponding data after renewing the package.
    /// Currently, it only supports and defaults to one month.
    ///
    /// Parameters:
    /// - `acc`: Account
    /// - `days`: Days of renewal
    fn update_puchased_package(acc: AccountOf<T>, days: u32) -> DispatchResult {
        <UserOwnedSpace<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
            let s = s_opt.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;
            let one_day = <T as pallet::Config>::OneDay::get();
            let now = <frame_system::Pallet<T>>::block_number();
            let sur_block: BlockNumberOf<T> =
                one_day.checked_mul(&days.saturated_into()).ok_or(Error::<T>::Overflow)?;
            if now > s.deadline {
                s.start = now;
                s.deadline = now.checked_add(&sur_block).ok_or(Error::<T>::Overflow)?;
            } else {
                s.deadline = s.deadline.checked_add(&sur_block).ok_or(Error::<T>::Overflow)?;
            }

            if s.deadline > now {
                s.state = SPACE_NORMAL
                .as_bytes()
                .to_vec()
                .try_into()
                .map_err(|_e| Error::<T>::BoundedVecError)?;
            }

            Ok(())
        })?;
        Ok(())
    }
    /// helper: Expand storage space.
    ///
    /// Relevant data of users after updating the expansion package.
    ///
    /// Parameters:
    /// - `space`: Size after expansion.
    /// - `package_type`: New package type.
    fn expension_puchased_package(
        acc: AccountOf<T>,
        space: u128,
    ) -> DispatchResult {
        <UserOwnedSpace<T>>::try_mutate(&acc, |s_opt| -> DispatchResult {
            let s = s_opt.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;
            s.remaining_space = s.remaining_space.checked_add(space).ok_or(Error::<T>::Overflow)?;
            s.total_space = s.total_space.checked_add(space).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })?;

        Ok(())
    }
    /// helper: Initial purchase space initialization method.
    ///
    /// Purchase a package and create data corresponding to the user.
    /// UserOwnedSpace Storage.
    ///
    /// Parameters:
    /// - `space`: Buy storage space size.
    /// - `month`: Month of purchase of package, It is 1 at present.
    /// - `package_type`: Package type.
    fn add_user_purchased_space(
        acc: AccountOf<T>,
        space: u128,
        days: u32,
    ) -> DispatchResult {
        let now = <frame_system::Pallet<T>>::block_number();
        let one_day = <T as pallet::Config>::OneDay::get();
        let sur_block: BlockNumberOf<T> = one_day
            .checked_mul(&days.saturated_into())
            .ok_or(Error::<T>::Overflow)?;
        let deadline = now.checked_add(&sur_block).ok_or(Error::<T>::Overflow)?;
        let info = OwnedSpaceDetails::<T> {
            total_space: space,
            used_space: 0,
            locked_space: u128::MIN,
            remaining_space: space,
            start: now,
            deadline,
            state: SPACE_NORMAL
                .as_bytes()
                .to_vec()
                .try_into()
                .map_err(|_e| Error::<T>::BoundedVecError)?,
        };
        <UserOwnedSpace<T>>::insert(&acc, info);
        Ok(())
    }

    /// helper: update user storage space.
    ///
    /// Modify the corresponding data after the user uploads the file or deletes the file
    /// Modify used_space, remaining_space
    /// operation = 1, add used_space
    /// operation = 2, sub used_space
    ///
    /// Parameters:
    /// - `operation`: operation type 1 or 2.
    /// - `size`: file size.
    fn update_user_space(acc: &AccountOf<T>, operation: u8, size: u128) -> DispatchResult {
        match operation {
            1 => {
                <UserOwnedSpace<T>>::try_mutate(acc, |s_opt| -> DispatchResult {
                    let s = s_opt.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;
                    if s.state.to_vec() == SPACE_FROZEN.as_bytes().to_vec() {
                        Err(Error::<T>::LeaseFreeze)?;
                    }
                    if size > s.remaining_space {
                        Err(Error::<T>::InsufficientStorage)?;
                    }
                    s.used_space =
                        s.used_space.checked_add(size).ok_or(Error::<T>::Overflow)?;
                    s.remaining_space =
                        s.remaining_space.checked_sub(size).ok_or(Error::<T>::Overflow)?;
                    Ok(())
                })?;
            },
            2 => <UserOwnedSpace<T>>::try_mutate(acc, |s_opt| -> DispatchResult {
                let s = s_opt.as_mut().unwrap();
                s.used_space = s.used_space.checked_sub(size).ok_or(Error::<T>::Overflow)?;
                s.remaining_space =
                    s.total_space.checked_sub(s.used_space).ok_or(Error::<T>::Overflow)?;
                Ok(())
            })?,
            _ => Err(Error::<T>::WrongOperation)?,
        }
        Ok(())
    }

    fn frozen_task() -> (Weight, Vec<AccountOf<T>>) {
        let now: BlockNumberOf<T> = <frame_system::Pallet<T>>::block_number();
        let number: u128 = now.saturated_into();
    
        let mut weight: Weight = Weight::zero();
        let mut clear_acc_list: Vec<AccountOf<T>> = Default::default();

        log::info!("Start lease expiration check");
        for (acc, info) in <UserOwnedSpace<T>>::iter() {
            weight = weight.saturating_add(T::DbWeight::get().reads(1 as u64));
            if now > info.deadline {
                let frozen_day: BlockNumberOf<T> = <T as pallet::Config>::FrozenDays::get();
                if now > info.deadline + frozen_day {
                    log::info!("clear user:#{}'s files", number);
                    let result = <UserOwnedSpace<T>>::try_mutate(
                        &acc,
                        |s_opt| -> DispatchResult {
                            let s = s_opt
                                .as_mut()
                                .ok_or(Error::<T>::NotPurchasedSpace)?;
                            s.state = SPACE_DEAD
                                .as_bytes()
                                .to_vec()
                                .try_into()
                                .map_err(|_e| Error::<T>::BoundedVecError)?;
                            Ok(())
                        },
                    );
                    match result {
                        Ok(()) => log::info!("user space dead: #{}", number),
                        Err(e) => log::error!("space mark dead failed: {:?}", e),
                    }
                    weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
                    clear_acc_list.push(acc);
                } else {
                    if info.state.to_vec() != SPACE_FROZEN.as_bytes().to_vec() {
                        let result = <UserOwnedSpace<T>>::try_mutate(
                            &acc,
                            |s_opt| -> DispatchResult {
                                let s = s_opt
                                    .as_mut()
                                    .ok_or(Error::<T>::NotPurchasedSpace)?;
                                s.state = SPACE_FROZEN
                                    .as_bytes()
                                    .to_vec()
                                    .try_into()
                                    .map_err(|_e| Error::<T>::BoundedVecError)?;
                                Ok(())
                            },
                        );
                        match result {
                            Ok(()) => log::info!("user space frozen: #{}", number),
                            Err(e) => log::error!("frozen failed: {:?}", e),
                        }
                        weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
                    }
                }
            }
        }
        log::info!("End lease expiration check");
        (weight, clear_acc_list)
    }

    pub fn lock_user_space(acc: &T::AccountId, needed_space: u128) -> DispatchResult {
        <UserOwnedSpace<T>>::try_mutate(acc, |storage_space_opt| -> DispatchResult {
            let storage_space = storage_space_opt.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;
            if storage_space.state.to_vec() == SPACE_FROZEN.as_bytes().to_vec() {
                Err(Error::<T>::LeaseFreeze)?;
            }
            if storage_space.remaining_space < needed_space {
                Err(Error::<T>::InsufficientStorage)?;
            }
            storage_space.locked_space = storage_space.locked_space.checked_add(needed_space).ok_or(Error::<T>::Overflow)?;
            storage_space.remaining_space = storage_space.remaining_space.checked_sub(needed_space).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }

    pub fn unlock_user_space(acc: &T::AccountId, needed_space: u128) -> DispatchResult {
        <UserOwnedSpace<T>>::try_mutate(acc, |storage_space_opt| -> DispatchResult {
            let storage_space = storage_space_opt.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;
            storage_space.locked_space = storage_space.locked_space.checked_sub(needed_space).ok_or(Error::<T>::Overflow)?;
            storage_space.remaining_space = storage_space.remaining_space.checked_add(needed_space).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }

    pub fn unlock_and_used_user_space(acc: &T::AccountId, needed_space: u128) -> DispatchResult {
        <UserOwnedSpace<T>>::try_mutate(acc, |storage_space_opt| -> DispatchResult {
            let storage_space = storage_space_opt.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;
            storage_space.locked_space = storage_space.locked_space.checked_sub(needed_space).ok_or(Error::<T>::Overflow)?;
            storage_space.used_space = storage_space.used_space.checked_add(needed_space).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }

    pub fn check_user_space(acc: &T::AccountId, needed_space: u128) -> Result<bool, DispatchError> {
        let user_storage = <UserOwnedSpace<T>>::try_get(acc).map_err(|_e| Error::<T>::NotPurchasedSpace)?;

        Ok(user_storage.remaining_space >= needed_space)
    }

    //Get the available space on the current chain.
    pub fn get_total_space() -> Result<u128, DispatchError> {
        let purchased_space = <PurchasedSpace<T>>::get();
        let total_space = <TotalIdleSpace<T>>::get().checked_add(<TotalServiceSpace<T>>::get()).ok_or(Error::<T>::Overflow)?;
        //If the total space on the current chain is less than the purchased space, 0 will be
        // returned.
        if total_space < purchased_space {
            return Ok(0);
        }
        //Calculate available space.
        let value = total_space.checked_sub(purchased_space).ok_or(Error::<T>::Overflow)?;

        Ok(value)
    }

    fn add_total_idle_space(increment: u128) -> DispatchResult {
        TotalIdleSpace::<T>::try_mutate(|total_power| -> DispatchResult {
            *total_power = total_power.checked_add(increment).ok_or(Error::<T>::Overflow)?;
            Ok(())
        }) //read 1 write 1
    }

    fn add_total_service_space(increment: u128) -> DispatchResult {
        TotalServiceSpace::<T>::try_mutate(|total_space| -> DispatchResult {
            *total_space = total_space.checked_add(increment).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }

    fn sub_total_idle_space(decrement: u128) -> DispatchResult {
        TotalIdleSpace::<T>::try_mutate(|total_power| -> DispatchResult {
            *total_power = total_power.checked_sub(decrement).ok_or(Error::<T>::Overflow)?;
            Ok(())
        }) //read 1 write 1
    }

    fn sub_total_service_space(decrement: u128) -> DispatchResult {
        TotalServiceSpace::<T>::try_mutate(|total_space| -> DispatchResult {
            *total_space = total_space.checked_sub(decrement).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }

    fn add_purchased_space(size: u128) -> DispatchResult {
        <PurchasedSpace<T>>::try_mutate(|purchased_space| -> DispatchResult {
            let total_space = <TotalIdleSpace<T>>::get().checked_add(<TotalServiceSpace<T>>::get()).ok_or(Error::<T>::Overflow)?;
            let new_space = purchased_space.checked_add(size).ok_or(Error::<T>::Overflow)?;
            if new_space > total_space {
                Err(<Error<T>>::InsufficientAvailableSpace)?;
            }
            *purchased_space = new_space;
            Ok(())
        })
    }

    fn sub_purchased_space(size: u128) -> DispatchResult {
        <PurchasedSpace<T>>::try_mutate(|purchased_space| -> DispatchResult {
            *purchased_space = purchased_space.checked_sub(size).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })
    }
}

pub trait StorageHandle<AccountId> {
    fn update_user_space(acc: &AccountId, opeartion: u8, size: u128) -> DispatchResult;
    fn add_total_idle_space(increment: u128) -> DispatchResult;
	fn sub_total_idle_space(decrement: u128) -> DispatchResult;
	fn add_total_service_space(increment: u128) -> DispatchResult;
	fn sub_total_service_space(decrement: u128) -> DispatchResult;
    fn get_total_idle_space() -> u128;
    fn get_total_service_space() -> u128;
    fn add_purchased_space(size: u128) -> DispatchResult;
	fn sub_purchased_space(size: u128) -> DispatchResult;
    fn get_total_space() -> Result<u128, DispatchError>;
    fn lock_user_space(acc: &AccountId, needed_space: u128) -> DispatchResult;
    fn unlock_user_space(acc: &AccountId, needed_space: u128) -> DispatchResult;
    fn unlock_and_used_user_space(acc: &AccountId, needed_space: u128) -> DispatchResult;
    fn get_user_avail_space(acc: &AccountId) -> Result<u128, DispatchError>;
    fn frozen_task() -> (Weight, Vec<AccountId>);
    fn delete_user_space_storage(acc: &AccountId) -> Result<Weight, DispatchError>;
}

impl<T: Config> StorageHandle<T::AccountId> for Pallet<T> {
    fn update_user_space(acc: &T::AccountId, opeartion: u8, size: u128) -> DispatchResult {
        Pallet::<T>::update_user_space(acc, opeartion, size)
    }

    fn add_total_idle_space(increment: u128) -> DispatchResult {
        Pallet::<T>::add_total_idle_space(increment)
    }

	fn sub_total_idle_space(decrement: u128) -> DispatchResult {
        Pallet::<T>::sub_total_idle_space(decrement)
    }

	fn add_total_service_space(increment: u128) -> DispatchResult {
        Pallet::<T>::add_total_service_space(increment)
    }

	fn sub_total_service_space(decrement: u128) -> DispatchResult {
        Pallet::<T>::sub_total_service_space(decrement)
    }  

    fn add_purchased_space(size: u128) -> DispatchResult {
		Pallet::<T>::add_purchased_space(size)
	}

	fn sub_purchased_space(size: u128) -> DispatchResult {
		Pallet::<T>::sub_purchased_space(size)
	}

    fn get_total_space() -> Result<u128, DispatchError> {
		Pallet::<T>::get_total_space()
	}

    fn lock_user_space(acc: &T::AccountId, needed_space: u128) -> DispatchResult {
        Pallet::<T>::lock_user_space(acc, needed_space)
    }

    fn unlock_user_space(acc: &T::AccountId, needed_space: u128) -> DispatchResult {
        Pallet::<T>::unlock_user_space(acc, needed_space)
    }

    fn unlock_and_used_user_space(acc: &T::AccountId, needed_space: u128) -> DispatchResult {
        Pallet::<T>::unlock_and_used_user_space(acc, needed_space)
    }

    fn get_user_avail_space(acc: &T::AccountId) -> Result<u128, DispatchError> {
        let info = <UserOwnedSpace<T>>::try_get(acc).map_err(|_e| Error::<T>::NotPurchasedSpace)?;
        Ok(info.remaining_space)
    }

    fn frozen_task() -> (Weight, Vec<AccountOf<T>>) {
        Self::frozen_task()
    }

    fn delete_user_space_storage(acc: &T::AccountId) -> Result<Weight, DispatchError> {
        let mut weight: Weight = Weight::zero();

        let space_info = <UserOwnedSpace<T>>::try_get(acc).map_err(|_| Error::<T>::NotPurchasedSpace)?;
        weight = weight.saturating_add(T::DbWeight::get().reads(1 as u64));

        Self::sub_purchased_space(space_info.total_space)?;
        weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

        <UserOwnedSpace<T>>::remove(acc);
        weight = weight.saturating_add(T::DbWeight::get().writes(1 as u64));

        Ok(weight)
    }

    fn get_total_idle_space() -> u128 {
        <TotalIdleSpace<T>>::get()
    }

    fn get_total_service_space() -> u128 {
        <TotalServiceSpace<T>>::get()
    }
}