pub mod legacy;

pub mod ias_quote_consts {
	pub const IAS_QUOTE_STATUS_LEVEL_1: &[&str] = &["OK"];
	pub const IAS_QUOTE_STATUS_LEVEL_2: &[&str] = &["SW_HARDENING_NEEDED"];
	pub const IAS_QUOTE_STATUS_LEVEL_3: &[&str] = &["CONFIGURATION_NEEDED", "CONFIGURATION_AND_SW_HARDENING_NEEDED"];
	// LEVEL 4 is LEVEL 3 with advisors which not included in whitelist
	pub const IAS_QUOTE_STATUS_LEVEL_5: &[&str] = &["GROUP_OUT_OF_DATE"];
	pub const IAS_QUOTE_ADVISORY_ID_WHITELIST: &[&str] =
		&["INTEL-SA-00334", "INTEL-SA-00219", "INTEL-SA-00381", "INTEL-SA-00389"];
}

use ias_quote_consts::*;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::vec::Vec;

#[cfg(feature = "enable_serde")]
use serde::{Deserialize, Serialize};

#[derive(Encode, Decode, TypeInfo, Debug, Clone, PartialEq, Eq)]
pub enum AttestationReport {
	SgxIas { ra_report: Vec<u8>, signature: Vec<u8>, raw_signing_cert: Vec<u8> },
}

#[cfg_attr(feature = "enable_serde", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, TypeInfo, Debug, Copy, Clone, PartialEq, Eq)]
pub enum AttestationProvider {
	#[cfg_attr(feature = "enable_serde", serde(rename = "root"))]
	Root,
	#[cfg_attr(feature = "enable_serde", serde(rename = "ias"))]
	Ias,
}

#[derive(Encode, Decode, TypeInfo, Debug, Clone, PartialEq, Eq)]
pub enum Error {
	CesealRejected,
	InvalidIASSigningCert,
	InvalidReport,
	InvalidQuoteStatus,
	BadIASReport,
	OutdatedIASReport,
	UnknownQuoteBodyFormat,
	InvalidUserDataHash,
	NoneAttestationDisabled,
}

#[derive(Encode, Decode, TypeInfo, Debug, Clone, PartialEq, Eq)]
pub struct ConfidentialReport {
	pub confidence_level: u8,
	pub provider: Option<AttestationProvider>,
	pub measurement_hash: H256,
}

fn fixed_measurement(mr_enclave: &[u8], isv_prod_id: &[u8], isv_svn: &[u8], mr_signer: &[u8]) -> Vec<u8> {
	let mut data = Vec::new();
	data.extend_from_slice(mr_enclave);
	data.extend_from_slice(isv_prod_id);
	data.extend_from_slice(isv_svn);
	data.extend_from_slice(mr_signer);
	data
}

#[cfg(feature = "full_crypto")]
fn fixed_measurement_hash(data: &[u8]) -> H256 {
	H256(sp_core::blake2_256(data))
}

#[cfg(not(feature = "full_crypto"))]
fn fixed_measurement_hash(_data: &[u8]) -> H256 {
	log::error!("The measurement hash must be in SGX enviroment with \"full_crypto\" feature, now return zero");
	H256::default()
}

#[derive(Encode, Decode, TypeInfo, Debug, Clone, PartialEq, Eq)]
pub struct ExtendMeasurement {
	pub mr_enclave: [u8; 32],
	pub mr_signer: [u8; 32],
	pub isv_prod_id: [u8; 2],
	pub isv_svn: [u8; 2],
}

impl ExtendMeasurement {
	pub fn measurement(&self) -> Vec<u8> {
		fixed_measurement(&self.mr_enclave, &self.isv_prod_id, &self.isv_svn, &self.mr_signer)
	}

	pub fn measurement_hash(&self) -> H256 {
		fixed_measurement_hash(&self.measurement()[..])
	}
}

#[derive(Encode, Decode, TypeInfo, Debug, Clone, PartialEq, Eq)]
pub struct IasFields {
	pub mr_enclave: [u8; 32],
	pub mr_signer: [u8; 32],
	pub isv_prod_id: [u8; 2],
	pub isv_svn: [u8; 2],
	pub report_data: [u8; 64],
	pub confidence_level: u8,
}

impl IasFields {
	pub fn from_ias_report(report: &[u8]) -> Result<(IasFields, i64), Error> {
		// Validate related fields
		let parsed_report: sgx_attestation::ias::RaReport =
			serde_json::from_slice(report).or(Err(Error::InvalidReport))?;

		// Extract report time
		let raw_report_timestamp = parsed_report.timestamp.clone() + "Z";
		let report_timestamp = chrono::DateTime::parse_from_rfc3339(&raw_report_timestamp)
			.or(Err(Error::BadIASReport))?
			.timestamp();

		// Filter valid `isvEnclaveQuoteStatus`
		let quote_status = &parsed_report.isv_enclave_quote_status.as_str();
		let mut confidence_level: u8 = 128;
		if IAS_QUOTE_STATUS_LEVEL_1.contains(quote_status) {
			confidence_level = 1;
		} else if IAS_QUOTE_STATUS_LEVEL_2.contains(quote_status) {
			confidence_level = 2;
		} else if IAS_QUOTE_STATUS_LEVEL_3.contains(quote_status) {
			confidence_level = 3;
		} else if IAS_QUOTE_STATUS_LEVEL_5.contains(quote_status) {
			confidence_level = 5;
		}
		if confidence_level == 128 {
			return Err(Error::InvalidQuoteStatus)
		}
		// CL 1 means there is no known issue of the CPU
		// CL 2 means the worker's firmware up to date, and the worker has well configured to prevent known issues
		// CL 3 means the worker's firmware up to date, but needs to well configure its BIOS to prevent known issues
		// CL 5 means the worker's firmware is outdated
		// For CL 3, we don't know which vulnerable (aka SA) the worker not well configured, so we need to check the
		// allow list
		if confidence_level == 3 {
			// Filter AdvisoryIDs. `advisoryIDs` is optional
			for advisory_id in parsed_report.advisory_ids.iter() {
				let advisory_id = advisory_id.as_str();
				if !IAS_QUOTE_ADVISORY_ID_WHITELIST.contains(&advisory_id) {
					confidence_level = 4;
				}
			}
		}

		// Extract quote fields
		let quote = parsed_report.decode_quote().or(Err(Error::UnknownQuoteBodyFormat))?;

		Ok((
			IasFields {
				mr_enclave: quote.mr_enclave,
				mr_signer: quote.mr_signer,
				isv_prod_id: quote.isv_prod_id,
				isv_svn: quote.isv_svn,
				report_data: quote.report_data,
				confidence_level,
			},
			report_timestamp,
		))
	}

	pub fn measurement(&self) -> Vec<u8> {
		fixed_measurement(&self.mr_enclave, &self.isv_prod_id, &self.isv_svn, &self.mr_signer)
	}

	pub fn measurement_hash(&self) -> H256 {
		fixed_measurement_hash(&self.measurement()[..])
	}
}

pub fn validate(
	attestation: Option<AttestationReport>,
	user_data_hash: &[u8; 32],
	now: u64,
	verify_ceseal_hash: bool,
	ceseal_bin_allowlist: Vec<H256>,
	opt_out_enabled: bool,
) -> Result<ConfidentialReport, Error> {
	match attestation {
		Some(AttestationReport::SgxIas { ra_report, signature, raw_signing_cert }) => validate_ias_report(
			user_data_hash,
			ra_report.as_slice(),
			signature.as_slice(),
			raw_signing_cert.as_slice(),
			now,
			verify_ceseal_hash,
			ceseal_bin_allowlist,
		),
		None =>
			if opt_out_enabled {
				Ok(ConfidentialReport { provider: None, measurement_hash: Default::default(), confidence_level: 128u8 })
			} else {
				Err(Error::NoneAttestationDisabled)
			},
	}
}

pub fn validate_ias_report(
	user_data_hash: &[u8],
	report: &[u8],
	signature: &[u8],
	raw_signing_cert: &[u8],
	now: u64,
	verify_ceseal_hash: bool,
	ceseal_bin_allowlist: Vec<H256>,
) -> Result<ConfidentialReport, Error> {
	// Validate report
	sgx_attestation::ias::verify_signature(report, signature, raw_signing_cert, core::time::Duration::from_secs(now))
		.or(Err(Error::InvalidIASSigningCert))?;

	let (ias_fields, report_timestamp) = IasFields::from_ias_report(report)?;

	// Validate Ceseal
	let ceseal_hash = ias_fields.measurement_hash();
	if verify_ceseal_hash && !ceseal_bin_allowlist.contains(&ceseal_hash) {
		return Err(Error::CesealRejected)
	}

	// Validate time
	if (now as i64 - report_timestamp) >= 7200 {
		return Err(Error::OutdatedIASReport)
	}

	let commit = &ias_fields.report_data[..32];
	if commit != user_data_hash {
		return Err(Error::InvalidUserDataHash)
	}

	// Check the following fields
	Ok(ConfidentialReport {
		provider: Some(AttestationProvider::Ias),
		measurement_hash: ceseal_hash,
		confidence_level: ias_fields.confidence_level,
	})
}

#[cfg(test)]
mod test {
	use super::*;
	use frame_support::assert_ok;

	pub const ATTESTATION_SAMPLE: &[u8] = include_bytes!("../sample/ias_attestation.json");
	pub const ATTESTATION_TIMESTAMP: u64 = 1631441180; // 2021-09-12T18:06:20.402478
	pub const PRUNTIME_HASH: &str = "518422fa769d2d55982015a0e0417c6a8521fdfc7308f5ec18aaa1b6924bd0f300000000815f42f11cf64430c30bab7816ba596a1da0130c3b028b673133a66cf9a3e0e6";

	#[test]
	fn test_ias_validator() {
		let sample: sgx_attestation::ias::SignedIasReport = serde_json::from_slice(ATTESTATION_SAMPLE).unwrap();

		let report = sample.ra_report.as_bytes();
		let parsed_report = sample.parse_report().unwrap();
		let signature = hex::decode(sample.signature.as_str()).unwrap();
		let raw_signing_cert = hex::decode(sample.raw_signing_cert.as_str()).unwrap();

		let quote = parsed_report.decode_quote().unwrap();
		let commit = &quote.report_data[..32];

		assert_eq!(
			validate_ias_report(&[0u8], report, &signature, &raw_signing_cert, ATTESTATION_TIMESTAMP, false, vec![]),
			Err(Error::InvalidUserDataHash)
		);

		assert_eq!(
			validate_ias_report(
				commit,
				report,
				&signature,
				&raw_signing_cert,
				ATTESTATION_TIMESTAMP + 10000000,
				false,
				vec![]
			),
			Err(Error::OutdatedIASReport)
		);

		assert_eq!(
			validate_ias_report(commit, report, &signature, &raw_signing_cert, ATTESTATION_TIMESTAMP, true, vec![]),
			Err(Error::CesealRejected)
		);

		let m_hash = fixed_measurement_hash(&hex::decode(PRUNTIME_HASH).unwrap());
		assert_ok!(validate_ias_report(
			commit,
			report,
			&signature,
			&raw_signing_cert,
			ATTESTATION_TIMESTAMP,
			true,
			vec![m_hash]
		));
	}
}
