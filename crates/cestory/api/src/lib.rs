extern crate alloc;

pub mod chain_client;

pub mod pois {
    tonic::include_proto!("pois");
}

#[allow(non_camel_case_types)]
pub mod podr2 {
    tonic::include_proto!("podr2");
}

pub mod pubkeys {
    tonic::include_proto!("ceseal.pubkeys");
}

pub mod handover {
    tonic::include_proto!("handover_api");

    use super::chain_client::BlockNumber;
    use parity_scale_codec::{Decode, Encode, Error as ScaleDecodeError};

    impl HandoverChallenge {
        pub fn decode_challenge(&self) -> Result<ces_types::HandoverChallenge<BlockNumber>, ScaleDecodeError> {
            Decode::decode(&mut &self.encoded_challenge[..])
        }
        pub fn new(challenge: ces_types::HandoverChallenge<BlockNumber>) -> Self {
            Self { encoded_challenge: challenge.encode() }
        }
    }

    impl HandoverChallengeResponse {
        pub fn decode_challenge_handler(
            &self,
        ) -> Result<ces_types::ChallengeHandlerInfo<BlockNumber>, ScaleDecodeError> {
            Decode::decode(&mut &self.encoded_challenge_handler[..])
        }
        pub fn new(
            challenge_handler: ces_types::ChallengeHandlerInfo<BlockNumber>,
            attestation: Option<Attestation>,
        ) -> Self {
            Self { encoded_challenge_handler: challenge_handler.encode(), attestation }
        }
    }

    impl HandoverWorkerKey {
        pub fn decode_worker_key(&self) -> Result<ces_types::EncryptedWorkerKey, ScaleDecodeError> {
            Decode::decode(&mut &self.encoded_worker_key[..])
        }
        pub fn new(worker_key: ces_types::EncryptedWorkerKey, attestation: Option<Attestation>) -> Self {
            Self { encoded_worker_key: worker_key.encode(), attestation }
        }
    }
}
