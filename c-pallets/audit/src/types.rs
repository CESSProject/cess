use cp_cess_common::Hash;
use super::*;

// type AccountOf<T> = <T as frame_system::Config>::AccountId;
// type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
pub type BoundedList<T> =
	BoundedVec<BoundedVec<u8, <T as Config>::StringLimit>, <T as Config>::StringLimit>;
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ChallengeInfo<T: pallet::Config> {
	pub(super) file_size: u64,
	pub(super) file_type: DataType,
	pub(super) block_list: BoundedVec<u32, T::StringLimit>,
	pub(super) file_id: Hash,
	pub(super) shard_id: [u8; 68],
	//48 bit random number
	pub(super) random: BoundedList<T>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct NetSnapShot<Block> {
	pub(super) start: Block,
	pub(super) total_reward: u128,
	pub(super) total_idle_space: u128,
	pub(super) total_service_space: u128,
	pub(super) random: [u8; 20],
}

// Structure for storing miner certificates
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ProveInfo<T: pallet::Config> {
	pub(super) file_id: Hash,
	pub(super) miner_acc: AccountOf<T>,
	//Verify required parameters
	pub(super) challenge_info: ChallengeInfo<T>,
	pub(super) u: BoundedVec<u8, T::StringLimit>,
	//Proof of relevant information
	pub(super) mu: BoundedVec<u8, T::StringLimit>,
	//Proof of relevant information
	pub(super) sigma: BoundedVec<u8, T::StringLimit>,
	pub(super) omega: BoundedVec<u8, T::StringLimit>,
	pub(super) sig_root_hash: BoundedVec<u8, T::StringLimit>,
	pub(super) hash_mi: BoundedList<T>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct VerifyResult<T: pallet::Config> {
	pub(super) miner_acc: AccountOf<T>,
	pub(super) file_id: Hash,
	pub(super) shard_id: [u8; 68],
	pub(super) result: bool,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct SegDigest<BlockNumber> {
	pub(super) validators_len: u32,
	pub(super) block_num: BlockNumber,
	pub(super) validators_index: u16,
	pub(super) network_state: OpaqueNetworkState,
}
