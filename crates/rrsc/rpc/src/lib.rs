// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! RPC api for rrsc.

use std::{collections::HashMap, sync::Arc};

use futures::TryFutureExt;
use jsonrpsee::{
	core::async_trait,
	proc_macros::rpc,
	types::{ErrorObject, ErrorObjectOwned},
	Extensions,
};
use serde::{Deserialize, Serialize};

use cessc_consensus_rrsc::{authorship, RRSCWorkerHandle};
use sc_consensus_epochs::Epoch as EpochT;
use sc_rpc_api::{check_if_safe, UnsafeRpcError};
use sp_api::ProvideRuntimeApi;
use sp_application_crypto::AppCrypto;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_consensus::{Error as ConsensusError, SelectChain};
use cessp_consensus_rrsc::{digests::PreDigest, AuthorityId, RRSCApi as RRSCRuntimeApi};
use sp_core::crypto::ByteArray;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::{Block as BlockT, Header as _};

const RRSC_ERROR: i32 = 9000;

/// Provides rpc methods for interacting with RRSC.
#[rpc(client, server)]
pub trait RRSCApi {
	/// Returns data about which slots (primary or secondary) can be claimed in the current epoch
	/// with the keys in the keystore.
	#[method(name = "rrsc_epochAuthorship", with_extensions)]
	async fn epoch_authorship(&self) -> Result<HashMap<AuthorityId, EpochAuthorship>, Error>;
}

/// Provides RPC methods for interacting with RRSC.
pub struct RRSC<B: BlockT, C, SC> {
	/// shared reference to the client.
	client: Arc<C>,
	/// A handle to the RRSC worker for issuing requests.
	rrsc_worker_handle: RRSCWorkerHandle<B>,
	/// shared reference to the Keystore
	keystore: KeystorePtr,
	/// The SelectChain strategy
	select_chain: SC,
}

impl<B: BlockT, C, SC> RRSC<B, C, SC> {
	/// Creates a new instance of the RRSC Rpc handler.
	pub fn new(
		client: Arc<C>,
		rrsc_worker_handle: RRSCWorkerHandle<B>,
		keystore: KeystorePtr,
		select_chain: SC,
	) -> Self {
		Self { client, rrsc_worker_handle, keystore, select_chain }
	}
}

#[async_trait]
impl<B: BlockT, C, SC> RRSCApiServer for RRSC<B, C, SC>
where
	B: BlockT,
	C: ProvideRuntimeApi<B>
		+ HeaderBackend<B>
		+ HeaderMetadata<B, Error = BlockChainError>
		+ 'static,
	C::Api: RRSCRuntimeApi<B>,
	SC: SelectChain<B> + Clone + 'static,
{
	async fn epoch_authorship(
		&self,
		ext: &Extensions,
	) -> Result<HashMap<AuthorityId, EpochAuthorship>, Error> {
		check_if_safe(ext)?;

		let best_header = self.select_chain.best_chain().map_err(Error::SelectChain).await?;

		let epoch_start = self
			.client
			.runtime_api()
			.current_epoch_start(best_header.hash())
			.map_err(|_| Error::FetchEpoch)?;

		let epoch = self
			.rrsc_worker_handle
			.epoch_data_for_child_of(best_header.hash(), *best_header.number(), epoch_start)
			.await
			.map_err(|_| Error::FetchEpoch)?;

		let (epoch_start, epoch_end) = (epoch.start_slot(), epoch.end_slot());
		let mut claims: HashMap<AuthorityId, EpochAuthorship> = HashMap::new();

		let keys = {
			epoch
				.authorities
				.iter()
				.enumerate()
				.filter_map(|(i, a)| {
					if self.keystore.has_keys(&[(a.0.to_raw_vec(), AuthorityId::ID)]) {
						Some((a.0.clone(), i))
					} else {
						None
					}
				})
				.collect::<Vec<_>>()
		};

		for slot in *epoch_start..*epoch_end {
			if let Some((claim, key)) =
				authorship::claim_slot_using_keys(slot.into(), &epoch, &self.keystore, &keys)
			{
				match claim {
					PreDigest::Primary { .. } => {
						claims.entry(key).or_default().primary.push(slot);
					},
					PreDigest::SecondaryPlain { .. } => {
						claims.entry(key).or_default().secondary.push(slot);
					},
					PreDigest::SecondaryVRF { .. } => {
						claims.entry(key).or_default().secondary_vrf.push(slot.into());
					},
				};
			}
		}

		Ok(claims)
	}
}

/// Holds information about the `slot`'s that can be claimed by a given key.
#[derive(Clone, Default, Debug, Deserialize, Serialize)]
pub struct EpochAuthorship {
	/// the array of primary slots that can be claimed
	primary: Vec<u64>,
	/// the array of secondary slots that can be claimed
	secondary: Vec<u64>,
	/// The array of secondary VRF slots that can be claimed.
	secondary_vrf: Vec<u64>,
}

/// Top-level error type for the RPC handler.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// Failed to fetch the current best header.
	#[error("Failed to fetch the current best header: {0}")]
	SelectChain(ConsensusError),
	/// Failed to fetch epoch data.
	#[error("Failed to fetch epoch data")]
	FetchEpoch,
	/// Consensus error
	#[error(transparent)]
	Consensus(#[from] ConsensusError),
	/// Errors that can be formatted as a String
	#[error("{0}")]
	StringError(String),
	/// Call to an unsafe RPC was denied.
	#[error(transparent)]
	UnsafeRpcCalled(#[from] UnsafeRpcError),
}

impl From<Error> for ErrorObjectOwned {
	fn from(error: Error) -> Self {
		match error {
			Error::SelectChain(e) => ErrorObject::owned(RRSC_ERROR + 1, e.to_string(), None::<()>),
			Error::FetchEpoch => ErrorObject::owned(RRSC_ERROR + 2, error.to_string(), None::<()>),
			Error::Consensus(e) => ErrorObject::owned(RRSC_ERROR + 3, e.to_string(), None::<()>),
			Error::StringError(e) => ErrorObject::owned(RRSC_ERROR + 4, e, None::<()>),
			Error::UnsafeRpcCalled(e) => e.into(),
		}
	}
}