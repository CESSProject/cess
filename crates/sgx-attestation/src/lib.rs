#![cfg_attr(all(not(test), not(feature = "std")), no_std)]

#[macro_use]
extern crate alloc;

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

pub mod dcap;
pub mod ias;
pub mod types;

#[cfg(feature = "report")]
pub mod gramine;

#[derive(Encode, Decode, TypeInfo, Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidCertificate,
    InvalidSignature,
    CodecError,

    // DCAP
    TCBInfoExpired,
    KeyLengthIsInvalid,
    PublicKeyIsInvalid,
    RsaSignatureIsInvalid,
    DerEncodingError,
    UnsupportedDCAPQuoteVersion,
    UnsupportedDCAPAttestationKeyType,
    UnsupportedQuoteAuthData,
    UnsupportedDCAPPckCertFormat,
    LeafCertificateParsingError,
    CertificateChainIsInvalid,
    CertificateChainIsTooShort,
    IntelExtensionCertificateDecodingError,
    IntelExtensionAmbiguity,
    CpuSvnLengthMismatch,
    CpuSvnDecodingError,
    PceSvnDecodingError,
    PceSvnLengthMismatch,
    FmspcLengthMismatch,
    FmspcDecodingError,
    FmspcMismatch,
    QEReportHashMismatch,
    IsvEnclaveReportSignatureIsInvalid,
    DerDecodingError,
    OidIsMissing,
}

pub mod quote_status_levels {
    pub const SGX_QUOTE_STATUS_LEVEL_1: &[&str] = &[
        // IAS
        "OK",
        // DCAP
        "UpToDate",
    ];
    pub const SGX_QUOTE_STATUS_LEVEL_2: &[&str] = &[
        // IAS
        "SW_HARDENING_NEEDED",
        // DCAP
        "SWHardeningNeeded",
    ];
    pub const SGX_QUOTE_STATUS_LEVEL_3: &[&str] = &[
        // IAS
        "CONFIGURATION_NEEDED",
        "CONFIGURATION_AND_SW_HARDENING_NEEDED",
        // DCAP
        "ConfigurationNeeded",
        "ConfigurationAndSWHardeningNeeded",
    ];
    // LEVEL 4 is LEVEL 3 with advisors which not included in whitelist
    pub const SGX_QUOTE_STATUS_LEVEL_5: &[&str] = &[
        // IAS
        "GROUP_OUT_OF_DATE",
        // DCAP
        "OutOfDate",
        "OutOfDateConfigurationNeeded",
    ];
    pub const SGX_QUOTE_ADVISORY_ID_WHITELIST: &[&str] = &[
        "INTEL-SA-00334",
        "INTEL-SA-00219",
        "INTEL-SA-00381",
        "INTEL-SA-00389",
    ];   
}