use super::*;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ChallengeInfo<T: pallet::Config> {
    pub(super) file_size: u64,
    pub(super) file_type: u8,
    pub(super) block_list: BoundedVec<u32, T::StringLimit>,
    pub(super) file_id: BoundedVec<u8, T::StringLimit>,
    //48 bit random number
    pub(super) random: BoundedVec<u8, T::RandomLimit>,
}

//Structure for storing miner certificates
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ProveInfo<T: pallet::Config> {
    pub(super) miner_id: u64,
    //Verify required parameters
    pub(super) challenge_info: ChallengeInfo<T>,
    //Proof of relevant information
    pub(super) mu: BoundedVec<u8, T::StringLimit>,
    //Proof of relevant information
    pub(super) sigma: BoundedVec<u8, T::StringLimit>,
}