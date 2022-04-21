use super::*;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

#[derive(Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
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
#[derive(Default, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FileDuplicateInfo<T: pallet::Config> {
	pub(super) dupl_id: BoundedVec<u8, T::StringLimit>,
	pub(super) rand_key: BoundedVec<u8, T::StringLimit>,
	pub(super) slice_num: u16,
	pub(super) file_slice: BoundedVec<FileSliceInfo<T>, T::StringLimit>,
}

// impl<T: pallet::Config> Clone for FileDuplicateInfo<T> {
//     fn clone(&self) -> Self {
//         *self
//     }
// }

// impl<T: pallet::Config> PartialEq for FileDuplicateInfo<T> {
//     fn eq(&self, other: &FileDuplicateInfo<T>) -> bool {
//         true
//     }

// 	fn ne(&self, other: &FileDuplicateInfo<T>) -> bool {
//         true
//     }
// }

// impl<T: pallet::Config> PartialEq for FileInfo<T> {
//     fn eq(&self, other: &Self) -> bool {
//         true
//     }

// 	fn ne(&self, other: &Self) -> bool {
//         true
//     }
// }

//slice info
//Slice consists of shard
#[derive(PartialEq, Eq, Default, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FileSliceInfo<T: pallet::Config> {
	pub(super) slice_id: BoundedVec<u8, T::StringLimit>,
	pub(super) slice_size: u32,
	pub(super) slice_hash: BoundedVec<u8, T::StringLimit>,
	pub(super) file_shard: FileShardInfo<T>,
}

//shard info
//Slice consists of shard
#[derive(PartialEq, Eq, Default, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FileShardInfo<T: pallet::Config> {
	pub(super) data_shard_num: u8,
	pub(super) redun_shard_num: u8,
	pub(super) shard_hash: BoundedVec<BoundedVec<u8, T::StringLimit>, T::StringLimit>,
	pub(super) shard_addr: BoundedVec<BoundedVec<u8, T::StringLimit>, T::StringLimit>,
	pub(super) wallet_addr: BoundedVec<u64, T::StringLimit>,
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
