use super::*;
type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FileInfo<T: pallet::Config> {
	pub(super) file_size: u64,
	pub(super) block_num: u32,
	pub(super) scan_size: u32,
	pub(super) segment_size: u32,
	pub(super) miner_acc: AccountOf<T>,
	pub(super) miner_ip: BoundedVec<u8, T::StringLimit>,
	pub(super) user: BoundedVec<AccountOf<T>, T::StringLimit>,
	pub(super) file_name: BoundedVec<BoundedVec<u8, T::StringLimit>, T::StringLimit>,
	pub(super) file_state: BoundedVec<u8, T::StringLimit>,
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
	pub(super) filler_size: u64,
	pub(super) block_num: u32,
	pub(super) segment_size: u32,
	pub(super) scan_size: u32,
	pub(super) miner_address: AccountOf<T>,
	pub(super) filler_id: BoundedVec<u8, T::StringLimit>,
	pub(super) filler_hash: BoundedVec<u8, T::StringLimit>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct UserFileSliceInfo<T: pallet::Config> {
	pub(super) file_hash: BoundedVec<u8, T::StringLimit>,
	pub(super) file_size: u64,
}