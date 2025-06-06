use crate::{Config, RegistrationInfo};
use anyhow::{anyhow, Result};
use ces_types::{attestation::SgxFields, AttestationProvider, AttestationReport};
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

const ATTESTATION_FILE: &str = "attestation.seal";

pub fn try_load_attestation<Platform: pal::Platform>(
    platform: &Platform,
    config: &Config,
) -> Result<Option<AttestationInfo>> {
    use std::path::PathBuf;
    let attest_file = PathBuf::from(&config.sealing_path).join(ATTESTATION_FILE);
    if let Some(data) = platform
        .unseal_data(attest_file)
        .map_err(|e| anyhow!("unseal attestation error: {:?}", e))?
    {
        let data: AttestationInfo = Decode::decode(&mut &data[..])?;
        Ok(Some(data))
    } else {
        Ok(None)
    }
}

pub fn save_attestation<Platform: pal::Platform>(
    platform: &Platform,
    attestation: &AttestationInfo,
    config: &Config,
) -> Result<()> {
    use std::path::PathBuf;
    let data = attestation.encode();
    let attest_file = PathBuf::from(&config.sealing_path).join(ATTESTATION_FILE);
    platform
        .seal_data(attest_file, &data)
        .map_err(|e| anyhow!("seal attestation error: {:?}", e))?;
    Ok(())
}

pub fn create_register_attestation_report<Platform: pal::Platform>(
    platform: &Platform,
    registration_info: &RegistrationInfo,
    config: &Config,
) -> Result<AttestationInfo> {
    let encoded_reg_info = registration_info.encode();
    let reg_info_hash = sp_core::hashing::blake2_256(&encoded_reg_info);
    debug!("Encoded registration info: {}, hash: {}", hex::encode(&encoded_reg_info), hex::encode(&reg_info_hash));
    let report = create_attestation_report_on(
        platform,
        config.attestation_provider.clone(),
        &reg_info_hash,
        config.ra_timeout,
        config.ra_max_retries,
    )?;
    {
        let mut encoded_report = &report.encoded_report[..];
        let report: Option<AttestationReport> = Decode::decode(&mut encoded_report)?;
        if let Some(report) = report {
            match report {
                AttestationReport::SgxIas { ra_report, .. } => match SgxFields::from_ias_report(&ra_report[..]) {
                    Ok((sgx_fields, _)) => {
                        info!("EPID RA report measurement       :{}", hex::encode(sgx_fields.measurement()));
                        info!("EPID RA report measurement hash  :{:?}", sgx_fields.measurement_hash());
                    },
                    Err(e) => {
                        error!("deserial ias report to SgxFields failed: {:?}", e);
                    },
                },
                AttestationReport::SgxDcap { quote, collateral: _ } => {
                    match SgxFields::from_dcap_quote_report(&quote) {
                        Ok((sgx_fields, _)) => {
                            info!("DCAP measurement       :{}", hex::encode(sgx_fields.measurement()));
                            info!("DCAP measurement hash  :{:?}", sgx_fields.measurement_hash());
                        },
                        Err(e) => {
                            error!("deserial dcap report to SgxFields failed: {:?}", e);
                        },
                    }
                },
            }
        }
    }
    Ok(report)
}
