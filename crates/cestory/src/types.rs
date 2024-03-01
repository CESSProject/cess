use crate::system::WorkerIdentityKey;
use ces_types::WorkerRole;
use parity_scale_codec::{Decode, Encode, Error as CodecError};
use std::{
    fmt::{self, Debug},
    sync::{Arc, Mutex},
};
use thiserror::Error;
use threadpool::ThreadPool;
use tokio::sync::oneshot;

extern crate runtime as chain;

pub type KeyfairyReadySender = oneshot::Sender<CesealProperties>;
pub type KeyfairyReadyReceiver = oneshot::Receiver<CesealProperties>;
pub type ThreadPoolSafeBox = Arc<Mutex<ThreadPool>>;
pub type MasterKey = sp_core::sr25519::Pair;

#[derive(Clone)]
pub struct CesealProperties {
    pub role: WorkerRole,
    pub podr2_key: ces_pdp::Keys,
    pub master_key: MasterKey,
    pub identity_key: WorkerIdentityKey,
    pub cores: u32,
}

impl fmt::Debug for CesealProperties {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CesealProperties {{ role: {:?}, podr2_key: <omitted>, identity_key: <omitted>, cores: {} }}",
            self.role, self.cores
        )
    }
}

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
