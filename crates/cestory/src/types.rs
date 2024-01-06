use parity_scale_codec::{Decode, Encode, Error as CodecError};
use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};
use thiserror::Error;

extern crate runtime as chain;

// supportive

pub struct BlockDispatchContext<'a> {
    /// The block number.
    pub block_number: chain::BlockNumber,
    /// The timestamp of this block.
    pub now_ms: u64,
    /// The storage snapshot after this block executed.
    pub storage: Arc<Mutex<crate::ChainStorage>>,
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
#[error("{:?}", self)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    IoError(#[from] anyhow::Error),
    DecodeError(#[from] CodecError),
    PersistentRuntimeNotFound,
}
