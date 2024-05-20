use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
	StartCesealFailed(String),
    RedirectCesealLogFailed(String),
    DetectCesealRunningStatueFailed(String),
    PreviousVersionFailed(String)
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Error::StartCesealFailed(e) => write!(f, "{:?}", e),
            Error::RedirectCesealLogFailed(e) => write!(f, "{:?}", e),
            Error::DetectCesealRunningStatueFailed(e) => write!(f, "{:?}", e),
			Error::PreviousVersionFailed(e) => write!(f, "{:?}", e),
		}
	}
}

impl error::Error for Error {}
