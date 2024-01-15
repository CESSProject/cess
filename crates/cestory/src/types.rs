use parity_scale_codec::{Decode, Encode, Error as CodecError};
use std::fmt::Debug;
use thiserror::Error;
use tokio::sync::oneshot;

extern crate runtime as chain;

pub type KeyfairyReadySender = oneshot::Sender<ces_pdp::Keys>;
pub type KeyfairyReadyReceiver = oneshot::Receiver<ces_pdp::Keys>;

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
    #[error("Ceseal system not ready")]
    SystemNotReady,

    #[error("Keyfairy not ready")]
    KeyfairyNotReady,

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
