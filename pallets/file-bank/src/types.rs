use super::*;
// Substrate type
type AccountOf<T> = <T as frame_system::Config>::AccountId;
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
	pub(super) index: u8,
	pub(super) miner: Option<AccountOf<T>>,
	pub(super) fragment_list: BoundedVec<Hash,  <T as pallet::Config>::MissionCount>,
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
	pub(super) file_size: u128,
	pub(super) segment_list: BoundedVec<SegmentList<T>, T::SegmentCount>,
	pub(super) user: UserBrief<T>,
	pub(super) complete_list: BoundedVec<CompleteInfo<T>, T::FragmentCount>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct CompleteInfo<T: Config> {
	pub(super) index: u8,
	pub(super) miner: AccountOf<T>,
}

//TODO! BoundedVec type -> BTreeMap
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct FileInfo<T: Config> {
	pub(super) segment_list: BoundedVec<SegmentInfo<T>, T::SegmentCount>,
	pub(super) owner: BoundedVec<UserBrief<T>, T::OwnerLimit>,
	pub(super) file_size: u128,
	pub(super) completion: BlockNumberFor<T>,
	pub(super) stat: FileState,
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

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct UserFileSliceInfo {
	pub(super) file_hash: Hash,
	pub(super) file_size: u128,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct BucketInfo<T: Config> {
	pub(super) object_list: BoundedVec<Hash, T::UserFileLimit>,
	pub(super) authority: BoundedVec<AccountOf<T>, ConstU32<1032>>,
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
pub struct RestoralTargetInfo<Account, Block> {
	pub(super) miner: Account,
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
	pub(super) gen_block: BlockNumberFor<T>,
	pub(super) deadline: BlockNumberFor<T>,
}

