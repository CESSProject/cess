use super::*;

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct TeeWorkerInfo<T: pallet::Config> {
    pub peer_id: [u8; 53],
    pub stash_account: AccountOf<T>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, Default, MaxEncodedLen, TypeInfo)]
pub struct SgxAttestationReport {
    pub report_json_raw: Report,
    pub sign: ReportSign,
    pub cert_der: Cert,
}