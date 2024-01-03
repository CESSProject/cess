use anyhow::{anyhow, Result};
use ces_pdp::{HashSelf, Keys, QElement, Tag as PdpTag};
use cestory_api::podr2::{
    podr2_api_server::Podr2Api, podr2_verifier_api_server::Podr2VerifierApi, request_batch_verify::Qslice, tag,
    RequestBatchVerify, RequestGenTag, ResponseBatchVerify, ResponseGenTag, Tag as ApiTag,
};
use cp_bloom_filter::{binary, BloomFilter};
use crypto::{digest::Digest, sha2::Sha256};
use log::info;
use parity_scale_codec::Encode;
use sp_core::{bounded::BoundedVec, crypto::AccountId32, sr25519, ConstU32, Pair};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use threadpool::ThreadPool;
use tonic::{Request, Response, Status};

type Podr2Result<T> = Result<Response<T>, Status>;

pub struct Podr2Server {
    pub podr2_keys: Keys,
    pub threadpool: Arc<Mutex<ThreadPool>>,
    pub block_num: u64,
    pub tee_controller_account: [u8; 32],
}

pub struct Podr2VerifierServer {
    pub podr2_keys: Keys,
    pub tee_controller_account: [u8; 32],
    pub threadpool: Arc<Mutex<ThreadPool>>,
    pub block_num: u64,
}

struct Podr2Hash {
    alg: Sha256,
}

impl HashSelf for Podr2Hash {
    fn new() -> Self {
        Podr2Hash { alg: Sha256::new() }
    }

    fn load_field(&mut self, d: &[u8]) {
        self.alg.input(d);
    }
    fn c_hash(&mut self) -> Vec<u8> {
        let mut hash_result = vec![0u8; self.alg.output_bytes()];
        self.alg.result(&mut hash_result);
        hash_result
    }
}

#[derive(Encode)]
pub struct Hash(pub [u8; 64]);
#[derive(Encode)]
struct TagSigInfo {
    pub miner: AccountId32,
    pub file_hash: Hash,
    pub tee_acc: AccountId32,
}

#[derive(Encode)]
struct VerifyServiceResultInfo {
    pub miner_pbk: AccountId32,
    pub tee_account_id: AccountId32,
    pub sigma: Vec<u8>,
    pub result: bool,
    pub chal: Challenge,
    pub service_bloom_filter: BloomFilter,
}

#[derive(Encode)]
pub struct Challenge {
    pub random_index_list: BoundedVec<u32, ConstU32<1024>>,
    pub random_list: BoundedVec<[u8; 20], ConstU32<1024>>,
}

pub mod chain_client {
    #[async_trait::async_trait]
    pub trait Podr2ChainHelper: Send + Sync + 'static {
        type Error: std::error::Error + Send;

        async fn check_if_file_hash_in_deal_map(
            &self,
            file_hash: [u8; 64],
            fragment_hash: [u8; 64],
        ) -> Result<(), Self::Error>;

        async fn query_fragment_infomation(&self, fragment_hash: [u8; 64]) -> Result<(u32, u32), Self::Error>;

        async fn query_system_sync_state(&self) -> Result<u64, Self::Error>;
    }
}

#[tonic::async_trait]
impl Podr2Api for Podr2Server {
    #[must_use]
    async fn request_gen_tag(&self, request: Request<RequestGenTag>) -> Podr2Result<ResponseGenTag> {
        let now = Instant::now();
        let request = request.into_inner();
        let mut h = Podr2Hash::new();
        h.load_field(request.custom_data.as_bytes());

        //check this tag should be calculate or not
        info!(
            "[🚀Generate tag] Request to generate tag for file hash [{:?}] bytes length is {}",
            request.file_name,
            request.file_name.len()
        );

        let pool = self
            .threadpool
            .lock()
            .map_err(|e| Status::internal("lock global threadpool fail:".to_string() + &e.to_string()))?;
        //check fragement data is equal to fragement name
        let mut check_file_hash = Podr2Hash::new();
        check_file_hash.load_field(&request.fragment_data);
        let fragment_data_hash = hex::encode(check_file_hash.c_hash());
        if !fragment_data_hash.eq(&request.fragment_name) {
            return Err(Status::invalid_argument(format!(
                "file: {:?} hash is :{:?}",
                &request.fragment_name, &fragment_data_hash
            )))
        }
        let tag = self
            .podr2_keys
            .sig_gen_with_data(request.fragment_data, self.block_num, &request.fragment_name, h, pool.clone())
            .map_err(|e| Status::internal(format!("AlgorithmError: {}", e.error_code.to_string())))?;
        let u_sig = self.podr2_keys.sign_data_with_sha256(tag.t.u.as_bytes()).map_err(|e| {
            Status::invalid_argument(format!("Failed to calculate u's signature {:?}", e.error_code.to_string()))
        })?;
        let tag_sig_info = TagSigInfo {
            miner: request.miner_id.clone()[..].try_into().map_err(|_| {
                Status::invalid_argument(format!(
                    "request generate tag fail,length of miner id should be 32,but now is {}",
                    request.miner_id.len()
                ))
            })?,
            file_hash: Hash(request.file_name.clone().into_bytes().try_into().map_err(|_| {
                Status::invalid_argument(format!(
                    "request generate tag fail,length of file_name id should be 64,but now is {}",
                    request.file_name
                ))
            })?),
            tee_acc: self.tee_controller_account.into(),
        };
        let tag_sig_info = self
            .podr2_keys
            .sign_data(&calculate_hash(&tag_sig_info.encode()))
            .map_err(|e| {
                Status::invalid_argument(format!("Failed to calculate tag_sig_info {:?}", e.error_code.to_string()))
            })?;
        info!("[🚀Generate tag] PoDR2 Sig Gen Completed in: {:.2?}. file name is {:?}", now.elapsed(), &tag.t.name);

        Ok(Response::new(ResponseGenTag { tag: Some(convert_to_tag(tag)), u_sig, tag_sig_info }))
    }
}

#[tonic::async_trait]
impl Podr2VerifierApi for Podr2VerifierServer {
    #[must_use]
    async fn request_batch_verify(
        &self,
        request: tonic::Request<RequestBatchVerify>,
    ) -> Podr2Result<ResponseBatchVerify> {
        let request = request.into_inner();
        let now = Instant::now();
        let mut result = ResponseBatchVerify::default();
        let agg_proof = if let Some(agg_proof) = request.agg_proof {
            agg_proof
        } else {
            return Err(Status::invalid_argument("Lack of request parameter agg_proof"))
        };
        let qslices = if let Some(qslices) = request.qslices {
            qslices
        } else {
            return Err(Status::invalid_argument("Lack of request parameter qslices"))
        };
        let q_elements = convert_to_q_elements(qslices.clone())?;

        let mut service_bloom_filter = BloomFilter([0u64; 256]);
        let miner_pbk: [u8; 32] = request
            .miner_pbk
            .clone()
            .try_into()
            .map_err(|_| Status::internal("There is a problem with the format of miner_pbk"))?;
        let miner_pbk = AccountId32::from(miner_pbk);

        //verify signature
        if !verify_signature(request.miner_pbk.clone(), request.miner_peer_id_sign, request.peer_id.as_ref())
            .map_err(|e| Status::internal(format!("verify miner signature error: {}", e.to_string())))?
        {
            return Err(Status::invalid_argument("invalid signature"))
        }

        //compute bloom
        for name in agg_proof.names.clone() {
            let filehash: [u8; 64] = name
                .as_bytes()
                .try_into()
                .map_err(|_| Status::invalid_argument(format!("The provided name hash {:?} is incorrect", &name)))?;
            let data = binary(filehash)
                .map_err(|_| Status::invalid_argument(format!("The provided hex {:?} is wrong", &name)))?;
            service_bloom_filter
                .insert(*data)
                .map_err(|e| Status::internal(format!("Failed to calculate bloom filter :{:?}", e)))?;
        }

        //verify batch proof
        let pool = self
            .threadpool
            .lock()
            .map_err(|e| Status::internal("lock global threadpool fail:".to_string() + &e.to_string()))?;
        info!("[Batch verify] Getting a lock takes time: {:.2?}", now.elapsed());
        if agg_proof.names.is_empty() {
            result.batch_verify_result = true;
        } else {
            //Check the u is from teeworker or not
            let mut iterator = request
                .u_sigs
                .iter()
                .zip(agg_proof.names.iter())
                .take((request.u_sigs.len() as f64 * 0.049).ceil() as usize);
            if !iterator.all(|(u_sig, name)| {
                match self.podr2_keys.verify_data(&calculate_hash(name.as_bytes()), &u_sig) {
                    Ok(_) => true,
                    Err(_) => {
                        info!("[Batch verify] u_sig is:{:?} name is:{:?} is inconsistent!", u_sig, name);
                        false
                    },
                }
            }) {
                return Err(Status::internal("The u_sig passed in is inconsistent with the u in the corresponding tag."))
            }

            result.batch_verify_result = self
                .podr2_keys
                .batch_verify(
                    agg_proof.us,
                    agg_proof.names,
                    q_elements.0,
                    agg_proof.sigma.clone(),
                    agg_proof.mus,
                    pool.clone(),
                )
                .map_err(|e| {
                    Status::aborted(format!(
                        "AlgorithmError: aggregate verify idle file error {:?}",
                        e.error_code.to_string()
                    ))
                })?;
        }

        let raw = VerifyServiceResultInfo {
            miner_pbk,
            tee_account_id: self.tee_controller_account.clone().into(),
            result: result.batch_verify_result,
            sigma: agg_proof.sigma.into_bytes(),
            chal: q_elements.1,
            service_bloom_filter: service_bloom_filter.clone(),
        };
        let raw_hash = calculate_hash(&raw.encode());
        //using podr2 keypair sign
        let podr2_sign = self.podr2_keys.sign_data(&raw_hash).map_err(|e| {
            Status::internal(format!(
                "Error thrown when signing with the podr2 key when submitting the verify result: {:?}",
                e.error_code.to_string()
            ))
        })?;

        result.tee_account_id = self.tee_controller_account.to_vec();
        result.service_bloom_filter = service_bloom_filter.0.to_vec();
        result.signature = podr2_sign;
        info!("[Batch verify] Batch Verify Completed in: {:.2?}.", now.elapsed());
        Ok(Response::new(result))
    }
}

pub fn calculate_hash(input: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.input(input);
    let mut output = vec![0u8; hasher.output_bytes()];
    hasher.result(&mut output);
    output
}

fn convert_to_q_elements(qslices: Qslice) -> Result<(Vec<QElement>, Challenge), Status> {
    let mut chal1 = Vec::new();
    let mut chal2 = Challenge { random_index_list: BoundedVec::default(), random_list: BoundedVec::default() };
    let mut index = 0;
    for q_index in qslices.random_index_list {
        let q = QElement { i: q_index as u64, v: qslices.random_list[index].clone() };
        chal1.push(q);

        let single_index: u32 = q_index as u32;
        let random_number: [u8; 20] = qslices.random_list[index]
            .clone()
            .try_into()
            .map_err(|_| Status::invalid_argument("The challenge number length of a single challenge is not 20!"))?;
        chal2
            .random_index_list
            .try_push(single_index)
            .map_err(|_| Status::internal("Failed to convert random_index_list for calculate encode value!"))?;
        chal2
            .random_list
            .try_push(random_number)
            .map_err(|_| Status::internal("Failed to convert random_list for calculate encode value!"))?;
        index += 1;
    }

    Ok((chal1, chal2))
}

fn convert_to_tag(value: PdpTag) -> ApiTag {
    ApiTag {
        t: Some(tag::T { name: value.t.name, u: value.t.u, phi: value.t.phi }),
        phi_hash: value.phi_hash,
        attest: value.attest,
    }
}

/// verify_signature
/// Used to verify data signed with the private key of the miner's wallet
fn verify_signature(spk: Vec<u8>, sig: Vec<u8>, raw: &[u8]) -> Result<bool> {
    //verify miner_peer_id_sign first
    let spk: [u8; 32] = match spk.try_into() {
        Ok(pbk) => pbk,
        Err(_) => return Err(anyhow!("There is a problem with the format of public key")),
    };
    let sig: [u8; 64] = match sig.try_into() {
        Ok(sign) => sign,
        Err(_) => return Err(anyhow!("There is a problem with the format of signature")),
    };

    let spk = sr25519::Public::from_raw(spk);
    let sig = sr25519::Signature::from_raw(sig);
    Ok(sr25519::Pair::verify(&sig, raw, &spk))
}
