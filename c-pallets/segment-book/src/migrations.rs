use crate::*;
use codec::{Decode, Encode};
use frame_support::{
	codec, generate_storage_alias,
	traits::{Get},
};
use frame_support::traits::OnRuntimeUpgrade;

pub struct MigrationSegmentBook<T: crate::Config>(sp_std::marker::PhantomData<T>);
impl<T: crate::Config> OnRuntimeUpgrade for MigrationSegmentBook<T> {
	fn on_runtime_upgrade() -> Weight {
        log::info!("SegmentBook migrate start!");
		migrate::<T>()
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<(), &'static str> {
		log::info!("segment-book check access");
		return Ok(())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade() -> Result<(), &'static str> {
		let weights = migrate::<T>();
		return Ok(())
	}
}

pub fn migrate<T: Config>() -> Weight {
	let version = StorageVersion::get::<Pallet<T>>();
	let mut weight: Weight = 0;

	if version < 1 {
        log::info!("SegmentBook version 1 -> 2 migrations start!");
        weight = weight.saturating_add(v2::migrate::<T>());
        StorageVersion::new(2).put::<Pallet<T>>();
	}

	weight
}

mod v2 {
    use super::*;

    #[derive(Decode, Encode)]
    struct OldProveInfo<T: Config> {
	    file_id: Hash,
	    miner_acc: AccountOf<T>,
	    challenge_info: ChallengeInfo<T>,
	    mu: BoundedList<T>,
	    sigma: BoundedVec<u8, T::StringLimit>,
	    name: BoundedVec<u8, T::StringLimit>,
	    u: BoundedList<T>,
    }

    generate_storage_alias!(
		SegmentBook,
		UnVerifyProof<T: Config> => Map<
            (Blake2_128Concat, T::AccountId),
            BoundedVec<OldProveInfo<T>, T::ChallengeMaximum>
        >
	);

    pub fn migrate<T: Config>() -> Weight {
        let mut weight: Weight = 0;

        for (acc, prove_list) in <UnVerifyProof<T>>::iter() {
            log::info!("prove_list len is: {:?}", prove_list.len());
            <UnVerifyProof<T>>::remove(&acc);
            weight = weight.saturating_add(T::DbWeight::get().writes(1 as Weight));
        }

        weight
    }
}