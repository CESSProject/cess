use super::{*, MinerItems as NewMinerItems};
use sp_runtime::{TryRuntimeError, Saturating};
#[cfg(feature = "try-runtime")]
use sp_std::collections::btree_map::BTreeMap;

use frame_support::{
	storage_alias, weights::WeightMeter, Blake2_128Concat, DebugNoBound, DefaultNoBound,
};

#[derive(Encode, Decode, MaxEncodedLen, DefaultNoBound)]
pub struct Migration<T: Config> {
	last_key: Option<T::AccountId>,
}

impl<T: Config> MigrationStep for Migration<T> {
	const VERSION: u16 = 1;

	fn max_step_weight() -> Weight {
		<T as pallet::Config>::WeightInfo::v03_migration_step()
	}

	fn step(
		&mut self, 
		meter: &mut WeightMeter
	) -> IsFinished {
		let mut iter = if let Some(last_key) = self.last_key.take() {
			MinerItems::<T>::iter_from(MinerItems::<T>::hashed_key_for(last_key))
		} else {
			MinerItems::<T>::iter()
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
			NewMinerItems::<T>::insert(&last_key, miner_info);
			meter.consume(<T as pallet::Config>::WeightInfo::v03_migration_step());
			self.last_key = Some(last_key); // Return the processed key as the new cursor.
			return IsFinished::No;
		} else {
			meter.consume(<T as pallet::Config>::WeightInfo::v03_migration_step());
			return IsFinished::Yes;
		}
	}
}

#[storage_alias]
pub type MinerItems<T: Config> = StorageMap<Pallet<T>, Blake2_128Concat, AccountOf<T>, OldMinerInfo<T>>;

#[derive(Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
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
