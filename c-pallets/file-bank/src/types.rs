use super::*;
type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FileInfo<T: pallet::Config> {
	pub(super) file_size: u64,
	pub(super) index: u32,
	pub(super) file_state: BoundedVec<u8, T::StringLimit>,
	pub(super) user: BoundedVec<AccountOf<T>, T::StringLimit>,
	pub(super) file_name: BoundedVec<BoundedVec<u8, T::StringLimit>, T::StringLimit>,
	pub(super) slice_info: BoundedVec<SliceInfo<T>, T::StringLimit>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct SliceInfo<T: pallet::Config> {
	pub(super) miner_id: u64,
	pub(super) shard_size: u64,
	pub(super) block_num: u32,
	pub(super) shard_id: BoundedVec<u8, T::StringLimit>,
	pub(super) miner_ip: BoundedVec<u8, T::StringLimit>,
	pub(super) miner_acc: AccountOf<T>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct StorageSpace {
	pub(super) purchased_space: u128,
	pub(super) used_space: u128,
	pub(super) remaining_space: u128,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct SpaceInfo<T: pallet::Config> {
	pub(super) size: u128,
	pub(super) deadline: BlockNumberOf<T>,
}

//Fill in file structure information
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FillerInfo<T: pallet::Config> {
	pub filler_size: u64,
	pub index: u32,
	pub block_num: u32,
	pub segment_size: u32,
	pub scan_size: u32,
	pub miner_address: AccountOf<T>,
	pub filler_id: BoundedVec<u8, T::StringLimit>,
	pub filler_hash: BoundedVec<u8, T::StringLimit>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct UserFileSliceInfo<T: pallet::Config> {
	pub(super) file_hash: BoundedVec<u8, T::StringLimit>,
	pub(super) file_size: u64,
}