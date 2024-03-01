#![warn(unused_imports)]
#![warn(unused_extern_crates)]

#[macro_use]
extern crate log;
extern crate cestory_pal as pal;
extern crate runtime as chain;

use glob::PatternError;
use rand::*;
use serde::{
    de::{self, DeserializeOwned, SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::light_validation::LightValidation;
use anyhow::{anyhow, Context as _, Result};
use ces_crypto::{
    aead,
    ecdh::EcdhKey,
    sr25519::{Persistence as Persist, Sr25519SecretKey, KDF, SEED_BYTES},
};
use ces_mq::{BindTopic, MessageDispatcher, MessageSendQueue};
use ces_serde_more as more;
use ces_types::{AttestationProvider, HandoverChallenge};
use cestory_api::{
    crpc::{ceseal_api_server::CesealApiServer, GetEndpointResponse, InitRuntimeResponse},
    ecall_args::InitArgs,
    storage_sync::{StorageSynchronizer, Synchronizer},
};
use parity_scale_codec::{Decode, Encode};
use ring::rand::SecureRandom;
use scale_info::TypeInfo;
use sp_core::{crypto::Pair, sr25519, H256};
use std::{
    fs::File,
    io::{ErrorKind, Write},
    marker::PhantomData,
    path::{Path, PathBuf},
    str,
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};
use thiserror::Error;
use tokio::sync::oneshot;
use types::Error;

pub use ceseal_service::RpcService;
pub use chain::BlockNumber;
pub use storage::ChainStorage;
pub use types::{BlockDispatchContext, CesealProperties};
pub type CesealLightValidation = LightValidation<chain::Runtime>;

mod ceseal_service;
mod cryptography;
pub mod expert;
mod light_validation;
pub mod podr2;
pub mod pois;
mod pubkeys;
mod secret_channel;
mod storage;
mod system;
mod types;

pub use podr2::verify_signature;

// TODO: Completely remove the reference to cess-node-runtime. Instead we can create a minimal
// runtime definition locally.
type RuntimeHasher = <chain::Runtime as frame_system::Config>::Hashing;

#[derive(Serialize, Deserialize, Clone, ::scale_info::TypeInfo)]
struct RuntimeState {
    #[codec(skip)]
    send_mq: MessageSendQueue,

    #[serde(skip)]
    #[codec(skip)]
    recv_mq: MessageDispatcher,

    // chain storage synchonizing
    #[cfg_attr(not(test), codec(skip))]
    storage_synchronizer: Synchronizer<LightValidation<chain::Runtime>>,

    // TODO: use a better serialization approach
    #[codec(skip)]
    chain_storage: Arc<ChainStorage>,

    #[serde(with = "more::scale_bytes")]
    genesis_block_hash: H256,
}

impl RuntimeState {
    fn purge_mq(&mut self) {
        self.send_mq.purge(|sender| self.chain_storage.mq_sequence(sender))
    }
}

const RUNTIME_SEALED_DATA_FILE: &str = "runtime-data.seal";
const CHECKPOINT_FILE: &str = "checkpoint.seal";
const CHECKPOINT_VERSION: u32 = 2;

fn checkpoint_filename_for(block_number: chain::BlockNumber, basedir: &str) -> String {
    format!("{basedir}/{CHECKPOINT_FILE}-{block_number:0>9}")
}

fn checkpoint_info_filename_for(filename: &str) -> String {
    format!("{filename}.info.json")
}

fn checkpoint_filename_pattern(basedir: &str) -> String {
    format!("{basedir}/{CHECKPOINT_FILE}-*")
}

fn glob_checkpoint_files(basedir: &str) -> Result<impl Iterator<Item = PathBuf>, PatternError> {
    let pattern = checkpoint_filename_pattern(basedir);
    Ok(glob::glob(&pattern)?.filter_map(|path| path.ok()))
}

fn glob_checkpoint_files_sorted(basedir: &str) -> Result<Vec<(chain::BlockNumber, PathBuf)>, PatternError> {
    fn parse_block(filename: &Path) -> Option<chain::BlockNumber> {
        let filename = filename.to_str()?;
        let block_number = filename.rsplit('-').next()?.parse().ok()?;
        Some(block_number)
    }
    let mut files = Vec::new();

    for filename in glob_checkpoint_files(basedir)? {
        if let Some(block_number) = parse_block(&filename) {
            files.push((block_number, filename));
        }
    }
    files.sort_by_key(|(block_number, _)| std::cmp::Reverse(*block_number));
    Ok(files)
}

fn maybe_remove_checkpoints(basedir: &str) {
    match glob_checkpoint_files(basedir) {
        Err(err) => error!("Error globbing checkpoints: {:?}", err),
        Ok(iter) =>
            for filename in iter {
                info!("Removing {}", filename.display());
                if let Err(e) = std::fs::remove_file(&filename) {
                    error!("Failed to remove {}: {}", filename.display(), e);
                }
            },
    }
}

fn remove_outdated_checkpoints(basedir: &str, max_kept: u32, current_block: chain::BlockNumber) -> Result<()> {
    let mut kept = 0_u32;
    for (block, filename) in glob_checkpoint_files_sorted(basedir)? {
        if block > current_block {
            continue
        }
        kept += 1;
        if kept > max_kept {
            match remove_checkpoint(&filename) {
                Err(e) => error!("Failed to remove checkpoint {}: {e}", filename.display()),
                Ok(_) => {
                    info!("Removed {}", filename.display());
                },
            }
        }
    }
    Ok(())
}

fn remove_checkpoint(filename: &Path) -> Result<()> {
    std::fs::remove_file(filename).context("Failed to remove checkpoint file")?;
    let info_filename = checkpoint_info_filename_for(&filename.display().to_string());
    std::fs::remove_file(info_filename).context("Failed to remove checkpoint info file")?;
    Ok(())
}

#[derive(Encode, Decode, Clone, Debug)]
struct PersistentRuntimeData {
    genesis_block_hash: H256,
    sk: Sr25519SecretKey,
    trusted_sk: bool,
    dev_mode: bool,
}

impl PersistentRuntimeData {
    pub fn decode_keys(&self) -> (sr25519::Pair, EcdhKey) {
        // load identity
        let identity_sk = sr25519::Pair::restore_from_secret_key(&self.sk);
        info!("Identity pubkey: {:?}", hex::encode(identity_sk.public()));

        // derive ecdh key
        let ecdh_key = identity_sk.derive_ecdh_key().expect("Unable to derive ecdh key");
        info!("ECDH pubkey: {:?}", hex::encode(ecdh_key.public()));
        (identity_sk, ecdh_key)
    }
}

#[derive(Encode, Decode, Clone, Debug)]
enum RuntimeDataSeal {
    V1(PersistentRuntimeData),
}

#[derive(Serialize, Deserialize, TypeInfo)]
#[serde(bound(deserialize = "Platform: Deserialize<'de>"))]
pub struct Ceseal<Platform> {
    platform: Platform,
    #[serde(skip)]
    #[codec(skip)]
    pub args: Arc<InitArgs>,
    dev_mode: bool,
    attestation_provider: Option<AttestationProvider>,
    machine_id: Vec<u8>,
    runtime_info: Option<InitRuntimeResponse>,
    runtime_state: Option<RuntimeState>,
    endpoint: Option<String>,
    #[serde(skip)]
    #[codec(skip)]
    signed_endpoint: Option<GetEndpointResponse>,
    // The deserialzation of system requires the mq, which inside the runtime_state, to be ready.
    #[serde(skip)]
    system: Option<system::System<Platform>>,

    // tmp key for WorkerKey handover encryption
    #[codec(skip)]
    #[serde(skip)]
    pub(crate) handover_ecdh_key: Option<EcdhKey>,

    #[codec(skip)]
    #[serde(skip)]
    handover_last_challenge: Option<HandoverChallenge<chain::BlockNumber>>,

    #[codec(skip)]
    #[serde(skip)]
    #[serde(default = "Instant::now")]
    last_checkpoint: Instant,

    #[codec(skip)]
    #[serde(skip)]
    can_load_chain_state: bool,

    #[codec(skip)]
    #[serde(skip)]
    trusted_sk: bool,

    #[codec(skip)]
    #[serde(skip)]
    pub(crate) rcu_dispatching: bool,

    #[codec(skip)]
    #[serde(skip)]
    #[serde(default = "Instant::now")]
    started_at: Instant,

    #[codec(skip)]
    #[serde(skip)]
    pub(crate) keyfairy_ready_sender: Option<types::KeyfairyReadySender>,
}

#[test]
fn show_type_changes_that_affect_the_checkpoint() {
    fn travel_types<T: TypeInfo>() -> String {
        use scale_info::{IntoPortable, PortableRegistry};
        let mut registry = Default::default();
        let _ = T::type_info().into_portable(&mut registry);
        serde_json::to_string_pretty(&PortableRegistry::from(registry).types).unwrap()
    }
    insta::assert_display_snapshot!(travel_types::<Ceseal<()>>());
}

impl<Platform: pal::Platform> Ceseal<Platform> {
    pub fn new(platform: Platform) -> Self {
        let machine_id = platform.machine_id();
        Ceseal {
            platform,
            args: Default::default(),
            dev_mode: false,
            attestation_provider: None,
            machine_id,
            runtime_info: None,
            runtime_state: None,
            system: None,
            endpoint: None,
            signed_endpoint: None,
            handover_ecdh_key: None,
            handover_last_challenge: None,
            last_checkpoint: Instant::now(),
            can_load_chain_state: false,
            trusted_sk: false,
            rcu_dispatching: false,
            started_at: Instant::now(),
            keyfairy_ready_sender: None,
        }
    }

    pub fn init(&mut self, args: InitArgs) -> types::KeyfairyReadyReceiver {
        self.can_load_chain_state = !system::gk_master_key_exists(&args.sealing_path);
        self.args = Arc::new(args);

        let (tx, rx) = oneshot::channel();
        self.keyfairy_ready_sender = if let Some(ref mut system) = self.system {
            if system.keyfairy.is_some() {
                system.keyfairy_ready_sender = Some(tx);
                None
            } else {
                Some(tx)
            }
        } else {
            Some(tx)
        };
        rx
    }

    pub fn set_args(&mut self, args: InitArgs) {
        self.args = Arc::new(args);
        if let Some(system) = &mut self.system {
            system.sealing_path = self.args.sealing_path.clone();
            system.storage_path = self.args.storage_path.clone();
            system.args = self.args.clone();
        }
    }

    pub fn start_at(&self) -> &Instant {
        &self.started_at
    }

    pub fn get_pdp_key(&self) -> Result<ces_pdp::Keys, types::Error> {
        if let Some(ref system) = self.system {
            if let Some(ref keyfariy) = system.keyfairy {
                Ok(ces_pdp::gen_keypair_from_private_key(keyfariy.rsa_private_key().clone()))
            } else {
                return Err(types::Error::KeyfairyNotReady)
            }
        } else {
            return Err(types::Error::SystemNotReady)
        }
    }

    fn init_runtime_data(
        &self,
        genesis_block_hash: H256,
        predefined_identity_key: Option<sr25519::Pair>,
    ) -> Result<PersistentRuntimeData> {
        let data = if let Some(identity_sk) = predefined_identity_key {
            self.save_runtime_data(genesis_block_hash, identity_sk, false, true)?
        } else {
            match Self::load_runtime_data(&self.platform, &self.args.sealing_path) {
                Ok(data) => data,
                Err(Error::PersistentRuntimeNotFound) => {
                    warn!("Persistent data not found.");
                    let identity_sk = new_sr25519_key();
                    self.save_runtime_data(genesis_block_hash, identity_sk, true, false)?
                },
                Err(err) => return Err(anyhow!("Failed to load persistent data: {}", err)),
            }
        };

        // check genesis block hash
        if genesis_block_hash != data.genesis_block_hash {
            anyhow::bail!("Genesis block hash mismatches with saved keys, expected {}", data.genesis_block_hash);
        }
        info!("Init done.");
        Ok(data)
    }

    fn save_runtime_data(
        &self,
        genesis_block_hash: H256,
        sr25519_sk: sr25519::Pair,
        trusted_sk: bool,
        dev_mode: bool,
    ) -> Result<PersistentRuntimeData> {
        // Put in PresistentRuntimeData
        let sk = sr25519_sk.dump_secret_key();

        let data = PersistentRuntimeData { genesis_block_hash, sk, trusted_sk, dev_mode };
        {
            let data = RuntimeDataSeal::V1(data.clone());
            let encoded_vec = data.encode();
            info!("Length of encoded slice: {}", encoded_vec.len());
            let filepath = PathBuf::from(&self.args.sealing_path).join(RUNTIME_SEALED_DATA_FILE);
            self.platform
                .seal_data(filepath, &encoded_vec)
                .map_err(Into::into)
                .context("Failed to seal runtime data")?;
            info!("Persistent Runtime Data V2 saved");
        }
        Ok(data)
    }

    /// Loads the persistent runtime data from the sealing path
    fn persistent_runtime_data(&self) -> Result<PersistentRuntimeData, Error> {
        Self::load_runtime_data(&self.platform, &self.args.sealing_path)
    }

    fn load_runtime_data(platform: &Platform, sealing_path: &str) -> Result<PersistentRuntimeData, Error> {
        let filepath = PathBuf::from(sealing_path).join(RUNTIME_SEALED_DATA_FILE);
        let data = platform
            .unseal_data(filepath)
            .map_err(Into::into)?
            .ok_or(Error::PersistentRuntimeNotFound)?;
        let data: RuntimeDataSeal = Decode::decode(&mut &data[..]).map_err(Error::DecodeError)?;
        match data {
            RuntimeDataSeal::V1(data) => Ok(data),
        }
    }
}

impl<P: pal::Platform> Ceseal<P> {
    // Restored from checkpoint
    pub fn on_restored(&mut self) -> Result<()> {
        self.check_requirements();
        self.update_runtime_info(|_| {});
        self.trusted_sk = Self::load_runtime_data(&self.platform, &self.args.sealing_path)?.trusted_sk;
        if let Some(system) = &mut self.system {
            system.on_restored(self.args.safe_mode_level)?;
        }
        if self.args.safe_mode_level >= 2 {
            if let Some(state) = &mut self.runtime_state {
                info!("Clearing the storage data to save memory");
                let Some(chain_storage) = Arc::get_mut(&mut state.chain_storage) else {
                    return Err(anyhow!(types::Error::MultiRefToChainStorage.to_string()))
                };
                chain_storage.inner_mut().load_proof(vec![])
            }
        }
        Ok(())
    }

    fn check_requirements(&self) {
        let ver = P::app_version();
        let chain_storage = &self.runtime_state.as_ref().expect("BUG: no runtime state").chain_storage;
        let min_version = chain_storage.minimum_ceseal_version();

        let measurement = self.platform.measurement().unwrap_or_else(|| vec![0; 32]);
        let in_whitelist = chain_storage.is_ceseal_bin_in_whitelist(&measurement);

        if (ver.major, ver.minor, ver.patch) < min_version && !in_whitelist {
            error!("This ceseal is outdated. Please update to the latest version.");
            std::process::abort();
        }
    }

    fn update_runtime_info(&mut self, f: impl FnOnce(&mut ces_types::WorkerRegistrationInfo<chain::AccountId>)) {
        let Some(cached_resp) = self.runtime_info.as_mut() else { return };
        let mut runtime_info = cached_resp.decode_runtime_info().expect("BUG: Decode runtime_info failed");
        runtime_info.version = Self::compat_app_version();
        f(&mut runtime_info);
        cached_resp.encoded_runtime_info = runtime_info.encode();
        cached_resp.attestation = None;
    }

    fn compat_app_version() -> u32 {
        let version = P::app_version();
        (version.major << 16) + (version.minor << 8) + version.patch
    }
}

impl<Platform: pal::Platform + Serialize + DeserializeOwned> Ceseal<Platform> {
    pub fn take_checkpoint(&mut self) -> anyhow::Result<chain::BlockNumber> {
        if self.args.safe_mode_level > 0 {
            anyhow::bail!("Checkpoint is disabled in safe mode");
        }
        let (current_block, _) = self.current_block()?;
        let key = self
            .system
            .as_ref()
            .context("Take checkpoint failed, runtime is not ready")?
            .identity_key
            .dump_secret_key();
        let checkpoint_file = checkpoint_filename_for(current_block, &self.args.storage_path);
        info!("Taking checkpoint to {checkpoint_file}...");
        self.save_checkpoint_info(&checkpoint_file)?;
        let file = File::create(&checkpoint_file).context("Failed to create checkpoint file")?;
        self.take_checkpoint_to_writer(&key, file)
            .context("Take checkpoint to writer failed")?;
        info!("Checkpoint saved to {checkpoint_file}");
        self.last_checkpoint = Instant::now();
        remove_outdated_checkpoints(&self.args.storage_path, self.args.max_checkpoint_files, current_block)?;
        Ok(current_block)
    }

    pub fn save_checkpoint_info(&self, filename: &str) -> anyhow::Result<()> {
        let info = self.get_info();
        let content = serde_json::to_string_pretty(&info).context("Failed to serialize checkpoint info")?;
        let info_filename = checkpoint_info_filename_for(filename);
        let mut file = File::create(info_filename).context("Failed to create checkpoint info file")?;
        file.write_all(content.as_bytes())
            .context("Failed to write checkpoint info file")?;
        Ok(())
    }

    pub fn take_checkpoint_to_writer<W: std::io::Write>(&mut self, key: &[u8], writer: W) -> anyhow::Result<()> {
        let key128 = derive_key_for_checkpoint(key);
        let nonce = rand::thread_rng().gen();
        let mut enc_writer = aead::stream::new_aes128gcm_writer(key128, nonce, writer);
        serialize_ceseal_to_writer(self, &mut enc_writer).context("Failed to write checkpoint")?;
        enc_writer.flush().context("Failed to flush encrypted writer")?;
        Ok(())
    }

    pub fn restore_from_checkpoint(platform: &Platform, args: &InitArgs) -> anyhow::Result<Option<Self>> {
        let runtime_data = match Self::load_runtime_data(platform, &args.sealing_path) {
            Err(Error::PersistentRuntimeNotFound) => return Ok(None),
            other => other.context("Failed to load persistent data")?,
        };
        let files = glob_checkpoint_files_sorted(&args.storage_path).context("Glob checkpoint files failed")?;
        if files.is_empty() {
            return Ok(None)
        }
        let (_block, ckpt_filename) = &files[0];

        let file = match File::open(ckpt_filename) {
            Ok(file) => file,
            Err(err) if matches!(err.kind(), ErrorKind::NotFound) => {
                // This should never happen unless it was removed just after the glob.
                anyhow::bail!("Checkpoint file {ckpt_filename:?} is not found");
            },
            Err(err) => {
                error!("Failed to open checkpoint file {ckpt_filename:?}: {err:?}",);
                if args.remove_corrupted_checkpoint {
                    error!("Removing {ckpt_filename:?}");
                    std::fs::remove_file(ckpt_filename).context("Failed to remove corrupted checkpoint file")?;
                }
                anyhow::bail!("Failed to open checkpoint file {ckpt_filename:?}: {err:?}");
            },
        };

        info!("Loading checkpoint from file {ckpt_filename:?}");
        match Self::restore_from_checkpoint_reader(&runtime_data.sk, file, args) {
            Ok(state) => {
                info!("Succeeded to load checkpoint file {ckpt_filename:?}");
                Ok(Some(state))
            },
            Err(_err /* Don't leak it into the log */) => {
                error!("Failed to load checkpoint file {ckpt_filename:?}");
                if args.remove_corrupted_checkpoint {
                    error!("Removing {:?}", ckpt_filename);
                    std::fs::remove_file(ckpt_filename).context("Failed to remove corrupted checkpoint file")?;
                }
                anyhow::bail!("Failed to load checkpoint file {ckpt_filename:?}");
            },
        }
    }

    pub fn restore_from_checkpoint_reader<R: std::io::Read>(
        key: &[u8],
        reader: R,
        args: &InitArgs,
    ) -> anyhow::Result<Self> {
        let key128 = derive_key_for_checkpoint(key);
        let dec_reader = aead::stream::new_aes128gcm_reader(key128, reader);

        let mut factory =
            deserialize_ceseal_from_reader(dec_reader, args.safe_mode_level).context("Failed to deserialize Ceseal")?;
        factory.set_args(args.clone());
        factory.on_restored().context("Failed to restore Ceseal")?;
        Ok(factory)
    }
}

impl<Platform: Serialize + DeserializeOwned> Ceseal<Platform> {
    fn dump_state<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_seq(None)?;
        state.serialize_element(&CHECKPOINT_VERSION)?;
        state.serialize_element(&self)?;
        state.serialize_element(&self.system)?;
        state.end()
    }

    fn load_state<'de, D: Deserializer<'de>>(deserializer: D, safe_mode_level: u8) -> Result<Self, D::Error> {
        struct CesealVisitor<Platform> {
            safe_mode_level: u8,
            _marker: PhantomData<Platform>,
        }

        impl<'de, Platform: Serialize + DeserializeOwned> Visitor<'de> for CesealVisitor<Platform> {
            type Value = Ceseal<Platform>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("Ceseal")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let version: u32 = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::custom("Checkpoint version missing"))?;
                if version > CHECKPOINT_VERSION {
                    return Err(de::Error::custom(format!("Checkpoint version {version} is not supported")))
                }

                let mut factory: Self::Value =
                    seq.next_element()?.ok_or_else(|| de::Error::custom("Missing Ceseal"))?;

                if self.safe_mode_level < 2 {
                    factory.system = {
                        let runtime_state = factory
                            .runtime_state
                            .as_mut()
                            .ok_or_else(|| de::Error::custom("Missing runtime_state"))?;

                        let recv_mq = &mut runtime_state.recv_mq;
                        let send_mq = &mut runtime_state.send_mq;
                        let seq = &mut seq;
                        ces_mq::checkpoint_helper::using_dispatcher(recv_mq, move || {
                            ces_mq::checkpoint_helper::using_send_mq(send_mq, || {
                                seq.next_element()?.ok_or_else(|| de::Error::custom("Missing System"))
                            })
                        })?
                    };
                } else {
                    let _: Option<serde::de::IgnoredAny> = seq.next_element()?;
                }
                Ok(factory)
            }
        }

        deserializer.deserialize_seq(CesealVisitor { safe_mode_level, _marker: PhantomData })
    }
}

fn deserialize_ceseal_from_reader<Platform, R>(reader: R, safe_mode_level: u8) -> Result<Ceseal<Platform>>
where
    Platform: Serialize + DeserializeOwned,
    R: std::io::Read,
{
    let mut deserializer = serde_cbor::Deserializer::from_reader(reader);
    Ceseal::load_state(&mut deserializer, safe_mode_level).context("Failed to load factory")
}

fn serialize_ceseal_to_writer<Platform, R>(phatory: &Ceseal<Platform>, writer: R) -> Result<()>
where
    Platform: Serialize + DeserializeOwned,
    R: std::io::Write,
{
    let mut writer = serde_cbor::ser::IoWrite::new(writer);
    let mut serializer = serde_cbor::Serializer::new(&mut writer);
    phatory.dump_state(&mut serializer)?;
    Ok(())
}

pub(crate) fn new_sr25519_key() -> sr25519::Pair {
    let mut rng = rand::thread_rng();
    let mut seed = [0_u8; SEED_BYTES];
    rng.fill_bytes(&mut seed);
    sr25519::Pair::from_seed(&seed)
}
use crypto::digest::Digest;
use rsa::{pkcs1::EncodeRsaPublicKey, pkcs8::EncodePrivateKey};
fn new_podr2_key() -> rsa::RsaPrivateKey {
    let rsa_key = ces_pdp::gen_keypair(2048);
    rsa_key.skey
}

fn get_sr25519_from_rsa_key(skey: rsa::RsaPrivateKey) -> sr25519::Pair {
    let rsa_key_byte = skey.to_pkcs8_der().unwrap().as_bytes().to_vec();
    let mut hasher = crypto::sha2::Sha256::new();
    hasher.input(&rsa_key_byte);
    let mut seed = vec![0u8; hasher.output_bytes()];
    hasher.result(&mut seed);
    sr25519::Pair::from_seed(&seed.try_into().expect("panic:get sr25519 from rsa key fail!"))
}

// TODO: Move to cestory-api when the std ready.
fn generate_random_iv() -> aead::IV {
    let mut nonce_vec = [0u8; aead::IV_BYTES];
    let rand = ring::rand::SystemRandom::new();
    rand.fill(&mut nonce_vec).unwrap();
    nonce_vec
}

fn generate_random_info() -> [u8; 32] {
    let mut nonce_vec = [0u8; 32];
    let rand = ring::rand::SystemRandom::new();
    rand.fill(&mut nonce_vec).unwrap();
    nonce_vec
}

fn derive_key_for_checkpoint(identity_key: &[u8]) -> [u8; 16] {
    sp_core::blake2_128(&(identity_key, b"/checkpoint").encode())
}

pub fn public_data_dir(storage_path: impl AsRef<Path>) -> PathBuf {
    storage_path.as_ref().to_path_buf().join("public")
}

#[derive(Clone)]
pub struct CesealSafeBox<Platform>(Arc<Mutex<Ceseal<Platform>>>);

#[derive(Error, Debug)]
#[error("{:?}", self)]
pub enum CesealLockError {
    Poison(String),
    #[error("RCU in progress, please try the request again later")]
    Rcu,
    #[error("This RPC is disabled in safe mode")]
    SafeMode,
}

impl<Platform: pal::Platform> CesealSafeBox<Platform> {
    pub fn new(platform: Platform) -> Self {
        CesealSafeBox(Arc::new(Mutex::new(Ceseal::new(platform))))
    }

    pub fn new_with(ceseal: Ceseal<Platform>) -> Self {
        CesealSafeBox(Arc::new(Mutex::new(ceseal)))
    }

    pub fn lock(
        &self,
        allow_rcu: bool,
        allow_safemode: bool,
    ) -> Result<LogOnDrop<MutexGuard<'_, Ceseal<Platform>>>, CesealLockError> {
        trace!(target: "cestory::lock", "Locking cestory...");
        let guard = self.0.lock().map_err(|e| CesealLockError::Poison(e.to_string()))?;
        trace!(target: "cestory::lock", "Locked cestory");
        if !allow_rcu && guard.rcu_dispatching {
            return Err(CesealLockError::Rcu)
        }
        if !allow_safemode && guard.args.safe_mode_level > 0 {
            return Err(CesealLockError::SafeMode)
        }
        Ok(LogOnDrop { inner: guard, msg: "Unlocked cestory" })
    }
}

pub struct LogOnDrop<T> {
    inner: T,
    msg: &'static str,
}

impl<T> core::ops::Deref for LogOnDrop<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<T> core::ops::DerefMut for LogOnDrop<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T> Drop for LogOnDrop<T> {
    fn drop(&mut self) {
        trace!(target: "cestory::lock", "{}", self.msg);
    }
}

use std::net::SocketAddr;
use tonic::transport::Server;

pub async fn run_ceseal_server<Platform>(
    init_args: InitArgs,
    platform: Platform,
    inner_listener_addr: SocketAddr,
    public_listener_addr: SocketAddr,
) -> Result<()>
where
    Platform: pal::Platform + Serialize + DeserializeOwned,
{
    let (cestory, keyfairy_ready_by_restore) = if init_args.enable_checkpoint {
        match Ceseal::restore_from_checkpoint(&platform, &init_args) {
            Ok(Some(factory)) => {
                info!("Loaded checkpoint");
                let keyfairy_ready_by_restore = if let Some(ref system) = factory.system {
                    if system.keyfairy.is_some() {
                        info!("keyfairy is ready by restore");
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                (CesealSafeBox::new_with(factory), keyfairy_ready_by_restore)
            },
            Ok(None) => {
                info!("No checkpoint found");
                (CesealSafeBox::new(platform), false)
            },
            Err(err) => {
                anyhow::bail!("Failed to load checkpoint: {:?}", err);
            },
        }
    } else {
        info!("Checkpoint disabled.");
        (CesealSafeBox::new(platform), false)
    };

    let keyfairy_ready_rx = cestory.lock(true, true).expect("Failed to lock Ceseal").init(init_args);
    info!("Enclave init OK");

    let external_server_handler =
        tokio::spawn(run_external_server(cestory.clone(), keyfairy_ready_rx, public_listener_addr));
    let restored_keyfairy_ready_handler =
        tokio::spawn(maybe_keyfairy_ready_by_restore(cestory.clone(), keyfairy_ready_by_restore));
    let ceseal_server_handler = tokio::spawn(async move {
        let ceseal_service = RpcService::new_with(cestory);
        info!("Ceseal internal server will listening on {}", inner_listener_addr);
        Server::builder()
            .add_service(CesealApiServer::new(ceseal_service))
            .serve(inner_listener_addr)
            .await
            .expect("Ceseal server catch panic");
    });
    let result = tokio::try_join!(ceseal_server_handler, external_server_handler, restored_keyfairy_ready_handler);
    if let Err(e) = result {
        panic!("Error in run_ceseal_server() tokio::try_join!: {:?}", e)
    }

    info!("Ceseal quited");
    Ok(())
}

#[derive(Debug, Clone)]
pub struct OnChainConfigs {
    pub pois_param: (i64, i64, i64),
}

async fn run_external_server<Platform>(
    ceseal: CesealSafeBox<Platform>,
    keyfairy_ready_rx: types::KeyfairyReadyReceiver,
    public_listener_addr: SocketAddr,
) where
    Platform: pal::Platform + Serialize + DeserializeOwned,
{
    const MAX_ENCODED_MSG_SIZE: usize = 104857600; // 100MiB
    const MAX_DECODED_MSG_SIZE: usize = MAX_ENCODED_MSG_SIZE;

    let ceseal_props = keyfairy_ready_rx.await.expect("expect keyfairy ready");
    //FIXME: SHOULD BE DISABLE LOG KEY ON PRODUCTION !!!
    debug!(
        "Successfully load podr2 key public key is: {:?}",
        hex::encode(&ceseal_props.podr2_key.pkey.to_pkcs1_der().unwrap().as_bytes())
    );

    let (ceseal_expert, expert_cmd_rx) = expert::CesealExpertStub::new(ceseal_props.clone());
    tokio::spawn(expert::run(ceseal.clone(), expert_cmd_rx));

    let (on_chain_cfg_tx, on_chain_cfg_rx) = oneshot::channel();
    let ceseal_expert_clone = ceseal_expert.clone();
    tokio::spawn(async move {
        let pois_param = panic_if_no_pois_expender_param(ceseal_expert_clone).await;
        info!("pois expender param on chain: {pois_param:?}");
        let _ = on_chain_cfg_tx.send(OnChainConfigs { pois_param });
    });

    let (ext_srv_quit_tx, ext_srv_quit_rx) = oneshot::channel();
    let ceseal_expert_clone = ceseal_expert.clone();
    tokio::spawn(async move {
        let on_chain_cfg = on_chain_cfg_rx.await.expect("on chain config fetch failed");
        let ceseal_expert = ceseal_expert_clone;
        let pois_param = on_chain_cfg.pois_param.clone();
        let podr2_srv = podr2::new_podr2_api_server(ceseal_expert.clone())
            .max_decoding_message_size(MAX_DECODED_MSG_SIZE)
            .max_encoding_message_size(MAX_ENCODED_MSG_SIZE);
        let podr2v_srv = podr2::new_podr2_verifier_api_server(ceseal_expert.clone())
            .max_decoding_message_size(MAX_DECODED_MSG_SIZE)
            .max_encoding_message_size(MAX_ENCODED_MSG_SIZE);
        let pois_srv = pois::new_pois_certifier_api_server(pois_param.clone(), ceseal_expert.clone())
            .max_decoding_message_size(MAX_DECODED_MSG_SIZE)
            .max_encoding_message_size(MAX_ENCODED_MSG_SIZE);
        let poisv_srv = pois::new_pois_verifier_api_server(pois_param, ceseal_expert.clone())
            .max_decoding_message_size(MAX_DECODED_MSG_SIZE)
            .max_encoding_message_size(MAX_ENCODED_MSG_SIZE);
        let pubkeys = pubkeys::new_pubkeys_provider_server(ceseal_expert);

        info!(
            "keyfairy ready, external server will listening on {} run with {:?} role",
            public_listener_addr, ceseal_props.role
        );
        let mut server = Server::builder();
        let router = match ceseal_props.role {
            ces_types::WorkerRole::Full => server
                .add_service(pubkeys)
                .add_service(podr2_srv)
                .add_service(podr2v_srv)
                .add_service(pois_srv)
                .add_service(poisv_srv),
            ces_types::WorkerRole::Verifier => server.add_service(pubkeys).add_service(podr2v_srv).add_service(poisv_srv),
            ces_types::WorkerRole::Marker => server.add_service(pubkeys).add_service(podr2_srv).add_service(pois_srv),
        };
        let result = router.serve(public_listener_addr).await;
        let _ = ext_srv_quit_tx.send(result);
        info!("external server shutdown!");
    });

    let _ = ext_srv_quit_rx.await;
}

async fn maybe_keyfairy_ready_by_restore<Platform>(ceseal: CesealSafeBox<Platform>, keyfairy_ready_by_restore: bool)
where
    Platform: pal::Platform + Serialize + DeserializeOwned,
{
    if keyfairy_ready_by_restore {
        //FIXME: The ideal solution is to wait for the ceseal service to start and complete
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        ceseal
            .lock(true, true)
            .expect("Failed to lock Ceseal")
            .system
            .as_mut()
            .expect("the checkpoint restore bug: system field must not be empty")
            .send_keyfairy_ready();
    }
}

async fn panic_if_no_pois_expender_param(ceseal_expert: expert::CesealExpertStub) -> (i64, i64, i64) {
    ceseal_expert
        .using_chain_storage(move |opt| {
            if let Some(cs) = opt {
                cs.get_pois_expender_param()
                    .expect("the pois expender param not config on chain")
            } else {
                panic!("chain storage must be inited")
            }
        })
        .await
        .map(|r| (r.0 as i64, r.1 as i64, r.2 as i64)) //FIXME: It is best to unify the type with POIS Verifier
        .expect("chain storage not ready")
}
