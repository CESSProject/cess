use cp_cess_common::Hash;
use super::*;

// type AccountOf<T> = <T as frame_system::Config>::AccountId;
// type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
pub type BoundedList<T> =
	BoundedVec<BoundedVec<u8, <T as Config>::StringLimit>, <T as Config>::StringLimit>;

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ReportMessage<T: pallet::Config> {
	pub(super) idle_filter: BloomFilter,
	pub(super) service_filter: BloomFilter,
	pub(super) autonomy_filter: BloomFilter,
	pub(super) random: [u8; 20],
	pub(super) failed_idle_file: BoundedVec<Hash, T::StringLimit>,
	pub(super) failed_service_file: BoundedVec<[u8; 68], T::StringLimit>,
	pub(super) failed_autonomy_file: BoundedVec<Hash, T::StringLimit>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ChallengeReport<T: pallet::Config> {
	pub(super) message: BoundedVec<u8, T::StringLimit>,
	pub(super) signature: SgxSignature,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct NetworkSnapshot<T: pallet::Config> {
	pub(super) total_power: u128,
	pub(super) reward: BalanceOf<T>,
	pub(super) random: [u8; 20],
	pub(super) start: BlockNumberOf<T>,
	pub(super) deadline: BlockNumberOf<T>,
}


#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct MinerSnapshot<T: pallet::Config> {
	pub(super) power: u128,
	pub(super) miner_acc: AccountOf<T>,
	pub(super) idle_filter: BloomFilter,
	pub(super) service_filter: BloomFilter,
	pub(super) autonomy_filter: BloomFilter,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct SegDigest<BlockNumber> {
	pub(super) validators_len: u32,
	pub(super) block_num: BlockNumber,
	pub(super) validators_index: u16,
	pub(super) network_state: OpaqueNetworkState,
}
