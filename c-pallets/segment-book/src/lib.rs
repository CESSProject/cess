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

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub use pallet::*;
mod benchmarking;
pub mod weights;
use sp_runtime::{
	RuntimeDebug,
	traits::{SaturatedConversion},
};
mod types;
use types::*;

use sp_std::prelude::*;
use codec::{Encode, Decode};
use frame_support::{
	storage::bounded_vec::BoundedVec,
	dispatch::DispatchResult,
	PalletId,
	traits::{ReservableCurrency, Get, Randomness,},
};
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
pub use weights::WeightInfo;
type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;



#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		ensure,
		
		traits::Get,
	};
	use frame_system::{ensure_signed, pallet_prelude::*};

	pub type BoundedString<T> = BoundedVec<u8, <T as Config>::SegStringLimit>;
	pub type BoundedList<T> = BoundedVec<BoundedVec<u8, <T as Config>::SegStringLimit>, <T as Config>::SegStringLimit>;

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_sminer::Config + pallet_file_bank::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency trait.
		type Currency: ReservableCurrency<Self::AccountId>;

		//the weights
		type WeightInfo: WeightInfo;
		#[pallet::constant]
		/// The pallet id
		type MyPalletId: Get<PalletId>;

		#[pallet::constant]
		type SegStringLimit: Get<u32>;
		/// randomness for seeds.
		type MyRandomness: Randomness<Self::Hash, Self::BlockNumber>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A series of params was generated.
		ParamSet{peer_id: u64, segment_id: u64, random: u32},
		/// vpa proof submitted.
		VPASubmitted{peer_id: u64, segment_id: u64},
		/// vpa proof verified.
		VPAVerified{peer_id: u64, segment_id: u64},
		// vpb proof submitted.
		VPBSubmitted{peer_id: u64, segment_id: u64},
		// vpb proof verified.
		VPBVerified{peer_id: u64, segment_id: u64},
		// vpc proof submitted.
		VPCSubmitted{peer_id: u64, segment_id: u64},
		// vpc proof verified.
		VPCVerified{peer_id: u64, segment_id: u64},
		// vpd proof submitted.
		VPDSubmitted{peer_id: u64, segment_id: u64},
		// vpd proof verified.
		VPDVerified{peer_id: u64, segment_id: u64},
		//The time certificate was not submitted on time
		PPBNoOnTimeSubmit{acc: AccountOf<T>, segment_id: u64},
		//The time certificate was not submitted on time
		PPDNoOnTimeSubmit{acc: AccountOf<T>, segment_id: u64},
		//for test update runtime
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

		FileNonExistent,
	}

	// #[pallet::storage]
	// #[pallet::getter(fn param_set_a)]
	// pub(super) type ParamSetA<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ParamInfo>;

	// #[pallet::storage]
	// #[pallet::getter(fn param_set_b)]
	// pub(super) type ParamSetB<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ParamInfo>;

	// #[pallet::storage]
	// #[pallet::getter(fn param_set_c)]
	// pub(super) type ParamSetC<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ParamInfo>;

	// #[pallet::storage]
	// #[pallet::getter(fn param_set_d)]
	// pub(super) type ParamSetD<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ParamInfo>;

	//It is used for miners to query themselves. It needs to provide spatiotemporal proof for those data segments
	#[pallet::storage]
	#[pallet::getter(fn con_proof_info_a)]
	pub(super) type ConProofInfoA<T: Config> = StorageMap<
		_,
		Twox64Concat,
		//peer acc
		T::AccountId,
		//segment id
		BoundedVec<ContinuousProofPool<BoundedString<T>>, T::SegStringLimit>,
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
		BoundedVec<ContinuousProofPoolVec<BoundedString<T>, BoundedList<T>>, T::SegStringLimit>,
		ValueQuery,
	>;

	//One Accout total blocknumber use For polling

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
		ProofInfoVPA<BoundedString<T>, BlockNumberOf<T>>,
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
		ProofInfoPPA<BoundedString<T>, BlockNumberOf<T>>,
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
		ProofInfoVPB<BoundedString<T>, BlockNumberOf<T>>,
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
		ProofInfoPPB<BoundedString<T>, BlockNumberOf<T>>,
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
		ProofInfoVPC<BoundedList<T>, BlockNumberOf<T>>,
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
		ProofInfoPPC<BoundedList<T>, BlockNumberOf<T>>,
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
		ProofInfoVPD<BoundedList<T>, BlockNumberOf<T>>,
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
		ProofInfoPPD<BoundedList<T>, BlockNumberOf<T>>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn miner_hold_slice)]
	pub(super) type MinerHoldSlice<T: Config> = StorageMap<
		_,
		Twox64Concat,
		//peer acc
		T::AccountId,
		//segment id
		BoundedVec<FileSilceInfo<BoundedString<T>, BoundedList<T>>, T::SegStringLimit>,

		ValueQuery
	>;
	
	//Unverified pool ABCD
	//Vec<(T::Account, peer_id, segment_id, poof, sealed_cid, rand, size_type)>
	#[pallet::storage]
	#[pallet::getter(fn un_verified_a)]
	pub(super) type UnVerifiedA<T: Config> = StorageValue<_, BoundedVec<UnverifiedPool<AccountOf<T>, BoundedString<T>>, T::SegStringLimit>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn un_verified_b)]
	pub(super) type UnVerifiedB<T: Config> = StorageValue<_, BoundedVec<UnverifiedPool<AccountOf<T>, BoundedString<T>>, T::SegStringLimit>, ValueQuery>;


	#[pallet::storage]
	#[pallet::getter(fn un_verified_c)]
	pub(super) type UnVerifiedC<T: Config> = StorageValue<_, BoundedVec<UnverifiedPoolVec<AccountOf<T>, BoundedList<T>>, T::SegStringLimit>, ValueQuery>;


	#[pallet::storage]
	#[pallet::getter(fn un_verified_d)]
	pub(super) type UnVerifiedD<T: Config> = StorageValue<_, BoundedVec<UnverifiedPoolVecD<AccountOf<T>, BoundedList<T>>, T::SegStringLimit>, ValueQuery>;

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
			if number % 14400 == 0 {
						for (acc, key2, res) in <PrePoolB<T>>::iter() {
							let blocknum2: u128 = res.block_num.unwrap().saturated_into();
							if number - 14400 > blocknum2 {
								let peerid = pallet_sminer::Pallet::<T>::get_peerid(&acc);
								let _ = pallet_sminer::Pallet::<T>::sub_power(peerid, res.size_type);
								let _ = pallet_sminer::Pallet::<T>::sub_available_space(res.size_type);
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
								<VerPoolB<T>>::remove(&acc, key2);
								<PrePoolB<T>>::remove(&acc, key2);
								Self::deposit_event(Event::<T>::PPBNoOnTimeSubmit{acc: acc.clone(), segment_id: key2});
							}
						}
					
				//Polling for proof of service time and space
						for (acc, key2, res) in <PrePoolD<T>>::iter() {
							let blocknum2: u128 = res.block_num.unwrap().saturated_into();
							let peerid = pallet_sminer::Pallet::<T>::get_peerid(&acc);
							if number - 14400 > blocknum2 {
								let _ = pallet_sminer::Pallet::<T>::punish(acc.clone());
								let _ = pallet_sminer::Pallet::<T>::sub_power(peerid, res.size_type);
								let _ = pallet_sminer::Pallet::<T>::sub_space(peerid, res.size_type);
								Self::clean_service_proofs_signle(peerid, key2);
								//let _ = pallet_sminer::Pallet::<T>::fine_money(&acc);
								Self::deposit_event(Event::<T>::PPDNoOnTimeSubmit{acc: acc.clone(), segment_id: key2});
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
						ProofInfoVPA::<BoundedString<T>, BlockNumberOf<T>> {
							is_ready: false,
							size_type: size,
							proof: None,
							sealed_cid: None,
							rand: random,
							block_num: None,
						}
					);
					// <ParamSetA<T>>::insert(
					// 	&sender,
					// 	ParamInfo {
					// 		peer_id,
					// 		segment_id,
					// 		rand: random,
					// 	}
					// );
					Self::deposit_event(Event::<T>::ParamSet{peer_id: peer_id, segment_id: segment_id, random: random});
				}
				2u8 => {
					let acc = pallet_sminer::Pallet::<T>::get_acc(peerid);
					let segment_id = pallet_sminer::Pallet::<T>::get_segmentid(&acc)?;
					ensure!(!<VerPoolC<T>>::contains_key(&acc, segment_id), Error::<T>::YetIntennt);
					<VerPoolC<T>>::insert(
						&acc,
						segment_id,
						ProofInfoVPC::<BoundedList<T>, BlockNumberOf<T>> {
							is_ready: false,
							size_type: size,
							proof: None,
							sealed_cid: None,
							rand: random,
							block_num: None,
						}
					);
					let silce_info = FileSilceInfo::<BoundedString<T>, BoundedList<T>> {
						peer_id: peerid,
						segment_id: segment_id,
						uncid: Self::veclist_to_boundlist(uncid)?,
						rand: random,
						shardhash: Self::vec_to_bound::<u8>(shardhash)?,
					};
					if <MinerHoldSlice<T>>::contains_key(&acc) {
						<MinerHoldSlice<T>>::try_mutate(&acc, |s| -> DispatchResult {
							(*s).try_push(silce_info).expect("Length exceeded");
							Ok(())
						})?;
					} else {
						let mut value: Vec<FileSilceInfo<BoundedString<T>, BoundedList<T>>> = Vec::new();
						value.push(silce_info);
						let value_bound = Self::vec_to_bound::<FileSilceInfo<BoundedString<T>, BoundedList<T>>>(value)?;
						<MinerHoldSlice<T>>::insert(
							&acc,
							value_bound
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
						ProofInfoVPB::<BoundedString<T>, BlockNumberOf<T>> {
							is_ready: false,
							size_type: size,
							proof: None,
							sealed_cid: None,
							rand: random,
							block_num: None,
						}
					);
					// <ParamSetB<T>>::insert(
					// 	&sender,
					// 	ParamInfo {
					// 		peer_id,
					// 		segment_id,
					// 		rand: random,
					// 	}
					// );
				}
				2u8 => {
					ensure!(!<VerPoolD<T>>::contains_key(&sender, segment_id), Error::<T>::YetIntennt);
					ensure!(<PrePoolD<T>>::contains_key(&sender, segment_id), Error::<T>::SegmentUnExis);
					<VerPoolD<T>>::insert(
						&sender,
						segment_id,
						ProofInfoVPD::<BoundedList<T>, BlockNumberOf<T>> {
							is_ready: false,
							size_type: size,
							sealed_cid: None,
							proof: None,
							rand: random,
							block_num: None,
						}
					);
					// <ParamSetD<T>>::insert(
					// 	&sender,
					// 	ParamInfo {
					// 		peer_id,
					// 		segment_id,
					// 		rand: random,
					// 	}
					// );
				}
				_ => {
					ensure!(false, Error::<T>::SubmitTypeError);
				}
			}
			Self::deposit_event(Event::<T>::ParamSet{peer_id: peer_id, segment_id: segment_id, random: random});
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
			VerPoolA::<T>::try_mutate(&sender, segment_id, |s_opt| -> DispatchResult {
				let s = s_opt.as_mut().unwrap();
				s.is_ready = true;
				s.proof = Some(Self::vec_to_bound::<u8>(proof.clone())?);
				s.sealed_cid = Some(Self::vec_to_bound::<u8>(sealed_cid.clone())?);
				s.block_num = Some(<frame_system::Pallet<T>>::block_number());
				let x = UnverifiedPool::<AccountOf<T>, BoundedString<T>>{
					acc: sender.clone(), 
					peer_id: peer_id, 
					segment_id: segment_id, 
					proof: Self::vec_to_bound::<u8>(proof.clone())?, 
					sealed_cid: Self::vec_to_bound::<u8>(sealed_cid.clone())?, 
					rand: s.rand,
					size_type: s.size_type,
				};
				UnVerifiedA::<T>::try_mutate(|a| -> DispatchResult {			
					(*a).try_push(x).expect("Length exceeded");
					Ok(())
				})?;
				Ok(())
			})?;
			Self::deposit_event(Event::<T>::VPASubmitted{peer_id: peer_id, segment_id: segment_id});
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

			let ua = UnVerifiedA::<T>::get();
			let res = Self::unverify_remove(ua.to_vec(), peer_id, segment_id);
			let res_bound = Self::vec_to_bound::<UnverifiedPool<AccountOf<T>, BoundedString<T>>>(res)?;
			UnVerifiedA::<T>::put(res_bound);

			ensure!(<VerPoolA<T>>::contains_key(&sender, segment_id), Error::<T>::NotExistInVPA);

			let vpa = VerPoolA::<T>::get(&sender, segment_id).unwrap();

			ensure!(vpa.is_ready, Error::<T>::NotReadyInVPA);
			
			if result {
				<PrePoolA<T>>::insert(
					&sender,
					segment_id,
					ProofInfoPPA::<BoundedString<T>, BlockNumberOf<T>> {
						size_type: vpa.size_type,
						proof: vpa.proof,
						sealed_cid: vpa.sealed_cid,
						block_num: Some(<frame_system::Pallet<T>>::block_number()),
					}
				);
				<PrePoolB<T>>::insert(
					&sender,
					segment_id,
					ProofInfoPPB::<BoundedString<T>, BlockNumberOf<T>> {
						size_type: vpa.size_type.clone(),
						proof: None,
						sealed_cid: None,
						block_num: Some(<frame_system::Pallet<T>>::block_number()),
					}
				);
				
				let _ = pallet_sminer::Pallet::<T>::add_power(peer_id, vpa.size_type)?;
				pallet_sminer::Pallet::<T>::add_available_space(vpa.size_type)?;
				let size = vpa.size_type;
				let bo = VerPoolA::<T>::get(&sender, segment_id).unwrap();
				let sealed_cid = bo.sealed_cid.unwrap();
				if <ConProofInfoA<T>>::contains_key(&sender) {
					<ConProofInfoA<T>>::mutate(sender.clone(), |v| {
						let value = ContinuousProofPool::<BoundedString<T>> {
							peer_id: peer_id,
							segment_id: segment_id,
							sealed_cid: sealed_cid,
							size_type: size,
						};
						(*v).try_push(value).expect("Length exceeded");
					});
				} else {
					let mut v: Vec<ContinuousProofPool<BoundedString<T>>> = Vec::new();
					let value = ContinuousProofPool::<BoundedString<T>> {
						peer_id: peer_id,
						segment_id: segment_id,
						sealed_cid: sealed_cid,
						size_type: size,
					};
					v.push(value);
					let v_bound = Self::vec_to_bound::<ContinuousProofPool<BoundedString<T>>>(v)?;
					<ConProofInfoA<T>>::insert(
						&sender,
						v_bound
					);
				}
			}

			<VerPoolA<T>>::remove(&sender, segment_id);

			Self::deposit_event(Event::<T>::VPAVerified{peer_id: peer_id, segment_id: segment_id});
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

			VerPoolB::<T>::try_mutate(&sender, segment_id, |s_opt| -> DispatchResult {
				let s = s_opt.as_mut().unwrap();
				s.is_ready = true;
				s.proof = Some(Self::vec_to_bound::<u8>(proof.clone())?);
				s.sealed_cid = Some(Self::vec_to_bound::<u8>(sealed_cid.clone())?);
				s.block_num = Some(<frame_system::Pallet<T>>::block_number());
				let x = UnverifiedPool::<AccountOf<T>, BoundedString<T>>{
					acc: sender.clone(), 
					peer_id: peer_id, 
					segment_id: segment_id, 
					proof: Self::vec_to_bound::<u8>(proof.clone())?, 
					sealed_cid: Self::vec_to_bound::<u8>(sealed_cid.clone())?, 
					rand: (*s).rand, 
					size_type: (*s).size_type,
				};
				UnVerifiedB::<T>::try_mutate(|a| -> DispatchResult {			
					(*a).try_push(x).expect("Length exceeded");
					Ok(())
				})?;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::VPBSubmitted{peer_id: peer_id, segment_id: segment_id});	
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

			let ua = UnVerifiedB::<T>::get();
			let res = Self::unverify_remove(ua.to_vec(), peer_id, segment_id);
			let res_bound = Self::vec_to_bound::<UnverifiedPool<AccountOf<T>, BoundedString<T>>>(res)?;
			UnVerifiedB::<T>::put(res_bound);

			ensure!(<VerPoolB<T>>::contains_key(&sender, segment_id), Error::<T>::NoIntentSubmitYet);
			ensure!(<PrePoolB<T>>::contains_key(&sender, segment_id), Error::<T>::NoIntentSubmitYet);

			let vpb = VerPoolB::<T>::get(&sender, segment_id).unwrap();

			ensure!(vpb.is_ready, Error::<T>::NotReadyInVPB);
			
			if result {
					PrePoolB::<T>::try_mutate(&sender, segment_id, |s_opt| -> DispatchResult {
						let s = s_opt.as_mut().unwrap();
					
						s.proof = Some(vpb.proof.unwrap());
						s.sealed_cid = Some(vpb.sealed_cid.unwrap());
						s.block_num = Some(now_block);
						Ok(())
					})?;
			}
			<VerPoolB<T>>::remove(&sender, segment_id);

			Self::deposit_event(Event::<T>::VPBVerified{peer_id: peer_id, segment_id: segment_id});
			Ok(())
		}

		///C Submit a copy certificate of the service data segment
		/// Parameters:
		///  - `peer_id`: Miner's ID.
		///  - `segment_id`: Segment ID.
		///  - `proof`: proof, It could be a slice, so multiple proofs
		///  - `sealed_cid`: Required for verification certificate, It could be a slice, so multiple proofs
		#[pallet::weight(<T as pallet::Config>::WeightInfo::submit_to_vpc())]
		pub fn submit_to_vpc(origin: OriginFor<T>, peer_id: u64, segment_id: u64, proof: Vec<Vec<u8>>, sealed_cid: Vec<Vec<u8>>, fileid: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(<VerPoolC<T>>::contains_key(&sender, segment_id), Error::<T>::NoIntentSubmitYet);
			if !pallet_file_bank::Pallet::<T>::check_file_exist(fileid) {
				Self::clean_service_proofs_signle(peer_id, segment_id);
				Err(Error::<T>::FileNonExistent)?;
			}
			
			VerPoolC::<T>::try_mutate(&sender, segment_id, |s_opt| -> DispatchResult {
				let s = s_opt.as_mut().unwrap();
				s.is_ready = true;
				s.proof = Some(Self::veclist_to_boundlist(proof.clone())?);
				s.sealed_cid = Some(Self::veclist_to_boundlist(sealed_cid.clone())?);
				s.block_num = Some(<frame_system::Pallet<T>>::block_number());
				Ok(())
			})?;

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
				let mut uncid: BoundedList<T> = Default::default();
				for i in value {
					if i.segment_id == segment_id {
						uncid = i.uncid;
					}
				}
				let x = UnverifiedPoolVec::<AccountOf<T>, BoundedList<T>>{
					acc: sender.clone(), 
					peer_id: peer_id, 
					segment_id: segment_id, 
					proof: Self::veclist_to_boundlist(proof.clone())?, 
					sealed_cid: Self::veclist_to_boundlist(sealed_cid.clone())?, 
					uncid: uncid,
					rand: v.rand, 
					size_type: v.size_type,
				};
				UnVerifiedC::<T>::try_mutate(|a| -> DispatchResult {			
					(*a).try_push(x).expect("Length exceeded");
					Ok(())
				})?;
			}
			

			Self::deposit_event(Event::<T>::VPCSubmitted{peer_id: peer_id, segment_id: segment_id});	
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

			let ua = UnVerifiedC::<T>::get();
			let res = Self::unverify_remove_vec(ua.to_vec(), peer_id, segment_id);
			let res_bound = Self::vec_to_bound::<UnverifiedPoolVec<AccountOf<T>, BoundedList<T>>>(res)?;
			UnVerifiedC::<T>::put(res_bound);

			ensure!(<VerPoolC<T>>::contains_key(&sender, segment_id), Error::<T>::NotExistInVPC);

			let vpc = VerPoolC::<T>::get(&sender, segment_id).unwrap();

			ensure!(vpc.is_ready, Error::<T>::NotReadyInVPC);
			
			if result {
				<PrePoolC<T>>::insert(
					&sender,
					segment_id,
					ProofInfoPPC::<BoundedList<T>, BlockNumberOf<T>> {
						size_type: vpc.size_type,
						proof: vpc.proof,
						sealed_cid: vpc.sealed_cid,
						block_num: Some(<frame_system::Pallet<T>>::block_number()),
					}
				);
				<PrePoolD<T>>::insert(
					&sender,
					segment_id,
					ProofInfoPPD::<BoundedList<T>, BlockNumberOf<T>> {
						size_type: vpc.size_type.clone(),
						proof: None,
						sealed_cid: None,
						block_num: Some(<frame_system::Pallet<T>>::block_number()),
					}
				);
		
				let _ = pallet_sminer::Pallet::<T>::add_power(peer_id, vpc.size_type)?;
				let _ = pallet_sminer::Pallet::<T>::add_space(peer_id, vpc.size_type)?;

				let size = vpc.size_type;
				let bo = VerPoolC::<T>::get(&sender, segment_id).unwrap();
				let sealed_cid = bo.sealed_cid.unwrap();

				let mut hash: Vec<u8> = Vec::new();
				let value = MinerHoldSlice::<T>::get(&sender);
				for i in value {
					if i.segment_id == segment_id {
						hash = i.shardhash.to_vec();
					}
				}

				if <ConProofInfoC<T>>::contains_key(&sender) {
					<ConProofInfoC<T>>::try_mutate(sender.clone(), |v| -> DispatchResult {
						let value = ContinuousProofPoolVec::<BoundedString<T>, BoundedList<T>> {
							peer_id: peer_id,
							segment_id: segment_id,
							sealed_cid: sealed_cid.clone(),
							hash: Self::vec_to_bound::<u8>(hash)?,
							size_type: size,
						};
						(*v).try_push(value).expect("Length exceeded");
						Ok(())
					})?;
				} else {
					let mut v: Vec<ContinuousProofPoolVec<BoundedString<T>, BoundedList<T>>> = Vec::new();
					let value = ContinuousProofPoolVec::<BoundedString<T>, BoundedList<T>> {
						peer_id: peer_id,
						segment_id: segment_id,
						sealed_cid: sealed_cid,
						hash: Self::vec_to_bound::<u8>(hash)?,
						size_type: size,
					};
					v.push(value);
					let v_bound = Self::vec_to_bound::<ContinuousProofPoolVec<BoundedString<T>, BoundedList<T>>>(v)?;
					<ConProofInfoC<T>>::insert(
						&sender,
						v_bound
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
				let vpc = <VerPoolC<T>>::get(sender.clone(), segment_id).unwrap();
				let _ = pallet_sminer::Pallet::<T>::sub_power(peer_id, vpc.size_type);
				let _ = pallet_sminer::Pallet::<T>::sub_space(peer_id, vpc.size_type);
				pallet_sminer::Pallet::<T>::punish(sender.clone())?;
				Self::clean_service_proofs_signle(peer_id, segment_id);
				<VerPoolC<T>>::mutate(&sender, segment_id, |x_opt| {
					let s = x_opt.as_mut().unwrap();
					s.is_ready = false;
					s.proof = None;
					s.sealed_cid = None;
					s.block_num = None;
				});
			}

			Self::deposit_event(Event::<T>::VPCVerified{peer_id: peer_id, segment_id: segment_id});
		
			Ok(())
		
		}

		///D: Submit spatio-temporal proof of service data segment
		/// Parameters:
		///  - `peer_id`: Miner's ID.
		///  - `segment_id`: Segment ID.
		///  - `proof`: proof, It could be a slice, so multiple proofs
		///  - `sealed_cid`: Required for verification certificate, It could be a slice, so multiple proofs
		#[pallet::weight(<T as pallet::Config>::WeightInfo::submit_to_vpd())]
		pub fn submit_to_vpd(origin: OriginFor<T>, peer_id: u64, segment_id: u64, proof: Vec<Vec<u8>>, sealed_cid: Vec<Vec<u8>>, fileid: Vec<u8>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let now_block = <frame_system::Pallet<T>>::block_number();
			ensure!(<VerPoolD<T>>::contains_key(&sender, segment_id), Error::<T>::NoIntentSubmitYet);
			// ensure!(<VerPoolC<T>>::contains_key(&sender, segment_id), Error::<T>::NoIntentSubmitYet);
			if !pallet_file_bank::Pallet::<T>::check_file_exist(fileid) {
				Self::clean_service_proofs_signle(peer_id, segment_id);
				Err(Error::<T>::FileNonExistent)?;
			}

			VerPoolD::<T>::try_mutate(&sender, segment_id, |s_opt| -> DispatchResult {
				let s = s_opt.as_mut().unwrap();
				s.is_ready = true;
				s.proof = Some(Self::veclist_to_boundlist(proof.clone())?);
				s.sealed_cid = Some(Self::veclist_to_boundlist(sealed_cid.clone())?);
				s.block_num = Some(now_block);
				let x = UnverifiedPoolVecD::<AccountOf<T>, BoundedList<T>>{
					acc: sender.clone(), 
					peer_id: peer_id, 
					segment_id: segment_id, 
					proof: Self::veclist_to_boundlist(proof.clone())?, 
					sealed_cid: Self::veclist_to_boundlist(sealed_cid.clone())?, 
					rand: (*s).rand, 
					size_type: (*s).size_type,
				};
				UnVerifiedD::<T>::try_mutate(|a| -> DispatchResult {
					(*a).try_push(x).expect("Length exceeded"); 
					Ok(())
				})?;
				Ok(())
			})?;

			Self::deposit_event(Event::<T>::VPDSubmitted{peer_id: peer_id, segment_id: segment_id});	
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

			let ua = UnVerifiedD::<T>::get();
			let res = Self::unverify_remove_vec_d(ua.to_vec(), peer_id, segment_id);
			let res_bound = Self::vec_to_bound::<UnverifiedPoolVecD<AccountOf<T>, BoundedList<T>>>(res)?;
			UnVerifiedD::<T>::put(res_bound);

			ensure!(<VerPoolD<T>>::contains_key(&sender, segment_id), Error::<T>::NoIntentSubmitYet);
			ensure!(<PrePoolD<T>>::contains_key(&sender, segment_id), Error::<T>::NoIntentSubmitYet);

			let vpd = VerPoolD::<T>::get(&sender, segment_id).unwrap();

			ensure!(vpd.is_ready, Error::<T>::NotReadyInVPD);
			
			if result {
				PrePoolD::<T>::try_mutate(&sender, segment_id, |s_opt| -> DispatchResult {
					let s = s_opt.as_mut().unwrap();
					s.proof = Some(vpd.proof.unwrap());
					s.sealed_cid = Some(vpd.sealed_cid.unwrap());
					s.block_num = Some(now_block);
					Ok(())
				})?;
			} else {
				let vpc = <PrePoolC<T>>::get(sender.clone(), segment_id).unwrap();
				let _ = pallet_sminer::Pallet::<T>::sub_power(peer_id, vpc.size_type);
				let _ = pallet_sminer::Pallet::<T>::sub_space(peer_id, vpc.size_type);
				pallet_sminer::Pallet::<T>::punish(sender.clone())?;
				Self::clean_service_proofs_signle(peer_id, segment_id);
			}

			<VerPoolD<T>>::remove(&sender, segment_id);

			Self::deposit_event(Event::<T>::VPDVerified{peer_id: peer_id, segment_id: segment_id});
			Ok(())
		}
	}
}

// pub trait UnVerify {
// 	fn get_peerid(&self) -> u64;
// 	fn get_segmentid(&self) -> u64;
// }

// impl<T: Config> UnVerify for UnverifiedPool<T> {
// 	fn get_peerid(&self) -> u64 {
// 		self.peer_id
// 	}
// 	fn get_segmentid(&self) -> u64 {
// 		self.segment_id
// 	}
// }

// impl<T: Config> UnVerify for UnverifiedPoolVec<T> {
// 	fn get_peerid(&self) -> u64 {
// 		self.peer_id
// 	}
// 	fn get_segmentid(&self) -> u64 {
// 		self.segment_id
// 	}
// }

// impl<T: Config> UnVerify for UnverifiedPoolVecD<T> {
// 	fn get_peerid(&self) -> u64 {
// 		self.peer_id
// 	}
// 	fn get_segmentid(&self) -> u64 {
// 		self.segment_id
// 	}
// }

impl<T: Config> Pallet<T> {

	// Generate a random number from a given seed.
	fn generate_random_number(seed: u32) -> u32 {
		let (random_seed, _) = T::MyRandomness::random(&(T::MyPalletId::get(), seed).encode());
		let random_number = <u32>::decode(&mut random_seed.as_ref())
			.expect("secure hashes should always be bigger than u32; qed");
		random_number
	}
	//Remove eligible data from VEC
	// fn _unverify_usually<U: UnVerify>(mut list: Vec<U>, peer_id: u64, segment_id:u64) -> Vec<U> {
	// 	let mut k = 0;
	// 	for i in list.iter() {
	// 		if (*i).get_peerid() == peer_id && (*i).get_segmentid() == segment_id {
	// 			break;
	// 		}
	// 		k += 1;
	// 	}
	// 	list.remove(k);
	// 	list
	// }

	fn unverify_remove(mut list: Vec<UnverifiedPool<AccountOf<T>, BoundedString<T>>>, peer_id: u64, segment_id: u64) -> Vec<UnverifiedPool<AccountOf<T>, BoundedString<T>>>{
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
	fn unverify_remove_vec(mut list: Vec<UnverifiedPoolVec<AccountOf<T>, BoundedList<T>>>, peer_id: u64, segment_id: u64) -> Vec<UnverifiedPoolVec<AccountOf<T>, BoundedList<T>>>{
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
	fn unverify_remove_vec_d(mut list: Vec<UnverifiedPoolVecD<AccountOf<T>, BoundedList<T>>>, peer_id: u64, segment_id: u64) -> Vec<UnverifiedPoolVecD<AccountOf<T>, BoundedList<T>>>{
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

	// fn check_file_exist(fileid: Vec<u8>) -> bool {
	// 	pallet_file_bank::Pallet::<T>::check_file_exist(fileid)
	// }

	//Clear the certificate of a document slice and related certificates
	fn clean_service_proofs_signle(peer_id: u64, segment_id: u64) {
		let acc = pallet_sminer::Pallet::<T>::get_acc(peer_id);
		<ConProofInfoC<T>>::mutate(&acc, |s|{
			for i in 0..s.len() {
				let v = s.get(i);
				if v.unwrap().segment_id == segment_id {
					s.remove(i);
					break;
				}
			}
		});
		let mut unb = <UnVerifiedD<T>>::get();
		let mut k = 0;
		for i in unb.clone().iter() {
			if peer_id == i.peer_id && segment_id == i.segment_id {
				unb.remove(k);
				break;
			}
			k += 1;
		}
		//remove related storage
		<UnVerifiedD<T>>::put(unb);
		<PrePoolC<T>>::remove(&acc, segment_id);
		<VerPoolD<T>>::remove(&acc, segment_id);
		<PrePoolD<T>>::remove(&acc, segment_id);
	}

	fn vec_to_bound<P>(param: Vec<P>) -> Result<BoundedVec<P, T::SegStringLimit>, DispatchError> {
		let result: BoundedVec<P, T::SegStringLimit> = param.try_into().expect("too long");
		Ok(result)
	}

	fn veclist_to_boundlist(param: Vec<Vec<u8>>) -> Result<BoundedList<T>, DispatchError> {
		let mut result: BoundedList<T> = Vec::new().try_into().expect("...");

		for v in param {
			let string: BoundedVec<u8, T::SegStringLimit> = v.try_into().expect("keywords too long");
			result.try_push(string).expect("keywords too long");
		}

		Ok(result)
	}
}


