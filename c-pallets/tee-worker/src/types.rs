use super::*;

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct TeeWorkerInfo<T: pallet::Config> {
    pub peer_id: [u8; 53],
    pub stash_account: AccountOf<T>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, Default, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ExceptionReport<T: pallet::Config> {
    count: u32,
    reporters: BoundedVec<AccountOf<T>, T::StringLimit>,
}

// #[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, Default, MaxEncodedLen, TypeInfo)]
// #[scale_info(skip_type_params(T))]
// #[codec(mel_bound())]
// pub struct SgxAttestationReport {
//     report_json_raw: BoundedVec<u8; T::ReportLength>,
//     sign: [u8; 344],
//     cert_der: [u8; 1588],
// }