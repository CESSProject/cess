use super::*;
type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;



#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FileInfo<T: pallet::Config> {
	pub(super) file_name: BoundedVec<u8, T::StringLimit>,
	pub(super) file_size: u64,
	pub(super) file_hash: BoundedVec<u8, T::StringLimit>,
	//Public or not
	pub(super) public: bool,
	pub(super) user_addr: AccountOf<T>,
	//normal or repairing
	pub(super) file_state: BoundedVec<u8, T::StringLimit>,
	//Number of backups
	pub(super) backups: u8,
	pub(super) downloadfee: BalanceOf<T>,
	//Backup information
	pub(super) file_dupl: BoundedVec<FileDuplicateInfo<T>, T::StringLimit>,
}

//backups info struct
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FileDuplicateInfo<T: pallet::Config> {
	pub(super) miner_id: u64,
	pub(super) block_num: u32,
	pub(super) acc: AccountOf<T>,
	pub(super) miner_ip: BoundedVec<u8, T::StringLimit>,
	pub(super) dupl_id: BoundedVec<u8, T::StringLimit>,
	pub(super) rand_key: BoundedVec<u8, T::StringLimit>,
	pub(super) block_info: BoundedVec<FileBlock, T::StringLimit>,
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

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct UserInfo<T: pallet::Config> {
	pub(super) collaterals: BalanceOf<T>,
}

//Fill in file structure information
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FillerInfo<T: pallet::Config> {	
	pub(super) miner_id: u64,
	pub(super) filler_size: u64,
	pub(super) block_num: u32,
	pub(super) miner_address: AccountOf<T>,	
	pub(super) filler_block: BoundedVec<FileBlock, T::StringLimit>,
	pub(super) filler_id: BoundedVec<u8, T::StringLimit>,
	pub(super) filler_hash: BoundedVec<u8, T::StringLimit>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[codec(mel_bound())]
pub struct FileBlock {
	pub(super) block_index: u32,
	pub(super) block_size: u32,
	pub(super) segment_size: u32,
}
