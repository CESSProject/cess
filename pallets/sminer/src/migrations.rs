use crate::{AccountOf, Config, Pallet};
use codec::{Decode, Encode};
use frame_support::{
	storage_alias,
	pallet_prelude::*,
	traits::{Get},
	weights::Weight,
};
use sp_std::vec::Vec;
use frame_support::traits::OnRuntimeUpgrade;
#[cfg(feature = "try-runtime")]
use sp_runtime::TryRuntimeError;
/// A struct that does not migration, but only checks that the counter prefix exists and is correct.
pub struct MigrationSminer<T: crate::Config>(sp_std::marker::PhantomData<T>);
impl<T: crate::Config> OnRuntimeUpgrade for MigrationSminer<T> {
	fn on_runtime_upgrade() -> Weight {
		migrate::<T>()
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		log::info!("🙋🏽sminer check access");
		return Ok(Default::default())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
		let weights = migrate::<T>();
		return Ok(())
	}
}

pub fn migrate<T: Config>() -> Weight {

	let version = StorageVersion::get::<Pallet<T>>();
	let mut weight: Weight = Weight::zero();

	if version < 2 {
        weight = weight.saturating_add(v2::migrate::<T>());
        StorageVersion::new(2).put::<Pallet<T>>();
	}

	weight
}

mod v2 {
	use super::*;
	use crate::{
		SpaceProofInfo, MinerItems, MinerInfo as NewMinerInfo, PeerId,
		T_BYTE, BloomFilter, TeeRsaSignature, BalanceOf
	};

	#[derive(Decode, Encode)]
	pub struct OldMinerInfo<T: Config> {
		//Income account
		pub(super) beneficiary: AccountOf<T>,
		pub(super) peer_id: PeerId,
		pub(super) collaterals: BalanceOf<T>,
		pub(super) debt: BalanceOf<T>,
		//nomal, exit, frozen, e_frozen
		pub(super) state: BoundedVec<u8, T::ItemLimit>,
		pub(super) idle_space: u128,
		pub(super) service_space: u128,
		pub(super) lock_space: u128,
		pub(super) space_proof_info: Option<SpaceProofInfo<AccountOf<T>>>,
		pub(super) service_bloom_filter: BloomFilter,
		pub(super) tee_signature: TeeRsaSignature,
	}

	// #[storage_alias]
	// type MinerItems<T: Config> = StorageMap<Sminer, Blake2_128Concat, AccountOf<T>, OldMinerInfo<T>>;

	pub fn migrate<T: Config>() -> Weight {
		let mut weight: Weight = Weight::zero();
		log::info!("start migrate miner info");
		<MinerItems<T>>::translate(|acc, old_miner_info: OldMinerInfo<T>| {
			log::info!("miner: {:?}", acc);
			weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));
			let cur_total_space = old_miner_info.service_space + old_miner_info.idle_space + old_miner_info.lock_space;
			let mut tib_count = cur_total_space / T_BYTE;
			if cur_total_space % T_BYTE != 0 {
				tib_count += 1;
			}
			let declaration_space = T_BYTE * tib_count;
			Some(NewMinerInfo::<T>{
				beneficiary: old_miner_info.beneficiary,
				staking_account: acc,
				peer_id: old_miner_info.peer_id,
				collaterals: old_miner_info.collaterals,
				debt: old_miner_info.debt,
				state: old_miner_info.state,
				declaration_space: declaration_space,
				idle_space: old_miner_info.idle_space,
				service_space: old_miner_info.service_space,
				lock_space: old_miner_info.lock_space,
				space_proof_info: old_miner_info.space_proof_info,
				service_bloom_filter: old_miner_info.service_bloom_filter,
				tee_signature: old_miner_info.tee_signature,
			})
		});

		weight
	}
}

