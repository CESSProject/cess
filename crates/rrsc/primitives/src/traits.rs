//! Traits for dealing with validation and validators.
use sp_std::{collections::btree_map::BTreeMap};

/// Trait used to retrieve credits of validators.
pub trait ValidatorCredits<ValidatorId> {
	/// Returns the full score.
	fn full_credit() -> u32;

	/// Returns the credits of all validators when `epoch_index`.
	fn credits(epoch_index: u64) -> BTreeMap<ValidatorId, u32>;
}

impl<ValidatorId> ValidatorCredits<ValidatorId> for () {
	fn full_credit()-> u32 {
		1000
	}

	fn credits(_epoch_index: u64) -> BTreeMap<ValidatorId, u32> {
		BTreeMap::new()
	}
}
