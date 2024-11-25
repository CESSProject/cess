use super::*;
use frame_support::traits::OnRuntimeUpgrade;
use sp_runtime::{TryRuntimeError, Saturating};
#[cfg(feature = "try-runtime")]
use sp_std::collections::btree_map::BTreeMap;

use frame_support::{
	storage_alias, weights::WeightMeter, Blake2_128Concat,
	migrations::{MigrationId, SteppedMigration, SteppedMigrationError},
};

pub const PALLET_MIGRATIONS_ID: &[u8; 23] = b"pallet-sminer-migration";
pub struct SteppedSminer<T: Config, W: weights::WeightInfo>(PhantomData<(T, W)>);

impl<T: Config, W: weights::WeightInfo> SteppedMigration for SteppedSminer<T, W> {
	type Cursor = T::AccountId;

	type Identifier = MigrationId<23>;

	fn id() -> Self::Identifier {
		MigrationId { pallet_id: *PALLET_MIGRATIONS_ID, version_from: 1, version_to: 2 }
	}

	fn step(
		mut cursor: Option<Self::Cursor>, 
		meter: &mut WeightMeter
	) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
		let required = W::migration_step();

		if meter.remaining().any_lt(required) {
			return Err(SteppedMigrationError::InsufficientWeight { required });
		}

		loop {
			if meter.try_consume(required).is_err() {
				break;
			}

			let mut iter = if let Some(last_key) = cursor {
				v2::MinerItems::<T>::iter_from(v2::MinerItems::<T>::hashed_key_for(last_key))
			} else {
				v2::MinerItems::<T>::iter()
			};

			if let Some((last_key, value)) = iter.next() {
				// We can just insert here since the old and the new map share the same key-space.
				// Otherwise it would have to invert the concat hash function and re-hash it.
				let miner_info = MinerInfo::<T>{
					beneficiary: value.beneficiary,
					staking_account: value.staking_account,
					endpoint: Default::default(),
					collaterals: value.collaterals,
					debt: value.debt,
					state: value.state,
					declaration_space: value.declaration_space,
					idle_space: value.idle_space,
					service_space: value.service_space,
					lock_space: value.lock_space,
					space_proof_info: value.space_proof_info,
					service_bloom_filter: value.service_bloom_filter,
					tee_signature: value.tee_signature,
				};
				MinerItems::<T>::insert(&last_key, miner_info);

				cursor = Some(last_key) // Return the processed key as the new cursor.
			} else {
				cursor = None; // Signal that the migration is complete (no more items to process).
				break
			}
		}

		Ok(cursor)
	}
}

// pub struct MigrationSminer<T: crate::Config>(sp_std::marker::PhantomData<T>);
// impl<T: crate::Config> OnRuntimeUpgrade for MigrationSminer<T> {
// 	fn on_runtime_upgrade() -> Weight {
// 		migrate::<T>()
// 	}

// 	#[cfg(feature = "try-runtime")]
// 	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
// 		log::info!("sminer check access");
// 		return Ok(Default::default())
// 	}

// 	#[cfg(feature = "try-runtime")]
// 	fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
// 		let weights = migrate::<T>();
// 		return Ok(())
// 	}
// }

// pub fn migrate<T: Config>() -> Weight {
// 	use frame_support::traits::StorageVersion;

// 	let version = StorageVersion::get::<Pallet<T>>();
// 	let mut weight: Weight = Weight::zero();

// 	if version < 2 {
// 		weight = weight.saturating_add(v2_bkp::migrate::<T>());
//         StorageVersion::new(2).put::<Pallet<T>>();
// 	}

// 	weight
// }

pub mod v2 {
	use super::{*, MinerItems as NewMinerItems};

	#[storage_alias]
	pub type MinerItems<T: Config> = StorageMap<Pallet<T>, Blake2_128Concat, AccountOf<T>, OldMinerInfo<T>>;

	#[derive(Encode, Decode, RuntimeDebug, TypeInfo)]
    pub struct OldMinerInfo<T: Config> {
        pub beneficiary: AccountOf<T>,
        pub staking_account: AccountOf<T>,
        pub peer_id: PeerId,
        pub collaterals: BalanceOf<T>,
        pub debt: BalanceOf<T>,
        pub state: BoundedVec<u8, T::ItemLimit>,
        pub declaration_space: u128,
        pub idle_space: u128,
        pub service_space: u128,
        pub lock_space: u128,
        pub space_proof_info: Option<SpaceProofInfo<AccountOf<T>>>,
        pub service_bloom_filter: BloomFilter,
        pub tee_signature: TeeSig,
    }
}

mod v2_bkp {
    use super::*;

    #[derive(Decode)]
    pub struct OldMinerInfo<T: Config> {
        pub beneficiary: AccountOf<T>,
        pub staking_account: AccountOf<T>,
        pub peer_id: PeerId,
        pub collaterals: BalanceOf<T>,
        pub debt: BalanceOf<T>,
        pub state: BoundedVec<u8, T::ItemLimit>,
        pub declaration_space: u128,
        pub idle_space: u128,
        pub service_space: u128,
        pub lock_space: u128,
        pub space_proof_info: Option<SpaceProofInfo<AccountOf<T>>>,
        pub service_bloom_filter: BloomFilter,
        pub tee_signature: TeeSig,
    }

    pub fn migrate<T: Config>() -> Weight {
		let mut weight: Weight = Weight::zero();
		let mut translated = 0u64;

		log::info!("File-Bank migrate start");

		MinerItems::<T>::translate::<OldMinerInfo<T>, _>(|_key, old_value| {
			translated.saturating_inc();

			Some(MinerInfo::<T>{
				beneficiary: old_value.beneficiary,
                staking_account: old_value.staking_account,
                endpoint: Default::default(),
                collaterals: old_value.collaterals,
                debt: old_value.debt,
                state: old_value.state,
                declaration_space: old_value.declaration_space,
                idle_space: old_value.idle_space,
                service_space: old_value.service_space,
                lock_space: old_value.lock_space,
                space_proof_info: old_value.space_proof_info,
                service_bloom_filter: old_value.service_bloom_filter,
                tee_signature: old_value.tee_signature,
			})
		});

		log::info!(
			"Upgraded {} pools, storage to version {}",
			translated,
			2,
		);

		T::DbWeight::get().reads_writes(translated + 1, translated + 1)
	}
}