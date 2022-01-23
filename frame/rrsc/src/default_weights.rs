//! Default weights for the RRSC Pallet
//! This file was not auto-generated.

use frame_support::weights::{
	constants::{RocksDbWeight as DbWeight, WEIGHT_PER_MICROS, WEIGHT_PER_NANOS},
	Weight,
};

impl crate::WeightInfo for () {
	fn plan_config_change() -> Weight {
		DbWeight::get().writes(1)
	}

	fn report_equivocation(validator_count: u32) -> Weight {
		// we take the validator set count from the membership proof to
		// calculate the weight but we set a floor of 100 validators.
		let validator_count = validator_count.max(100) as u64;

		// worst case we are considering is that the given offender
		// is backed by 200 nominators
		const MAX_NOMINATORS: u64 = 200;

		// checking membership proof
		(35 * WEIGHT_PER_MICROS)
			.saturating_add((175 * WEIGHT_PER_NANOS).saturating_mul(validator_count))
			.saturating_add(DbWeight::get().reads(5))
			// check equivocation proof
			.saturating_add(110 * WEIGHT_PER_MICROS)
			// report offence
			.saturating_add(110 * WEIGHT_PER_MICROS)
			.saturating_add(25 * WEIGHT_PER_MICROS * MAX_NOMINATORS)
			.saturating_add(DbWeight::get().reads(14 + 3 * MAX_NOMINATORS))
			.saturating_add(DbWeight::get().writes(10 + 3 * MAX_NOMINATORS))
	}
}
