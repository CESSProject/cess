#![cfg_attr(not(feature = "std"), no_std)]

use frame_system::{
    ensure_root, ensure_signed,
    pallet_prelude::*,
};
use frame_support::{
    dispatch::{Parameter},
    Blake2_128Concat, PalletId, weights::Weight, ensure, transactional,
    storage::bounded_vec::BoundedVec,
    traits::{
        StorageVersion, Currency, ReservableCurrency, Randomness, ExistenceRequirement::KeepAlive,
        schedule::v3::{Named as ScheduleNamed},
        schedule, schedule::DispatchTime, QueryPreimage, StorePreimage,
    },
    pallet_prelude::*,
};
use sp_runtime::{
	traits::{
        CheckedAdd, CheckedMul, CheckedDiv, CheckedSub,
		SaturatedConversion, Dispatchable,
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

pub mod impls;
pub use impls::*;

pub use pallet::*;

pub const SPACE_NORMAL: &str = "normal";
pub const SPACE_FROZEN: &str = "frozen";
pub const SPACE_DEAD: &str = "dead";

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> =
	<<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type TokenId = H256;

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

        type FScheduler: ScheduleNamed<
            BlockNumberFor<Self>, 
            Self::SProposal,
            Self::PalletsOrigin,
            Hasher = Self::Hashing,
        >;

        type SProposal: Parameter + Dispatchable<RuntimeOrigin = Self::RuntimeOrigin> + From<Call<Self>>;

        type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>>;

        #[pallet::constant]
		type OneDay: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
		type OneHours: Get<BlockNumberFor<Self>>;

        /// pallet address.
		#[pallet::constant]
		type RewardPalletId: Get<PalletId>;

        #[pallet::constant]
        type StateStringMax: Get<u32> + Clone + Eq + PartialEq;

        #[pallet::constant]
        type NameLimit: Get<u32>;

        #[pallet::constant]
        type FrozenLimit: Get<u32>;

        #[pallet::constant]
        type ConsignmentRemainingBlock: Get<BlockNumberFor<Self>>;

        #[pallet::constant]
        type LockingBlock: Get<BlockNumberFor<Self>>;
        
		#[pallet::constant]
		type FrozenDays: Get<BlockNumberFor<Self>> + Clone + Eq + PartialEq;

        type CessTreasuryHandle: TreasuryHandle<AccountOf<Self>, BalanceOf<Self>>;

        type MyRandomness: Randomness<Option<Self::Hash>, BlockNumberFor<Self>>;

        /// The preimage provider with which we look up call hashes to get the call.
		type Preimages: QueryPreimage<H = Self::Hashing> + StorePreimage;
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

        MintTerritory { 
            token: TokenId, 
            name: TerrName, 
            storage_capacity: u128, 
            spend: BalanceOf<T>,
        },

        ExpansionTerritory { 
            name: TerrName, 
            expansion_space: u128, 
            spend: BalanceOf<T>,
        },

        RenewalTerritory {
            name: TerrName, 
            days: u32, 
            spend: BalanceOf<T>,
        },

        ReactivateTerritory {
            name: TerrName, 
            days: u32, 
            spend: BalanceOf<T>,
        },

        Consignment {
            name: TerrName,
            token: TokenId,
            price: BalanceOf<T>,
        },

        BuyConsignment {
            name: TerrName,
            token: TokenId,
            price: BalanceOf<T>,
        },

        CancleConsignment {
            token: TokenId,
        },

        CancelPurchaseAction {
            token: TokenId,
        },

        ExecConsignment {
            buyer: AccountOf<T>,
            seller: AccountOf<T>,
            token: TokenId,
        },
    }

    #[pallet::error]
	pub enum Error<T> {
        /// System method errors that should not occur
        BugInvalid,
        /// Convert bounded vec error 
        BoundedVecError,
        /// Insufficient available space on the network, unable to purchase
        InsufficientAvailableSpace,
        /// Balance not enough
        InsufficientBalance,
        /// The user currently has insufficient available space
        InsufficientStorage,
        /// Data operation overflow
        Overflow,
        /// Wrong operator input, can only be 1 or 2
        WrongOperation,
        /// Space has already been purchased and cannot be purchased again
        PurchasedSpace,
        /// Space not purchased, please purchase space first before calling this transaction
        NotPurchasedSpace,
        /// storage space frozen
        LeaseFreeze,
        /// Space has expired
        LeaseExpired,
        /// Order has expired
        OrderExpired,
        /// Random number generation error1
        RandomErr,
        /// There is no such order
        NoOrder,
        /// Parameter error, please check the parameters. The expiration time cannot exceed one hour
        ParamError,
        /// There is already an identical token on the chain
        DuplicateTokens,
        /// This user does not have this territory
        NotHaveTerritory,
        /// The territory is not active
        NotActive,
        /// The territory is not expired
        NotExpire,
        /// The territory is not currently on consignment
        NotOnConsignment,
        /// The territory does not have enough lease time remaining to allow consignment sale
        InsufficientLease,
        /// The current delegation already exists and cannot be created again
        ConsignmentExisted,
        /// It is on consignment, so it cannot be renewed
        OnConsignment,
        /// The current territory's state does not support this operation
        StateError,
        /// A territory must have nothing stored in it before it can be consigned
        ObjectNotZero,
        /// The delegation corresponding to the token value does not exist
        NonExistentConsignment,
        /// The consignment has been purchased by someone else and is locked. It cannot be purchased again. Or cancel the order
        ConsignmentLocked,
        /// The status of the order is abnormal and the purchase action fails
        ConsignmentUnLocked,
        /// Logically speaking, errors that should not occur
        Unexpected,
        /// Not the buyer of this consignment, no right to operate
        NotBuyer,
        /// This is an invalid order, Because the price can't match
        InvalidOrder,
        /// Unable to purchase own consignment
        OwnConsignment,
    }

    #[pallet::storage]
    #[pallet::getter(fn territory_key)]
    pub(super) type TerritoryKey<T: Config> = 
        StorageMap<_, Blake2_128Concat, TokenId, (AccountOf<T>, TerrName)>;
    
    #[pallet::storage]
    #[pallet::getter(fn territory)]
    pub(super) type Territory<T: Config> = 
        StorageDoubleMap<
            _,
            Blake2_128Concat,
            AccountOf<T>,
            Blake2_128Concat,
            TerrName,
            TerritoryInfo<T>,
        >;

    #[pallet::storage]
    #[pallet::getter(fn consignment)]
    pub(super) type Consignment<T: Config> =
        StorageMap<_, Blake2_128Concat, TokenId, ConsignmentInfo<T>>;

    #[pallet::storage]
    #[pallet::getter(fn territory_frozen)]
    pub(super) type TerritoryFrozen<T: Config> =
        StorageDoubleMap<
            _, 
            Blake2_128Concat, 
            BlockNumberFor<T>,
            Blake2_128Concat, 
            TokenId,
            bool,
        >;
    
    #[pallet::storage]
    #[pallet::getter(fn territory_frozen_counter)]
    pub(super) type TerritoryFrozenCounter<T: Config> =
        StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, u32, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn territory_expired)]
    pub(super) type TerritoryExpired<T: Config> =
        StorageDoubleMap<
            _, 
            Blake2_128Concat, 
            BlockNumberFor<T>,
            Blake2_128Concat, 
            TokenId,
            bool,
        >;
 
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

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				price: 30u32.saturated_into(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			UnitPrice::<T>::put(self.price);
		}
	}

    #[pallet::call]
	impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::mint_territory())]
		pub fn mint_territory(
            origin: OriginFor<T>, 
            gib_count: u32, 
            territory_name: TerrName,
        ) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(!<Territory<T>>::contains_key(&sender, &territory_name), Error::<T>::PurchasedSpace);

            let now = <frame_system::Pallet<T>>::block_number();
            let seed = (sender.clone(), now, territory_name.clone());
            let (random_seed, _) =
                T::MyRandomness::random(&(T::RewardPalletId::get(), seed).encode());
            let token = match random_seed {
				Some(random_seed) => <H256>::decode(&mut random_seed.as_ref()).map_err(|_| Error::<T>::RandomErr)?,
				None => Default::default(),
			};
            ensure!(!<TerritoryKey<T>>::contains_key(&token), Error::<T>::DuplicateTokens);

			let space = G_BYTE.checked_mul(gib_count as u128).ok_or(Error::<T>::Overflow)?;
			let unit_price = <UnitPrice<T>>::try_get()
				.map_err(|_e| Error::<T>::BugInvalid)?;

			Self::storage_territory(token, sender.clone(), space, 30, territory_name.clone())?;
			Self::add_purchased_space(space)?;
			let price: BalanceOf<T> = unit_price
				.checked_mul(&gib_count.saturated_into())
				.ok_or(Error::<T>::Overflow)?;
            
			ensure!(
				<T as pallet::Config>::Currency::can_slash(&sender, price.clone()),
				Error::<T>::InsufficientBalance
			);

            T::CessTreasuryHandle::send_to_sid(sender.clone(), price)?;

			Self::deposit_event(Event::<T>::MintTerritory {
                token: token, 
                name: territory_name, 
                storage_capacity: space, 
                spend: price,
            });

			Ok(())
		}

        #[pallet::call_index(1)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::expanding_territory())]
        pub fn expanding_territory(
            origin: OriginFor<T>, 
            territory_name: TerrName, 
            gib_count: u32,
        ) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let cur_owned_space = <Territory<T>>::try_get(&sender, &territory_name)
				.map_err(|_e| Error::<T>::NotHaveTerritory)?;
			let now = <frame_system::Pallet<T>>::block_number();
			ensure!(
				cur_owned_space.state == TerritoryState::Active,
				Error::<T>::NotActive
			);
			// The unit price recorded in UnitPrice is the unit price of one month.
			// Here, the daily unit price is calculated.
			let day_unit_price = <UnitPrice<T>>::try_get()
				.map_err(|_e| Error::<T>::BugInvalid)?
				.checked_div(&30u32.saturated_into()).ok_or(Error::<T>::Overflow)?;
			let space = G_BYTE.checked_mul(gib_count as u128).ok_or(Error::<T>::Overflow)?;
			//Calculate remaining days.
			let block_oneday: BlockNumberFor<T> = <T as pallet::Config>::OneDay::get();
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

			Self::add_purchased_space(space)?;
			Self::update_territory_space(sender.clone(), territory_name.clone(), space)?;

            T::CessTreasuryHandle::send_to_sid(sender.clone(), price.clone())?;

			Self::deposit_event(Event::<T>::ExpansionTerritory {
				name: territory_name,
				expansion_space: space,
				spend: price,
			});

			Ok(())
		}

        #[pallet::call_index(2)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::renewal_territory())]
        pub fn renewal_territory(
            origin: OriginFor<T>, 
            territory_name: TerrName, 
            days: u32,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
			let cur_owned_space = <Territory<T>>::try_get(&sender, &territory_name)
				.map_err(|_e| Error::<T>::NotHaveTerritory)?;

            ensure!(
                cur_owned_space.state == TerritoryState::Active || cur_owned_space.state == TerritoryState::Frozen, 
                Error::<T>::StateError,
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

			Self::update_territory_days(sender.clone(), territory_name.clone(), days)?;
			Self::deposit_event(Event::<T>::RenewalTerritory {
				name: territory_name,
				days: days,
				spend: price,
			});
			Ok(())
        }

        #[pallet::call_index(101)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::reactivate_territory())]
        pub fn reactivate_territory(
            origin: OriginFor<T>, 
            territory_name: TerrName,
            days: u32,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let territory = <Territory<T>>::try_get(&sender, &territory_name)
                .map_err(|_| Error::<T>::NotHaveTerritory)?;
            
            ensure!(territory.state == TerritoryState::Expired, Error::<T>::NotExpire);

            let days_unit_price = <UnitPrice<T>>::try_get()
                .map_err(|_e| Error::<T>::BugInvalid)?
                .checked_div(&30u32.saturated_into())
                .ok_or(Error::<T>::Overflow)?;

            let gib_count = territory.total_space.checked_div(G_BYTE).ok_or(Error::<T>::Overflow)?;

            let price = days_unit_price
                .checked_mul(&days.saturated_into())
                .ok_or(Error::<T>::Overflow)?
                .checked_mul(&gib_count.saturated_into())
                .ok_or(Error::<T>::Overflow)?;
            ensure!(
                <T as pallet::Config>::Currency::can_slash(&sender, price.clone()),
                Error::<T>::InsufficientBalance
            );

            T::CessTreasuryHandle::send_to_sid(sender.clone(), price.clone())?;

            Self::add_purchased_space(territory.total_space)?;
            Self::initial_territory(sender.clone(), territory_name.clone(), days)?;
            
            Self::deposit_event(Event::<T>::ReactivateTerritory {
				name: territory_name,
				days: days,
				spend: price,
			});

            Ok(())
        }

        #[pallet::call_index(102)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::territory_consignment())]
        pub fn territory_consignment(
            origin: OriginFor<T>, 
            territory_name: TerrName, 
            price: BalanceOf<T>
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let token = <Territory<T>>::try_mutate(&sender, &territory_name, |t_opt| -> Result<TokenId, DispatchError> {
                let t = t_opt.as_mut().ok_or(Error::<T>::NotHaveTerritory)?;

                ensure!(t.state == TerritoryState::Active, Error::<T>::NotActive);
                ensure!(t.total_space == t.remaining_space, Error::<T>::ObjectNotZero);

                let now = <frame_system::Pallet<T>>::block_number();
                let remain_block = t.deadline.checked_sub(&now).ok_or(Error::<T>::Overflow)?;
                let limit_block = T::ConsignmentRemainingBlock::get();
                ensure!(remain_block > limit_block, Error::<T>::InsufficientLease);

                t.state = TerritoryState::OnConsignment;

                Ok(t.token)
            })?;

            ensure!(!<Consignment<T>>::contains_key(&token), Error::<T>::ConsignmentExisted);
            let consignment_info = ConsignmentInfo::<T>{
                user: sender,
                price: price,
                buyers: None,
                exec: None,
                locked: false,
            };
            <Consignment<T>>::insert(&token, consignment_info);

            Self::deposit_event(Event::<T>::Consignment {
                name: territory_name,
                token: token,
                price: price,
            });

            Ok(())
        }

        #[pallet::call_index(103)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::buy_consignment())]
        pub fn buy_consignment(
            origin: OriginFor<T>,
            token: TokenId,
            rename: TerrName,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let consignment = <Consignment<T>>::try_get(&token).map_err(|_| Error::<T>::NonExistentConsignment)?;
            ensure!(!consignment.locked, Error::<T>::ConsignmentLocked);
            ensure!(consignment.user != sender, Error::<T>::OwnConsignment);

            <Consignment<T>>::try_mutate(&token, |c_opt| -> DispatchResult {
                let c = c_opt.as_mut().ok_or(Error::<T>::NonExistentConsignment)?;

                let now = <frame_system::Pallet<T>>::block_number();
                let lock_block = T::LockingBlock::get();
                let exec_block = now.checked_add(&lock_block).ok_or(Error::<T>::Overflow)?;

                c.buyers = Some(sender.clone());
                c.exec = Some(exec_block);
                c.locked = true;

                <T as pallet::Config>::Currency::reserve(&sender, c.price);

                let call: <T as Config>::SProposal = Call::exec_consignment{token: token.clone(), territory_name: rename.clone()}.into();
                T::FScheduler::schedule_named(
                    *(token.as_fixed_bytes()),
                    DispatchTime::At(exec_block),
                    Option::None,
                    schedule::HARD_DEADLINE,
                    frame_system::RawOrigin::Root.into(),
                    T::Preimages::bound(*Box::new(
                        call
                    ))?, 
                ).map_err(|_| Error::<T>::Unexpected)?;

                Self::deposit_event(Event::<T>::BuyConsignment {
                    name: rename.clone(),
                    token: token,
                    price: c.price,
                });

                Ok(())
            })
        }

        #[pallet::call_index(104)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::exec_consignment())]
        pub fn exec_consignment(origin: OriginFor<T>, token: TokenId, territory_name: TerrName) -> DispatchResult {
            ensure_root(origin)?;

            let consignment = <Consignment<T>>::try_get(&token).map_err(|_| Error::<T>::NonExistentConsignment)?;
            let buyer = consignment.buyers.ok_or(Error::<T>::Unexpected)?;
            ensure!(consignment.locked, Error::<T>::ConsignmentUnLocked);
            ensure!(
                <T as pallet::Config>::Currency::can_slash(&buyer, consignment.price),
                Error::<T>::InsufficientBalance
            );

            let (holder, name) = <TerritoryKey<T>>::try_get(&token).map_err(|_| Error::<T>::Unexpected)?;
            let mut territory = <Territory<T>>::try_get(&holder, &name).map_err(|_| Error::<T>::Unexpected)?;
            ensure!(territory.state == TerritoryState::OnConsignment, Error::<T>::Unexpected);

            <Territory<T>>::remove(&holder, &name);
            territory.state = TerritoryState::Active;
            <Territory<T>>::insert(&buyer, &territory_name, territory);

            <TerritoryKey<T>>::insert(&token, (buyer.clone(), territory_name));
            <Consignment<T>>::remove(&token);
            <T as pallet::Config>::Currency::unreserve(&buyer, consignment.price);
            <T as pallet::Config>::Currency::transfer(&buyer, &holder, consignment.price, KeepAlive)?;

            Self::deposit_event(Event::<T>::ExecConsignment {
                buyer: buyer,
                seller: holder,
                token: token,
            });

            Ok(())
        }

        #[pallet::call_index(105)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::cancel_consignment())]
        pub fn cancel_consignment(origin: OriginFor<T>, territory_name: TerrName) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let mut territory = <Territory<T>>::try_get(&sender, &territory_name).map_err(|_| Error::<T>::NotHaveTerritory)?;
            ensure!(territory.state == TerritoryState::OnConsignment, Error::<T>::NotOnConsignment);
            let consignment = <Consignment<T>>::try_get(&territory.token).map_err(|_| Error::<T>::NonExistentConsignment)?;
            ensure!(!consignment.locked, Error::<T>::ConsignmentLocked);

            <Consignment<T>>::remove(&territory.token);
            territory.state = TerritoryState::Active;
            <Territory<T>>::insert(&sender, &territory_name, territory.clone());

            Self::deposit_event(Event::<T>::CancleConsignment {token: territory.token});

            Ok(())
        }

        #[pallet::call_index(106)]
		#[transactional]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::cancel_purchase_action())]
        pub fn cancel_purchase_action(origin: OriginFor<T>, token: TokenId) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            <Consignment<T>>::try_mutate(&token, |c_opt| -> DispatchResult {
                let c = c_opt.as_mut().ok_or(Error::<T>::NonExistentConsignment)?;

                let buyer = c.buyers.as_ref().ok_or(Error::<T>::NotBuyer)?;
                ensure!(&sender == buyer, Error::<T>::NotBuyer);
                <T as pallet::Config>::Currency::unreserve(&buyer, c.price);
                c.buyers = None;
                c.exec = None;
                c.locked = false;
                T::FScheduler::cancel_named(*(token.as_fixed_bytes()))?;

                Ok(())
            })?;
            
            Self::deposit_event(Event::<T>::CancelPurchaseAction {token: token});
            
            Ok(())
        }

        #[pallet::call_index(107)]
        #[transactional]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::territory_grants())]
        pub fn territory_grants(
            origin: OriginFor<T>, 
            territory_name: TerrName, 
            receiver: AccountOf<T>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let territory = <Territory<T>>::try_get(&sender, &territory_name).map_err(|_| Error::<T>::NotHaveTerritory)?;
            ensure!(territory.state == TerritoryState::Active, Error::<T>::NotActive);
            ensure!(territory.total_space == territory.remaining_space, Error::<T>::ObjectNotZero);
            let new_name: TerrName = territory.token.0.to_vec().try_into().map_err(|_| Error::<T>::BoundedVecError)?;
            <Territory<T>>::remove(&sender, &territory_name);
            <Territory<T>>::insert(
                &receiver, 
                &new_name, 
                territory.clone()
            );
            <TerritoryKey<T>>::try_mutate(&territory.token, |info_opt| -> DispatchResult {
                let info = info_opt.as_mut().ok_or(Error::<T>::Unexpected)?;

                info.0 = receiver;
                info.1 = new_name;

                Ok(())
            })?;

            Ok(())
        }

        #[pallet::call_index(108)]
        #[transactional]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::territory_rename())]
        pub fn territory_rename(
            origin: OriginFor<T>,
            old_name: TerrName,
            new_name: TerrName,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let territory = <Territory<T>>::try_get(&sender, &old_name).map_err(|_| Error::<T>::NotHaveTerritory)?;
            ensure!(territory.state == TerritoryState::Active, Error::<T>::NotActive);
            ensure!(territory.total_space == territory.remaining_space, Error::<T>::ObjectNotZero);
            <Territory<T>>::remove(&sender, &old_name);
            <Territory<T>>::insert(
                &sender,
                &new_name, 
                territory.clone()
            );
            <TerritoryKey<T>>::try_mutate(&territory.token, |info_opt| -> DispatchResult {
                let info = info_opt.as_mut().ok_or(Error::<T>::Unexpected)?;
                info.1 = new_name;
                Ok(())
            })?;
            
            Ok(())
        }

        // FOR TEST
		#[pallet::call_index(4)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
		pub fn update_price(origin: OriginFor<T>) -> DispatchResult {
			let _ = ensure_root(origin)?;
			let default_price: BalanceOf<T> = 30_000_000_000_000_000_000u128.try_into().map_err(|_| Error::<T>::Overflow)?;
			UnitPrice::<T>::put(default_price);

			Ok(())
		}
        // FOR TEST
        #[pallet::call_index(5)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
        pub fn update_user_territory_life(
            origin: OriginFor<T>, 
            user: AccountOf<T>,
            terr_name: TerrName, 
            deadline: BlockNumberFor<T>
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;

            <Territory<T>>::try_mutate(&user, &terr_name, |space_opt| -> DispatchResult {
                let space_info = space_opt.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;

                // TerritoryFrozenCounter::<T>::mutate(&space_info.deadline, |counter| -> DispatchResult {
                //     *counter = counter.checked_sub(1).ok_or(Error::<T>::Overflow)?;
                //     Ok(())
                // })?;

                TerritoryFrozen::<T>::remove(&space_info.deadline, &space_info.token);

                space_info.deadline = deadline;

                TerritoryFrozen::<T>::insert(&space_info.deadline, &space_info.token, true);

                Ok(())
            })
        }

        #[pallet::call_index(201)]
		#[transactional]
		#[pallet::weight(Weight::zero())]
        pub fn update_expired_exec(
            origin: OriginFor<T>, 
            old_block: BlockNumberFor<T>, 
            new_block: BlockNumberFor<T>,
            token: TokenId
        ) -> DispatchResult {
            let _ = ensure_root(origin)?;

            TerritoryExpired::<T>::remove(&old_block, &token);

            TerritoryExpired::<T>::insert(&new_block, &token, true);

            Ok(())
        }


        #[pallet::call_index(6)]
        #[transactional]
        #[pallet::weight(<T as pallet::Config>::WeightInfo::create_order())]
        pub fn create_order(
            origin: OriginFor<T>, 
            target_acc: AccountOf<T>,
            territory_name: TerrName,
            order_type: OrderType, 
            gib_count: u32, 
            days: u32,
            // minute
            expired: u32,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;

            let expired: BlockNumberFor<T> = (expired
                .checked_mul(6).ok_or(Error::<T>::Overflow)?).saturated_into();
            ensure!(expired < T::OneHours::get(), Error::<T>::ParamError);

            let price = match order_type {
                OrderType::Buy => {
                    ensure!(!<Territory<T>>::contains_key(&target_acc, &territory_name), Error::<T>::PurchasedSpace);
                    let price = Self::calculate_price(gib_count, days)?;
                    price
                },
                OrderType::Expansion => {
                    let user_owned_space = <Territory<T>>::try_get(&target_acc, &territory_name).map_err(|_| Error::<T>::NotHaveTerritory)?;
                    let remain_day = Self::calculate_remain_day(user_owned_space.deadline)?;
                    let price = Self::calculate_price(gib_count, remain_day.saturated_into())?;
                    price
                },
                OrderType::Renewal => {
                    let user_owned_space = <Territory<T>>::try_get(&target_acc, &territory_name).map_err(|_| Error::<T>::NotHaveTerritory)?;
                    let gib_count = user_owned_space.total_space.checked_div(G_BYTE).ok_or(Error::<T>::Overflow)?;
                    let price = Self::calculate_price(gib_count as u32, days)?;
                    price
                },
            };

            let now = <frame_system::Pallet<T>>::block_number();
            let expired = now.checked_add(&expired.saturated_into()).ok_or(Error::<T>::Overflow)?;
            let pay_order = OrderInfo::<T> {
                territory_name,
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
        #[pallet::weight(<T as pallet::Config>::WeightInfo::exec_order())]
        pub fn exec_order(origin: OriginFor<T>, order_id: BoundedVec<u8, ConstU32<32>>) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            let order = <PayOrder<T>>::try_get(&order_id).map_err(|_| Error::<T>::NoOrder)?;
            let now = <frame_system::Pallet<T>>::block_number();
            ensure!(order.expired > now, Error::<T>::OrderExpired);
            match order.order_type {
                OrderType::Buy => {
                    ensure!(!<Territory<T>>::contains_key(&order.target_acc, &order.territory_name), Error::<T>::PurchasedSpace);
                    let space = G_BYTE.checked_mul(order.gib_count as u128).ok_or(Error::<T>::Overflow)?;
                    let now = <frame_system::Pallet<T>>::block_number();
                    let seed = (sender.clone(), now, order.territory_name.clone());
                    let (random_seed, _) =
                        T::MyRandomness::random(&(T::RewardPalletId::get(), seed).encode());
                    let token = match random_seed {
                        Some(random_seed) => <H256>::decode(&mut random_seed.as_ref()).map_err(|_| Error::<T>::RandomErr)?,
                        None => Default::default(),
                    };
                    ensure!(!<TerritoryKey<T>>::contains_key(&token), Error::<T>::DuplicateTokens);
                    Self::storage_territory(token, order.target_acc, space, order.days, order.territory_name)?;
			        Self::add_purchased_space(space)?;
                },
                OrderType::Expansion => {
                    let user_owned_space = <Territory<T>>::try_get(&order.target_acc, &order.territory_name).map_err(|_| Error::<T>::NotHaveTerritory)?;
                    let remain_day = Self::calculate_remain_day(user_owned_space.deadline)?;
                    let price = Self::calculate_price(order.gib_count, remain_day.saturated_into())?;
                    // todo! If the price becomes dynamic in the future, the judgment basis will become invalid. 
                    // Make sure that the territory data does not change before and after the order is created.
                    ensure!(price == order.pay, Error::<T>::InvalidOrder);
                    let space = G_BYTE.checked_mul(order.gib_count as u128).ok_or(Error::<T>::Overflow)?;
                    Self::add_purchased_space(space)?;
                    Self::update_territory_space(order.target_acc, order.territory_name, space)?;
                },
                OrderType::Renewal => {
                    let user_owned_space = <Territory<T>>::try_get(&order.target_acc, &order.territory_name).map_err(|_| Error::<T>::NotHaveTerritory)?;
                    let gib_count = user_owned_space.total_space.checked_div(G_BYTE).ok_or(Error::<T>::Overflow)?;
                    let price = Self::calculate_price(gib_count as u32, order.days)?;
                    // todo! If the price becomes dynamic in the future, the judgment basis will become invalid. 
                    // Make sure that the territory data does not change before and after the order is created.
                    ensure!(price == order.pay, Error::<T>::InvalidOrder);
                    Self::update_territory_days(order.target_acc, order.territory_name, order.days)?;
                },
            };

            T::CessTreasuryHandle::send_to_sid(sender, order.pay)?;
            Self::deposit_event(Event::<T>::PaidOrder { order_hash: order_id });

            Ok(())
        }

        // FOR TESTING
        #[pallet::call_index(8)]
        #[pallet::weight(Weight::zero())]
        pub fn clear_service_space(origin: OriginFor<T>) -> DispatchResult {
            let _ = ensure_root(origin)?;

            TotalServiceSpace::<T>::try_mutate(|total_space| -> DispatchResult {
                *total_space = 0;
                Ok(())
            })?;

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

    fn calculate_remain_day(deadline: BlockNumberFor<T>) -> Result<BlockNumberFor<T>, DispatchError>{
        let now = <frame_system::Pallet<T>>::block_number();
        //Calculate remaining days.
        let block_oneday: BlockNumberFor<T> = <T as pallet::Config>::OneDay::get();
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

    fn storage_territory(
        token: TokenId,
        user: AccountOf<T>, 
        space: u128, 
        days: u32,
        tname: TerrName,
    ) -> DispatchResult {
        let now = <frame_system::Pallet<T>>::block_number();
        let one_day = <T as pallet::Config>::OneDay::get();
        let sur_block: BlockNumberFor<T> = one_day
            .checked_mul(&days.saturated_into())
            .ok_or(Error::<T>::Overflow)?;
        let deadline = now.checked_add(&sur_block).ok_or(Error::<T>::Overflow)?;

        let info = TerritoryInfo::<T> {
            token: token.clone(),
            total_space: space,
            used_space: u128::MIN,
            locked_space: u128::MIN,
            remaining_space: space,
            start: now,
            deadline,
            state: TerritoryState::Active,
        };
        <Territory<T>>::insert(&user, &tname, info);
        <TerritoryKey<T>>::insert(&token, (user, tname));
        <TerritoryFrozen<T>>::insert(&deadline, &token, true);
        <TerritoryFrozenCounter<T>>::mutate(&deadline, |counter| -> DispatchResult {
            *counter = counter.checked_add(1).ok_or(Error::<T>::Overflow)?;
            ensure!(*counter < T::FrozenLimit::get(), Error::<T>::Overflow);
            Ok(())
        })?;

        Ok(())
    }

    // Before calling this method, please determine the state of the territory
    fn update_territory_space(
        user: AccountOf<T>,
        tname: TerrName,
        space: u128
    ) -> DispatchResult {
        <Territory<T>>::try_mutate(&user, &tname, |t_opt| -> DispatchResult {
            let t = t_opt.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;
            t.remaining_space = t.remaining_space.checked_add(space).ok_or(Error::<T>::Overflow)?;
            t.total_space = t.total_space.checked_add(space).ok_or(Error::<T>::Overflow)?;
            Ok(())
        })?;

        Ok(())
    }

    // Before calling this method, please determine the state of the territory
    fn update_territory_days(
        user: AccountOf<T>,
        tname: TerrName,
        days: u32,
    ) -> DispatchResult {
        <Territory<T>>::try_mutate(&user, &tname, |t_opt| -> DispatchResult {
            let t = t_opt.as_mut().ok_or(Error::<T>::NotHaveTerritory)?;
            <TerritoryFrozen<T>>::remove(&t.deadline, &t.token);
            <TerritoryFrozenCounter<T>>::mutate(&t.deadline, |counter| -> DispatchResult {
                *counter = counter.checked_sub(1).ok_or(Error::<T>::Overflow)?;
                Ok(())
            })?;
            let frozen_days = <T as pallet::Config>::FrozenDays::get();
            let expired_block = t.deadline.checked_add(&frozen_days).ok_or(Error::<T>::Overflow)?;
            <TerritoryExpired<T>>::remove(&expired_block, &t.token);
            let one_day = <T as pallet::Config>::OneDay::get();
            let now = <frame_system::Pallet<T>>::block_number();
            let sur_block: BlockNumberFor<T> =
                one_day.checked_mul(&days.saturated_into()).ok_or(Error::<T>::Overflow)?;
            if now > t.deadline {
                t.start = now;
                t.deadline = now.checked_add(&sur_block).ok_or(Error::<T>::Overflow)?;
            } else {
                t.deadline = t.deadline.checked_add(&sur_block).ok_or(Error::<T>::Overflow)?;
            }

            if t.deadline > now {
                t.state = TerritoryState::Active;
            }

            <TerritoryFrozen<T>>::insert(&t.deadline, &t.token, true);
            <TerritoryFrozenCounter<T>>::mutate(&t.deadline, |counter| -> DispatchResult {
                *counter = counter.checked_add(1).ok_or(Error::<T>::Overflow)?;
                ensure!(*counter < T::FrozenLimit::get(), Error::<T>::Overflow);
                Ok(())
            })?;

            Ok(())
        })?;
        Ok(())
    }

    // Before calling this method, please determine the state of the territory
    fn initial_territory(
        user: AccountOf<T>,
        tname: TerrName,
        days: u32
    ) -> DispatchResult {
        <Territory<T>>::try_mutate(&user, &tname, |t_opt| -> DispatchResult {
            let t = t_opt.as_mut().ok_or(Error::<T>::NotPurchasedSpace)?;

            let now = <frame_system::Pallet<T>>::block_number();
            
            t.state = TerritoryState::Active;
            t.remaining_space = 0;
            t.locked_space = 0;
            t.used_space = 0;
            t.start = now;

            let one_day = T::OneDay::get();
            let deadline: BlockNumberFor<T> = one_day.checked_mul(&days.saturated_into()).ok_or(Error::<T>::Overflow)?;
            t.deadline = now.checked_add(&deadline).ok_or(Error::<T>::Overflow)?;
            <TerritoryFrozen<T>>::insert(&t.deadline, &t.token, true);
            <TerritoryFrozenCounter<T>>::mutate(&t.deadline, |counter| -> DispatchResult {
                *counter = counter.checked_add(1).ok_or(Error::<T>::Overflow)?;
                ensure!(*counter < T::FrozenLimit::get(), Error::<T>::Overflow);
                Ok(())
            })?;
            
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

    fn frozen_task() -> (Weight, Vec<(AccountOf<T>, TerrName)>) {
        let now: BlockNumberFor<T> = <frame_system::Pallet<T>>::block_number();
        let mut weight: Weight = Weight::zero();

        log::info!("Start lease expiration check");
        for (token, _) in <TerritoryFrozen<T>>::iter_prefix(&now) {
            weight = weight.saturating_add(T::DbWeight::get().reads(1 as u64));
            let result = <TerritoryKey<T>>::try_get(&token).map_err(|_| Error::<T>::Unexpected);
            weight = weight.saturating_add(T::DbWeight::get().reads(1 as u64));
            match result {
                Ok((acc, territory_name)) => {
                    let _ = <Territory<T>>::try_mutate(&acc, &territory_name, |t_opt| -> DispatchResult {
                        let t = t_opt.as_mut().ok_or(Error::<T>::Unexpected)?;
                        if t.state == TerritoryState::OnConsignment {
                            <Consignment<T>>::remove(&token);
                            weight = weight.saturating_add(T::DbWeight::get().writes(1));
                        }

                        t.state = TerritoryState::Frozen;
                        let frozen_days = <T as pallet::Config>::FrozenDays::get();
                        let expired_block = t.deadline.checked_add(&frozen_days).ok_or(Error::<T>::Overflow)?;
                        <TerritoryExpired<T>>::insert(&expired_block, &t.token, true);
                        weight = weight.saturating_add(T::DbWeight::get().writes(1));
                        Ok(())
                    });
                    weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
                },
                Err(e) => log::error!("[StorageHanle] -> [frozen_task]: TerritoryKey read failed {:?}", e),
            }
        }

        let mut list: Vec<(AccountOf<T>, TerrName)> = Default::default();

        for (token, _) in <TerritoryExpired<T>>::iter_prefix(&now) {
            weight = weight.saturating_add(T::DbWeight::get().reads(1 as u64));
            let result = <TerritoryKey<T>>::try_get(&token).map_err(|_| Error::<T>::Unexpected);
            weight = weight.saturating_add(T::DbWeight::get().reads(1 as u64));
            match result {
                Ok((acc, territory_name)) => {
                    let _ = <Territory<T>>::try_mutate(&acc, &territory_name, |t_opt| -> DispatchResult {
                        let t = t_opt.as_mut().ok_or(Error::<T>::Unexpected)?;
                        Self::sub_purchased_space(t.total_space)?;
                        t.state = TerritoryState::Expired;
                        Ok(())
                    });
                    weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
                    list.push((acc, territory_name));
                },
                Err(e) => log::error!("[StorageHanle] -> [frozen_task]: TerritoryKey read failed {:?}", e),
            }
        }

        log::info!("End lease expiration check");
        (weight, list)
    }
}