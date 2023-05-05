use super::*;

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct TeeWorkerInfo<T: pallet::Config> {
    pub controller_account: AccountOf<T>,
    pub peer_id: [u8; 52],
    pub node_key: NodePublicKey,
    pub stash_account: AccountOf<T>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, Default, MaxEncodedLen, TypeInfo)]
pub struct SgxAttestationReport {
    pub report_json_raw: Report,
    pub sign: ReportSign,
    pub cert_der: Cert,
}