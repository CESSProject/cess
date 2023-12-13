#![cfg_attr(not(feature = "std"), no_std)]

//! CESS Pallets
//!
//! This is the central crate of CESS tightly-coupled pallets.

extern crate alloc;

// Re-export
use utils::{attestation, constants};

pub mod utils;

pub mod mq;
pub mod registry;

use frame_support::traits::LockableCurrency;
use frame_system::pallet_prelude::BlockNumberFor;

/// The unified config of the compute pallets
pub trait CesConfig: frame_system::Config {
	type Currency: LockableCurrency<Self::AccountId, Moment = BlockNumberFor<Self>>;
}
/// The unified type Balance of pallets from the runtime T.
#[allow(unused)]
type BalanceOf<T> = <<T as CesConfig>::Currency as frame_support::traits::Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;
/// The unified type ImBalance of pallets from the runtime T.
#[allow(unused)]
type NegativeImbalanceOf<T> = <<T as CesConfig>::Currency as frame_support::traits::Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

// Alias
pub use mq as pallet_mq;
pub use registry as pallet_registry;

#[cfg(feature = "native")]
use sp_core::hashing;

#[cfg(not(feature = "native"))]
use sp_io::hashing;
