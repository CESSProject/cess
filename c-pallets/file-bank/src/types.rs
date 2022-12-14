use cp_cess_common::IpAddress;
use super::*;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum SpaceState {
	Nomal,
	Frozen,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum OrderState {
	Open,
	Assigned,
	Success,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct DealInfo<T: Config> {
	pub(super) file_size: u64,
	pub(super) user_details: Details<T>,
	pub(super) slices: BoundedVec<Hash, T::StringLimit>,
	pub(super) backups: [Backup<T>; 3],
	pub(super) scheduler: AccountOf<T>,
	pub(super) requester: AccountOf<T>,
	pub(super) state: OrderState,
	pub(super) time_task: [u8; 64],
	pub(super) survival_block: BlockNumberOf<T>,
	pub(super) executive_counts: u8,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FileInfo<T: Config> {
	pub(super) file_size: u64,
	pub(super) file_state: BoundedVec<u8, T::StringLimit>,
	pub(super) user_details_list: BoundedVec<Details<T>, T::StringLimit>,
	pub(super) backups: [Backup<T>; 3],
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct Backup<T: Config> {
	pub(super) index: u8,
	pub(super) slices: BoundedVec<SliceInfo<T>, T::StringLimit>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct SliceSummary<T: Config> {
	pub(super) miner: AccountOf<T>,
	pub(super) signature: Signature,
	pub(super) message: BoundedVec<u8, T::StringLimit>,
}

// #[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
// pub struct SliceMessage {
// 	pub(super) shard_id: [u8; 68],
// 	pub(super) miner_ip: IpAddress,
// 	pub(super) shard_size: u64,
// 	pub(super) slice_hash: Hash,
// }

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct SliceInfo<T: Config> {
	pub shard_id: [u8; 68],
	pub slice_hash: Hash,
	pub shard_size: u64,
	pub miner_ip: IpAddress,
	pub miner_acc: AccountOf<T>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct OwnedSpaceDetails<T: Config> {
	pub(super) total_space: u128,
	pub(super) free_space: u128,
	pub(super) used_space: u128,
	pub(super) locked_space: u128,
	pub(super) start: BlockNumberOf<T>,
	pub(super) deadline: BlockNumberOf<T>,
	pub(super) state: SpaceState,
}

//Fill in file structure information
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FillerInfo<T: Config> {
	pub filler_size: u64,
	pub miner_address: AccountOf<T>,
	pub filler_hash: Hash,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct UserFileSliceInfo {
	pub(super) file_hash: Hash,
	pub(super) file_size: u64,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct BucketInfo<T: Config> {
	pub(super) total_capacity: u32,
	pub(super) available_capacity: u32,
	pub(super) object_num: u32,
	pub(super) object_list: BoundedVec<Hash, T::FileListLimit>,
	pub(super) authority: BoundedVec<AccountOf<T>, T::StringLimit>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct Details<T: Config> {
	pub user: AccountOf<T>,
	pub file_name: BoundedVec<u8, T::NameStrLimit>,
	pub bucket_name:  BoundedVec<u8, T::NameStrLimit>,
}
