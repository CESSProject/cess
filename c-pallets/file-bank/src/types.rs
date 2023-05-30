use super::*;
// Substrate type
type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
// Cess type
// pub(super) type SegmentList<T> = BoundedVec<(Hash, BoundedVec<Hash, <T as pallet::Config>::FragmentCount>),  <T as pallet::Config>::SegmentCount>;
// pub(super) type MinerTaskList<T> = BoundedVec<(AccountOf<T>, BoundedVec<Hash,  <T as pallet::Config>::FragmentCount>),  <T as pallet::Config>::FragmentCount>;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct SegmentList<T: Config> {
	pub(super) hash: Hash,
	pub(super) fragment_list: BoundedVec<Hash, <T as pallet::Config>::FragmentCount>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct MinerTaskList<T: Config> {
	pub(super) miner: AccountOf<T>,
	pub(super) fragment_list:  BoundedVec<Hash,  <T as pallet::Config>::FragmentCount>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum FileState {
	Active,
	Calculate,
	Missing,
	Recovery,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct DealInfo<T: Config> {
	// There are two stages in total: 
	// the first stage and the second stage, represented by 1 or 2, respectively.
	pub(super) stage: u8, 
	pub(super) count: u8,
	pub(super) segment_list: BoundedVec<SegmentList<T>, T::SegmentCount>,
	pub(super) needed_list: BoundedVec<SegmentList<T>, T::SegmentCount>,
	pub(super) user: UserBrief<T>,
	pub(super) assigned_miner: BoundedVec<MinerTaskList<T>, T::StringLimit>,
	pub(super) share_info: BoundedVec<SegmentInfo<T>, T::SegmentCount>,
	pub(super) complete_list: BoundedVec<AccountOf<T>, T::FragmentCount>,
}

//TODO! BoundedVec type -> BTreeMap
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FileInfo<T: Config> {
	pub(super) completion: BlockNumberOf<T>,
	pub(super) stat: FileState,
	pub(super) segment_list: BoundedVec<SegmentInfo<T>, T::SegmentCount>,
	pub(super) owner: BoundedVec<UserBrief<T>, T::OwnerLimit>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct SegmentInfo<T: Config> {
	pub(super) hash: Hash,
	pub(super) fragment_list: BoundedVec<FragmentInfo<T>, T::FragmentCount>,
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
	pub block_num: u32,
	pub miner_address: AccountOf<T>,
	pub filler_hash: Hash,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct UserFileSliceInfo {
	pub(super) file_hash: Hash,
	pub(super) file_size: u128,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct BucketInfo<T: Config> {
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

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct RestoralTargetInfo<Block> {
	pub(super) service_space: u128,
	pub(super) restored_space: u128,
	pub(super) cooling_block: Block,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct RestoralOrderInfo<T: Config> {
	pub(super) count: u32,
	pub(super) miner: AccountOf<T>,
	pub(super) origin_miner: AccountOf<T>,
	pub(super) fragment_hash: Hash,
	pub(super) file_hash: Hash,
	pub(super) gen_block: BlockNumberOf<T>,
	pub(super) deadline: BlockNumberOf<T>,
}

