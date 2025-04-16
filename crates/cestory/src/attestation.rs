use ces_types::AttestationProvider;
use parity_scale_codec::{Decode, Encode};
use std::time::{Duration, SystemTime};
use tracing::{error, info};

#[derive(thiserror::Error, Debug)]
#[error("failed to create attestation report: {source_msg}")]
pub struct Error {
    pub source_msg: String,
    pub retries: u32,
}

#[derive(Debug, Encode, Decode, Default, Clone)]
pub struct AttestationInfo {
    pub provider: String,
    pub encoded_report: Vec<u8>,
    pub timestamp: u64,
}

impl AttestationInfo {
    pub fn is_attestation_expired(&self, expired_duration_secs: Option<u64>) -> bool {
        const MAX_ATTESTATION_AGE: u64 = 60 * 60; // default expired duration: 60mins
        let eds = expired_duration_secs.unwrap_or(MAX_ATTESTATION_AGE);
        crate::unix_now() > self.timestamp + eds
    }
}

pub fn create_attestation_report_on<RA: pal::RA>(
    ra: &RA,
    attestation_provider: Option<AttestationProvider>,
    data: &[u8],
    timeout: Duration,
    max_retries: u32,
) -> Result<AttestationInfo, Error> {
    let mut tried = 0;
    let encoded_report = loop {
        break match ra.create_attestation_report(attestation_provider, data, timeout) {
            Ok(r) => r,
            Err(e) => {
                let message = format!("Failed to create attestation report: {e:?}");
                error!("{}", message);
                if tried >= max_retries {
                    return Err(Error { source_msg: format!("{:?}", e), retries: tried });
                }
                let sleep_secs = (1 << tried).min(8);
                info!("Retrying after {} seconds...", sleep_secs);
                std::thread::sleep(Duration::from_secs(sleep_secs));
                tried += 1;
                continue;
            },
        };
    };
    Ok(AttestationInfo {
        provider: serde_json::to_string(&attestation_provider).unwrap(),
        encoded_report,
        timestamp: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
    })
}
