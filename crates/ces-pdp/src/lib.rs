use crypto::{digest::Digest, sha2::Sha256};
use hex;
use num_bigint::{BigUint, ToBigUint};
use num_bigint_dig::RandBigInt;
use num_integer::Integer;
use num_traits::Zero;
use rand::{Rng, RngCore};
use rsa::{
    pkcs1::EncodeRsaPublicKey, rand_core::OsRng, Pkcs1v15Sign, PublicKey, PublicKeyParts, RsaPrivateKey, RsaPublicKey,
};
use std::{
    collections::HashSet,
    convert::From,
    fs,
    io::{Read, Seek, SeekFrom},
    str::FromStr,
    sync::{mpsc::channel, Arc},
};
// use rsa::pkcs8::{DecodePrivateKey};
use serde::{Deserialize, Serialize};
use threadpool::ThreadPool;

#[derive(Debug)]
pub enum FailCode {
    InternalError(String),
    ParameterError(String),
}

#[derive(Debug)]
pub struct PDPError {
    pub error_code: FailCode,
}

impl std::fmt::Display for FailCode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FailCode::InternalError(s) => write!(f, "{}", s),
            FailCode::ParameterError(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Keys {
    pub skey: RsaPrivateKey,
    pub pkey: RsaPublicKey,
}

pub trait HashSelf {
    fn new() -> Self;
    fn load_field(&mut self, d: &[u8]);
    fn c_hash(&mut self) -> Vec<u8>;
}

// The struct of the calculation result of the sig_gen_with_path() function
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Tag {
    pub t: T,
    pub phi_hash: String,
    pub attest: String,
}

impl Default for Tag {
    fn default() -> Self {
        Tag {
            t: T { name: "".to_string(), u: "".to_string(), phi: vec![] },
            phi_hash: "".to_string(),
            attest: "".to_string(),
        }
    }
}

//Information about the file
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub struct T {
    pub name: String,
    pub u: String,
    pub phi: Vec<String>,
}

//The W structure that needs to be used when calculating sigma
#[derive(Serialize, Clone, Debug)]
pub struct W {
    name: String,
    i: u64,
}

impl W {
    fn new() -> W {
        W { name: "".to_string(), i: 0 }
    }

    fn hash(&mut self) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.input(self.name.as_bytes());
        hasher.input(format!("{}", self.i).as_bytes());
        let mut hash_result = vec![0u8; hasher.output_bytes()];
        hasher.result(&mut hash_result);
        hash_result
    }
}

/*
    Challenge Part
*/

#[derive(Serialize, Clone, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub struct QElement {
    pub i: u64,
    pub v: Vec<u8>,
}

impl QElement {
    fn new() -> Self {
        QElement { i: 0, v: Vec::new() }
    }
}

/*
    CSS part
*/

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Proof {
    pub mu: String,
    pub sigma: String,
}

pub fn gen_keypair(bits: usize) -> Keys {
    let mut rng = OsRng;
    // let skey =
    // RsaPrivateKey::from_pkcs8_der(&hex::decode("
    // 308204bf020100300d06092a864886f70d0101010500048204a9308204a50201000282010100b75a87cf2882afe73186dccac8be83ce9180b189503ab35a835273c56658caa881d8bef3d5c41c14a0e88a444a98e0a6d0b47f301f07b7856a9e8ce631907e147c3d6839e41894abff8c1cebd00dd668014f1f62acefe7ee4f22e2795a5c8e93dc9864106dae91223743ced4729ff75b4e6355b691bd93bdf4f323c37d3566a91d1f3236b88ed40680e102cceb7fc4b299253d710926a695c448379def285f0ef6601f45475750c0403cb444b4666856dfd0b076c9cc51f34987ef4443bc57e9fefed1002134fd4d26fe28f1ae8bcb2f8c8f4c410630364b553d90d1bb44578fb31fb8174042622645a37ea2133c2e375f09f09f8ed03c86d754038944de33ef020301000102820101008e0d0f66c985e66e018af08812dab71754d715b4c27997f6aa03393a583eb653b2b58fcb2d745025133cd5d26ed0de4b9f2a17d7da528a364d12252b3a7f2d8f056a35d3940a0f34ea394d36ccebcb8eac64f675e671bf887bbb1526db8115dd2c6ccc29863056b4e1882137aace903f270e029bfd719882f4c16295d3ed89686f15d6784b009e4f5fd189335856df4dfe2699b1912814ac79b11159f9ba47068e1e969e505cea0721ac190bdb3d0960b0959e96cc50f78861289a70cd11761f36f67de7952027ce7c1513b94c17663288825b4b4ad0e9f6b7b072e684791a87547107d2a3390abcb50ef5b19c9863730e4fc2fdfdcc173e504599c7b9dd224102818100eb3f291a9c5786cb8fc05a5dec971013b3fa0a2f1706f2e6ec9b7d4512cad13c668821ecb3b26e13069f48c3ebe8584f4cfc9f7d6676594ec43cf4f05a36586babe0271f0cd605d4dfa3160e4468dbf6bc3c07d60e2faa0b07862888c8e2e952ae1965c74ec13af2e32357dc547def29876741d90216fff6f6053a3d206cd83f02818100c78769114af8712dec45cab44e018b43226a5618bd1abdd082c2af761942c7063747f7b6f26fc81af83c1b4e579182e821d39ddfd9dcf9acacd11b17e7c7103d49c36e1f240aec391a88f772ba59b2dc2b5e4a3a5a8d12bb611caabba4ed94eb033006f7901351c94b06e619fc0b0e1ea4afff39e7d2358aa6255c8404e03851028181009e67df63bdd6ea3b7446d012e2d72dca36acade2db9ca03f831f8890d480a1b80c4aaa9d5abb51879a33f3a989d6e07035a4fe3850a06caeaf516495dc09302d0085659270f044e8fcd63269d502ceeb2c01383d993bdb42a6045e930cc24ede8fc12659b8dc40b780df31b0796c7b78f9663c5ec61b7aac6f2941f81b376fbd028181009180fdedce6e3e9aeea236a026029a3beaed00bc29ab46a0b7baa199cdf2149143df07963255b1e778fedc2ad55117d5905571dbbc5498fe83483a29c4ac35fb7bbf389f1cf99a2a4a5f779402b146eda7f2aacec319fd7f07e28fdf26f6da924750cec3da1d3c973e4f599db95f967c623cb632d40b476044a91a3e6c0fd7010281807fc27079e38f6574ce13d730f22771e3dc82eea435785a3602bd9518d8d804cb31b1c80ebcf5cecbcbcd333ee0095510fdb7a1bb3cd57e412b374d7c63f83af6022d4b8dd081713c50c48cadfa4d0a934483616f53cb8edb59617f61ed3a1ba4d7026cd7aeeff3e76e319f98f4c13a69476c7f8128152724f819a3822e4c9dcf"
    // ).unwrap()).unwrap();
    let skey = RsaPrivateKey::new(&mut rng, bits).expect("failed to generate a key");
    let pkey = RsaPublicKey::from(&skey);
    println!("Podr2 public key is {:?}", hex::encode(pkey.to_pkcs1_der().unwrap().to_vec()));

    Keys { skey, pkey }
}

pub fn gen_keypair_from_private_key(skey: RsaPrivateKey) -> Keys {
    let pkey = skey.to_public_key();
    Keys { skey, pkey }
}

use rsa::{
    pkcs1v15::{Signature, SigningKey},
    signature::{RandomizedSigner, SignatureEncoding},
};

impl Keys {
    pub fn sign_data_with_sha256(&self, data: &[u8]) -> Result<Vec<u8>, PDPError> {
        let mut rng = rand::thread_rng();
        let signing_key: SigningKey<sha2::Sha256> = SigningKey::<sha2::Sha256>::new_with_prefix(self.skey.clone());
        let signature: Signature = signing_key.sign_with_rng(&mut rng, data);
        Ok(signature.to_bytes().to_vec())
    }

    pub fn sign_data(&self, data: &[u8]) -> Result<Vec<u8>, PDPError> {
        self.skey
            .sign(Pkcs1v15Sign::new_raw(), data)
            .map_err(|e| PDPError { error_code: FailCode::ParameterError(e.to_string()) })
    }

    pub fn verify_data(&self, hashed: &[u8], sig: &[u8]) -> Result<(), PDPError> {
        self.pkey
            .verify(Pkcs1v15Sign::new_raw(), hashed, sig)
            .map_err(|e| PDPError { error_code: FailCode::ParameterError(e.to_string()) })
    }

    pub fn sig_gen_with_path<H: HashSelf>(
        &self,
        file_path: &String,
        n_blocks: u64,
        name: &String,
        mut h: H,
        pool: ThreadPool,
    ) -> Result<Tag, PDPError> {
        let mut result = Tag::default();
        //generate u
        let mut rng = rand::thread_rng();
        let u_tmp = rng.gen_biguint_range(&Zero::zero(), self.pkey.n());
        let u = Arc::new(
            num_bigint::BigUint::from_str(&u_tmp.to_string())
                .map_err(|e| PDPError { error_code: FailCode::InternalError(e.to_string()) })?,
        );
        //open a file
        let mut f =
            fs::File::open(file_path).map_err(|e| PDPError { error_code: FailCode::ParameterError(e.to_string()) })?;

        //detect file size
        let file_size = match f.seek(SeekFrom::End(0)) {
            Ok(s) => s,
            Err(e) => return Err(PDPError { error_code: FailCode::InternalError(e.to_string()) }),
        };
        if file_size % n_blocks != 0 {
            return Err(PDPError {
                error_code: FailCode::InternalError(format!(
                    "The size of file {:?} is {}, which cannot be divisible by {} blocks",
                    name, file_size, n_blocks
                )),
            });
        };

        let each_chunks_size = file_size / n_blocks;
        let mut offset = 0_u64;

        let (tx, rx) = channel();
        result.t.phi = vec!["".to_string(); n_blocks as usize];
        for i in 0..n_blocks {
            let mut chunks = vec![0u8; each_chunks_size as usize];

            let _ = f.seek(SeekFrom::Start(offset));
            match f.read_exact(&mut chunks) {
                Ok(_) => {},
                Err(e) => {
                    return Err(PDPError {
                        error_code: FailCode::InternalError(format!("Fail in read file :{:?}", e.to_string())),
                    })
                },
            };

            let tx = tx.clone();
            let u = u.clone();
            let ssk = self.skey.clone();
            let name = name.clone();
            pool.execute(move || {
                tx.send((generate_sigma(ssk, chunks, name, i, u), i)).unwrap();
            });
            offset += each_chunks_size;
        }
        let iter = rx.iter().take(n_blocks as usize);
        let mut phi_hasher = Sha256::new();
        for k in iter {
            result.t.phi[k.1 as usize] = k.0.clone();
            phi_hasher.input(k.0.as_bytes());
        }
        let mut phi_hash = vec![0u8; phi_hasher.output_bytes()];
        phi_hasher.result(&mut phi_hash);

        result.t.name = name.clone();
        result.t.u = u.clone().to_string();
        result.phi_hash = hex::encode(phi_hash.clone());

        h.load_field(&name.as_bytes());
        h.load_field(&u.clone().to_string().as_bytes());
        h.load_field(&phi_hash);
        let attest = self.sign_data(&h.c_hash())?;
        result.attest = hex::encode(attest);

        Ok(result)
    }

    pub fn sig_gen_with_data<H: HashSelf>(
        &self,
        data: Vec<u8>,
        n_blocks: u64,
        name: &String,
        mut h: H,
        pool: ThreadPool,
    ) -> Result<Tag, PDPError> {
        let mut result = Tag {
            t: T { name: "".to_string(), u: "".to_string(), phi: vec![] },
            phi_hash: "".to_string(),
            attest: "".to_string(),
        };
        //generate u
        let mut rng = rand::thread_rng();
        let u_tmp = rng.gen_biguint_range(&Zero::zero(), self.pkey.n());
        let u = Arc::new(
            num_bigint::BigUint::from_str(&u_tmp.to_string())
                .map_err(|e| PDPError { error_code: FailCode::InternalError(e.to_string()) })?,
        );

        //detect file size
        let file_size = data.len() as u64;
        if file_size % n_blocks != 0 {
            return Err(PDPError {
                error_code: FailCode::InternalError(format!(
                    "The size of file {:?} is {}, which cannot be divisible by {} blocks",
                    name, file_size, n_blocks
                )),
            });
        };

        let each_chunks_size = file_size / n_blocks;

        let (tx, rx) = channel();
        result.t.phi = vec!["".to_string(); n_blocks as usize];
        for i in 0..n_blocks {
            // let mut chunks = vec![0u8; each_chunks_size as usize];
            let chunks: Vec<u8> = data[(i * each_chunks_size) as usize..((i + 1) * each_chunks_size) as usize].to_vec();

            let tx = tx.clone();
            let u = u.clone();
            let ssk = self.skey.clone();
            let name = name.clone();
            pool.execute(move || {
                tx.send((generate_sigma(ssk, chunks, name, i, u), i)).unwrap();
            });
        }
        let iter = rx.iter().take(n_blocks as usize);
        let mut phi_hasher = Sha256::new();
        for k in iter {
            result.t.phi[k.1 as usize] = k.0.clone();
            phi_hasher.input(k.0.as_bytes());
        }
        let mut phi_hash = vec![0u8; phi_hasher.output_bytes()];
        phi_hasher.result(&mut phi_hash);

        result.t.name = name.clone();
        result.t.u = u.clone().to_string();
        result.phi_hash = hex::encode(phi_hash.clone());

        h.load_field(&name.as_bytes());
        h.load_field(&u.clone().to_string().as_bytes());
        h.load_field(&phi_hash);
        let attest = self.sign_data_with_sha256(&h.c_hash())?;
        result.attest = hex::encode(attest);

        Ok(result)
    }

    pub fn redo_sig_gen_with_new_key<H: HashSelf>(
        &self,
        tag: Tag,
        mut h: H,
        pool: ThreadPool,
    ) -> Result<Tag, PDPError> {
        let mut i = 0;
        let mut result = Tag::default();
        result.t.phi = vec!["".to_string(); tag.t.phi.len() as usize];
        let (tx, rx) = channel();

        for phi in tag.t.phi.clone() {
            let ssk = self.skey.clone();
            let tx = tx.clone();
            pool.execute(move || {
                tx.send((redo_generate_sigma(ssk, phi), i)).unwrap();
            });
            i += 1;
        }
        let iter = rx.iter().take(tag.t.phi.len() as usize);
        let mut phi_hasher = Sha256::new();
        for k in iter {
            let phi = k.0?;
            result.t.phi[k.1 as usize] = phi.clone();
            phi_hasher.input(phi.as_bytes());
        }
        let mut phi_hash = vec![0u8; phi_hasher.output_bytes()];
        phi_hasher.result(&mut phi_hash);
        h.load_field(&tag.t.name.as_bytes());
        h.load_field(&tag.t.u.as_bytes());
        h.load_field(&phi_hash);
        let attest = self.sign_data_with_sha256(&h.c_hash())?;

        result.t.name = tag.t.name;
        result.t.u = tag.t.u;
        result.attest = hex::encode(attest);
        result.phi_hash = hex::encode(phi_hash);

        Ok(result)
    }

    pub fn proof_gen(&self, file_path: String, q_slice: Vec<QElement>, t: Tag) -> Result<Proof, PDPError> {
        let n = num_bigint::BigUint::from_bytes_be(&self.pkey.n().to_bytes_be());
        let mut mu = 0.to_biguint().unwrap();
        let mut sigma = 1.to_biguint().unwrap();
        let block_num = t.t.phi.len();
        //open a file
        let mut f =
            fs::File::open(file_path).map_err(|e| PDPError { error_code: FailCode::ParameterError(e.to_string()) })?;

        let file_size = match f.seek(SeekFrom::End(0)) {
            Ok(s) => s,
            Err(e) => return Err(PDPError { error_code: FailCode::ParameterError(e.to_string()) }),
        };

        let each_size = file_size / block_num as u64;
        for q in q_slice {
            let _ = f.seek(SeekFrom::Start(each_size * q.i));
            let mut data = vec![0u8; each_size as usize];
            let _ = f.read(&mut data);

            //µ =Σ νi*mi ∈ Zp (i ∈ [1, n])
            let vi = num_bigint::BigUint::from_bytes_be(&q.v);
            let mi = num_bigint::BigUint::from_bytes_be(&data);
            mu = vi.clone() * mi + mu;

            //σ =∏ σi^vi ∈ G (i ∈ [1, n])
            let mut sigma_i = num_bigint::BigUint::from_str(&t.t.phi[q.i as usize])
                .map_err(|e| PDPError { error_code: FailCode::InternalError(e.to_string()) })?;
            sigma_i = sigma_i.modpow(&vi, &n);
            sigma = sigma * sigma_i
        }
        sigma = sigma.mod_floor(&n);

        Ok(Proof { mu: mu.to_string(), sigma: sigma.to_string() })
    }

    pub fn aggr_proof_gen(&self, q_slice: Vec<QElement>, tags: Vec<Tag>) -> Result<String, PDPError> {
        let n = num_bigint::BigUint::from_bytes_be(&self.pkey.n().to_bytes_be());
        let mut sigma = 1.to_biguint().unwrap();

        for tag in &tags {
            for q in &q_slice {
                let vi = num_bigint::BigUint::from_bytes_be(&q.v);

                //σ =∏ σi^vi ∈ G (i ∈ [1, n])
                let mut sigma_i = num_bigint::BigUint::from_str(&tag.t.phi[q.i as usize])
                    .map_err(|e| PDPError { error_code: FailCode::InternalError(e.to_string()) })?;

                sigma_i = sigma_i.modpow(&vi, &n);
                sigma = sigma * sigma_i
            }
            sigma = sigma.mod_floor(&n);
        }
        Ok(sigma.to_string())
    }

    pub fn aggr_append_proof(&self, mut aggr_sigma: String, mut sub_sigma: String) -> Result<String, PDPError> {
        let n = num_bigint::BigUint::from_bytes_be(&self.pkey.n().to_bytes_be());
        if aggr_sigma == "".to_string() {
            aggr_sigma = "1".to_string()
        }
        let mut sigma = num_bigint::BigUint::from_str(&aggr_sigma)
            .map_err(|e| PDPError { error_code: FailCode::InternalError(e.to_string()) })?;

        let mut sub_sigma = num_bigint::BigUint::from_str(&aggr_sigma)
            .map_err(|e| PDPError { error_code: FailCode::InternalError(e.to_string()) })?;
        sigma = sigma * sub_sigma;
        sigma = sigma.mod_floor(&n);

        Ok(sigma.to_string())
    }

    pub fn verify(
        &self,
        u: String,
        name: String,
        q_slice: Vec<QElement>,
        sigma: String,
        mu: String,
        _thread_num: usize,
    ) -> Result<bool, PDPError> {
        let n = num_bigint::BigUint::from_bytes_be(&self.pkey.n().to_bytes_be());
        let e = num_bigint::BigUint::from_bytes_be(&self.pkey.e().to_bytes_be());
        let mut multiply = 1.to_biguint().unwrap();
        let mut w = W::new();
        w.name = name;

        for q in q_slice {
            w.i = q.i;
            let w_hash = num_bigint::BigUint::from_bytes_be(&w.hash());
            let pow = w_hash.modpow(&num_bigint::BigUint::from_bytes_be(&q.v), &n);
            multiply *= pow;
        }
        let u = num_bigint::BigUint::from_str(&u)
            .map_err(|e| PDPError { error_code: FailCode::ParameterError(e.to_string()) })?;
        let mu = num_bigint::BigUint::from_str(&mu)
            .map_err(|e| PDPError { error_code: FailCode::ParameterError(e.to_string()) })?;
        let sigma = num_bigint::BigUint::from_str(&sigma)
            .map_err(|e| PDPError { error_code: FailCode::ParameterError(e.to_string()) })?;
        let u_pow_mu = u.modpow(&mu, &n);

        multiply *= u_pow_mu;
        multiply = multiply.mod_floor(&n);

        let sigma = sigma.modpow(&e, &n);
        Ok(sigma.cmp(&multiply).is_eq())
    }

    pub fn batch_verify(
        &self,
        us: Vec<String>,
        names: Vec<String>,
        q_slice: Vec<QElement>,
        sigma: String,
        mus: Vec<String>,
        pool: ThreadPool,
    ) -> Result<bool, PDPError> {
        let n = num_bigint::BigUint::from_bytes_be(&self.pkey.n().to_bytes_be());
        let e = num_bigint::BigUint::from_bytes_be(&self.pkey.e().to_bytes_be());

        let mut multiply = 1.to_biguint().unwrap();
        let mut index = 0;
        // Multithreading
        let (hash_pow_tx, hash_pow_rx) = channel();
        for name in &names {
            let mut w = W::new();
            w.name = name.clone();
            let tx = hash_pow_tx.clone();
            let q_slice = q_slice.clone();
            let n = n.clone();
            let u = us[index].clone();
            let mu = mus[index].clone();
            pool.execute(move || {
                let mut mul = 1.to_biguint().unwrap();
                for q in &q_slice {
                    w.i = q.i;
                    let w_hash = num_bigint::BigUint::from_bytes_be(&w.hash());
                    let pow = w_hash.modpow(&num_bigint::BigUint::from_bytes_be(&q.v), &n);
                    mul *= pow;
                }
                let u = num_bigint::BigUint::from_str(&u)
                    .map_err(|e| PDPError { error_code: FailCode::ParameterError(e.to_string()) })
                    .unwrap();
                let mu = num_bigint::BigUint::from_str(&mu)
                    .map_err(|e| PDPError { error_code: FailCode::ParameterError(e.to_string()) })
                    .unwrap();
                let u_pow_mu = u.modpow(&mu, &n);
                mul *= u_pow_mu;
                mul = mul.mod_floor(&n);
                tx.send(mul).unwrap();
            });
            index += 1;
        }

        let iter = hash_pow_rx.iter().take(names.len());
        for mul in iter {
            multiply *= mul;
            multiply = multiply.mod_floor(&n);
        }
        //// Multithreading
        let mut sigma = num_bigint::BigUint::from_str(&sigma).unwrap();
        sigma = sigma.modpow(&e, &n);
        Ok(sigma.cmp(&multiply).is_eq())
    }
}

fn generate_sigma(ssk: RsaPrivateKey, data: Vec<u8>, name: String, i: u64, u_bigint: Arc<BigUint>) -> String {
    let d = num_bigint::BigUint::from_bytes_be(&ssk.d().to_bytes_be());
    let n = num_bigint::BigUint::from_bytes_be(&ssk.n().to_bytes_be());

    let mut w_i = W::new();
    w_i.i = i;
    w_i.name = name;

    //H(Wi)
    let w_i_hash = w_i.hash();
    let data_bigint = num_bigint::BigUint::from_bytes_be(&data);
    let w_i_hash_bigint = num_bigint::BigUint::from_bytes_be(&w_i_hash);

    //(H(Wi) · u^mi )^d
    let umi = u_bigint.modpow(&data_bigint, &n);

    let mut summary = w_i_hash_bigint * umi;
    summary = summary.mod_floor(&n);
    let productory = summary.modpow(&d, &n);

    productory.to_string()
}

fn redo_generate_sigma(ssk: RsaPrivateKey, phi: String) -> Result<String, PDPError> {
    let d = num_bigint::BigUint::from_bytes_be(&ssk.d().to_bytes_be());
    let n = num_bigint::BigUint::from_bytes_be(&ssk.n().to_bytes_be());
    let e = num_bigint::BigUint::from_bytes_be(&ssk.e().to_bytes_be());

    let phi = num_bigint::BigUint::from_str(&phi).map_err(|_e| PDPError {
        error_code: FailCode::ParameterError("There is something wrong with the format of phi!".to_string()),
    })?;
    let summary = phi.modpow(&e, &n);

    Ok(summary.modpow(&d, &n).to_string())
}

pub fn gen_chall(n: u64) -> Vec<QElement> {
    //select 4.6% block to challenge
    let mut rng = rand::thread_rng();
    let per: f64 = 4.6 / (100 as f64);
    let mut l = ((n as f64) * per) as u64;
    if l == 0 {
        l = 1;
    }
    let mut challenge = vec![QElement::new(); l as usize];
    let mut unique_set: HashSet<u64> = HashSet::new();
    while unique_set.len() < l as usize {
        let index = rng.gen_range(0..n);
        unique_set.insert(index);
    }
    let mut index = 0;
    for block in unique_set.iter() {
        let mut q = QElement::new();
        q.i = *block;
        loop {
            let mut q_bytes = vec![0u8; 20];
            rng.fill_bytes(&mut q_bytes);
            let v = num_bigint::BigUint::from_bytes_be(&q_bytes);

            if v > BigUint::from(0_u64) {
                q.v = v.to_bytes_be();
                challenge[index] = q;
                index += 1;
                break;
            }
        }
    }
    challenge
}

#[cfg(test)]
mod tests {
    use crypto::sha2::Sha256;

    use super::*;

    pub struct PdpTest {
        alg: Sha256,
    }

    impl HashSelf for PdpTest {
        fn new() -> Self {
            PdpTest { alg: Sha256::new() }
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

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }

    #[test]
    fn test_gen_keypair() {
        let _ = gen_keypair(2048);
    }

    #[test]
    fn test_sig_gen() {
        let keys = gen_keypair(2048);
        let thread_num = 8_usize;
        let file_path = "./test.txt".to_string();
        let n_blocks = 2_u64;
        let name = "TestFile".to_string();
        let custom_data = "hello_test".to_string();
        let mut h = PdpTest::new();
        h.load_field(custom_data.as_bytes());
        let pool = ThreadPool::new(thread_num);
        let result = match keys.sig_gen_with_path(&file_path, n_blocks, &name, h, pool) {
            Ok(r) => r,
            Err(e) => {
                panic!("{:?}", e)
            },
        };
        dbg!(result.t);
        dbg!(result.attest);
        dbg!(result.phi_hash);
    }

    #[test]
    fn test_gen_chall() {
        let n = 100;
        gen_chall(n);
    }

    #[test]
    fn test_gen_proof() {
        //1st:gen key
        let keys = gen_keypair(2048);

        //2nd:get file tag
        let thread_num = 8_usize;
        let file_path = "./test.txt".to_string();
        let n_blocks = 2_u64;
        let name = "TestFile".to_string();
        let custom_data = "hello_test".to_string();
        let mut h = PdpTest::new();
        h.load_field(custom_data.as_bytes());
        let pool = ThreadPool::new(thread_num);
        let result = match keys.sig_gen_with_path(&file_path, n_blocks, &name, h, pool) {
            Ok(r) => r,
            Err(e) => {
                panic!("{:?}", e)
            },
        };

        //3rd:get challenge set
        let q_slice = gen_chall(n_blocks);

        //4th:compute proof
        let proof = match keys.proof_gen(file_path, q_slice, result) {
            Ok(proof) => proof,
            Err(e) => {
                panic!("{:?}", e)
            },
        };

        dbg!(proof.mu);
        dbg!(proof.sigma);
    }

    #[test]
    fn test_verify() {
        //1st:gen key
        println!("start run gen_keypair");
        let keys = gen_keypair(2048);

        //2nd:get file tag
        println!("start run sig_gen_with_path");
        let thread_num = 8_usize;
        let file_path = "./test1.txt".to_string();
        let n_blocks = 2_u64;
        let name = "TestFile1".to_string();
        let custom_data = "hello_test".to_string();
        let mut h = PdpTest::new();
        h.load_field(custom_data.as_bytes());
        let pool = ThreadPool::new(thread_num);
        let result = match keys.sig_gen_with_path(&file_path, n_blocks, &name, h, pool) {
            Ok(r) => r,
            Err(e) => {
                panic!("{:?}", e)
            },
        };

        //3rd:get challenge set
        println!("start run gen_chall");
        let q_slice = gen_chall(n_blocks);

        println!("start run proof_gen");
        //4th:compute proof
        let proof = match keys.proof_gen(file_path, q_slice.clone(), result.clone()) {
            Ok(proof) => proof,
            Err(e) => {
                panic!("{:?}", e)
            },
        };

        //5th:verify proof
        println!("start run verify");
        let result = match keys.verify(result.t.u, result.t.name, q_slice, proof.sigma, proof.mu, thread_num) {
            Ok(r) => r,
            Err(e) => {
                panic!("{:?}", e)
            },
        };
        assert_eq!(result, true);
    }

    #[test]
    fn test_batch_verify() {
        println!("start run gen_keypair");
        let keys = gen_keypair(2048);

        //test.txt tag
        println!("start run sig_gen_with_path for test.txt");
        let thread_num = 8_usize;
        let file_path = "./test.txt".to_string();
        let n_blocks = 2_u64;
        let name = "TestFile".to_string();
        let custom_data = "hello_test".to_string();
        let mut h = PdpTest::new();
        h.load_field(custom_data.as_bytes());
        let pool = ThreadPool::new(thread_num);
        let tag = match keys.sig_gen_with_path(&file_path, n_blocks, &name, h, pool) {
            Ok(r) => r,
            Err(e) => {
                panic!("{:?}", e)
            },
        };

        //test1.txt tag
        println!("start run sig_gen_with_path for test1.txt");
        let thread_num = 8_usize;
        let file_path1 = "./test1.txt".to_string();
        let n_blocks = 2_u64;
        let name1 = "TestFile1".to_string();
        let custom_data = "hello_test".to_string();
        let mut h1 = PdpTest::new();
        h1.load_field(custom_data.as_bytes());
        let pool = ThreadPool::new(thread_num);
        let tag1 = match keys.sig_gen_with_path(&file_path1, n_blocks, &name1, h1, pool.clone()) {
            Ok(r) => r,
            Err(e) => {
                panic!("{:?}", e)
            },
        };

        println!("start run gen_chall");
        let q_slice = gen_chall(n_blocks);

        //test.txt proof
        println!("start generate test.txt proof");
        let proof0 = match keys.proof_gen(file_path.clone(), q_slice.clone(), tag.clone()) {
            Ok(proof) => proof,
            Err(e) => {
                panic!("{:?}", e)
            },
        };

        //test.txt verify
        println!("start verify test.txt proof");
        let ok0 = match keys.verify(
            tag.t.u.clone(),
            tag.t.name.clone(),
            q_slice.clone(),
            proof0.sigma.clone(),
            proof0.mu.clone(),
            thread_num,
        ) {
            Ok(r) => r,
            Err(e) => {
                panic!("{:?}", e)
            },
        };

        //test1.txt proof
        println!("start generate test1.txt proof");
        let proof1 = match keys.proof_gen(file_path1.clone(), q_slice.clone(), tag1.clone()) {
            Ok(proof) => proof,
            Err(e) => {
                panic!("{:?}", e)
            },
        };

        //test1.txt verify
        println!("start verify test1.txt proof");
        let ok1 = match keys.verify(
            tag1.t.u.clone(),
            tag1.t.name.clone(),
            q_slice.clone(),
            proof1.sigma.clone(),
            proof1.mu.clone(),
            thread_num,
        ) {
            Ok(r) => r,
            Err(e) => {
                panic!("{:?}", e)
            },
        };

        assert_eq!((ok0, ok1), (true, true));

        //generate aggregate proof
        println!("start generate aggregate proof");
        let tags = vec![tag.clone(), tag1.clone()];
        let sigma = match keys.aggr_proof_gen(q_slice.clone(), tags.clone()) {
            Ok(proof) => proof,
            Err(e) => {
                panic!("{:?}", e)
            },
        };

        println!("start verify aggregate proof");
        let us = vec![tag.t.u, tag1.t.u];
        let names = vec![tag.t.name, tag1.t.name];
        let mus = vec![proof0.mu, proof1.mu];
        let r = match keys.batch_verify(us, names, q_slice, sigma, mus, pool) {
            Ok(r) => r,
            Err(e) => {
                panic!("{:?}", e)
            },
        };
        assert_eq!(r, true);
    }

    #[test]
    fn test_read_file() {
        let mut f1 = fs::File::open("./test.txt").unwrap();
        let mut f2 = fs::File::open("test1.txt").unwrap();

        let mut data1 = vec![0u8; f1.metadata().unwrap().len() as usize];
        let mut data2 = vec![0u8; f2.metadata().unwrap().len() as usize];

        let _ = f1.read(&mut data1);
        let _ = f2.read(&mut data2);

        println!("data1 {:?}", data1);
        println!("data2 {:?}", data2);
    }
}
