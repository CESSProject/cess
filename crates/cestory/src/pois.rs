use crate::{expert::CesealExpertStub, verify_signature};
use ces_crypto::sr25519::Signing;
use ces_pdp::Keys;
use ces_pois::{
    acc::{multi_level_acc::WitnessNode, rsa_keygen, RsaKey},
    pois::{
        prove::{AccProof, CommitProof, Commits, DeletionProof, MhtProof, SpaceProof},
        verify::{ProverNode, Verifier},
    },
};
use cestory_api::pois::{
    pois_certifier_api_server::{self, PoisCertifierApi},
    pois_verifier_api_server::{self, PoisVerifierApi},
    AccWitnessNode, Challenge, Int64Slice, PoisStatus, ProofHashAndLeftRight, RequestMinerCommitGenChall,
    RequestMinerInitParam, RequestSpaceProofVerify, RequestSpaceProofVerifyTotal, RequestVerifyCommitAndAccProof,
    RequestVerifyDeletionProof, ResponseMinerInitParam, ResponseSpaceProofVerify, ResponseSpaceProofVerifyTotal,
    ResponseVerifyCommitOrDeletionProof,
};
use crypto::{digest::Digest, sha2::Sha256};
use dashmap::DashMap;
use log::{info, warn};
use num_bigint_dig::BigUint;
use parity_scale_codec::Encode;
use prost::Message;
use rsa::pkcs1::EncodeRsaPublicKey;
use sp_core::{crypto::AccountId32, sr25519, ByteArray};
use std::{
    fmt::{Debug, Display, Formatter},
    time::Instant,
};
use tonic::{Request, Response, Status};

mod proxy;

pub type PoisCertifierApiServer =
    pois_certifier_api_server::PoisCertifierApiServer<PoisCertifierApiServerProxy<PoisCertifierServer>>;
pub type PoisVerifierApiServer =
    pois_verifier_api_server::PoisVerifierApiServer<PoisVerifierApiServerProxy<PoisVerifierServer>>;
pub type PoisResult<T> = Result<Response<T>, Status>;

pub use proxy::{PoisCertifierApiServerProxy, PoisVerifierApiServerProxy};

pub fn new_pois_certifier_api_server(
    pois_param: (i64, i64, i64),
    ceseal_expert: CesealExpertStub,
) -> PoisCertifierApiServer {
    let podr2_keys = ceseal_expert.podr2_key().clone();
    let master_key = crate::get_sr25519_from_rsa_key(podr2_keys.clone().skey);
    let inner = PoisCertifierApiServerProxy {
        inner: PoisCertifierServer {
            podr2_keys,
            master_key,
            verifier: Verifier::new(pois_param.0, pois_param.1, pois_param.2),
            commit_acc_proof_chals_map: DashMap::new(),
            ceseal_identity_key: ceseal_expert.identify_public_key().0,
            ceseal_expert: ceseal_expert.clone(),
        },
        ceseal_expert,
    };
    PoisCertifierApiServer::new(inner)
}

pub fn new_pois_verifier_api_server(
    pois_param: (i64, i64, i64),
    ceseal_expert: CesealExpertStub,
) -> PoisVerifierApiServer {
    let podr2_keys = ceseal_expert.podr2_key().clone();
    let master_key = crate::get_sr25519_from_rsa_key(podr2_keys.clone().skey);
    let inner = PoisVerifierApiServerProxy {
        inner: PoisVerifierServer {
            podr2_keys,
            master_key,
            verifier: Verifier::new(pois_param.0, pois_param.1, pois_param.2),
            ceseal_identity_key: ceseal_expert.identify_public_key().0,
        },
        ceseal_expert,
    };
    PoisVerifierApiServer::new(inner)
}

#[derive(Encode)]
pub struct MinerCommitProofInfo {
    pub miner_id: AccountId32,
    pub front: u64,
    pub rear: u64,
    pub pois_key: PoISKey,
    pub accumulator: [u8; 256],
}
#[derive(Encode)]
pub struct MinerPoisInfo {
    pub miner_id: AccountId32,
    pub front: u64,
    pub rear: u64,
    pub pois_key: PoISKey,
    pub accumulator: [u8; 256],
}

impl Default for MinerPoisInfo {
    fn default() -> Self {
        MinerPoisInfo {
            miner_id: AccountId32::from([0u8; 32]),
            front: 0,
            rear: 0,
            pois_key: PoISKey { g: [0u8; 256], n: [0u8; 256] },
            accumulator: [0u8; 256],
        }
    }
}

#[derive(Encode)]
pub struct PoISKey {
    pub g: [u8; 256],
    pub n: [u8; 256],
}

pub struct MinerKeyInfo {
    pub info: MinerPoisInfo,
    pub signature: Vec<u8>,
    pub is_register_key: bool,
}

#[derive(Encode)]
pub struct ResponseSpaceProofVerifyTotalSignatureMember {
    pub miner_id: AccountId32,
    pub total_proof_hash: Vec<u8>,
    pub front: u64,
    pub rear: u64,
    pub acc: [u8; 256],
    pub space_chals: [u64; 8],
    pub result: bool,
    pub tee_acc: AccountId32,
}

pub struct PoisCertifierServer {
    pub podr2_keys: Keys,
    pub master_key: sr25519::Pair,
    pub verifier: Verifier,
    pub commit_acc_proof_chals_map: DashMap<Vec<u8>, Vec<Vec<i64>>>,
    pub ceseal_identity_key: [u8; 32],
    ceseal_expert: CesealExpertStub,
}

pub struct PoisVerifierServer {
    pub podr2_keys: Keys,
    pub master_key: sr25519::Pair,
    pub verifier: Verifier,
    pub ceseal_identity_key: [u8; 32],
}

fn try_into_proto_byte_hash<T>(data: &T) -> Result<Vec<u8>, Status>
where
    T: prost::Message,
{
    let mut proto_byte = Vec::new();
    data.encode(&mut proto_byte)
        .map_err(|e| Status::internal("Error when encode the signature from miner:".to_string() + &e.to_string()))?;
    let mut proto_byte_hash = vec![0u8; 32];
    let mut hasher = Sha256::new();
    hasher.input(&proto_byte);
    hasher.result(&mut proto_byte_hash);
    Ok(proto_byte_hash)
}

const MINER_NOT_READY: &'static [u8] = b"not ready";

#[tonic::async_trait]
impl PoisCertifierApi for PoisCertifierServer {
    async fn request_miner_get_new_key(
        &self,
        request: Request<RequestMinerInitParam>,
    ) -> PoisResult<ResponseMinerInitParam> {
        //create key for miner
        let miner_id: Vec<u8> = request.into_inner().miner_id;
        let now = Instant::now();
        info!("[Pois Request New Key] miner {:?} request new key...", get_ss58_address(&miner_id)?);

        //FIXME: TO OPTIMIZE
        let miner_acc_id =
            AccountId32::from_slice(&miner_id[..]).map_err(|_| Status::internal("invalid input account"))?;
        let miner_info = self
            .ceseal_expert
            .using_chain_storage(
                move |opt| if let Some(cs) = opt { cs.get_storage_miner_info(miner_acc_id) } else { None },
            )
            .await
            .map_err(|e| Status::internal(format!("internal error: {}", e.to_string())))?;
        let Some(miner_info) = miner_info else { return Err(Status::internal("the miner not exists")) };

        if !matches!(miner_info.state.as_slice(), MINER_NOT_READY) {
            return Err(Status::invalid_argument("You have already registered the POIS key!"))
        }
        let key = rsa_keygen(2048);
        let acc = key.g.to_bytes_be();

        let (_, status_tee_sign, signature_with_tee_controller) = get_pois_status_and_signature(
            acc.clone(),
            0,
            0,
            key.n.to_bytes_be(),
            key.g.to_bytes_be(),
            miner_id.clone(),
            &self.master_key,
            self.ceseal_identity_key.clone(),
        )?;

        info!(
            "[Pois Request New Key] success generate new key for miner :{:?} g length is {} . used time :{:.2?}",
            get_ss58_address(&miner_id)?,
            acc.len(),
            now.elapsed()
        );
        let reply = ResponseMinerInitParam {
            acc,
            key_n: key.n.to_bytes_be(),
            key_g: key.g.to_bytes_be(),
            front: 0,
            rear: 0,
            miner_id,
            status_tee_sign,
            signature_with_tee_controller,
            podr2_pbk: self.podr2_keys.pkey.to_pkcs1_der().unwrap().to_vec(),
        };
        Ok(Response::new(reply))
    }
    async fn request_miner_commit_gen_chall(
        &self,
        request: Request<RequestMinerCommitGenChall>,
    ) -> PoisResult<Challenge> {
        let req = request.into_inner();
        let now = Instant::now();
        //get a structural to verify the sign from miner
        let miner_req = RequestMinerCommitGenChall {
            miner_id: req.miner_id.clone(),
            commit: req.commit.clone(),
            miner_sign: Vec::new(),
        };
        let miner_cess_address = get_ss58_address(&miner_req.miner_id)?;
        info!("[Pois Commit Chall] miner {:?} request for commit challenge...", miner_cess_address);
        let miner_req_hash = try_into_proto_byte_hash(&miner_req)?;

        if verify_signature(miner_req.miner_id, req.miner_sign, &miner_req_hash)
            .map_err(|e| Status::internal("Error check miner request signature fail:".to_string() + &e.to_string()))?
        {
            return Err(Status::unauthenticated("The miner request signature is incorrect!"))
        };

        let miner_id = req.miner_id;
        let mut commits = convert_to_commits(
            req.commit
                .clone()
                .ok_or(Status::invalid_argument("Miner request data is invalid, loss parameter 'commit'"))?,
        );
        //register a empty prover node for receive commits
        self.verifier.register_prover_node_empty(&miner_id);

        //verify received commit
        if !self.verifier.receive_commits(&miner_id, &mut commits) {
            self.verifier.logout_prover_node(&miner_id).unwrap();
            return Err(Status::invalid_argument("The commit submitted by the miner is wrong".to_string()))
        };
        //gen commit chals for this miner and save it in verifier
        let chals = match self.verifier.commit_challenges(&miner_id) {
            Ok(chals) => chals,
            Err(e) => {
                self.verifier.logout_prover_node(&miner_id).unwrap();
                return Err(Status::internal(e.to_string()))
            },
        };
        self.commit_acc_proof_chals_map.insert(miner_id, chals.clone());

        let mut challenge = convert_to_challenge(chals);
        let miner_req_hash = try_into_proto_byte_hash(&challenge)?;
        let chall_tee_sign = self
            .podr2_keys
            .sign_data(&miner_req_hash)
            .map_err(|e| Status::internal("Get TeeWorker fail:".to_string() + &e.error_code.to_string()))?;
        info!("[Pois Commit Chall] miner : {:?} ,chall_tee_sign is {:?}", miner_cess_address, chall_tee_sign.clone());
        challenge.chall_tee_sign = chall_tee_sign;
        info!("[Pois Commit Chall] miner {:?} get commit challenge use: {:.2?}", miner_cess_address, now.elapsed());
        Ok(Response::new(challenge))
    }
    async fn request_verify_commit_proof(
        &self,
        request: Request<RequestVerifyCommitAndAccProof>,
    ) -> PoisResult<ResponseVerifyCommitOrDeletionProof> {
        let now = Instant::now();
        let mut commit_and_acc_proof = request.into_inner();
        let miner_sign = commit_and_acc_proof.miner_sign;
        let miner_id = commit_and_acc_proof.miner_id.clone();
        let miner_cess_address = get_ss58_address(&miner_id)?;
        info!("[Pois Verify Commit Proof] miner {:?} commit proof for verify...", miner_cess_address);

        //Check if the miner has logged out
        if self.verifier.is_logout(&miner_id) {
            return Err(Status::unauthenticated(format!("This tee never know the miner {:?}", &miner_id)))
        }

        //Check whether the data sent by the miner is incorrect
        commit_and_acc_proof.miner_sign = Vec::new();
        let miner_req_hash = try_into_proto_byte_hash(&commit_and_acc_proof)?;
        if verify_signature(miner_id.clone(), miner_sign, &miner_req_hash)
            .map_err(|e| Status::internal("Error check miner request signature fail:".to_string() + &e.to_string()))?
        {
            return Err(Status::unauthenticated("The miner request signature is incorrect!"))
        };

        //Check the miner pois info is from tee or not
        let miner_pois_info = commit_and_acc_proof
            .pois_info
            .ok_or(Status::invalid_argument("Miner request data is invalid, loss parameter 'pois_info'"))?;

        let mut miner_key_info = MinerPoisInfo::default();
        miner_key_info.miner_id = miner_id.clone()[..]
            .try_into()
            .map_err(|_| Status::invalid_argument("The length of miner_id should be 32!"))?;
        miner_key_info.front = miner_pois_info.front as u64;
        miner_key_info.rear = miner_pois_info.rear as u64;
        miner_key_info.pois_key.g = miner_pois_info
            .key_g
            .clone()
            .try_into()
            .map_err(|_| Status::invalid_argument("The length of pois key g should be 256!"))?;
        miner_key_info.pois_key.n = miner_pois_info
            .key_n
            .clone()
            .try_into()
            .map_err(|_| Status::invalid_argument("The length of pois key n should be 256!"))?;
        miner_key_info.accumulator = miner_pois_info
            .acc
            .clone()
            .try_into()
            .map_err(|_| Status::invalid_argument("The length of pois acc should be 256!"))?;
        info!("[Pois Commit Verify] miner : {:?} ,front is {}", miner_cess_address, miner_key_info.front.clone());
        info!("[Pois Commit Verify] miner : {:?} ,rear is {}", miner_cess_address, miner_key_info.rear.clone());
        info!(
            "[Pois Commit Verify] miner : {:?} ,key_g is {:?}",
            miner_cess_address,
            miner_key_info.pois_key.g.clone()
        );
        info!(
            "[Pois Commit Verify] miner : {:?} ,key_n is {:?}",
            miner_cess_address,
            miner_key_info.pois_key.n.clone()
        );
        info!(
            "[Pois Commit Verify] miner : {:?} ,accumulator is {:?}",
            miner_cess_address,
            miner_key_info.accumulator.clone()
        );
        info!(
            "[Pois Commit Verify] miner : {:?} ,Tee signature is {:?}",
            miner_cess_address, &miner_pois_info.status_tee_sign
        );
        verify_pois_status_signature(miner_key_info, &self.master_key, miner_pois_info.status_tee_sign)?;

        let commit_proof_group_inner = if let Some(commit_proof_group) = commit_and_acc_proof.commit_proof_group {
            commit_proof_group.commit_proof_group_inner
        } else {
            return Err(Status::invalid_argument("Miner request data is invalid, loss parameter 'commit_proof_group'"))
        };
        let mut commit_proofs = Vec::new();
        for inner in commit_proof_group_inner {
            let mut commit_proof_row = Vec::new();
            for commit_proof in inner.commit_proof {
                let cp = convert_to_commit_proof(&commit_proof);
                if cp.elders.is_empty() {
                    return Err(Status::invalid_argument(
                        "Miner request data is invalid, loss parameter 'elders' in 'CommitProof'",
                    ))
                };
                if cp.parents.is_empty() {
                    return Err(Status::invalid_argument(
                        "Miner request data is invalid, loss parameter 'parents' in 'CommitProof'",
                    ))
                }
                commit_proof_row.push(cp);
            }
            commit_proofs.push(commit_proof_row);
        }

        //Get miner key info
        let key = RsaKey {
            n: BigUint::from_bytes_be(&miner_pois_info.key_n),
            g: BigUint::from_bytes_be(&miner_pois_info.key_g),
        };
        self.verifier.update_prover_node_force(
            &miner_id,
            key,
            &miner_pois_info.acc,
            miner_pois_info.front as i64,
            miner_pois_info.rear as i64,
        );

        //Get miner challenge info
        let chals = if let Some((_, chals)) = self.commit_acc_proof_chals_map.remove(&miner_id) {
            chals
        } else {
            return Err(Status::data_loss("This miner does not have a corresponding commit challenge".to_string()))
        };
        match self
            .verifier
            .verify_commit_proofs(&miner_id, chals.clone(), commit_proofs.clone())
        {
            Ok(_) => {},
            Err(e) =>
                return Err(Status::invalid_argument(
                    format!("Error when verify commit :{:?}", e.to_string()).to_string(),
                )),
        };
        let pb_acc_proof = commit_and_acc_proof
            .acc_proof
            .ok_or(Status::invalid_argument("Miner request data is invalid, loss parameter 'acc_proof'"))?;
        let acc_proof = convert_to_acc_proof(&pb_acc_proof);

        match self.verifier.verify_acc(&miner_id, chals, acc_proof) {
            Ok(_) => {},
            Err(e) =>
                return Err(Status::invalid_argument(format!("Error when verify acc :{:?}", e.to_string()).to_string())),
        };
        //The miners will verify the 16G commit proof every time. After each verification, they need to log out and
        // then sign and return to the state in tee. The miners will put the state on the chain to ensure that
        // the data on the chain is synchronized.
        let node_state = match self.verifier.logout_prover_node(&miner_id) {
            Ok(i) => i,
            Err(e) =>
                return Err(Status::invalid_argument(
                    format!("Error when verify proof and sig the result fail:{:?}", e.to_string()).to_string(),
                )),
        };
        let (pois_status, status_tee_sign, signature_with_tee_controller) = get_pois_status_and_signature(
            node_state.0,
            node_state.1,
            node_state.2,
            miner_pois_info.key_n,
            miner_pois_info.key_g,
            miner_id,
            &self.master_key,
            self.ceseal_identity_key.clone(),
        )?;
        info!(
            "[Pois Verify Commit Proof] miner {:?} Pois Verify Commit Proof in: {:.2?}",
            miner_cess_address,
            now.elapsed()
        );
        Ok(Response::new(ResponseVerifyCommitOrDeletionProof {
            pois_status: Some(pois_status),
            status_tee_sign,
            signature_with_tee_controller,
        }))
    }
    async fn request_verify_deletion_proof(
        &self,
        request: Request<RequestVerifyDeletionProof>,
    ) -> PoisResult<ResponseVerifyCommitOrDeletionProof> {
        let mut deletion_proof = request.into_inner();
        let miner_sign = deletion_proof.miner_sign;
        let miner_id = deletion_proof.miner_id.clone();
        let miner_cess_address = get_ss58_address(&miner_id)?;
        let now = Instant::now();
        info!("[Verify Deletion Proof] miner {:?} verify deletion proof...", miner_cess_address);

        deletion_proof.miner_sign = Vec::new();
        let miner_req_hash = try_into_proto_byte_hash(&deletion_proof)?;
        if verify_signature(miner_id.clone(), miner_sign, &miner_req_hash)
            .map_err(|e| Status::internal("Error check miner request signature fail:".to_string() + &e.to_string()))?
        {
            return Err(Status::unauthenticated("The miner request signature is incorrect!"))
        };

        let mut del_proof = convert_to_deletion_proof(&deletion_proof);
        let miner_pois_info = if let Some(pois_info) = deletion_proof.pois_info {
            pois_info
        } else {
            return Err(Status::invalid_argument("Miner send invalid parameter,loss 'pois_info'"))
        };

        let mut miner_key_info = MinerPoisInfo::default();
        miner_key_info.miner_id = miner_id.clone()[..]
            .try_into()
            .map_err(|_| Status::invalid_argument("The length of miner_id should be 32!"))?;
        miner_key_info.front = miner_pois_info.front as u64;
        miner_key_info.rear = miner_pois_info.rear as u64;
        miner_key_info.pois_key.g = miner_pois_info
            .key_g
            .clone()
            .try_into()
            .map_err(|_| Status::invalid_argument("The length of pois key g should be 256!"))?;
        miner_key_info.pois_key.n = miner_pois_info
            .key_n
            .clone()
            .try_into()
            .map_err(|_| Status::invalid_argument("The length of pois key n should be 256!"))?;
        miner_key_info.accumulator = miner_pois_info
            .acc
            .clone()
            .try_into()
            .map_err(|_| Status::invalid_argument("The length of pois acc should be 256!"))?;
        info!(
            "[Verify Deletion Proof] miner : {:?} ,Tee signature is {:?}",
            miner_cess_address, &miner_pois_info.status_tee_sign
        );

        verify_pois_status_signature(miner_key_info, &self.master_key, miner_pois_info.status_tee_sign)?;

        //register in verifier obj
        let key = RsaKey {
            n: BigUint::from_bytes_be(&miner_pois_info.key_n),
            g: BigUint::from_bytes_be(&miner_pois_info.key_g),
        };
        self.verifier.register_prover_node(
            &miner_id,
            key,
            &miner_pois_info.acc,
            miner_pois_info.front as i64,
            miner_pois_info.rear as i64,
        );

        self.verifier
            .verify_deletion(&miner_id, &mut del_proof)
            .map_err(|e| Status::invalid_argument("Failed to delete proof :".to_string() + &e.to_string()))?;
        //Every time the proof is deleted, the miner needs to delete the logout once, obtain the latest acc and upload
        // it to the chain
        let node_state = match self.verifier.logout_prover_node(&miner_id) {
            Ok(i) => i,
            Err(e) =>
                return Err(Status::invalid_argument(
                    format!("Error when verify proof and sig the result fail:{:?}", e.to_string()).to_string(),
                )),
        };
        let (pois_status, status_tee_sign, signature_with_tee_controller) = get_pois_status_and_signature(
            node_state.0,
            node_state.1,
            node_state.2,
            miner_pois_info.key_n,
            miner_pois_info.key_g,
            miner_id.clone(),
            &self.master_key,
            self.ceseal_identity_key.clone(),
        )?;
        info!(
            "[Verify Deletion Proof] miner {:?} finish verify deletion proof in {:.2?}",
            miner_cess_address,
            now.elapsed()
        );
        Ok(Response::new(ResponseVerifyCommitOrDeletionProof {
            pois_status: Some(pois_status),
            status_tee_sign,
            signature_with_tee_controller,
        }))
    }
}

#[tonic::async_trait]
impl PoisVerifierApi for PoisVerifierServer {
    async fn request_space_proof_verify_single_block(
        &self,
        request: Request<RequestSpaceProofVerify>,
    ) -> PoisResult<ResponseSpaceProofVerify> {
        //parse space challenge
        let now = std::time::Instant::now();
        let req = request.into_inner();
        let space_chals: Vec<i64> = req.space_chals;
        let miner_id: Vec<u8> = req.miner_id;
        let miner_pois_info = req
            .pois_info
            .ok_or(Status::invalid_argument("Miner request data is invalid, loss parameter 'pois_info'"))?;
        let acc: Vec<u8> = miner_pois_info.acc;
        let front: i64 = miner_pois_info.front;
        let rear: i64 = miner_pois_info.rear;
        let key_n: Vec<u8> = miner_pois_info.key_n;
        let key_g: Vec<u8> = miner_pois_info.key_g;
        let miner_space_proof_hash_polkadot_sig: Vec<u8> = req.miner_space_proof_hash_polkadot_sig;
        let proof = req.proof.ok_or(Status::invalid_argument("Error space proof missing proof"))?;
        let left = proof.left;
        let right = proof.right;
        let miner_cess_address = get_ss58_address(&miner_id)?;
        info!("[Pois Verify Single Space Proof] miner {:?} verify single space proof...", miner_cess_address);
        //first verify miner polkadot signature
        let miner_req_hash = try_into_proto_byte_hash(&proof)?;
        if !verify_signature(miner_id.clone(), miner_space_proof_hash_polkadot_sig, &miner_req_hash).map_err(|e| {
            Status::internal("Error happened when verify space proof hash :".to_string() + &e.to_string())
        })? {
            return Err(Status::unauthenticated("The space proof signature verification provided by the miner failed!"))
        };

        let proof_hash_and_left_right = ProofHashAndLeftRight {
            space_proof_hash: miner_req_hash,
            left,
            right,
            tee_id: self.ceseal_identity_key.to_vec(),
        };
        let proof_hash_and_left_right_hash = try_into_proto_byte_hash(&proof_hash_and_left_right)?;

        let mut space_proof = convert_to_space_proof(&proof);
        let p_node = ProverNode::new(
            &miner_id,
            RsaKey { n: BigUint::from_bytes_be(&key_n), g: BigUint::from_bytes_be(&key_g) },
            &acc,
            front,
            rear,
        );

        match self.verifier.verify_space(&p_node, space_chals, &mut space_proof) {
            Ok(_) => {},
            Err(e) =>
                return Err(Status::invalid_argument("Error space proof verify fail:".to_string() + &e.to_string())),
        };

        let signature = match self.podr2_keys.sign_data(&proof_hash_and_left_right_hash) {
            Ok(sig) => sig,
            Err(e) =>
                return Err(Status::internal(
                    "Error space proof verify compute signature fail:".to_string() + &e.error_code.to_string(),
                )),
        };
        info!(
            "[Pois Verify Single Space Proof] miner {:?} Pois Space Proof Verify Single Block in:{:.2?}",
            miner_cess_address,
            now.elapsed()
        );
        Ok(Response::new(ResponseSpaceProofVerify { signature }))
    }
    async fn request_verify_space_total(
        &self,
        request: Request<RequestSpaceProofVerifyTotal>,
    ) -> PoisResult<ResponseSpaceProofVerifyTotal> {
        let req = request.into_inner();
        let proof_list = &req.proof_list;
        let miner_id: [u8; 32] = req
            .miner_id
            .try_into()
            .map_err(|_| Status::invalid_argument("The length of miner_id should be 38!"))?;
        let acc: [u8; 256] = req
            .acc
            .try_into()
            .map_err(|_| Status::invalid_argument("The length of acc should be 256!"))?;
        let space_chals: [u64; 8] = req
            .space_chals
            .iter()
            .map(|&x| x as u64)
            .collect::<Vec<u64>>()
            .try_into()
            .map_err(|_| Status::invalid_argument("space_chals parameters passed incorrectly!"))?;
        let miner_cess_address = get_ss58_address(&miner_id)?;
        let now = Instant::now();
        info!("[Verify Space Total] miner {:?} verify space proof tatal...", miner_cess_address);
        //convert and sort
        let mut blocks_proof = Vec::new();
        for pl in proof_list {
            blocks_proof.push(convert_to_blocks_proof(pl))
        }
        blocks_proof.sort_by_key(|proof| proof.left);
        if blocks_proof.len() == 0 {
            return Err(Status::invalid_argument("Please pass in the correct length of proof_list!".to_string()))
        };

        //check proof list validity,simultaneously set it in haser
        let mut result = true;

        let mut total_proof_hasher = Sha256::new();
        for proof in &blocks_proof {
            if !is_valid_proof(proof, &self.podr2_keys, self.ceseal_identity_key.to_vec())? {
                result = false;
                break
            };
            total_proof_hasher.input(&proof.space_proof_hash);
        }

        //check whether the left and right in the proof are continuous in the entire [front, rear] interval
        //The rule:
        //left = front + 1
        //right = rear + 1
        let front: i64 = req.front;
        let rear: i64 = req.rear;
        if result {
            let mut prev_right = front + 1;
            for proof in &blocks_proof {
                if proof.left == prev_right {
                    //do nothing
                } else {
                    warn!("Provide a complete sequence of blocks for verification! the previous block right: {}, the left of this block is {}", prev_right, proof.left);
                    result = false;
                    break
                }
                prev_right = proof.right;
            }
            if prev_right != rear + 1 {
                warn!("The right({}) and rear({}) numbers of the last block do not equal", prev_right, rear);
                result = false;
            }
        }
        //Concatenate all hashes to calculate the total hash
        let mut total_proof_hash = vec![0u8; 32];
        total_proof_hasher.result(&mut total_proof_hash);

        //compute signature
        let sig_struct = ResponseSpaceProofVerifyTotalSignatureMember {
            miner_id: miner_id.clone().into(),
            total_proof_hash: total_proof_hash.to_vec(),
            front: front as u64,
            rear: rear as u64,
            acc,
            space_chals,
            result,
            tee_acc: self.ceseal_identity_key.clone().into(),
        };

        let mut sig_struct_hasher = Sha256::new();
        sig_struct_hasher.input(&sig_struct.encode());
        let mut sig_struct_hash: Vec<u8> = vec![0u8; 32];
        sig_struct_hasher.result(&mut sig_struct_hash);

        let signature = self.master_key.sign_data(&sig_struct_hash).0.to_vec();
        info!("[Verify Space Total] miner {:?} Verify Space Proof Total in:{:.2?}", miner_cess_address, now.elapsed());

        Ok(Response::new(ResponseSpaceProofVerifyTotal { miner_id: miner_id.to_vec(), idle_result: result, signature }))
    }
}

pub struct PoisChainApi {}
pub struct ChainError {
    error_msg: String,
}

impl Debug for ChainError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChainError: {:?}", self.error_msg)
    }
}

impl Display for ChainError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ChainError: {:?}", self.error_msg)
    }
}
struct BlocksProof {
    left: i64,
    right: i64,
    space_proof_hash: Vec<u8>,
    signature: Vec<u8>,
}

fn convert_to_commits(commits: cestory_api::pois::Commits) -> Commits {
    Commits { file_indexs: commits.file_indexs, roots: commits.roots }
}

fn convert_to_challenge(chals: Vec<Vec<i64>>) -> Challenge {
    let mut challenge = Challenge::default();
    for row in chals {
        let mut int64_slice = Int64Slice::default();
        int64_slice.values = row;
        challenge.rows.push(int64_slice);
    }

    challenge
}

fn convert_to_commit_proof(pb_commit_proof: &cestory_api::pois::CommitProof) -> CommitProof {
    CommitProof {
        node: convert_to_mht_proof(&pb_commit_proof.node.as_ref().unwrap()),
        parents: pb_commit_proof
            .parents
            .iter()
            .map(|pb_parent| convert_to_mht_proof(pb_parent))
            .collect(),
        elders: pb_commit_proof
            .elders
            .iter()
            .map(|pb_parent| convert_to_mht_proof(pb_parent))
            .collect(),
    }
}

fn convert_to_acc_proof(pb_acc_proof: &cestory_api::pois::AccProof) -> AccProof {
    let indexs = pb_acc_proof.indexs.to_vec();
    let labels = pb_acc_proof.labels.to_vec();
    let wit_chains = if let Some(pb_wit_chains) = &pb_acc_proof.wit_chains {
        Some(Box::new(convert_to_witness_node(pb_wit_chains)))
    } else {
        None
    };
    let acc_path = pb_acc_proof.acc_path.to_vec();

    AccProof { indexs, labels, wit_chains, acc_path }
}

fn convert_to_mht_proof(pb_proof: &cestory_api::pois::MhtProof) -> MhtProof {
    MhtProof {
        index: pb_proof.index,
        label: pb_proof.label.clone(),
        paths: pb_proof.paths.iter().cloned().collect(),
        locs: pb_proof.locs.clone(),
    }
}

fn convert_to_witness_node(pb_witness_node: &AccWitnessNode) -> WitnessNode {
    let elem = pb_witness_node.elem.to_vec();
    let wit = pb_witness_node.wit.to_vec();
    let acc =
        if let Some(pb_acc) = &pb_witness_node.acc { Some(Box::new(convert_to_witness_node(pb_acc))) } else { None };

    WitnessNode { elem, wit, acc }
}

pub fn convert_to_space_proof(message: &cestory_api::pois::SpaceProof) -> SpaceProof {
    let mut proofs = Vec::new();
    for proof_group in &message.proofs {
        let mut mht_proof_group = Vec::new();
        for proof in &proof_group.proofs {
            let mht_proof = MhtProof {
                index: proof.index,
                label: proof.label.clone(),
                paths: proof.paths.clone(),
                locs: proof.locs.clone(),
            };
            mht_proof_group.push(mht_proof);
        }
        proofs.push(mht_proof_group);
    }

    let roots = message.roots.clone();

    let mut wit_chains = Vec::new();
    for wit_chain in &message.wit_chains {
        let witness_node = convert_to_witness_node(wit_chain);
        wit_chains.push(witness_node);
    }

    SpaceProof { left: message.left, right: message.right, proofs, roots, wit_chains }
}

fn convert_to_blocks_proof(message: &cestory_api::pois::BlocksProof) -> BlocksProof {
    BlocksProof {
        left: message.proof_hash_and_left_right.clone().unwrap().left,
        right: message.proof_hash_and_left_right.clone().unwrap().right,
        space_proof_hash: message.proof_hash_and_left_right.clone().unwrap().space_proof_hash.to_vec(),
        signature: message.signature.to_vec(),
    }
}

pub fn convert_to_deletion_proof(pb_proof: &cestory_api::pois::RequestVerifyDeletionProof) -> DeletionProof {
    let mut roots: Vec<Vec<u8>> = Vec::new();
    for root in &pb_proof.roots {
        roots.push(root.to_vec());
    }

    let wit_chain = convert_to_witness_node(&pb_proof.wit_chain.as_ref().unwrap());

    let mut acc_path: Vec<Vec<u8>> = Vec::new();
    for path in &pb_proof.acc_path {
        acc_path.push(path.to_vec());
    }

    DeletionProof { roots, wit_chain, acc_path }
}

fn is_valid_proof(proof: &BlocksProof, podr2_keys: &Keys, tee_id: Vec<u8>) -> Result<bool, Status> {
    let proof_hash_and_left_right = ProofHashAndLeftRight {
        space_proof_hash: proof.space_proof_hash.clone(),
        left: proof.left,
        right: proof.right,
        tee_id,
    };
    let mut proof_hash_and_left_right_byte = Vec::new();
    proof_hash_and_left_right
        .encode(&mut proof_hash_and_left_right_byte)
        .map_err(|e| {
            Status::internal(
                "Error happened in check the proof is valid when compute proof_hash_and_left_right_byte:".to_string() +
                    &e.to_string(),
            )
        })?;
    let mut proof_hash_and_left_right_hash = vec![0u8; 32];
    let mut hasher = Sha256::new();
    hasher.input(&proof_hash_and_left_right_byte);
    hasher.result(&mut proof_hash_and_left_right_hash);

    match podr2_keys.verify_data(&proof_hash_and_left_right_hash, &proof.signature) {
        Ok(_) => {},
        Err(e) => {
            info!(
                "Check signature for blocks (left is {},right {} ) fail :{:?}",
                proof.left,
                proof.right,
                e.error_code.to_string()
            );
            return Ok(false)
        },
    }
    if proof.right <= proof.left {
        info!("The right of the block must be greater than the left!");
        return Ok(false)
    };
    Ok(true)
}

fn get_pois_status_and_signature(
    acc: Vec<u8>,
    front: i64,
    rear: i64,
    key_n: Vec<u8>,
    key_g: Vec<u8>,
    miner_id: Vec<u8>,
    master_key: &sr25519::Pair,
    tee_controller_account: [u8; 32],
) -> Result<(PoisStatus, Vec<u8>, Vec<u8>), Status> {
    let pois_status = PoisStatus { acc: acc.clone(), front, rear };
    let raw = MinerPoisInfo {
        miner_id: miner_id[..]
            .try_into()
            .map_err(|_| Status::invalid_argument("The length of miner_id should be 32!"))?,
        front: front as u64,
        rear: rear as u64,
        pois_key: PoISKey {
            g: key_g
                .try_into()
                .map_err(|_| Status::invalid_argument("The length of pois key g should be 256!"))?,
            n: key_n
                .try_into()
                .map_err(|_| Status::invalid_argument("The length of pois key n should be 256!"))?,
        },
        accumulator: acc
            .try_into()
            .map_err(|_| Status::invalid_argument("The length of acc should be 256!"))?,
    };
    let controller_account: AccountId32 = tee_controller_account.into();
    let tee_acc_encode = controller_account.encode();
    let mut original = Vec::new();
    let pois_info_encoded = raw.encode();
    original.extend_from_slice(&pois_info_encoded);
    original.extend_from_slice(&tee_acc_encode);

    let mut hash_raw = vec![0u8; 32];
    let mut hasher = Sha256::new();
    hasher.input(&original);
    hasher.result(&mut hash_raw);

    let signature_with_tee_controller = master_key.sign_data(&hash_raw).0.to_vec();
    hasher.reset();
    hasher.input(&pois_info_encoded);
    hasher.result(&mut hash_raw);
    let signature_pois_info = master_key.sign_data(&hash_raw).0.to_vec();

    info!("[signature_pois_info is {:?}", signature_pois_info.clone());

    Ok((pois_status, signature_pois_info, signature_with_tee_controller))
}

fn verify_pois_status_signature(
    pois_info: MinerPoisInfo,
    master_key: &sr25519::Pair,
    signature: Vec<u8>,
) -> Result<(), Status> {
    let mut hash_raw = vec![0u8; 32];
    let mut hasher = Sha256::new();
    hasher.input(&pois_info.encode());
    hasher.result(&mut hash_raw);

    if !master_key.verify_data(
        &sr25519::Signature(
            signature
                .try_into()
                .map_err(|_| Status::invalid_argument("signature length must be 64"))?,
        ),
        &hash_raw,
    ) {
        return Err(Status::unauthenticated("The miner provided the wrong signature!"))
    } else {
        return Ok(())
    };
}

//TODO: to refactor these code below
use sp_core::crypto::{Ss58AddressFormat, Ss58AddressFormatRegistry, Ss58Codec};

#[derive(thiserror::Error, Debug)]
#[error("{0}")]
struct AccountConvertError(String);

fn get_ss58_address(account: &[u8]) -> std::result::Result<String, AccountConvertError> {
    if account.len() != 32 {
        return Err(AccountConvertError("The length of raw account bytes must be 32".to_string()))
    }
    let ss58_address: AccountId32 = account
        .try_into()
        .map_err(|_| AccountConvertError("account bytes invalid".to_string()))?;
    let address_type = Ss58AddressFormatRegistry::CessTestnetAccount as u16;
    let ss58_cess_address = ss58_address.to_ss58check_with_version(Ss58AddressFormat::custom(address_type));
    Ok(ss58_cess_address)
}

impl From<AccountConvertError> for Status {
    fn from(value: AccountConvertError) -> Self {
        Status::internal(value.0)
    }
}
