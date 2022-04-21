use super::*;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

/// The custom struct for storing info of proofs in VPA. idle segment
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ProofInfoVPA<BoundedString, BlockNumber> {
	//When is_ready changed to true, the corresponding spatiotemporal proof can be submitted
	pub(super) is_ready: bool,
	//false for 8M segment, true for 512M segment
	pub(super) size_type: u128,
	pub(super) proof: Option<BoundedString>,
	pub(super) sealed_cid: Option<BoundedString>,
	pub(super) rand: u32,
	pub(super) block_num: Option<BlockNumber>,
}

/// The custom struct for storing info of proofs in PPA.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ProofInfoPPA<BoundedString, BlockNumber> {
	//1 for 8M segment, 2 for 512M segment
	pub(super) size_type: u128,
	pub(super) proof: Option<BoundedString>,
	pub(super) sealed_cid: Option<BoundedString>,
    pub(super) block_num: Option<BlockNumber>,
}

///Used to provide random numbers for miners
///Miners generate certificates based on random numbers and submit them
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ParamInfo {
	pub(super) peer_id: u64,
	pub(super) segment_id: u64,
	pub(super) rand: u32,
}

///When the VPA is submitted, and the verification is passed
///Miners began to submit idle time and space certificates
///The data is stored in the structure
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoVPB<BoundedString, BlockNumber> {
	//When is_ready changed to true, the corresponding spatiotemporal proof can be submitted
	pub(super) is_ready: bool,
	//1 for 8M segment, 2 for 512M segment
	pub(super) size_type: u128,
	pub(super) proof: Option<BoundedString>,
	pub(super) sealed_cid: Option<BoundedString>,
	pub(super) rand: u32,
	pub(super) block_num: Option<BlockNumber>,
}

//The spatiotemporal proof of validation is stored in the modified structure
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ProofInfoPPB<BoundedString, BlockNumber> {
	//1 for 8M segment, 2 for 512M segment
	pub(super) size_type: u128,
	pub(super) proof: Option<BoundedString>,
	pub(super) sealed_cid: Option<BoundedString>,
	pub(super) block_num: Option<BlockNumber>,
}

//Service copy certificate
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ProofInfoVPC<BoundedList, BlockNumber> {
	//When is_ready changed to true, the corresponding spatiotemporal proof can be submitted
	pub(super) is_ready: bool,
	//1 for 8M segment, 2 for 512M segment
	pub(super) size_type: u128,
	pub(super) proof: Option<BoundedList>,
	pub(super) sealed_cid: Option<BoundedList>,
	pub(super) rand: u32,
	pub(super) block_num: Option<BlockNumber>,
}

//Verification certificate
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ProofInfoPPC<BoundedList, BlockNumber> {
	//1 for 8M segment, 2 for 512M segment
	pub(super) size_type: u128,
	pub(super) proof: Option<BoundedList>,
    pub(super) 	sealed_cid: Option<BoundedList>,
    pub(super) 	block_num: Option<BlockNumber>,
}

//Submit spatiotemporal proof for proof of replication of service data segments
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoVPD<BoundedList, BlockNumber> {
	//When is_ready changed to true, the corresponding spatiotemporal proof can be submitted
    pub(super) 	is_ready: bool,
	//1 for 8M segment, 2 for 512M segment
    pub(super) 	size_type: u128,
	pub(super) proof: Option<BoundedList>,
	pub(super) sealed_cid: Option<BoundedList>,
	pub(super) rand: u32,
	pub(super) block_num: Option<BlockNumber>,
}

//Verification Corresponding certificate
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoPPD<BoundedList, BlockNumber> {
	//1 for 8M segment, 2 for 512M segment
    pub(super) 	size_type: u128,
	pub(super) proof: Option<BoundedList>,
	pub(super) sealed_cid: Option<BoundedList>,
    pub(super) 	block_num: Option<BlockNumber>,
}

//The scheduler reads the pool to know which data it should verify for AB
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct UnverifiedPool<AccountId, BoundedString> {
    pub(super) 	acc: AccountId,
    pub(super) 	peer_id: u64,
    pub(super) 	segment_id: u64,
    pub(super) 	proof: BoundedString,
    pub(super) 	sealed_cid: BoundedString,
	pub(super) rand: u32,
    pub(super) 	size_type: u128,
}

//The scheduler reads the pool to know which data it should verify for C
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct UnverifiedPoolVec<AccountId, BoundedList> {
    pub(super) 	acc: AccountId,
	pub(super) peer_id: u64,
    pub(super) 	segment_id: u64,
	//The file is sliced, and each slice generates a certificate
	pub(super) proof: BoundedList,
	pub(super) sealed_cid: BoundedList,
	pub(super) uncid: BoundedList,
	pub(super) rand: u32,
	pub(super) size_type: u128,
}

//The scheduler reads the pool to know which data it should verify for D
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct UnverifiedPoolVecD<AccountId, BoundedList> {
	pub(super) acc: AccountId,
    pub(super) 	peer_id: u64,
	pub(super) segment_id: u64,
	//The file is sliced, and each slice generates a certificate
	pub(super) proof: BoundedList,
	pub(super) sealed_cid: BoundedList,
	pub(super) rand: u32,
	pub(super) size_type: u128,
}

//Miners read the pool to know which data segments they need to submit time-space certificates
// for AB
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ContinuousProofPool<BoundedString> {
	pub(super) peer_id: u64,
	pub(super) segment_id: u64,
	pub(super) sealed_cid: BoundedString,
	pub(super) size_type: u128,
}
//Miners read the pool to know which data segments they need to submit time-space certificates
// for CD
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ContinuousProofPoolVec<BoundedString, BoundedList> {
	pub(super) peer_id: u64,
	pub(super) segment_id: u64,
	pub(super) sealed_cid: BoundedList,
	pub(super) hash: BoundedString,
	pub(super) size_type: u128,
}

//The miner reads the pool to generate a duplicate certificate of service
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct FileSilceInfo<BoundedString, BoundedList> {
    pub(super) peer_id: u64,
	pub(super) segment_id: u64,
	pub(super) uncid: BoundedList,
	pub(super) rand: u32,
	pub(super) shardhash: BoundedString,
}