use super::*;

// type AccountOf<T> = <T as frame_system::Config>::AccountId;
// type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ChallengeInfo<T: pallet::Config> {
	pub(super) net_snap_shot: NetSnapShot<BlockNumberOf<T>>,
	pub(super) miner_snapshot_list: BoundedVec<MinerSnapShot<AccountOf<T>, BlockNumberOf<T>>, T::ChallengeMinerMax>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct NetSnapShot<Block> {
	pub(super) start: Block,
	pub(super) life: Block,
	pub(super) total_reward: u128,
	pub(super) total_idle_space: u128,
	pub(super) total_service_space: u128,
	pub(super) random_index_list: BoundedVec<u32, ConstU32<1024>>,
	pub(super) random_list: BoundedVec<[u8; 20], ConstU32<1024>>,
	pub(super) space_challenge_param: SpaceChallengeParam,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct MinerSnapShot<AccountId, Block> {
	pub(super) miner: AccountId,
	pub(super) idle_life: Block,
    pub(super) service_life: Block,
	pub(super) idle_space: u128,
	pub(super) service_space: u128,
	pub(super) idle_submitted: bool,
	pub(super) service_submitted: bool,
}

// Structure for storing miner certificates
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct IdleProveInfo<T: pallet::Config> {
	pub(super) snap_shot: MinerSnapShot<AccountOf<T>, BlockNumberOf<T>>,
	pub(super) idle_prove: BoundedVec<u8, T::SigmaMax>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ServiceProveInfo<T: pallet::Config> {
	pub(super) snap_shot: MinerSnapShot<AccountOf<T>, BlockNumberOf<T>>,
	pub(super) service_prove: BoundedVec<u8, T::SigmaMax>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct VerifyResultInfo<T: pallet::Config> {
	pub(super) miner: AccountOf<T>,
	pub(super) miner_prove: BoundedVec<u8, T::SigmaMax>,
	pub(super) result: bool,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct SegDigest<BlockNumber> {
	pub(super) validators_len: u32,
	pub(super) block_num: BlockNumber,
	pub(super) network_state: OpaqueNetworkState,
}
