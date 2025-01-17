use super::*;
#[cfg(feature = "try-runtime")]
use sp_runtime::{TryRuntimeError};
#[cfg(feature = "try-runtime")]
use sp_std::collections::btree_map::BTreeMap;
#[cfg(feature = "try-runtime")]
use sp_std::vec::Vec;

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
		MigrationId { pallet_id: *PALLET_MIGRATIONS_ID, version_from: 0, version_to: 1 }
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

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		use codec::Encode;
		let miner_records: BTreeMap<_, _> = v2::MinerItems::<T>::iter()
			.take(10)
			.collect();
		Ok(miner_records.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(prev_state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let miner_state = <BTreeMap<AccountOf<T>, v2::OldMinerInfo<T>>>::decode(&mut &prev_state[..])
			.expect("Failed to decode the previous storage state");
		
		for (key, value) in miner_state {
			let miner = MinerItems::<T>::get(key)
				.expect("Migrated miner should exist");
			
			assert!(miner.idle_space == value.idle_space, "Idle space mismatch");
			assert!(miner.service_space == value.service_space, "Service space mismatch");
			assert!(miner.lock_space == value.lock_space, "Lock space mismatch");
			assert!(miner.declaration_space == value.declaration_space, "Declaration space mismatch");
			assert!(miner.collaterals == value.collaterals, "Collaterals mismatch");
			assert!(miner.debt == value.debt, "Debt mismatch");
			assert!(miner.state == value.state, "State mismatch");
			assert!(miner.space_proof_info == value.space_proof_info, "Space proof info mismatch");
			assert!(miner.service_bloom_filter == value.service_bloom_filter, "Service bloom filter mismatch");
			assert!(miner.tee_signature == value.tee_signature, "TEE signature mismatch");
			assert_eq!(miner.endpoint, EndPoint::default(), "Endpoint mismatch");
		}

		log::info!("sminer check access success: verified first 10 records of MinerItems");
		Ok(())
	}
}

pub mod v2 {
	use super::*;

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