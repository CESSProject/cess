use super::*;
use frame_system::pallet_prelude::BlockNumberFor;

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ChallengeInfo<T: pallet::Config> {
	pub(super) miner_snapshot: MinerSnapShot<T>,
	pub(super) challenge_element: ChallengeElement<T>,
	pub(super) prove_info: ProveInfo<T>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ProveInfo<T: pallet::Config> {
	pub(super) assign: u8,
	pub(super) idle_prove: Option<IdleProveInfo<T>>,
	pub(super) service_prove: Option<ServiceProveInfo<T>>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ProveInfoV2<T: pallet::Config> {
	pub(super) idle_prove: Option<IdleProveInfo<T>>,
	pub(super) assign: u8,
	pub(super) service_prove: Option<ServiceProveInfo<T>>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ChallengeElement<T: pallet::Config> {
	pub(super) start: BlockNumberFor<T>,
	pub(super) idle_slip: BlockNumberFor<T>,
	pub(super) service_slip: BlockNumberFor<T>,
	pub(super) verify_slip: BlockNumberFor<T>,
	pub(super) space_param: SpaceChallengeParam,
	pub(super) service_param: QElement,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct MinerSnapShot<T: pallet::Config> {
	pub(super) idle_space: u128,
	pub(super) service_space: u128,
	pub(super) service_bloom_filter: BloomFilter,
	pub(super) space_proof_info: SpaceProofInfo<AccountOf<T>>,
	pub(super) tee_signature: TeeRsaSignature,
}

// Structure for storing miner certificates
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct IdleProveInfo<T: pallet::Config> {
	pub(super) tee_acc: AccountOf<T>,
	pub(super) idle_prove: BoundedVec<u8, T::IdleTotalHashLength>,
	pub(super) verify_result: Option<bool>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ServiceProveInfo<T: pallet::Config> {
	pub(super) tee_acc: AccountOf<T>,
	pub(super) service_prove: BoundedVec<u8, T::SigmaMax>,
	pub(super) verify_result: Option<bool>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct VerifyIdleResultInfo<T: pallet::Config> {
	pub(super) miner: AccountOf<T>,
	pub(super) miner_prove: BoundedVec<u8, T::IdleTotalHashLength>,
	pub(super) front: u64,
	pub(super) rear: u64,
	pub(super) accumulator: Accumulator,
	pub(super) space_challenge_param: SpaceChallengeParam,
	pub(super) result: bool,
	pub(super) tee_acc: AccountOf<T>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct VerifyServiceResultInfo<T: pallet::Config> {
	pub(super) miner: AccountOf<T>,
	pub(super) tee_acc: AccountOf<T>,
	pub(super) miner_prove: BoundedVec<u8, T::SigmaMax>,
	pub(super) result: bool,
	pub(super) chal: QElement,
	pub(super) service_bloom_filter: BloomFilter,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct SegDigest<BlockNumber> {
	pub(super) validators_len: u32,
	pub(super) block_num: BlockNumber,
	pub(super) network_state: OpaqueNetworkState,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct QElement {
	pub(super) random_index_list: BoundedVec<u32, ConstU32<1024>>,
	pub(super) random_list: BoundedVec<[u8; 20], ConstU32<1024>>,
}
