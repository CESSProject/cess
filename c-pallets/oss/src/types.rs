use super::*;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct OssInfo {
    pub(super) peer_id: PeerId,
    pub(super) domain: BoundedVec<u8, ConstU32<50>>,
}