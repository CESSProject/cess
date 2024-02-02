use crate::{expert::CesealExpertStub, types::ThreadPoolSafeBox};
use anyhow::{anyhow, Result};
use ces_crypto::sr25519::Signing;
use ces_pdp::{HashSelf, Keys, QElement, Tag as PdpTag};
use cestory_api::podr2::{
    podr2_api_server::{self, Podr2Api},
    podr2_verifier_api_server::{self, Podr2VerifierApi},
    request_batch_verify::Qslice,
    tag, EchoMessage, GenTagMsg, RequestBatchVerify, RequestGenTag, ResponseBatchVerify, ResponseGenTag, Tag as ApiTag,
};
use cp_bloom_filter::{binary, BloomFilter};
use crypto::{digest::Digest, sha2::Sha256};
use log::info;
use parity_scale_codec::Encode;
use sp_core::{bounded::BoundedVec, crypto::AccountId32, sr25519, ByteArray, ConstU32, Pair};
use std::{error::Error, io::ErrorKind, time::Instant};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use tonic::{Request, Response, Status, Streaming};
// type ResponseStream = Pin<Box<dyn Stream<Item = Result<ResponseGenTag, Status>> + Send>>;

mod proxy;

pub type Podr2ApiServer = podr2_api_server::Podr2ApiServer<Podr2ApiServerProxy<Podr2Server>>;
pub type Podr2VerifierApiServer =
    podr2_verifier_api_server::Podr2VerifierApiServer<Podr2VerifierApiServerProxy<Podr2VerifierServer>>;
pub type Podr2Result<T> = Result<Response<T>, Status>;

pub use proxy::{Podr2ApiServerProxy, Podr2VerifierApiServerProxy};

pub fn new_podr2_api_server(ceseal_expert: CesealExpertStub) -> Podr2ApiServer {
    let podr2_keys = ceseal_expert.podr2_key().clone();
    let master_key = crate::get_sr25519_from_rsa_key(podr2_keys.clone().skey);
    let inner = Podr2ApiServerProxy {
        inner: Podr2Server {
            podr2_keys,
            master_key,
            threadpool: ceseal_expert.thread_pool(),
            block_num: 1024,
            ceseal_identity_key: ceseal_expert.identify_public_key().0,
        },
        ceseal_expert,
    };
    Podr2ApiServer::new(inner)
}

pub fn new_podr2_verifier_api_server(ceseal_expert: CesealExpertStub) -> Podr2VerifierApiServer {
    let podr2_keys = ceseal_expert.podr2_key().clone();
    let master_key = crate::get_sr25519_from_rsa_key(podr2_keys.clone().skey);
    let inner = Podr2VerifierApiServerProxy {
        inner: Podr2VerifierServer {
            podr2_keys,
            master_key,
            threadpool: ceseal_expert.thread_pool(),
            block_num: 1024,
            ceseal_identity_key: ceseal_expert.identify_public_key().0,
        },
        ceseal_expert,
    };
    Podr2VerifierApiServer::new(inner)
}

pub struct Podr2Server {
    pub podr2_keys: Keys,
    pub master_key: sr25519::Pair,
    pub threadpool: ThreadPoolSafeBox,
    pub block_num: u64,
    pub ceseal_identity_key: [u8; 32],
}

pub struct Podr2VerifierServer {
    pub podr2_keys: Keys,
    pub master_key: sr25519::Pair,
    pub ceseal_identity_key: [u8; 32],
    pub threadpool: ThreadPoolSafeBox,
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
#[derive(Encode)]
pub struct TagSigInfo {
    pub miner: AccountId32,
    pub digest: BoundedVec<DigestInfo, ConstU32<1000>>,
    pub file_hash: Hash,
}
#[derive(Encode)]
pub struct DigestInfo {
    pub fragment: Hash,
    pub tee_puk: sr25519::Public,
}

#[tonic::async_trait]
impl Podr2Api for Podr2Server {
    #[allow(non_camel_case_types)]
    type request_gen_tagStream = proxy::ResponseStream;
    #[must_use]
    async fn request_gen_tag<'life0>(
        &'life0 self,
        request: Request<Streaming<RequestGenTag>>,
    ) -> Podr2Result<Self::request_gen_tagStream> {
        let mut in_stream = request.into_inner();
        let mut stream_rec_times = 0;
        let (resp_tx, resp_rx) = mpsc::channel(128);

        let new_self = Podr2Server {
            podr2_keys: self.podr2_keys.clone(),
            master_key: self.master_key.clone(),
            threadpool: self.threadpool.clone(),
            block_num: self.block_num.clone(),
            ceseal_identity_key: self.ceseal_identity_key.clone(),
        };
        //start receive
        tokio::spawn(async move {
            while let Some(result) = in_stream.next().await {
                match result {
                    Ok(v) => {
                        if v.fragment_data.is_empty() && stream_rec_times == 0 {
                            resp_tx
                                .send(Ok(ResponseGenTag {
                                    processing: true,
                                    msg: Some(GenTagMsg {
                                        tag: Some(convert_to_tag(PdpTag::default())),
                                        u_sig: Vec::new(),
                                        signature: Vec::new(),
                                    }),
                                }))
                                .await
                                .expect("Sending GenTagMsg with processing true fail!");
                            stream_rec_times += 1;
                            continue
                        };
                        if !v.fragment_data.is_empty() && stream_rec_times == 1 {
                            match new_self.process_gen_tag_request(v) {
                                Ok(response) => resp_tx
                                    .send(Ok(response))
                                    .await
                                    .expect("Sending GenTagMsg final process result fail!"),
                                Err(err) => resp_tx
                                    .send(Err(err))
                                    .await
                                    .expect("Sending GenTagMsg failure msg to miner fail!"),
                            };
                            break
                        };
                        resp_tx
                            .send(Err(Status::invalid_argument(
                                "Please ask if TEEWorker is available before requesting to generate a tag!",
                            )))
                            .await
                            .expect("Sending error msg when gen tag fail!");
                        break
                    },
                    Err(err) => {
                        if let Some(io_err) = match_for_io_error(&err) {
                            if io_err.kind() == ErrorKind::BrokenPipe {
                                info!("The connection to the miner is interrupted");
                                break
                            }
                        }

                        match resp_tx.send(Err(err)).await {
                            Ok(_) => (),
                            Err(_err) => break,
                        }
                    },
                }
            }
        });
        let out_stream = ReceiverStream::new(resp_rx);

        Ok(Response::new(Box::pin(out_stream) as Self::request_gen_tagStream))
    }

    /// A echo rpc to measure network RTT.
    async fn echo(&self, request: Request<EchoMessage>) -> Podr2Result<EchoMessage> {
        let echo_msg = request.into_inner().echo_msg;
        Ok(Response::new(EchoMessage { echo_msg }))
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
        let miner_id: [u8; 32] = request
            .miner_id
            .clone()
            .try_into()
            .map_err(|_| Status::internal("There is a problem with the format of miner_id"))?;
        let miner_id = AccountId32::from(miner_id);

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
                .zip(agg_proof.us.iter())
                .take((request.u_sigs.len() as f64 * 0.049).ceil() as usize);
            if !iterator.all(|(u_sig, u)| match self.podr2_keys.verify_data(&calculate_hash(u.as_bytes()), &u_sig) {
                Ok(_) => true,
                Err(_) => {
                    info!("[Batch verify] u_sig is:{:?} name is:{:?} is inconsistent!", u_sig, u);
                    false
                },
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
            miner_pbk: miner_id,
            tee_account_id: self.ceseal_identity_key.into(),
            result: result.batch_verify_result,
            sigma: agg_proof.sigma.into_bytes(),
            chal: q_elements.1,
            service_bloom_filter: service_bloom_filter.clone(),
        };
        //using podr2 keypair sign
        let podr2_sign = self.master_key.sign_data(&calculate_hash(&raw.encode())).0.to_vec();

        result.tee_account_id = self.ceseal_identity_key.to_vec();
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

impl Podr2Server {
    fn process_gen_tag_request<'life0>(&'life0 self, request: RequestGenTag) -> Result<ResponseGenTag, Status> {
        let now = Instant::now();
        let mut h = Podr2Hash::new();
        h.load_field(request.custom_data.as_bytes());

        //check this tag should be calculate or not
        info!(
            "[🚀Generate tag] Request to generate tag for file hash [{:?}] bytes length is {}",
            request.file_name,
            request.file_name.as_bytes().len()
        );
        let file_hash: [u8; 64] = (*(request.file_name.as_bytes()))
            .try_into()
            .map_err(|_| Status::invalid_argument("file_name hash bytes length should be 64".to_string()))?;

        let pool = self
            .threadpool
            .lock()
            .map_err(|e| Status::internal("lock global threadpool fail:".to_string() + &e.to_string()))?;
        //check fragement data is equal to fragement name
        let mut check_fragment_hash = Podr2Hash::new();
        check_fragment_hash.load_field(&request.fragment_data);
        let fragment_data_hash_byte = check_fragment_hash.c_hash();
        let fragment_data_hash_string = hex::encode(&fragment_data_hash_byte);
        if !fragment_data_hash_string.eq(&request.fragment_name) {
            return Err(Status::invalid_argument(format!(
                "fragment: {:?} hash is :{:?}",
                &request.fragment_name, &fragment_data_hash_string
            )))
        }
        let tag = self
            .podr2_keys
            .sig_gen_with_data(request.fragment_data, self.block_num, &request.fragment_name, h, pool.clone())
            .map_err(|e| Status::internal(format!("AlgorithmError: {}", e.error_code.to_string())))?;
        let u_sig = self.podr2_keys.sign_data(&calculate_hash(tag.t.u.as_bytes())).map_err(|e| {
            Status::invalid_argument(format!("Failed to calculate u's signature {:?}", e.error_code.to_string()))
        })?;

        let mut tag_sig_info_history = TagSigInfo {
            miner: AccountId32::from_slice(&request.miner_id[..])
                .map_err(|_| Status::internal("invalid miner account"))?,
            digest: BoundedVec::new(),
            file_hash: Hash(file_hash),
        };
        if !request.tee_digest_list.is_empty() {
            for tdl in request.tee_digest_list {
                let digest_info_history = DigestInfo {
                    fragment: Hash(tdl.fragment_name.try_into().map_err(|_| {
                        Status::invalid_argument(
                            "The length of fragment name in tee_digest_list should be 64".to_string(),
                        )
                    })?),
                    tee_puk: sr25519::Public(tdl.tee_account_id.try_into().map_err(|_| {
                        Status::invalid_argument(
                            "The length of tee worker id in tee_digest_list should be 64".to_string(),
                        )
                    })?),
                };
                tag_sig_info_history
                    .digest
                    .try_push(digest_info_history)
                    .map_err(|_| Status::internal("Fail to conver tee_digest_list from miner into".to_string()))?;
            }
            if !self.master_key.verify_data(
                &sr25519::Signature(request.last_tee_signature.try_into().map_err(|_| {
                    Status::invalid_argument("The last_tee_signature you provided is length is not 64".to_string())
                })?),
                &calculate_hash(&tag_sig_info_history.encode()),
            ) {
                return Err(Status::invalid_argument("The last_tee_signature you provided is incorrect".to_string()))
            };
        };

        let new_tee_record = DigestInfo {
            fragment: Hash(request.fragment_name.as_bytes().try_into().unwrap()),
            tee_puk: sr25519::Public(self.ceseal_identity_key.clone()),
        };
        tag_sig_info_history.digest.try_push(new_tee_record).map_err(|_| {
            Status::invalid_argument("Can not push the new tee record into tag_sig_info_history".to_string())
        })?;

        let signature = self
            .master_key
            .sign_data(&calculate_hash(&tag_sig_info_history.encode()))
            .0
            .to_vec();
        info!("[🚀Generate tag] PoDR2 Sig Gen Completed in: {:.2?}. file name is {:?}", now.elapsed(), &tag.t.name);
        Ok(ResponseGenTag {
            processing: true,
            msg: Some(GenTagMsg { tag: Some(convert_to_tag(tag)), u_sig, signature }),
        })
    }
}

fn match_for_io_error(err_status: &Status) -> Option<&std::io::Error> {
    let mut err: &(dyn Error + 'static) = err_status;
    loop {
        if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
            return Some(io_err)
        }

        if let Some(h2_err) = err.downcast_ref::<h2::Error>() {
            if let Some(io_err) = h2_err.get_io() {
                return Some(io_err)
            }
        }

        err = match err.source() {
            Some(err) => err,
            None => return None,
        };
    }
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
pub fn verify_signature(spk: Vec<u8>, sig: Vec<u8>, raw: &[u8]) -> Result<bool> {
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
