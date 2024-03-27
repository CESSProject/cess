use super::*;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct OssInfo {
    pub(super) peer_id: PeerId,
    pub(super) domain: BoundedVec<u8, ConstU32<50>>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ProxyAuthPayload<T: Config> {
    pub(super) oss: AccountOf<T>,
    pub(super) exp: BlockNumberFor<T>,
}