use crate::{AccountOf, Config, Pallet, Weight};
use codec::{Decode, Encode};
use frame_support::{
	storage_alias,
	pallet_prelude::*,
	traits::{Get},
};
use frame_support::traits::OnRuntimeUpgrade;

/// A struct that does not migration, but only checks that the counter prefix exists and is correct.
pub struct MigrationSminer<T: crate::Config>(sp_std::marker::PhantomData<T>);
impl<T: crate::Config> OnRuntimeUpgrade for TestMigrationFileBank<T> {
	fn on_runtime_upgrade() -> Weight {
		migrate::<T>()
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		log::info!("🙋🏽‍file-bank check access");
		return Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		let weights = migrate::<T>();
		return Ok(())
	}
}

