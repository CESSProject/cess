//! Schema for RRSC epoch changes in the aux-db.

use codec::{Decode, Encode};
use log::info;

use crate::{migration::EpochV0, Epoch};
use sc_client_api::backend::AuxStore;
use sc_consensus_epochs::{migration::EpochChangesForV0, EpochChangesFor, SharedEpochChanges};
use sp_blockchain::{Error as ClientError, Result as ClientResult};
use sp_consensus_rrsc::{RRSCBlockWeight, RRSCGenesisConfiguration};
use sp_runtime::traits::Block as BlockT;

const RRSC_EPOCH_CHANGES_VERSION: &[u8] = b"rrsc_epoch_changes_version";
const RRSC_EPOCH_CHANGES_KEY: &[u8] = b"rrsc_epoch_changes";
const RRSC_EPOCH_CHANGES_CURRENT_VERSION: u32 = 2;

/// The aux storage key used to store the block weight of the given block hash.
pub fn block_weight_key<H: Encode>(block_hash: H) -> Vec<u8> {
	(b"block_weight", block_hash).encode()
}

fn load_decode<B, T>(backend: &B, key: &[u8]) -> ClientResult<Option<T>>
where
	B: AuxStore,
	T: Decode,
{
	let corrupt = |e: codec::Error| {
		ClientError::Backend(format!("RRSC DB is corrupted. Decode error: {}", e))
	};
	match backend.get_aux(key)? {
		None => Ok(None),
		Some(t) => T::decode(&mut &t[..]).map(Some).map_err(corrupt),
	}
}

/// Load or initialize persistent epoch change data from backend.
pub fn load_epoch_changes<Block: BlockT, B: AuxStore>(
	backend: &B,
	config: &RRSCGenesisConfiguration,
) -> ClientResult<SharedEpochChanges<Block, Epoch>> {
	let version = load_decode::<_, u32>(backend, RRSC_EPOCH_CHANGES_VERSION)?;

	let maybe_epoch_changes = match version {
		None =>
			load_decode::<_, EpochChangesForV0<Block, EpochV0>>(backend, RRSC_EPOCH_CHANGES_KEY)?
				.map(|v0| v0.migrate().map(|_, _, epoch| epoch.migrate(config))),
		Some(1) =>
			load_decode::<_, EpochChangesFor<Block, EpochV0>>(backend, RRSC_EPOCH_CHANGES_KEY)?
				.map(|v1| v1.map(|_, _, epoch| epoch.migrate(config))),
		Some(RRSC_EPOCH_CHANGES_CURRENT_VERSION) =>
			load_decode::<_, EpochChangesFor<Block, Epoch>>(backend, RRSC_EPOCH_CHANGES_KEY)?,
		Some(other) =>
			return Err(ClientError::Backend(format!("Unsupported RRSC DB version: {:?}", other))),
	};

	let epoch_changes =
		SharedEpochChanges::<Block, Epoch>::new(maybe_epoch_changes.unwrap_or_else(|| {
			info!(
				target: "rrsc",
				"👶 Creating empty RRSC epoch changes on what appears to be first startup.",
			);
			EpochChangesFor::<Block, Epoch>::default()
		}));

	// rebalance the tree after deserialization. this isn't strictly necessary
	// since the tree is now rebalanced on every update operation. but since the
	// tree wasn't rebalanced initially it's useful to temporarily leave it here
	// to avoid having to wait until an import for rebalancing.
	epoch_changes.shared_data().rebalance();

	Ok(epoch_changes)
}

/// Update the epoch changes on disk after a change.
pub(crate) fn write_epoch_changes<Block: BlockT, F, R>(
	epoch_changes: &EpochChangesFor<Block, Epoch>,
	write_aux: F,
) -> R
where
	F: FnOnce(&[(&'static [u8], &[u8])]) -> R,
{
	RRSC_EPOCH_CHANGES_CURRENT_VERSION.using_encoded(|version| {
		let encoded_epoch_changes = epoch_changes.encode();
		write_aux(&[
			(RRSC_EPOCH_CHANGES_KEY, encoded_epoch_changes.as_slice()),
			(RRSC_EPOCH_CHANGES_VERSION, version),
		])
	})
}

/// Write the cumulative chain-weight of a block ot aux storage.
pub(crate) fn write_block_weight<H: Encode, F, R>(
	block_hash: H,
	block_weight: RRSCBlockWeight,
	write_aux: F,
) -> R
where
	F: FnOnce(&[(Vec<u8>, &[u8])]) -> R,
{
	let key = block_weight_key(block_hash);
	block_weight.using_encoded(|s| write_aux(&[(key, s)]))
}

/// Load the cumulative chain-weight associated with a block.
pub fn load_block_weight<H: Encode, B: AuxStore>(
	backend: &B,
	block_hash: H,
) -> ClientResult<Option<RRSCBlockWeight>> {
	load_decode(backend, block_weight_key(block_hash).as_slice())
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::migration::EpochV0;
	use fork_tree::ForkTree;
	use sc_consensus_epochs::{EpochHeader, PersistedEpoch, PersistedEpochHeader};
	use sc_network_test::Block as TestBlock;
	use sp_consensus::Error as ConsensusError;
	use sp_consensus_rrsc::{AllowedSlots, RRSCGenesisConfiguration};
	use sp_core::H256;
	use sp_runtime::traits::NumberFor;
	use substrate_test_runtime_client;

	#[test]
	fn load_decode_from_v0_epoch_changes() {
		let epoch = EpochV0 {
			start_slot: 0.into(),
			authorities: vec![],
			randomness: [0; 32],
			epoch_index: 1,
			duration: 100,
		};
		let client = substrate_test_runtime_client::new();
		let mut v0_tree = ForkTree::<H256, NumberFor<TestBlock>, _>::new();
		v0_tree
			.import::<_, ConsensusError>(
				Default::default(),
				Default::default(),
				PersistedEpoch::Regular(epoch),
				&|_, _| Ok(false), // Test is single item only so this can be set to false.
			)
			.unwrap();

		client
			.insert_aux(
				&[(
					RRSC_EPOCH_CHANGES_KEY,
					&EpochChangesForV0::<TestBlock, EpochV0>::from_raw(v0_tree).encode()[..],
				)],
				&[],
			)
			.unwrap();

		assert_eq!(load_decode::<_, u32>(&client, RRSC_EPOCH_CHANGES_VERSION).unwrap(), None);

		let epoch_changes = load_epoch_changes::<TestBlock, _>(
			&client,
			&RRSCGenesisConfiguration {
				slot_duration: 10,
				epoch_length: 4,
				c: (3, 10),
				genesis_authorities: Vec::new(),
				randomness: Default::default(),
				allowed_slots: AllowedSlots::PrimaryAndSecondaryPlainSlots,
			},
		)
		.unwrap();

		assert!(
			epoch_changes
				.shared_data()
				.tree()
				.iter()
				.map(|(_, _, epoch)| epoch.clone())
				.collect::<Vec<_>>() ==
				vec![PersistedEpochHeader::Regular(EpochHeader {
					start_slot: 0.into(),
					end_slot: 100.into(),
				})],
		); // PersistedEpochHeader does not implement Debug, so we use assert! directly.

		write_epoch_changes::<TestBlock, _, _>(&epoch_changes.shared_data(), |values| {
			client.insert_aux(values, &[]).unwrap();
		});

		assert_eq!(load_decode::<_, u32>(&client, RRSC_EPOCH_CHANGES_VERSION).unwrap(), Some(2));
	}
}
