use cp_cess_common::IpAddress;
use super::*;
// Substrate type
type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
// Cess type
type SegmentList<T> = BoundedVec<(Hash, BoundedVec<Hash, T::FragmentCount>), T::SegmentCount>;
type MinerTaskList<T> = BoundedVec<(AccountOf<T>, BoundedVec<Hash, T::FragmentCount>), T::FragmentCount>;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub(super) enum FileState {
	Active,
	Calculate,
	Missing,
	Recovery,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct DealInfo<T: Config> {
	pub(super) stage: u8,
	pub(super) count: u8,
	pub(super) time_task: [u8; 32],
	pub(super) segment_list: SegmentList<T>,
	pub(super) user: UserBrief<T>,
	pub(super) assigned_miner: MinerTaskList<T>,
	pub(super) share_info: BoundedVec<SegmentInfo<T>, T::SegemtnCount>,
}
// [s...]
// s{acc, [Hash...]}
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FileInfo<T: Config> {
	pub(super) completion: BlockNumberOf<T>,
	pub(super) stat: FileState,
	pub(super) segment_list: BoundedVec<SegmentInfo<T>, T::SegemtnCount>,
	pub(super) owner: BoundedVec<UserBrief<T>, T::OwnerLimit>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct SegmentInfo<T: Config> {
	pub(super) hash: Hash,
	pub(super) fragment_list: BoundedVec<FragmentInfo<T: Config>, T::FragmentCount>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FragmentInfo<T: Config> {
	pub(super) hash: Hash,
	pub(super) avail: bool,
	pub(super) miner: AccountOf<T>,
}

//Fill in file structure information
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FillerInfo<T: Config> {
	pub filler_size: u64,
	pub index: u32,
	pub block_num: u32,
	pub segment_size: u32,
	pub scan_size: u32,
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
pub struct UserBrief<T: Config> {
	pub user: AccountOf<T>,
	pub file_name: BoundedVec<u8, T::NameStrLimit>,
	pub bucket_name:  BoundedVec<u8, T::NameStrLimit>,
}
