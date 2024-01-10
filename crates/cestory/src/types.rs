use crate::{
    expert::ExpertCmdReceiver,
    podr2::{Podr2ApiServer, Podr2VerifierApiServer},
};
use parity_scale_codec::{Decode, Encode, Error as CodecError};
use std::{fmt::Debug, sync::mpsc};
use thiserror::Error;

extern crate runtime as chain;

pub type ExternalServiceMadeSender = mpsc::Sender<(ExpertCmdReceiver, Podr2ApiServer, Podr2VerifierApiServer)>;
pub type ExternalServiceMadeReceiver = mpsc::Receiver<(ExpertCmdReceiver, Podr2ApiServer, Podr2VerifierApiServer)>;

// supportive

pub struct BlockDispatchContext<'a> {
    /// The block number.
    pub block_number: chain::BlockNumber,
    /// The timestamp of this block.
    pub now_ms: u64,
    /// The storage snapshot after this block executed.
    pub storage: &'a crate::ChainStorage,
    /// The message queue
    pub send_mq: &'a ces_mq::MessageSendQueue,
    pub recv_mq: &'a mut ces_mq::MessageDispatcher,
}

#[derive(Encode, Decode, Debug, Clone)]
pub struct TxRef {
    pub blocknum: chain::BlockNumber,
    pub index: u64,
}

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    #[error(transparent)]
    TonicTransport(#[from] tonic::transport::Error),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[error("multi reference to chain_storage at the same time")]
    MultiRefToChainStorage,

    #[error(transparent)]
    DecodeError(#[from] CodecError),

    #[error("{:?}", self)]
    PersistentRuntimeNotFound,

    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error),
}
