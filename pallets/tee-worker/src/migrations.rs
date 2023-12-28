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
pub struct MigrationTee<T: crate::Config>(sp_std::marker::PhantomData<T>);
impl<T: crate::Config> OnRuntimeUpgrade for MigrationTee<T> {
	fn on_runtime_upgrade() -> Weight {
		migrate::<T>()
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
		log::info!("🙋🏽tee worker check access");
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
    use crate::{TeeWorkerMap, PeerId, AccountOf, EndPoint, TeeWorkerInfo as NewTeeWorkerInfo, TeeType};

    #[derive(Decode, Encode)]
    pub struct OldTeeWorkerInfo<T: Config> {
        pub controller_account: AccountOf<T>,
        pub peer_id: PeerId,
        pub stash_account: AccountOf<T>,
        pub end_point: EndPoint,
    }

    pub fn migrate<T: Config>() -> Weight {
        let mut weight: Weight = Weight::zero();
        weight = weight.saturating_add(T::DbWeight::get().reads_writes(1, 1));

        TeeWorkerMap::<T>::translate(|tee, old_info: OldTeeWorkerInfo<T>| {
            Some(NewTeeWorkerInfo::<T>{
                worker_account: tee,
                peer_id: old_info.peer_id,
                bond_stash: Some(old_info.stash_account),
                end_point: old_info.end_point,
                tee_type: TeeType::Full,
            })
        });

        weight 
    }
}