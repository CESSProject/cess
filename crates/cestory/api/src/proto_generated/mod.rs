#[allow(clippy::derive_partial_eq_without_eq, clippy::let_unit_value)]

mod ceseal_rpc;

#[cfg(test)]
mod tests;

pub use ceseal_rpc::*;

pub const PROTO_DEF: &str = include_str!("../../proto/ceseal_rpc.proto");

/// Helper struct used to compat the output of `get_info` for logging.
#[derive(Debug)]
pub struct Info<'a> {
    pub reg: bool,
    pub hdr: u32,
    pub blk: u32,
    pub dev: bool,
    pub msgs: u64,
    pub ver: &'a str,
    pub git: &'a str,
    pub rmem: u64,
    pub mpeak: u64,
    pub rpeak: u64,
    pub rspike: u64,
    pub mfree: u64,
    pub gblk: u32,
}

impl CesealInfo {
    pub fn debug_info(&self) -> Info {
        let mem = self.memory_usage.clone().unwrap_or_default();
        Info {
            reg: self.system.as_ref().map(|s| s.registered).unwrap_or(false),
            hdr: self.headernum,
            blk: self.blocknum,
            dev: self.dev_mode,
            msgs: self.pending_messages,
            ver: &self.version,
            git: &self.git_revision[0..8],
            rmem: mem.rust_used,
            rpeak: mem.rust_peak_used,
            rspike: mem.rust_spike,
            mpeak: mem.total_peak_used,
            mfree: mem.free,
            gblk: self.system.as_ref().map(|s| s.genesis_block).unwrap_or(0),
        }
    }

    pub fn public_key(&self) -> Option<&str> {
        self.system.as_ref().map(|s| s.public_key.as_str())
    }
}

use crate::crpc::*;
use ::alloc::vec::Vec;
use ::parity_scale_codec::{Decode, Encode, Error as ScaleDecodeError};

impl crate::crpc::HeadersToSync {
    pub fn decode_headers(&self) -> Result<crate::blocks::HeadersToSync, ScaleDecodeError> {
        Decode::decode(&mut &self.encoded_headers[..])
    }
    pub fn decode_authority_set_change(
        &self,
    ) -> Result<Option<crate::blocks::AuthoritySetChange>, ScaleDecodeError> {
        self.encoded_authority_set_change
            .as_ref()
            .map(|v| Decode::decode(&mut &v[..]))
            .transpose()
    }
    pub fn new(
        headers: crate::blocks::HeadersToSync,
        authority_set_change: Option<crate::blocks::AuthoritySetChange>,
    ) -> Self {
        Self {
            encoded_headers: headers.encode(),
            encoded_authority_set_change: authority_set_change.map(|x| x.encode()),
        }
    }
}
impl crate::crpc::Blocks {
    pub fn decode_blocks(
        &self,
    ) -> Result<Vec<crate::blocks::BlockHeaderWithChanges>, ScaleDecodeError> {
        Decode::decode(&mut &self.encoded_blocks[..])
    }
    pub fn new(blocks: Vec<crate::blocks::BlockHeaderWithChanges>) -> Self {
        Self {
            encoded_blocks: blocks.encode(),
        }
    }
}
impl crate::crpc::InitRuntimeRequest {
    pub fn decode_genesis_info(&self) -> Result<crate::blocks::GenesisBlockInfo, ScaleDecodeError> {
        Decode::decode(&mut &self.encoded_genesis_info[..])
    }
    pub fn decode_genesis_state(&self) -> Result<crate::blocks::StorageState, ScaleDecodeError> {
        Decode::decode(&mut &self.encoded_genesis_state[..])
    }
    pub fn decode_operator(&self) -> Result<Option<chain::AccountId>, ScaleDecodeError> {
        self.encoded_operator
            .as_ref()
            .map(|v| Decode::decode(&mut &v[..]))
            .transpose()
    }
    pub fn decode_attestation_provider(
        &self,
    ) -> Result<Option<ces_types::AttestationProvider>, ScaleDecodeError> {
        self.attestation_provider
            .as_ref()
            .map(|v| Decode::decode(&mut &v[..]))
            .transpose()
    }
    pub fn new(
        skip_ra: bool,
        genesis_info: crate::blocks::GenesisBlockInfo,
        debug_set_key: Option<::prost::alloc::vec::Vec<u8>>,
        genesis_state: crate::blocks::StorageState,
        operator: Option<chain::AccountId>,
        attestation_provider: Option<ces_types::AttestationProvider>,
    ) -> Self {
        Self {
            skip_ra,
            encoded_genesis_info: genesis_info.encode(),
            debug_set_key,
            encoded_genesis_state: genesis_state.encode(),
            encoded_operator: operator.map(|x| x.encode()),
            attestation_provider: attestation_provider.map(|x| x.encode()),
        }
    }
}
impl crate::crpc::GetRuntimeInfoRequest {
    pub fn decode_operator(&self) -> Result<Option<chain::AccountId>, ScaleDecodeError> {
        self.encoded_operator
            .as_ref()
            .map(|v| Decode::decode(&mut &v[..]))
            .transpose()
    }
    pub fn new(force_refresh_ra: bool, operator: Option<chain::AccountId>) -> Self {
        Self {
            force_refresh_ra,
            encoded_operator: operator.map(|x| x.encode()),
        }
    }
}
impl crate::crpc::InitRuntimeResponse {
    pub fn decode_runtime_info(
        &self,
    ) -> Result<ces_types::WorkerRegistrationInfo<chain::AccountId>, ScaleDecodeError> {
        Decode::decode(&mut &self.encoded_runtime_info[..])
    }
    pub fn decode_genesis_block_hash(&self) -> Result<chain::Hash, ScaleDecodeError> {
        Decode::decode(&mut &self.encoded_genesis_block_hash[..])
    }
    pub fn decode_public_key(&self) -> Result<ces_types::WorkerPublicKey, ScaleDecodeError> {
        Decode::decode(&mut &self.encoded_public_key[..])
    }
    pub fn decode_ecdh_public_key(&self) -> Result<ces_types::EcdhPublicKey, ScaleDecodeError> {
        Decode::decode(&mut &self.encoded_ecdh_public_key[..])
    }
    pub fn new(
        runtime_info: ces_types::WorkerRegistrationInfo<chain::AccountId>,
        genesis_block_hash: chain::Hash,
        public_key: ces_types::WorkerPublicKey,
        ecdh_public_key: ces_types::EcdhPublicKey,
        attestation: Option<Attestation>,
    ) -> Self {
        Self {
            encoded_runtime_info: runtime_info.encode(),
            encoded_genesis_block_hash: genesis_block_hash.encode(),
            encoded_public_key: public_key.encode(),
            encoded_ecdh_public_key: ecdh_public_key.encode(),
            attestation,
        }
    }
}
impl crate::crpc::GetEgressMessagesResponse {
    pub fn decode_messages(&self) -> Result<EgressMessages, ScaleDecodeError> {
        Decode::decode(&mut &self.encoded_messages[..])
    }
    pub fn new(messages: EgressMessages) -> Self {
        Self {
            encoded_messages: messages.encode(),
        }
    }
}
impl crate::crpc::Certificate {
    pub fn decode_body(&self) -> Result<crate::crypto::CertificateBody, ScaleDecodeError> {
        Decode::decode(&mut &self.encoded_body[..])
    }
    pub fn new(
        body: crate::crypto::CertificateBody,
        signature: Option<::prost::alloc::boxed::Box<Signature>>,
    ) -> Self {
        Self {
            encoded_body: body.encode(),
            signature,
        }
    }
}
impl crate::crpc::HandoverChallenge {
    pub fn decode_challenge(
        &self,
    ) -> Result<ces_types::HandoverChallenge<chain::BlockNumber>, ScaleDecodeError> {
        Decode::decode(&mut &self.encoded_challenge[..])
    }
    pub fn new(challenge: ces_types::HandoverChallenge<chain::BlockNumber>) -> Self {
        Self {
            encoded_challenge: challenge.encode(),
        }
    }
}
impl crate::crpc::HandoverChallengeResponse {
    pub fn decode_challenge_handler(
        &self,
    ) -> Result<ces_types::ChallengeHandlerInfo<chain::BlockNumber>, ScaleDecodeError> {
        Decode::decode(&mut &self.encoded_challenge_handler[..])
    }
    pub fn new(
        challenge_handler: ces_types::ChallengeHandlerInfo<chain::BlockNumber>,
        attestation: Option<Attestation>,
    ) -> Self {
        Self {
            encoded_challenge_handler: challenge_handler.encode(),
            attestation,
        }
    }
}
impl crate::crpc::HandoverWorkerKey {
    pub fn decode_worker_key(&self) -> Result<ces_types::EncryptedWorkerKey, ScaleDecodeError> {
        Decode::decode(&mut &self.encoded_worker_key[..])
    }
    pub fn new(
        worker_key: ces_types::EncryptedWorkerKey,
        attestation: Option<Attestation>,
    ) -> Self {
        Self {
            encoded_worker_key: worker_key.encode(),
            attestation,
        }
    }
}

impl crate::crpc::SetEndpointRequest {
    pub fn new(
        endpoint: ::prost::alloc::string::String,
    ) -> Self {
        Self {
            endpoint,
        }
    }
}

impl crate::crpc::GetEndpointResponse {
    pub fn decode_endpoint_payload(
        &self,
    ) -> Result<Option<ces_types::WorkerEndpointPayload>, ScaleDecodeError> {
        self.encoded_endpoint_payload
            .as_ref()
            .map(|v| Decode::decode(&mut &v[..]))
            .transpose()
    }
    pub fn new(
        endpoint_payload: Option<ces_types::WorkerEndpointPayload>,
        signature: Option<::prost::alloc::vec::Vec<u8>>,
    ) -> Self {
        Self {
            encoded_endpoint_payload: endpoint_payload.map(|x| x.encode()),
            signature,
        }
    }
}

impl crate::crpc::ChainState {
    pub fn decode_state(&self) -> Result<crate::blocks::StorageState, ScaleDecodeError> {
        Decode::decode(&mut &self.encoded_state[..])
    }
    pub fn new(block_number: u32, state: crate::blocks::StorageState) -> Self {
        Self {
            block_number,
            encoded_state: state.encode(),
        }
    }
}
