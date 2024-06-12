use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    BlockHashNotFound,
    BlockNotFound,
    SearchSetIdChangeInEmptyRange,
    FailedToCallRegisterWorker,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BlockHashNotFound => write!(f, "block hash not found"),
            Error::BlockNotFound => write!(f, "block not found"),
            Error::SearchSetIdChangeInEmptyRange => write!(f, "list of known blocks is empty"),
            Error::FailedToCallRegisterWorker => write!(f, "failed to call register_worker"),
        }
    }
}

impl error::Error for Error {}
