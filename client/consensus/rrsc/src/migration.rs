use crate::{
	AuthorityId, RRSCAuthorityWeight, RRSCEpochConfiguration, RRSCGenesisConfiguration, Epoch,
	NextEpochDescriptor, VRF_OUTPUT_LENGTH,
};
use codec::{Decode, Encode};
use sc_consensus_epochs::Epoch as EpochT;
use sp_consensus_slots::Slot;

/// RRSC epoch information, version 0.
#[derive(Decode, Encode, PartialEq, Eq, Clone, Debug)]
pub struct EpochV0 {
	/// The epoch index.
	pub epoch_index: u64,
	/// The starting slot of the epoch.
	pub start_slot: Slot,
	/// The duration of this epoch.
	pub duration: u64,
	/// The authorities and their weights.
	pub authorities: Vec<(AuthorityId, RRSCAuthorityWeight)>,
	/// Randomness for this epoch.
	pub randomness: [u8; VRF_OUTPUT_LENGTH],
}

impl EpochT for EpochV0 {
	type NextEpochDescriptor = NextEpochDescriptor;
	type Slot = Slot;

	fn increment(&self, descriptor: NextEpochDescriptor) -> EpochV0 {
		EpochV0 {
			epoch_index: self.epoch_index + 1,
			start_slot: self.start_slot + self.duration,
			duration: self.duration,
			authorities: descriptor.authorities,
			randomness: descriptor.randomness,
		}
	}

	fn start_slot(&self) -> Slot {
		self.start_slot
	}

	fn end_slot(&self) -> Slot {
		self.start_slot + self.duration
	}
}

impl EpochV0 {
	/// Migrate the sturct to current epoch version.
	pub fn migrate(self, config: &RRSCGenesisConfiguration) -> Epoch {
		Epoch {
			epoch_index: self.epoch_index,
			start_slot: self.start_slot,
			duration: self.duration,
			authorities: self.authorities,
			randomness: self.randomness,
			config: RRSCEpochConfiguration { c: config.c, allowed_slots: config.allowed_slots },
		}
	}
}
