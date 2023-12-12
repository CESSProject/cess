use super::*;

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct TeeWorkerInfo<T: pallet::Config> {
    pub peer_id: PeerId,
    pub bond_stash: Option<AccountOf<T>>,
    pub end_point: EndPoint,
    pub tee_type: TeeType,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, Default, MaxEncodedLen, TypeInfo)]
pub struct SgxAttestationReport {
    pub report_json_raw: Report,
    pub sign: ReportSign,
    pub cert_der: Cert,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum TeeType {
    Full,
    Certifier,
    Verifier,
    Marker,
}