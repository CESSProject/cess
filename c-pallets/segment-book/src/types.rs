use super::*;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

/// The custom struct for storing info of proofs in VPA. idle segment
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoVPA<T: pallet::Config> {
	//When is_ready changed to true, the corresponding spatiotemporal proof can be submitted
	pub(super) is_ready: bool,
	//false for 8M segment, true for 512M segment
	pub(super) size_type: u128,
	pub(super) proof: Option<Vec<u8>>,
	pub(super) sealed_cid: Option<Vec<u8>>,
	pub(super) rand: u32,
	pub(super) block_num: Option<BlockNumberOf<T>>,
}

/// The custom struct for storing info of proofs in PPA.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoPPA<T: pallet::Config> {
	//1 for 8M segment, 2 for 512M segment
	pub(super) size_type: u128,
	pub(super) proof: Option<Vec<u8>>,
	pub(super) sealed_cid: Option<Vec<u8>>,
    pub(super) 	block_num: Option<BlockNumberOf<T>>,
}

///Used to provide random numbers for miners
///Miners generate certificates based on random numbers and submit them
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
pub struct ParamInfo {
	pub(super) peer_id: u64,
	pub(super) segment_id: u64,
	pub(super) rand: u32,
}

///When the VPA is submitted, and the verification is passed
///Miners began to submit idle time and space certificates
///The data is stored in the structure
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoVPB<T: pallet::Config> {
	//When is_ready changed to true, the corresponding spatiotemporal proof can be submitted
	pub(super) is_ready: bool,
	//1 for 8M segment, 2 for 512M segment
	pub(super) size_type: u128,
	pub(super) proof: Option<Vec<u8>>,
	pub(super) sealed_cid: Option<Vec<u8>>,
	pub(super) rand: u32,
	pub(super) block_num: Option<BlockNumberOf<T>>,
}

//The spatiotemporal proof of validation is stored in the modified structure
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoPPB<T: pallet::Config> {
	//1 for 8M segment, 2 for 512M segment
	pub(super) size_type: u128,
	pub(super) proof: Option<Vec<u8>>,
	pub(super) sealed_cid: Option<Vec<u8>>,
	pub(super) block_num: Option<BlockNumberOf<T>>,
}

//Service copy certificate
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoVPC<T: pallet::Config> {
	//When is_ready changed to true, the corresponding spatiotemporal proof can be submitted
	pub(super) is_ready: bool,
	//1 for 8M segment, 2 for 512M segment
	pub(super) size_type: u128,
	pub(super) proof: Option<Vec<Vec<u8>>>,
	pub(super) sealed_cid: Option<Vec<Vec<u8>>>,
	pub(super) rand: u32,
	pub(super) block_num: Option<BlockNumberOf<T>>,
}

//Verification certificate
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoPPC<T: pallet::Config> {
	//1 for 8M segment, 2 for 512M segment
	pub(super) size_type: u128,
	pub(super) proof: Option<Vec<Vec<u8>>>,
    pub(super) 	sealed_cid: Option<Vec<Vec<u8>>>,
    pub(super) 	block_num: Option<BlockNumberOf<T>>,
}

//Submit spatiotemporal proof for proof of replication of service data segments
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoVPD<T: pallet::Config> {
	//When is_ready changed to true, the corresponding spatiotemporal proof can be submitted
    pub(super) 	is_ready: bool,
	//1 for 8M segment, 2 for 512M segment
    pub(super) 	size_type: u128,
	pub(super) proof: Option<Vec<Vec<u8>>>,
	pub(super) sealed_cid: Option<Vec<Vec<u8>>>,
	pub(super) rand: u32,
	pub(super) block_num: Option<BlockNumberOf<T>>,
}

//Verification Corresponding certificate
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoPPD<T: pallet::Config> {
	//1 for 8M segment, 2 for 512M segment
    pub(super) 	size_type: u128,
	pub(super) proof: Option<Vec<Vec<u8>>>,
	pub(super) sealed_cid: Option<Vec<Vec<u8>>>,
    pub(super) 	block_num: Option<BlockNumberOf<T>>,
}

//A series of data required for polling
//The total number of blocks that store all the file data held by the miner

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct PeerFileNum {
    pub(super) 	block_num: u128,
	pub(super) total_num: u64,
}

//The scheduler reads the pool to know which data it should verify for AB
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct UnverifiedPool<T: pallet::Config> {
    pub(super) 	acc: AccountOf<T>,
    pub(super) 	peer_id: u64,
    pub(super) 	segment_id: u64,
    pub(super) 	proof: Vec<u8>,
    pub(super) 	sealed_cid: Vec<u8>,
	pub(super) rand: u32,
    pub(super) 	size_type: u128,
}

//The scheduler reads the pool to know which data it should verify for C
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct UnverifiedPoolVec<T: pallet::Config> {
    pub(super) 	acc: AccountOf<T>,
	pub(super) peer_id: u64,
    pub(super) 	segment_id: u64,
	//The file is sliced, and each slice generates a certificate
	pub(super) proof: Vec<Vec<u8>>,
	pub(super) sealed_cid: Vec<Vec<u8>>,
	pub(super) uncid: Vec<Vec<u8>>,
	pub(super) rand: u32,
	pub(super) size_type: u128,
}

//The scheduler reads the pool to know which data it should verify for D
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct UnverifiedPoolVecD<T: pallet::Config> {
	pub(super) acc: AccountOf<T>,
    pub(super) 	peer_id: u64,
	pub(super) segment_id: u64,
	//The file is sliced, and each slice generates a certificate
	pub(super) proof: Vec<Vec<u8>>,
	pub(super) sealed_cid: Vec<Vec<u8>>,
	pub(super) rand: u32,
	pub(super) size_type: u128,
}

//Miners read the pool to know which data segments they need to submit time-space certificates
// for AB
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ContinuousProofPool {
	pub(super) peer_id: u64,
	pub(super) segment_id: u64,
	pub(super) sealed_cid: Vec<u8>,
	pub(super) size_type: u128,
}
//Miners read the pool to know which data segments they need to submit time-space certificates
// for CD
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ContinuousProofPoolVec {
	pub(super) peer_id: u64,
	pub(super) segment_id: u64,
	pub(super) sealed_cid: Vec<Vec<u8>>,
	pub(super) hash: Vec<u8>,
	pub(super) size_type: u128,
}

//The miner reads the pool to generate a duplicate certificate of service
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct FileSilceInfo {
    pub(super) peer_id: u64,
	pub(super) segment_id: u64,
	pub(super) uncid: Vec<Vec<u8>>,
	pub(super) rand: u32,
	pub(super) shardhash: Vec<u8>,
}