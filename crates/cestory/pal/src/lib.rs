//! Platform abstraction layer for Trusted Execution Environments

use ces_types::AttestationProvider;
use core::time::Duration;
use std::{fmt::Debug, path::Path};

pub use ces_types::{attestation::ExtendMeasurement, MemoryUsage};

pub trait ErrorType: Debug + Into<anyhow::Error> {}
impl<T: Debug + Into<anyhow::Error>> ErrorType for T {}

pub trait Sealing {
    type SealError: ErrorType;
    type UnsealError: ErrorType;

    fn seal_data(&self, path: impl AsRef<Path>, data: &[u8]) -> Result<(), Self::SealError>;
    fn unseal_data(&self, path: impl AsRef<Path>) -> Result<Option<Vec<u8>>, Self::UnsealError>;
}

pub trait RA {
    type Error: ErrorType;
    fn create_attestation_report(
        &self,
        provider: Option<AttestationProvider>,
        data: &[u8],
        timeout: Duration,
    ) -> Result<Vec<u8>, Self::Error>;
    fn quote_test(&self, provider: Option<AttestationProvider>) -> Result<(), Self::Error>;
    fn mr_enclave(&self) -> Option<Vec<u8>>;
    fn extend_measurement(&self) -> Result<ExtendMeasurement, Self::Error>;
}

pub trait MemoryStats {
    fn memory_usage(&self) -> MemoryUsage;
}

pub trait Machine {
    fn machine_id(&self) -> Vec<u8>;
    fn cpu_core_num(&self) -> u32;
    fn cpu_feature_level(&self) -> u32;
}

pub struct AppVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

pub trait AppInfo {
    fn app_version() -> AppVersion;
}

pub trait SgxEnvAware {
    fn is_sgx_env() -> bool;
}

pub trait Platform: Sealing + RA + Machine + MemoryStats + AppInfo + SgxEnvAware + Clone + Send + 'static {}
impl<T: Sealing + RA + Machine + MemoryStats + AppInfo + SgxEnvAware + Clone + Send + 'static> Platform for T {}
