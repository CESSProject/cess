use crate::system::{CesealMasterKey, WorkerIdentityKey};
use ces_types::WorkerRole;
use parity_scale_codec::{Decode, Encode, Error as CodecError};
use std::{
    fmt::{self, Debug},
    sync::{Arc, Mutex},
};
use thiserror::Error;
use threadpool::ThreadPool;
use tokio::sync::{mpsc, oneshot};

extern crate runtime as chain;

pub type ThreadPoolSafeBox = Arc<Mutex<ThreadPool>>;

#[derive(Clone)]
pub struct CesealProperties {
    pub role: WorkerRole,
    pub master_key: CesealMasterKey,
    pub identity_key: WorkerIdentityKey,
    pub cores: u32,
}

impl fmt::Debug for CesealProperties {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CesealProperties {{ role: {:?}, master_key: <omitted>, identity_key: <omitted>, cores: {} }}",
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

    #[error(transparent)]
    DecodeError(#[from] CodecError),

    #[error("{:?}", self)]
    PersistentRuntimeNotFound,

    #[error("external server already started")]
    ExternalServerAlreadyServing,

    #[error("external server already closed")]
    ExternalServerAlreadyClosed,

    #[error("unseal error on load_runtime_data()")]
    UnsealOnLoad,

    #[error("{0}")]
    Anyhow(anyhow::Error),
}

pub type ExpertCmdSender = mpsc::Sender<ExpertCmd>;
pub type ExpertCmdReceiver = mpsc::Receiver<ExpertCmd>;

pub enum ExpertCmd {
    ChainStorage(oneshot::Sender<Option<super::ChainStorageReadBox>>),
    EgressMessage,
}
