use super::*;
use frame_support::traits::OnRuntimeUpgrade;
use sp_runtime::Saturating;

pub struct MigrationSminer<T: crate::Config>(sp_std::marker::PhantomData<T>);
impl<T: crate::Config> OnRuntimeUpgrade for MigrationSminer<T> {
	fn on_runtime_upgrade() -> Weight {
		migrate::<T>()
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		log::info!("sminer check access");
		return Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		let weights = migrate::<T>();
		return Ok(())
	}
}

pub fn migrate<T: Config>() -> Weight {
	use frame_support::traits::StorageVersion;

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