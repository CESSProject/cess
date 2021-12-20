#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::{
	account, benchmarks_instance_pallet, impl_benchmark_test_suite, whitelisted_caller, benchmarks,
};
use frame_system::RawOrigin as SystemOrigin;
use sp_runtime::traits::Bounded;
type BalanceOf<T> = <<T as pallet_sminer::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

fn new_miner<T: Config>() {
	let caller: T::AccountId = whitelisted_caller();
    pallet_sminer::Pallet::<T>::new_miner(caller);   
}

fn intent_one<T: Config>() {
    let caller: T::AccountId = whitelisted_caller();
    let peerid = 1;
    let segment_id = 1;
    let size: u128 = 8;
    let random = 222;
    <VerPoolA<T>>::insert(
        &caller,
        segment_id,
        ProofInfoVPA {
            is_ready: true,
            //false for 8M segment, true for 512M segment
            size_type: size,
            proof: None,
            sealed_cid: None,
            rand: random,
            block_num: None,
        }
    );
    <ParamSetA<T>>::insert(
        &caller,
        ParamInfo {
            peer_id: peerid,
            segment_id,
            rand: random,
        }
    );
}

fn add_UnVerA<T: Config>() {
    let caller: T::AccountId = whitelisted_caller();
    let peer_id = 1;
    let segment_id = 1;
    let rand = 222;
    let size_type = 8;
    let proof: Vec<u8> = "testproof0x981724981274".as_bytes().to_vec();
    let sealed_cid: Vec<u8> = "0x1241252352356".as_bytes().to_vec();
    VerPoolA::<T>::mutate(&caller, segment_id, |s_opt| {
        let s = s_opt.as_mut().unwrap();
        s.is_ready = true;
        s.proof = Some(proof.clone());
        s.sealed_cid = Some(sealed_cid.clone());
    });
    let x = UnverifiedPool{
        acc: caller.clone(), 
        peer_id: peer_id, 
        segment_id: segment_id, 
        proof: proof.clone(), 
        sealed_cid: sealed_cid.clone(), 
        rand: rand, 
        size_type: size_type,
    };
    UnVerifiedA::<T>::mutate(|a| (*a).push(x));
}

fn add_UnVerB<T: Config>() {
    let caller: T::AccountId = whitelisted_caller();
    let peer_id = 1;
    let segment_id = 1;
    let rand = 222;
    let size_type = 8;
    let proof: Vec<u8> = "testproof0x981724981274".as_bytes().to_vec();
    let sealed_cid: Vec<u8> = "0x1241252352356".as_bytes().to_vec();
    VerPoolB::<T>::mutate(&caller, segment_id, |s_opt| {
        let s = s_opt.as_mut().unwrap();
        s.is_ready = true;
        s.proof = Some(proof.clone());
        s.sealed_cid = Some(sealed_cid.clone());
    });
    let x = UnverifiedPool{
        acc: caller.clone(), 
        peer_id: peer_id, 
        segment_id: segment_id, 
        proof: proof.clone(), 
        sealed_cid: sealed_cid.clone(), 
        rand: rand, 
        size_type: size_type,
    };
    UnVerifiedB::<T>::mutate(|a| (*a).push(x));

    <PrePoolB<T>>::insert(
        &caller,
        segment_id,
        ProofInfoPPB {
            size_type: size_type,
            proof: None,
            sealed_cid: None,
            block_num: Some(<frame_system::Pallet<T>>::block_number()),
        }
    );
}

fn intent_two<T: Config>(){
    let caller: T::AccountId = whitelisted_caller();
    let peerid = 1;
    let segment_id = 1;
    let size: u128 = 8;
    let mut uncid: Vec<Vec<u8>> = Vec::new();
    uncid.push("testuncid1".as_bytes().to_vec());
    let hash: Vec<u8> = "testhash0x98ywef98ewhf9836423578hyv8jh9".as_bytes().to_vec();
    let shardhash: Vec<u8> = "testshardhash0x129847391284njksda".as_bytes().to_vec();
    let random = 222;
    <VerPoolC<T>>::insert(
        &caller,
        segment_id,
        ProofInfoVPC {
            is_ready: false,
            //false for 8M segment, true for 512M segment
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
    if <MinerHoldSlice<T>>::contains_key(&caller) {
        <MinerHoldSlice<T>>::mutate(&caller, |s| (*s).push(silce_info));
    } else {
        let mut value: Vec<FileSilceInfo> = Vec::new();
        value.push(silce_info);
        <MinerHoldSlice<T>>::insert(
            &caller,
            value
        )
    }
}

fn intent_po_st<T: Config>(){
    let caller: T::AccountId = whitelisted_caller();
    let peer_id = 1;
    let segment_id = 1;
    let size: u128 = 8;
    let random = 222;
    <VerPoolB<T>>::insert(
        &caller,
        segment_id,
        ProofInfoVPB {
            is_ready: false,
            //false for 8M segment, true for 512M segment
            size_type: size,
            proof: None,
            sealed_cid: None,
            rand: random,
            block_num: None,
        }
    );
    <ParamSetB<T>>::insert(
        &caller,
        ParamInfo {
            peer_id,
            segment_id,
            rand: random,
        }
    );

    <VerPoolD<T>>::insert(
        &caller,
        segment_id,
        ProofInfoVPD {
            is_ready: false,
            //false for 8M segment, true for 512M segment
            size_type: size,
            sealed_cid: None,
            proof: None,
            rand: random,
            block_num: None,
        }
    );
    <ParamSetD<T>>::insert(
        &caller,
        ParamInfo {
            peer_id,
            segment_id,
            rand: random,
        }
    );
}

fn add_BlockB<T: Config>() {
    let caller: T::AccountId = whitelisted_caller();
    <BlockNumberB<T>>::insert(
        &caller,
        PeerFileNum {
            block_num: Some(0u32.into()),
            total_num: 1,
        }
    );
}

fn add_BlockD<T: Config>() {
    let caller: T::AccountId = whitelisted_caller();
    <BlockNumberD<T>>::insert(
        &caller,
        PeerFileNum {
            block_num: Some(0u32.into()),
            total_num: 1,
        }
    );
}

fn add_UnVerC<T: Config>() {
    let caller: T::AccountId = whitelisted_caller();
    let peer_id = 1;
    let segment_id = 1;
    let size: u128 = 8;
    let random = 222;
    let mut uncid: Vec<Vec<u8>> = Vec::new();
    uncid.push("testuncid1".as_bytes().to_vec());
    let mut proof: Vec<Vec<u8>> = Vec::new();
    proof.push("testproof0x981724981274".as_bytes().to_vec());
    let mut sealed_cid: Vec<Vec<u8>> = Vec::new();
    proof.push("0x1241252352356".as_bytes().to_vec());
    VerPoolC::<T>::mutate(&caller, segment_id, |s_opt| {
        let s = s_opt.as_mut().unwrap();
        s.is_ready = true;
        s.proof = Some(proof.clone());
        s.sealed_cid = Some(sealed_cid.clone());
        s.block_num = Some(<frame_system::Pallet<T>>::block_number());
    });
    let x = UnverifiedPoolVec{
        acc: caller.clone(), 
        peer_id: peer_id, 
        segment_id: segment_id, 
        proof: proof.clone(), 
        sealed_cid: sealed_cid.clone(), 
        uncid: uncid,
        rand: random, 
        size_type: size,
    };
    UnVerifiedC::<T>::mutate(|a| (*a).push(x));
}

fn add_UnVerD<T: Config>() {
    let caller: T::AccountId = whitelisted_caller();
    let peer_id = 1;
    let segment_id = 1;
    let size: u128 = 8;
    let random = 222;
    let mut proof: Vec<Vec<u8>> = Vec::new();
    proof.push("testproof0x981724981274".as_bytes().to_vec());
    let mut sealed_cid: Vec<Vec<u8>> = Vec::new();
    proof.push("0x1241252352356".as_bytes().to_vec());
    <PrePoolD<T>>::insert(
        &caller,
        segment_id,
        ProofInfoPPD {
            size_type: size.clone(),
            proof: None,
            sealed_cid: None,
            block_num: Some(<frame_system::Pallet<T>>::block_number()),
        }
    );
    VerPoolD::<T>::mutate(&caller, segment_id, |s_opt| {
        let s = s_opt.as_mut().unwrap();
        s.is_ready = true;
        s.proof = Some(proof.clone());
        s.sealed_cid = Some(sealed_cid.clone());
        s.block_num = Some(<frame_system::Pallet<T>>::block_number());
    });
    let x = UnverifiedPoolVecD{
        acc: caller.clone(), 
        peer_id: peer_id, 
        segment_id: segment_id, 
        proof: proof.clone(), 
        sealed_cid: sealed_cid.clone(), 
        rand: random, 
        size_type: size,
    };
    UnVerifiedD::<T>::mutate(|a| (*a).push(x));
}

benchmarks! {
    intent_submit {
        new_miner::<T>();
        let caller: T::AccountId = whitelisted_caller();
        let size_type: u8 = 1; 
        let submit_type: u8 = 2;
        let mut uncid: Vec<Vec<u8>> = Vec::new();
        uncid.push("testuncid1".as_bytes().to_vec());
        let hash: Vec<u8> = "testhash0x98ywef98ewhf9836423578hyv8jh9".as_bytes().to_vec();
        let shardhash: Vec<u8> = "testshardhash0x129847391284njksda".as_bytes().to_vec();
    }: _(SystemOrigin::Signed(caller.clone()), size_type, submit_type, 1, uncid, hash, shardhash)
    verify {
            assert_eq!(<VerPoolC<T>>::get(&caller, 1).unwrap().size_type, 8u128);
    }

    intent_submit_po_st {
        new_miner::<T>();
        let caller: T::AccountId = whitelisted_caller();
        let segment_id: u64 = 1;
        let size_type: u8 = 1;
        let submit_type: u8 = 1;
    }: _(SystemOrigin::Signed(caller.clone()), segment_id, size_type, submit_type)
    verify {
        assert_eq!(<VerPoolB<T>>::get(&caller, 1).unwrap().size_type,8u128);
    }

    submit_to_vpa {
        new_miner::<T>();
        intent_one::<T>();
        let caller: T::AccountId = whitelisted_caller();
        let peer_id: u64 = 1;
        let segment_id: u64 = 1;
        let proof: Vec<u8> = "testproof0x981724981274".as_bytes().to_vec();
        let sealed_cid: Vec<u8> = "0x1241252352356".as_bytes().to_vec();
    }: _(SystemOrigin::Signed(caller.clone()), peer_id, segment_id, proof.clone(), sealed_cid)
    verify {
        assert_eq!(VerPoolA::<T>::get(&caller, segment_id).unwrap().proof.unwrap(), proof.clone());
    }

    verify_in_vpa {
        new_miner::<T>();
        intent_one::<T>();
        add_UnVerA::<T>();
        let caller: T::AccountId = whitelisted_caller();
        let peer_id: u64 = 1;
        let segment_id: u64 = 1;
        let result: bool = true;
    }: _(SystemOrigin::Signed(caller.clone()), peer_id, segment_id, result)
    verify {
        
    }

    submit_to_vpb {
        new_miner::<T>();
        intent_po_st::<T>();
        let caller: T::AccountId = whitelisted_caller();
        let peer_id: u64 = 1;
        let segment_id: u64 = 1;
        let proof: Vec<u8> = "testproof0x981724981274".as_bytes().to_vec();
        let sealed_cid: Vec<u8> = "0x1241252352356".as_bytes().to_vec();

    }: _(SystemOrigin::Signed(caller.clone()), peer_id, segment_id, proof, sealed_cid)
    verify {
        assert_eq!(VerPoolB::<T>::get(&caller, segment_id).unwrap().is_ready, true);
    }

    verify_in_vpb {
        new_miner::<T>();
        intent_po_st::<T>();
        add_UnVerB::<T>();
        add_BlockB::<T>();
        let caller: T::AccountId = whitelisted_caller();
        let peer_id: u64 = 1;
        let segment_id: u64 = 1;
        let result: bool = true;
    }: _(SystemOrigin::Signed(caller.clone()), peer_id, segment_id, result)
    verify {
        
    }

    submit_to_vpc {
        new_miner::<T>();
        intent_two::<T>();
        let caller: T::AccountId = whitelisted_caller();
        let peer_id: u64 = 1;
        let segment_id: u64 = 1;
        let mut proof: Vec<Vec<u8>> = Vec::new();
        proof.push("testproof0x981724981274".as_bytes().to_vec());
        let mut sealed_cid: Vec<Vec<u8>> = Vec::new();
        proof.push("0x1241252352356".as_bytes().to_vec());
    }: _(SystemOrigin::Signed(caller.clone()), peer_id, segment_id, proof, sealed_cid)
    verify {
        assert_eq!(VerPoolC::<T>::get(&caller, segment_id).unwrap().is_ready, true);
    }

    verify_in_vpc {
        new_miner::<T>();
        intent_two::<T>();
        add_UnVerC::<T>();
        let caller: T::AccountId = whitelisted_caller();
        let peer_id: u64 = 1;
        let segment_id: u64 = 1;
        let result: bool = true;
        let mut uncid: Vec<Vec<u8>> = Vec::new();
        uncid.push("testuncid1".as_bytes().to_vec());
    }: _(SystemOrigin::Signed(caller.clone()), peer_id, segment_id, uncid, result)
    verify {

    }

    submit_to_vpd {
        new_miner::<T>();
        intent_po_st::<T>();
        let caller: T::AccountId = whitelisted_caller();
        let peer_id: u64 = 1;
        let segment_id: u64 = 1;
        let mut proof: Vec<Vec<u8>> = Vec::new();
        proof.push("testproof0x981724981274".as_bytes().to_vec());
        let mut sealed_cid: Vec<Vec<u8>> = Vec::new();
        proof.push("0x1241252352356".as_bytes().to_vec());
    }: _(SystemOrigin::Signed(caller.clone()), peer_id, segment_id, proof, sealed_cid)
    verify {
        assert_eq!(VerPoolD::<T>::get(&caller, segment_id).unwrap().is_ready, true);
    }

    verify_in_vpd {
        new_miner::<T>();
        intent_po_st::<T>();
        add_UnVerD::<T>();
        add_BlockD::<T>();
        let caller: T::AccountId = whitelisted_caller();
        let peer_id: u64 = 1;
        let segment_id: u64 = 1;
        let result: bool = true;
    }: _(SystemOrigin::Signed(caller.clone()), peer_id, segment_id, result)
    verify {

    }
}