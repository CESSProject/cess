//! # Segemnt Book Module
//!
//! Contain operations related proof of storage.
//!
//! ### Terminology
//! 
//! * **uncid:** 		Necessary parameters for generating proof (unencrypted)
//! * **sealed_cid:** 	Necessary parameters for generating proof (encrypted)
//! * **segment_id:**	Allocated segment ID
//! * **is_ready:**		Used to know whether to submit a certificate
//! * **size_type:**	Segment size
//! * **peer_id:**		Miner's ID 
//! 
//! ### Interface
//!
//! ### Dispatchable Functions
//!
//! * `intent_submit` 		Pprovide miners with the necessary parameters to generate proof
//! * `intent_submit_po_st` Provide miners with the necessary parameters to generate proof
//! * `submit_to_vpa` 		Submit copy certificate of idle data segment
//! * `verify_in_vpa` 		Verify replication proof of idle data segments
//! * `submit_to_vpb` 		Submit space-time proof of idle data segments
//! * `verify_in_vpb` 		Verify the spatiotemporal proof of idle data segments
//! * `submit_to_vpc` 		Submit a copy certificate of the service data segment
//! * `verify_in_vpc` 		Verify the replication certificate of the service data segment
//! * `submit_to_vpd` 		Submit spatio-temporal proof of service data segment
//! * `verify_in_vpd` 		Verify the spatio-temporal proof of service data segments

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
mod benchmarking;
pub mod weights;
use sp_runtime::{
	RuntimeDebug,
	traits::{SaturatedConversion},
};

use sp_std::prelude::*;
use codec::{Encode, Decode};
use frame_support::{
	dispatch::DispatchResult,
	PalletId,
	traits::{ReservableCurrency, Get, Randomness,},
};
use scale_info::TypeInfo;
pub use weights::WeightInfo;
type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;

/// The custom struct for storing info of proofs in VPA. idle segment
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoVPA<T: pallet::Config> {
	//When is_ready changed to true, the corresponding spatiotemporal proof can be submitted
	is_ready: bool,
	//false for 8M segment, true for 512M segment
	size_type: u128,
	proof: Option<Vec<u8>>,
	sealed_cid: Option<Vec<u8>>,
	rand: u32,
	block_num: Option<BlockNumberOf<T>>,
}

/// The custom struct for storing info of proofs in PPA.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoPPA<T: pallet::Config> {
	//1 for 8M segment, 2 for 512M segment
	size_type: u128,
	proof: Option<Vec<u8>>,
	sealed_cid: Option<Vec<u8>>,
	block_num: Option<BlockNumberOf<T>>,
}

///Used to provide random numbers for miners
///Miners generate certificates based on random numbers and submit them
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
pub struct ParamInfo {
	peer_id: u64,
	segment_id: u64,
	rand: u32,
}

///When the VPA is submitted, and the verification is passed
///Miners began to submit idle time and space certificates
///The data is stored in the structure
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoVPB<T: pallet::Config> {
	//When is_ready changed to true, the corresponding spatiotemporal proof can be submitted
	is_ready: bool,
	//1 for 8M segment, 2 for 512M segment
	size_type: u128,
	proof: Option<Vec<u8>>,
	sealed_cid: Option<Vec<u8>>,
	rand: u32,
	block_num: Option<BlockNumberOf<T>>,
}

//The spatiotemporal proof of validation is stored in the modified structure
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoPPB<T: pallet::Config> {
	//1 for 8M segment, 2 for 512M segment
	size_type: u128,
	proof: Option<Vec<u8>>,
	sealed_cid: Option<Vec<u8>>,
	block_num: Option<BlockNumberOf<T>>,
}

//Service copy certificate
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoVPC<T: pallet::Config> {
	//When is_ready changed to true, the corresponding spatiotemporal proof can be submitted
	is_ready: bool,
	//1 for 8M segment, 2 for 512M segment
	size_type: u128,
	proof: Option<Vec<Vec<u8>>>,
	sealed_cid: Option<Vec<Vec<u8>>>,
	rand: u32,
	block_num: Option<BlockNumberOf<T>>,
}

//Verification certificate
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoPPC<T: pallet::Config> {
	//1 for 8M segment, 2 for 512M segment
	size_type: u128,
	proof: Option<Vec<Vec<u8>>>,
	sealed_cid: Option<Vec<Vec<u8>>>,
	block_num: Option<BlockNumberOf<T>>,
}

//Submit spatiotemporal proof for proof of replication of service data segments
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoVPD<T: pallet::Config> {
	//When is_ready changed to true, the corresponding spatiotemporal proof can be submitted
	is_ready: bool,
	//1 for 8M segment, 2 for 512M segment
	size_type: u128,
	proof: Option<Vec<Vec<u8>>>,
	sealed_cid: Option<Vec<Vec<u8>>>,
	rand: u32,
	block_num: Option<BlockNumberOf<T>>,
}

//Verification Corresponding certificate
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ProofInfoPPD<T: pallet::Config> {
	//1 for 8M segment, 2 for 512M segment
	size_type: u128,
	proof: Option<Vec<Vec<u8>>>,
	sealed_cid: Option<Vec<Vec<u8>>>,
	block_num: Option<BlockNumberOf<T>>,
}

//A series of data required for polling
//The total number of blocks that store all the file data held by the miner

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct PeerFileNum {
	block_num: u128,
	total_num: u64,
}

//The scheduler reads the pool to know which data it should verify for AB
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct UnverifiedPool<T: pallet::Config> {
	acc: AccountOf<T>,
	peer_id: u64,
	segment_id: u64,
	proof: Vec<u8>,
	sealed_cid: Vec<u8>,
	rand: u32,
	size_type: u128,
}

//The scheduler reads the pool to know which data it should verify for C
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct UnverifiedPoolVec<T: pallet::Config> {
	acc: AccountOf<T>,
	peer_id: u64,
	segment_id: u64,
	//The file is sliced, and each slice generates a certificate
	proof: Vec<Vec<u8>>,
	sealed_cid: Vec<Vec<u8>>,
	uncid: Vec<Vec<u8>>,
	rand: u32,
	size_type: u128,
}

//The scheduler reads the pool to know which data it should verify for D
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct UnverifiedPoolVecD<T: pallet::Config> {
	acc: AccountOf<T>,
	peer_id: u64,
	segment_id: u64,
	//The file is sliced, and each slice generates a certificate
	proof: Vec<Vec<u8>>,
	sealed_cid: Vec<Vec<u8>>,
	rand: u32,
	size_type: u128,
}

//Miners read the pool to know which data segments they need to submit time-space certificates
// for AB
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ContinuousProofPool {
	peer_id: u64,
	segment_id: u64,
	sealed_cid: Vec<u8>,
	size_type: u128,
}
//Miners read the pool to know which data segments they need to submit time-space certificates
// for CD
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ContinuousProofPoolVec {
	peer_id: u64,
	segment_id: u64,
	sealed_cid: Vec<Vec<u8>>,
	hash: Vec<u8>,
	size_type: u128,
}

//The miner reads the pool to generate a duplicate certificate of service
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct FileSilceInfo {
	peer_id: u64,
	segment_id: u64,
	uncid: Vec<Vec<u8>>,
	rand: u32,
	hash: Vec<u8>,
	shardhash: Vec<u8>,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		ensure,
		pallet_prelude::*,
		traits::Get,
	};
	use frame_system::{ensure_signed, pallet_prelude::*};

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_sminer::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		//the weights
		type WeightInfo: WeightInfo;
		#[pallet::constant]
		/// The pallet id
		type MyPalletId: Get<PalletId>;
		/// randomness for seeds.
		type MyRandomness: Randomness<Self::Hash, Self::BlockNumber>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A series of params was generated.
		ParamSet(u64, u64, u32),
		/// vpa proof submitted.
		VPASubmitted(u64, u64),
		/// vpa proof verified.
		VPAVerified(u64, u64),
		// vpb proof submitted.
		VPBSubmitted(u64, u64),
		// vpb proof verified.
		VPBVerified(u64, u64),
		// vpc proof submitted.
		VPCSubmitted(u64, u64),
		// vpc proof verified.
		VPCVerified(u64, u64),
		// vpd proof submitted.
		VPDSubmitted(u64, u64),
		// vpd proof verified.
		VPDVerified(u64, u64),
		//The time certificate was not submitted on time
		PPBNoOnTimeSubmit(AccountOf<T>, u64),
		//The time certificate was not submitted on time
		PPDNoOnTimeSubmit(AccountOf<T>, u64),
		//checked error
		OverFlow(AccountOf<T>, u64),
		//for test update runtime
		TestTestHaHa(),
	}

	/// Error for the nicks pallet.
	#[pallet::error]
	pub enum Error<T> {
		//submit without intent-submit call
		NoIntentSubmitYet,
		//the one to verify was not exist in VPA
		NotExistInVPA,
		//the one to verify was not exist in VPB
		NotExistInVPB,
		//the one to verify was not exist in VPC
		NotExistInVPC,
		//the one to verify was not exist in VPD
		NotExistInVPD,
		//the one to verify was not ready in VPA
		NotReadyInVPA,
		//the one to verify was not ready in VPB
		NotReadyInVPB,
		//the one to verify was not ready in VPC
		NotReadyInVPC,
		//the one to verify was not ready in VPD
		NotReadyInVPD,
		//There is an error in the field submitted when intent // submit_type should input 1 or 2
		SubmitTypeError,
		//There is an error in the field submitted when intent // size_type should input 1 or 2
		SizeTypeError,
		//Already intent
		YetIntennt,
		//A copy certificate has been submitted
		LastSubmitUnVerfy,
		//checked error
		OverFlow,

		MinerUnExis,

		SegmentUnExis,
	}

	#[pallet::storage]
	#[pallet::getter(fn param_set_a)]
	pub(super) type ParamSetA<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ParamInfo>;

	#[pallet::storage]
	#[pallet::getter(fn param_set_b)]
	pub(super) type ParamSetB<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ParamInfo>;

	#[pallet::storage]
	#[pallet::getter(fn param_set_c)]
	pub(super) type ParamSetC<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ParamInfo>;

	#[pallet::storage]
	#[pallet::getter(fn param_set_d)]
	pub(super) type ParamSetD<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ParamInfo>;

	//It is used for miners to query themselves. It needs to provide spatiotemporal proof for those data segments
	#[pallet::storage]
	#[pallet::getter(fn con_proof_info_a)]
	pub(super) type ConProofInfoA<T: Config> = StorageMap<
		_,
		Twox64Concat,
		//peer acc
		T::AccountId,
		//segment id
		Vec<ContinuousProofPool>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn con_proof_info_c)]
	pub(super) type ConProofInfoC<T: Config> = StorageMap<
		_,
		Twox64Concat,
		//peer acc
		T::AccountId,
		//segment id
		Vec<ContinuousProofPoolVec>,
		ValueQuery,
	>;

	//One Accout total blocknumber use For polling
	#[pallet::storage]
	#[pallet::getter(fn block_number_a)]
	pub(super) type BlockNumberD<T: Config> = StorageMap<
		_,
		Twox64Concat,
		//peer acc
		T::AccountId,
		//segment id
		PeerFileNum,
	>;

	#[pallet::storage]
	#[pallet::getter(fn block_number_b)]
	pub(super) type BlockNumberB<T: Config> = StorageMap<
		_,
		Twox64Concat,
		//peer acc
		T::AccountId,
		//segment id
		PeerFileNum,
	>;
	//One Accout total blocknumber use For polling

	#[pallet::storage]
	#[pallet::getter(fn ver_pool_a)]
	pub(super) type VerPoolA<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		//peer acc
		T::AccountId,
		Twox64Concat,
		//segment id
		u64,
		ProofInfoVPA<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn pre_pool_a)]
	pub(super) type PrePoolA<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		//peer acc
		T::AccountId,
		Twox64Concat,
		//segment id
		u64,
		ProofInfoPPA<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn vre_pool_b)]
	pub(super) type VerPoolB<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		//peer acc
		T::AccountId,
		Twox64Concat,
		//segment id
		u64,
		ProofInfoVPB<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn pre_pool_b)]
	pub(super) type PrePoolB<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		//peer acc
		T::AccountId,
		Twox64Concat,
		//segment id
		u64,
		ProofInfoPPB<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn vre_pool_c)]
	pub(super) type VerPoolC<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		//peer acc
		T::AccountId,
		Twox64Concat,
		//segment id
		u64,
		ProofInfoVPC<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn pre_pool_c)]
	pub(super) type PrePoolC<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		//peer acc
		T::AccountId,
		Twox64Concat,
		//segment id
		u64,
		ProofInfoPPC<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn vre_pool_d)]
	pub(super) type VerPoolD<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		//peer acc
		T::AccountId,
		Twox64Concat,
		//segment id
		u64,
		ProofInfoVPD<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn pre_pool_d)]
	pub(super) type PrePoolD<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		//peer acc
		T::AccountId,
		Twox64Concat,
		//segment id
		u64,
		ProofInfoPPD<T>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn miner_hold_slice)]
	pub(super) type MinerHoldSlice<T: Config> = StorageMap<
		_,
		Twox64Concat,
		//peer acc
		T::AccountId,
		//segment id
		Vec<FileSilceInfo>,

		ValueQuery
	>;
	
	//Unverified pool ABCD
	//Vec<(T::Account, peer_id, segment_id, poof, sealed_cid, rand, size_type)>
	#[pallet::storage]
	#[pallet::getter(fn un_verified_a)]
	pub(super) type UnVerifiedA<T: Config> = StorageValue<_, Vec<UnverifiedPool<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn un_verified_b)]
	pub(super) type UnVerifiedB<T: Config> = StorageValue<_, Vec<UnverifiedPool<T>>, ValueQuery>;


	#[pallet::storage]
	#[pallet::getter(fn un_verified_c)]
	pub(super) type UnVerifiedC<T: Config> = StorageValue<_, Vec<UnverifiedPoolVec<T>>, ValueQuery>;


	#[pallet::storage]
	#[pallet::getter(fn un_verified_d)]
	pub(super) type UnVerifiedD<T: Config> = StorageValue<_, Vec<UnverifiedPoolVecD<T>>, ValueQuery>;
	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberOf<T>> for Pallet<T> {
		//Used to calculate whether it is implied to submit spatiotemporal proof
		//Cycle every 7.2 hours
		//When there is an uncommitted space-time certificate, the corresponding miner will be punished 
		//and the corresponding data segment will be removed
		fn on_initialize(now: BlockNumberOf<T>) -> Weight {
			let number: u128 = now.saturated_into();
			Self::deposit_event(Event::<T>::TestTestHaHa());
			if number % 4320 == 0 {
						for (acc, key2, res) in <PrePoolB<T>>::iter() {
							let blocknum2: u128 = res.block_num.unwrap().saturated_into();
							if number - 4320 > blocknum2 {
								let peerid = pallet_sminer::Pallet::<T>::get_peerid(&acc);
								let _ = pallet_sminer::Pallet::<T>::sub_power(peerid, res.size_type);
								<ConProofInfoA<T>>::mutate(&acc, |s|{
									for i in 0..s.len() {
										let v = s.get(i);
										if v.unwrap().segment_id == key2 {
											s.remove(i);
											break;
										}
									}
								});
								let mut unb = <UnVerifiedB<T>>::get();
								let mut k = 0;
								for i in unb.clone().iter() {
									if peerid == i.peer_id && key2 == i.segment_id {
										unb.remove(k);
										break;
									}
									k += 1;
								}
								<UnVerifiedB<T>>::put(unb);
								<PrePoolA<T>>::remove(&acc, key2);
								<PrePoolB<T>>::remove(&acc, key2);
								Self::deposit_event(Event::<T>::PPBNoOnTimeSubmit(acc.clone(), key2));
							}
						}
					
				//Polling for proof of service time and space
						for (acc, key2, res) in <PrePoolD<T>>::iter() {
							let blocknum2: u128 = res.block_num.unwrap().saturated_into();
							let peerid = pallet_sminer::Pallet::<T>::get_peerid(&acc);
							if number - 4320 > blocknum2 {
								let _ = pallet_sminer::Pallet::<T>::sub_power(peerid, res.size_type);
								let _ = pallet_sminer::Pallet::<T>::sub_space(peerid, res.size_type);
								<ConProofInfoC<T>>::mutate(&acc, |s|{
									for i in 0..s.len() {
										let v = s.get(i);
										if v.unwrap().segment_id == key2 {
											s.remove(i);
											break;
										}
									}
								});
								let mut unb = <UnVerifiedD<T>>::get();
								let mut k = 0;
								for i in unb.clone().iter() {
									if peerid == i.peer_id && key2 == i.segment_id {
										unb.remove(k);
										break;
									}
									k += 1;
								}
								<UnVerifiedD<T>>::put(unb);
								<PrePoolC<T>>::remove(&acc, key2);
								<PrePoolD<T>>::remove(&acc, key2);
								//let _ = pallet_sminer::Pallet::<T>::fine_money(&acc);
								Self::deposit_event(Event::<T>::PPDNoOnTimeSubmit(acc.clone(), key2));
							}
						}
			}
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		///In order to obtain the random number that generates the proof
		///The intent operation is required before the miner submits the copy certificate
		///submit_type A for type 1 and C for type 2
		///
		/// Parameters:
		/// - `segment_id`: Segment id
		/// - `size_type`: Segment size , value = 1 is 8 , value = 2 is 512
		/// - `submit_type`: Submission type, value = 1 for Idle copy proof , value = 2 for Service copy certificate
		/// - `uncid`: The miner generates the necessary parameters to prove, for Idle copy proof.
		/// - `hash`: The miner generates the necessary parameters to prove, for Idle copy proof.
		/// - `shardhash`: The miner generates the necessary parameters to prove, for Idle copy proof.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::intent_submit())]
		pub fn intent_submit(
			origin: OriginFor<T>, 
			size_type: u8, 
			submit_type: u8, 
			peerid: u64, 
			uncid: Vec<Vec<u8>>, 
			hash: Vec<u8>, 
			shardhash: Vec<u8>
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let random = Self::generate_random_number(20211109);
			let size: u128 = match size_type {
				1 => 8,
				2 => 512,
				_ => 0,

			};
			if 0 == size {
				ensure!(false, Error::<T>::SizeTypeError);
			}
			match submit_type {
				1u8 => {
					let (peer_id, segment_id) = pallet_sminer::Pallet::<T>::get_ids(&sender)?;
					ensure!(!<VerPoolA<T>>::contains_key(&sender, segment_id), Error::<T>::YetIntennt);
					<VerPoolA<T>>::insert(
						&sender,
						segment_id,
						ProofInfoVPA {
							is_ready: false,
							size_type: size,
							proof: None,
							sealed_cid: None,
							rand: random,
							block_num: None,
						}
					);
					<ParamSetA<T>>::insert(
						&sender,
						ParamInfo {
							peer_id,
							segment_id,
							rand: random,
						}
					);
					Self::deposit_event(Event::<T>::ParamSet(peer_id, segment_id, random));
				}
				2u8 => {
					let acc = pallet_sminer::Pallet::<T>::get_acc(peerid);
					let segment_id = pallet_sminer::Pallet::<T>::get_segmentid(&acc)?;
					ensure!(!<VerPoolC<T>>::contains_key(&acc, segment_id), Error::<T>::YetIntennt);
					<VerPoolC<T>>::insert(
						&acc,
						segment_id,
						ProofInfoVPC {
							is_ready: false,
							size_type: size,
							proof: None,
							sealed_cid: None,
							rand: random,
							block_num: None,
						}
					);
					let silce_info = FileSilceInfo {
						peer_id: peerid,
						segment_id: segment_id,
						uncid: uncid,
						rand: random,
						hash: hash,
						shardhash: shardhash,
					};
					if <MinerHoldSlice<T>>::contains_key(&acc) {
						<MinerHoldSlice<T>>::mutate(&acc, |s| (*s).push(silce_info));
					} else {
						let mut value: Vec<FileSilceInfo> = Vec::new();
						value.push(silce_info);
						<MinerHoldSlice<T>>::insert(
							&acc,
							value
						)
					}
					
				}
				_ => {
					ensure!(false, Error::<T>::SubmitTypeError);
				}
			}	
			Ok(())
		}

		///In order to obtain the random number that generates the proof
		///Miners need to conduct intent before submitting space-time certificate_ po_ st operation
		///submit_type B for type 1 and D for type 2
		///
		/// Parameters:
		/// - `segment_id`: Segment id
		/// - `size_type`: Segment size , value = 1 is 8 , value = 2 is 512
		/// - `submit_type`: Submission type, value = 1 for Idle space-time proof , value = 2 for Service space-time certificate
		#[pallet::weight(<T as pallet::Config>::WeightInfo::intent_submit_po_st())]
		pub fn intent_submit_po_st(origin: OriginFor<T>, segment_id: u64, size_type: u8, submit_type: u8) -> DispatchResult {
			//PoSt intent
			let sender = ensure_signed(origin)?;

			let random = Self::generate_random_number(20211109);
			let peer_id = pallet_sminer::Pallet::<T>::get_peerid(&sender);
			let size: u128 = match size_type {
				1 => 8,
				2 => 512,
				_ => 0,

			};
			if size == 0 {
				ensure!(false, Error::<T>::SizeTypeError);
			}
			
			match submit_type {
				1u8 => {
					ensure!(!<VerPoolB<T>>::contains_key(&sender, segment_id), Error::<T>::YetIntennt);
					ensure!(<PrePoolB<T>>::contains_key(&sender, segment_id), Error::<T>::SegmentUnExis);
					<VerPoolB<T>>::insert(
						&sender,
						segment_id,
						ProofInfoVPB {
							is_ready: false,
							size_type: size,
							proof: None,
							sealed_cid: None,
							rand: random,
							block_num: None,
						}
					);
					<ParamSetB<T>>::insert(
						&sender,
						ParamInfo {
							peer_id,
							segment_id,
							rand: random,
						}
					);
				}
				2u8 => {
					ensure!(!<VerPoolD<T>>::contains_key(&sender, segment_id), Error::<T>::YetIntennt);
					ensure!(<PrePoolD<T>>::contains_key(&sender, segment_id), Error::<T>::SegmentUnExis);
					<VerPoolD<T>>::insert(
						&sender,
						segment_id,
						ProofInfoVPD {
							is_ready: false,
							size_type: size,
							sealed_cid: None,
							proof: None,
							rand: random,
							block_num: None,
						}
					);
					<ParamSetD<T>>::insert(
						&sender,
						ParamInfo {
							peer_id,
							segment_id,
							rand: random,
						}
					);
				}
				_ => {
					ensure!(false, Error::<T>::SubmitTypeError);
				}
			}
			Self::deposit_event(Event::<T>::ParamSet(peer_id, segment_id, random));
			Ok(())
		}

		///Submit copy certificate of idle data segment.
		///
		/// Parameters:
		///  - `peer_id`: Miner's ID.
		///  - `segment_id`: Segment ID.
		///  - `proof`: proof.
		///  - `sealed_cid`: Required for verification certificate.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::submit_to_vpa())]
		pub fn submit_to_vpa(origin: OriginFor<T>, peer_id: u64, segment_id: u64, proof: Vec<u8>, sealed_cid: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<VerPoolA<T>>::contains_key(&sender, segment_id), Error::<T>::NoIntentSubmitYet);
			VerPoolA::<T>::mutate(&sender, segment_id, |s_opt| {
				let s = s_opt.as_mut().unwrap();
				s.is_ready = true;
				s.proof = Some(proof.clone());
				s.sealed_cid = Some(sealed_cid.clone());
				s.block_num = Some(<frame_system::Pallet<T>>::block_number());
				let x = UnverifiedPool{
					acc: sender.clone(), 
					peer_id: peer_id, 
					segment_id: segment_id, 
					proof: proof.clone(), 
					sealed_cid: sealed_cid.clone(), 
					rand: s.rand,
					size_type: s.size_type,
				};
				UnVerifiedA::<T>::mutate(|a| (*a).push(x));
			});
			Self::deposit_event(Event::<T>::VPASubmitted(peer_id, segment_id));
			Ok(())
		}

		///Verify replication proof of idle data segments
		/// 
		/// Parameters:
		///  - `peer_id`: Miner's ID.
		///  - `segment_id`: Segment ID.
		///  - `result`: Verification results, true or false.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::verify_in_vpa())]
		pub fn verify_in_vpa(origin: OriginFor<T>, peer_id: u64, segment_id: u64, result: bool) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			let sender = pallet_sminer::Pallet::<T>::get_acc(peer_id);

			ensure!(<VerPoolA<T>>::contains_key(&sender, segment_id), Error::<T>::NotExistInVPA);

			let vpa = VerPoolA::<T>::get(&sender, segment_id).unwrap();

			ensure!(vpa.is_ready, Error::<T>::NotReadyInVPA);
			
			if result {
				<PrePoolA<T>>::insert(
					&sender,
					segment_id,
					ProofInfoPPA {
						size_type: vpa.size_type,
						proof: vpa.proof,
						sealed_cid: vpa.sealed_cid,
						block_num: Some(<frame_system::Pallet<T>>::block_number()),
					}
				);
				<PrePoolB<T>>::insert(
					&sender,
					segment_id,
					ProofInfoPPB {
						size_type: vpa.size_type.clone(),
						proof: None,
						sealed_cid: None,
						block_num: Some(<frame_system::Pallet<T>>::block_number()),
					}
				);
				if !(<BlockNumberB<T>>::contains_key(&sender)) {
					<BlockNumberB<T>>::insert(
						&sender,
						PeerFileNum {
							block_num: <frame_system::Pallet<T>>::block_number().saturated_into(),
							total_num: 1,
						}
					);
				} else {
					<BlockNumberB<T>>::try_mutate(&sender, |s_opt| -> DispatchResult {
						let now: u128 = <frame_system::Pallet<T>>::block_number().saturated_into();
						let s = s_opt.as_mut().unwrap();
						s.block_num = s.block_num.checked_add(now).ok_or(Error::<T>::OverFlow)?;
						s.total_num = s.total_num.checked_add(1).ok_or(Error::<T>::OverFlow)?;
						Ok(())
					})?;
				}
				let _ = pallet_sminer::Pallet::<T>::add_power(peer_id, vpa.size_type)?;
				let size = vpa.size_type;
				let bo = VerPoolA::<T>::get(&sender, segment_id).unwrap();
				let sealed_cid = bo.sealed_cid.unwrap();
				if <ConProofInfoA<T>>::contains_key(&sender) {
					<ConProofInfoA<T>>::mutate(sender.clone(), |v| {
						let value = ContinuousProofPool {
							peer_id: peer_id,
							segment_id: segment_id,
							sealed_cid: sealed_cid,
							size_type: size,
						};
						(*v).push(value);
					});
				} else {
					let mut v: Vec<ContinuousProofPool> = Vec::new();
					let value = ContinuousProofPool {
						peer_id: peer_id,
						segment_id: segment_id,
						sealed_cid: sealed_cid,
						size_type: size,
					};
					v.push(value);
					<ConProofInfoA<T>>::insert(
						&sender,
						v
					);
				}
			}

			<VerPoolA<T>>::remove(&sender, segment_id);

			let ua = UnVerifiedA::<T>::get();
			let res = Self::unverify_remove(ua, peer_id, segment_id);
			UnVerifiedA::<T>::put(res);

			Self::deposit_event(Event::<T>::VPAVerified(peer_id, segment_id));
			Ok(())
		}

		//Submit space-time proof of idle data segments
		/// Parameters:
		///  - `peer_id`: Miner's ID.
		///  - `segment_id`: Segment ID.
		///  - `proof`: proof.
		///  - `sealed_cid`: Required for verification certificate.
		#[pallet::weight(<T as pallet::Config>::WeightInfo::submit_to_vpb())]
		pub fn submit_to_vpb(origin: OriginFor<T>, peer_id: u64, segment_id: u64, proof: Vec<u8>, sealed_cid: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<VerPoolB<T>>::contains_key(&sender, segment_id), Error::<T>::NoIntentSubmitYet);

			VerPoolB::<T>::mutate(&sender, segment_id, |s_opt| {
				let s = s_opt.as_mut().unwrap();
				s.is_ready = true;
				s.proof = Some(proof.clone());
				s.sealed_cid = Some(sealed_cid.clone());
				s.block_num = Some(<frame_system::Pallet<T>>::block_number());
				let x = UnverifiedPool{
					acc: sender.clone(), 
					peer_id: peer_id, 
					segment_id: segment_id, 
					proof: proof.clone(), 
					sealed_cid: sealed_cid.clone(), 
					rand: (*s).rand, 
					size_type: (*s).size_type,
				};
				UnVerifiedB::<T>::mutate(|a| (*a).push(x));
			});

			Self::deposit_event(Event::<T>::VPBSubmitted(peer_id, segment_id));	
			Ok(())
		}

		//B: Verify the spatiotemporal proof of idle data segments
		/// Parameters:
		///  - `peer_id`: Miner's ID.
		///  - `segment_id`: Segment ID.
		///  - `result`: Verification results, true or false.		
		#[pallet::weight(<T as pallet::Config>::WeightInfo::verify_in_vpb())]
		pub fn verify_in_vpb(origin: OriginFor<T>, peer_id: u64, segment_id: u64, result: bool) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			let sender = pallet_sminer::Pallet::<T>::get_acc(peer_id);

			let now_block = <frame_system::Pallet<T>>::block_number();
			let now: u128 = now_block.saturated_into();
			ensure!(<VerPoolB<T>>::contains_key(&sender, segment_id), Error::<T>::NoIntentSubmitYet);
			ensure!(<PrePoolB<T>>::contains_key(&sender, segment_id), Error::<T>::NoIntentSubmitYet);
			let vpb = VerPoolB::<T>::get(&sender, segment_id).unwrap();

			ensure!(vpb.is_ready, Error::<T>::NotReadyInVPB);
			
			if result {
					PrePoolB::<T>::try_mutate(&sender, segment_id, |s_opt| -> DispatchResult {
						let s = s_opt.as_mut().unwrap();
						let last_blocknum: u128 = s.block_num.unwrap().saturated_into();
						<BlockNumberB<T>>::try_mutate(&sender, |a_opt| -> DispatchResult {
							let a = a_opt.as_mut().unwrap();
							a.block_num = a.block_num.checked_sub(last_blocknum).ok_or(Error::<T>::OverFlow)?;
							a.block_num = a.block_num.checked_add(now).ok_or(Error::<T>::OverFlow)?;
							Ok(())
						})?;
						s.proof = Some(vpb.proof.unwrap());
						s.sealed_cid = Some(vpb.sealed_cid.unwrap());
						s.block_num = Some(now_block);
						Ok(())
					})?;
			}
			<VerPoolB<T>>::remove(&sender, segment_id);

			let ua = UnVerifiedB::<T>::get();
			let res = Self::unverify_remove(ua, peer_id, segment_id);
			UnVerifiedB::<T>::put(res);

			Self::deposit_event(Event::<T>::VPBVerified(peer_id, segment_id));
			Ok(())
		}

		///C Submit a copy certificate of the service data segment
		/// Parameters:
		///  - `peer_id`: Miner's ID.
		///  - `segment_id`: Segment ID.
		///  - `proof`: proof, It could be a slice, so multiple proofs
		///  - `sealed_cid`: Required for verification certificate, It could be a slice, so multiple proofs
		#[pallet::weight(<T as pallet::Config>::WeightInfo::submit_to_vpc())]
		pub fn submit_to_vpc(origin: OriginFor<T>, peer_id: u64, segment_id: u64, proof: Vec<Vec<u8>>, sealed_cid: Vec<Vec<u8>>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<VerPoolC<T>>::contains_key(&sender, segment_id), Error::<T>::NoIntentSubmitYet);
			
			VerPoolC::<T>::mutate(&sender, segment_id, |s_opt| {
				let s = s_opt.as_mut().unwrap();
				s.is_ready = true;
				s.proof = Some(proof.clone());
				s.sealed_cid = Some(sealed_cid.clone());
				s.block_num = Some(<frame_system::Pallet<T>>::block_number());
			});

			let mut flag: bool = false;
			let unc = UnVerifiedC::<T>::get();
			for i in unc.iter() {
				if i.peer_id == peer_id && i.segment_id == segment_id {
					flag = true;
					break;
				}
			}
			if flag {
				ensure!(false, Error::<T>::LastSubmitUnVerfy);
			} else {
				let v = VerPoolC::<T>::get(&sender, segment_id).unwrap();
				let value = MinerHoldSlice::<T>::get(&sender);
				let mut uncid: Vec<Vec<u8>> = Vec::new();
				for i in value {
					if i.segment_id == segment_id {
						uncid = i.uncid;
					}
				}
				let x = UnverifiedPoolVec{
					acc: sender.clone(), 
					peer_id: peer_id, 
					segment_id: segment_id, 
					proof: proof.clone(), 
					sealed_cid: sealed_cid.clone(), 
					uncid: uncid,
					rand: v.rand, 
					size_type: v.size_type,
				};
				UnVerifiedC::<T>::mutate(|a| (*a).push(x));
			}
			

			Self::deposit_event(Event::<T>::VPCSubmitted(peer_id, segment_id));	
			Ok(())
		}

		///Verify the replication certificate of the service data segment
		/// Parameters:
		///  - `peer_id`: Miner's ID.
		///  - `segment_id`: Segment ID.
		///  - `uncid`: Used to remove validated data from the pool
		///  - `result`: Verification results
		#[pallet::weight(<T as pallet::Config>::WeightInfo::verify_in_vpc())]
		pub fn verify_in_vpc(origin: OriginFor<T>, peer_id: u64, segment_id: u64, _uncid: Vec<Vec<u8>>, result: bool) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			let sender = pallet_sminer::Pallet::<T>::get_acc(peer_id);
			ensure!(<VerPoolC<T>>::contains_key(&sender, segment_id), Error::<T>::NotExistInVPC);

			let vpc = VerPoolC::<T>::get(&sender, segment_id).unwrap();

			ensure!(vpc.is_ready, Error::<T>::NotReadyInVPC);
			
			if result {
				<PrePoolC<T>>::insert(
					&sender,
					segment_id,
					ProofInfoPPC {
						//false for 8M segment, true for 512M segment
						size_type: vpc.size_type,
						proof: vpc.proof,
						sealed_cid: vpc.sealed_cid,
						block_num: Some(<frame_system::Pallet<T>>::block_number()),
					}
				);
				<PrePoolD<T>>::insert(
					&sender,
					segment_id,
					ProofInfoPPD {
						size_type: vpc.size_type.clone(),
						proof: None,
						sealed_cid: None,
						block_num: Some(<frame_system::Pallet<T>>::block_number()),
					}
				);
				if !(<BlockNumberD<T>>::contains_key(&sender)) {
					<BlockNumberD<T>>::insert(
						&sender,
						PeerFileNum {
							block_num: <frame_system::Pallet<T>>::block_number().saturated_into(),
							total_num: 1,
						}
					);
				} else {
					let now: u128 = <frame_system::Pallet<T>>::block_number().saturated_into();
					<BlockNumberD<T>>::try_mutate(&sender, |s_opt| -> DispatchResult {
						let s = s_opt.as_mut().unwrap();
						s.block_num = s.block_num.checked_add(now).ok_or(Error::<T>::OverFlow)?;
						s.total_num = s.total_num.checked_add(1).ok_or(Error::<T>::OverFlow)?;
						Ok(())
					})?;
				}
				let _ = pallet_sminer::Pallet::<T>::add_power(peer_id, vpc.size_type)?;
				let _ = pallet_sminer::Pallet::<T>::add_space(peer_id, vpc.size_type)?;

				let size = vpc.size_type;
				let bo = VerPoolC::<T>::get(&sender, segment_id).unwrap();
				let sealed_cid = bo.sealed_cid.unwrap();

				let mut hash: Vec<u8> = Vec::new();
				let value = MinerHoldSlice::<T>::get(&sender);
				for i in value {
					if i.segment_id == segment_id {
						hash = i.hash;
					}
				}

				if <ConProofInfoC<T>>::contains_key(&sender) {
					<ConProofInfoC<T>>::mutate(sender.clone(), |v|{
						let value = ContinuousProofPoolVec {
							peer_id: peer_id,
							segment_id: segment_id,
							sealed_cid: sealed_cid.clone(),
							hash: hash,
							size_type: size,
						};
						(*v).push(value)
					});
				} else {
					let mut v: Vec<ContinuousProofPoolVec> = Vec::new();
					let value = ContinuousProofPoolVec {
						peer_id: peer_id,
						segment_id: segment_id,
						sealed_cid: sealed_cid,
						hash: hash,
						size_type: size,
					};
					v.push(value);
					<ConProofInfoC<T>>::insert(
						&sender,
						v
					);
				}

				if <MinerHoldSlice<T>>::contains_key(&sender) {
					let mut k = 0;
					let mut accfiles = <MinerHoldSlice<T>>::get(&sender);
					for i in accfiles.iter() {
						if i.segment_id == segment_id {
							break;
						}
						k += 1;
					}
					accfiles.remove(k);
					<MinerHoldSlice<T>>::insert(&sender, accfiles);
				}
				<VerPoolC<T>>::remove(&sender, segment_id);
			} else {
				<VerPoolC<T>>::mutate(&sender, segment_id, |x_opt| {
					let s = x_opt.as_mut().unwrap();
					s.is_ready = false;
					s.proof = None;
					s.sealed_cid = None;
					s.block_num = None;
				});
			}

			let ua = UnVerifiedC::<T>::get();
			let res = Self::unverify_remove_vec(ua, peer_id, segment_id);
			UnVerifiedC::<T>::put(res);

			Self::deposit_event(Event::<T>::VPCVerified(peer_id, segment_id));
		
			Ok(())
		
		}

		///D: Submit spatio-temporal proof of service data segment
		/// Parameters:
		///  - `peer_id`: Miner's ID.
		///  - `segment_id`: Segment ID.
		///  - `proof`: proof, It could be a slice, so multiple proofs
		///  - `sealed_cid`: Required for verification certificate, It could be a slice, so multiple proofs
		#[pallet::weight(<T as pallet::Config>::WeightInfo::submit_to_vpd())]
		pub fn submit_to_vpd(origin: OriginFor<T>, peer_id: u64, segment_id: u64, proof: Vec<Vec<u8>>, sealed_cid: Vec<Vec<u8>>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let now_block = <frame_system::Pallet<T>>::block_number();
			ensure!(<VerPoolD<T>>::contains_key(&sender, segment_id), Error::<T>::NoIntentSubmitYet);

			VerPoolD::<T>::mutate(&sender, segment_id, |s_opt| {
				let s = s_opt.as_mut().unwrap();
				s.is_ready = true;
				s.proof = Some(proof.clone());
				s.sealed_cid = Some(sealed_cid.clone());
				s.block_num = Some(now_block);
				let x = UnverifiedPoolVecD{
					acc: sender.clone(), 
					peer_id: peer_id, 
					segment_id: segment_id, 
					proof: proof.clone(), 
					sealed_cid: sealed_cid.clone(), 
					rand: (*s).rand, 
					size_type: (*s).size_type,
				};
				UnVerifiedD::<T>::mutate(|a| (*a).push(x));
			});

			Self::deposit_event(Event::<T>::VPDSubmitted(peer_id, segment_id));	
			Ok(())
		}

		//Verify the spatio-temporal proof of service data segments
		/// Parameters:
		///  - `peer_id`: Miner's ID.
		///  - `segment_id`: Segment ID.
		///  - `result`: Verification results 
		#[pallet::weight(<T as pallet::Config>::WeightInfo::verify_in_vpd())]
		pub fn verify_in_vpd(origin: OriginFor<T>, peer_id: u64, segment_id: u64, result: bool) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			let sender = pallet_sminer::Pallet::<T>::get_acc(peer_id);
			let now_block = <frame_system::Pallet<T>>::block_number();
			let now: u128 = <frame_system::Pallet<T>>::block_number().saturated_into();
			ensure!(<VerPoolD<T>>::contains_key(&sender, segment_id), Error::<T>::NotExistInVPD);

			let vpd = VerPoolD::<T>::get(&sender, segment_id).unwrap();

			ensure!(vpd.is_ready, Error::<T>::NotReadyInVPD);
			
			if result {
				PrePoolD::<T>::try_mutate(&sender, segment_id, |s_opt| -> DispatchResult {
					let s = s_opt.as_mut().unwrap();
					let last: u128 = s.block_num.unwrap().saturated_into();
					<BlockNumberD<T>>::try_mutate(&sender, |a_opt| -> DispatchResult {
						let a = a_opt.as_mut().unwrap();
						a.block_num = a.block_num.checked_sub(last).ok_or(Error::<T>::OverFlow)?;
						a.block_num = a.block_num.checked_add(now).ok_or(Error::<T>::OverFlow)?;
						Ok(())
					})?;
					s.proof = Some(vpd.proof.unwrap());
					s.sealed_cid = Some(vpd.sealed_cid.unwrap());
					s.block_num = Some(now_block);
					Ok(())
				})?;
			}
			<VerPoolD<T>>::remove(&sender, segment_id);

			let ua = UnVerifiedD::<T>::get();
			let res = Self::unverify_remove_vec_d(ua, peer_id, segment_id);
			UnVerifiedD::<T>::put(res);

			Self::deposit_event(Event::<T>::VPDVerified(peer_id, segment_id));
			Ok(())
		}
	}
}

pub trait UnVerify {
	fn get_peerid(&self) -> u64;
	fn get_segmentid(&self) -> u64;
}

impl<T: Config> UnVerify for UnverifiedPool<T> {
	fn get_peerid(&self) -> u64 {
		self.peer_id
	}
	fn get_segmentid(&self) -> u64 {
		self.segment_id
	}
}

impl<T: Config> UnVerify for UnverifiedPoolVec<T> {
	fn get_peerid(&self) -> u64 {
		self.peer_id
	}
	fn get_segmentid(&self) -> u64 {
		self.segment_id
	}
}

impl<T: Config> UnVerify for UnverifiedPoolVecD<T> {
	fn get_peerid(&self) -> u64 {
		self.peer_id
	}
	fn get_segmentid(&self) -> u64 {
		self.segment_id
	}
}

impl<T: Config> Pallet<T> {
	// Generate a random number from a given seed.
	fn generate_random_number(seed: u32) -> u32 {
		let (random_seed, _) = T::MyRandomness::random(&(T::MyPalletId::get(), seed).encode());
		let random_number = <u32>::decode(&mut random_seed.as_ref())
			.expect("secure hashes should always be bigger than u32; qed");
		random_number
	}
	//Remove eligible data from VEC
	fn _unverify_usually<U: UnVerify>(mut list: Vec<U>, peer_id: u64, segment_id:u64) -> Vec<U> {
		let mut k = 0;
		for i in list.iter() {
			if (*i).get_peerid() == peer_id && (*i).get_segmentid() == segment_id {
				break;
			}
			k += 1;
		}
		list.remove(k);
		list
	}

	fn unverify_remove(mut list: Vec<UnverifiedPool<T>>, peer_id: u64, segment_id: u64) -> Vec<UnverifiedPool<T>>{
		let mut k = 0;
		for i in list.iter() {
			if i.peer_id == peer_id && i.segment_id == segment_id {
				break;
			}
			k += 1;
		}
		list.remove(k);
		list
	}
	//Remove eligible data from VEC
	fn unverify_remove_vec(mut list: Vec<UnverifiedPoolVec<T>>, peer_id: u64, segment_id: u64) -> Vec<UnverifiedPoolVec<T>>{
		let mut k = 0;
		for i in list.iter() {
			if i.peer_id == peer_id && i.segment_id == segment_id {
				break;
			}
			k += 1;
		}
		list.remove(k);
		list
	}
	//Remove eligible data from VEC
	fn unverify_remove_vec_d(mut list: Vec<UnverifiedPoolVecD<T>>, peer_id: u64, segment_id: u64) -> Vec<UnverifiedPoolVecD<T>>{
		let mut k = 0;
		for i in list.iter() {
			if i.peer_id == peer_id && i.segment_id == segment_id {
				break;
			}
			k += 1;
		}
		list.remove(k);
		list
	}

}


