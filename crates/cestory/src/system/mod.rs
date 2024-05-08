pub mod keyfairy;
mod master_key;

use crate::{pal, secret_channel::ecdh_serde, types::BlockDispatchContext};
use anyhow::{Context, Result};
use ces_crypto::{ecdh::EcdhKey, key_share, rsa::RsaDer, sr25519::KDF, SecretKey};
use ces_mq::{
    traits::MessageChannel, BadOrigin, MessageDispatcher, MessageOrigin, MessageSendQueue, SignedMessageChannel,
    TypedReceiver,
};
use ces_serde_more as more;
use ces_types::{
    messaging::{AeadIV, DispatchMasterKeyEvent, MasterKeyApply, MasterKeyDistribution, MasterKeyLaunch, WorkerEvent},
    EcdhPublicKey, WorkerPublicKey,
};
pub use cestory_api::{crpc::SystemInfo, ecall_args::InitArgs};
use core::fmt;
pub use master_key::{persistence::is_master_key_exists_on_local, CesealMasterKey, Error as MasterKeyError};
use pallet_tee_worker::MasterKeySubmission;
use parity_scale_codec::{Decode, Encode};
use runtime::BlockNumber;
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair};
use std::sync::Arc;
use tracing::{error, info};

#[derive(Encode, Decode, Debug, Clone, thiserror::Error)]
#[error("TransactionError: {:?}", self)]
pub enum TransactionError {
    BadInput,
    BadOrigin,
    Other(String),
}

impl From<BadOrigin> for TransactionError {
    fn from(_: BadOrigin) -> TransactionError {
        TransactionError::BadOrigin
    }
}

impl From<String> for TransactionError {
    fn from(s: String) -> TransactionError {
        TransactionError::Other(s)
    }
}

#[derive(Serialize, Deserialize, Clone, derive_more::Deref, derive_more::DerefMut, derive_more::From)]
#[serde(transparent)]
pub struct WorkerIdentityKey(#[serde(with = "more::key_bytes")] pub sr25519::Pair);

// By mocking the public key of the identity key pair, we can pretend to be the first Keyfairy on Khala
// for "shadow-gk" simulation.
#[cfg(feature = "shadow-gk")]
impl WorkerIdentityKey {
    pub fn public(&self) -> sr25519::Public {
        // The pubkey of the first GK on khala
        sr25519::Public(hex_literal::hex!("60067697c486c809737e50d30a67480c5f0cede44be181b96f7d59bc2116a850"))
    }
}

#[derive(Serialize, Deserialize, ::scale_info::TypeInfo)]
pub struct System<Platform> {
    platform: Platform,
    // Configuration
    dev_mode: bool,
    pub(crate) sealing_path: String,
    pub(crate) storage_path: String,
    // Messageing
    egress: SignedMessageChannel,
    worker_events: TypedReceiver<WorkerEvent>,
    master_key_launch_events: TypedReceiver<MasterKeyLaunch>,
    master_key_distribution_events: TypedReceiver<MasterKeyDistribution>,
    master_key_apply_events: TypedReceiver<MasterKeyApply>,
    // Worker
    #[codec(skip)]
    pub(crate) identity_key: WorkerIdentityKey,
    #[codec(skip)]
    #[serde(with = "ecdh_serde")]
    pub(crate) ecdh_key: EcdhKey,
    registered: bool,

    // Keyfairy
    pub(crate) keyfairy: Option<keyfairy::Keyfairy<SignedMessageChannel>>,

    // Cached for query
    /// The block number of the last block that the worker has synced.
    /// Be careful to use this field, as it is not updated in safe mode.
    block_number: BlockNumber,
    /// The timestamp of the last block that the worker has synced.
    /// Be careful to use this field, as it is not updated in safe mode.
    pub(crate) now_ms: u64,

    // If non-zero indicates the block which this worker loaded the chain state from.
    pub(crate) genesis_block: BlockNumber,

    #[serde(skip)]
    #[codec(skip)]
    pub(crate) args: Arc<InitArgs>,
}

impl<Platform: pal::Platform> System<Platform> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        platform: Platform,
        dev_mode: bool,
        sealing_path: String,
        storage_path: String,
        identity_key: sr25519::Pair,
        ecdh_key: EcdhKey,
        send_mq: &MessageSendQueue,
        recv_mq: &mut MessageDispatcher,
        args: Arc<InitArgs>,
    ) -> Self {
        // Trigger panic early if platform is not properly implemented.
        let _ = Platform::app_version();

        let identity_key = WorkerIdentityKey(identity_key);
        let pubkey = identity_key.public();
        let sender = MessageOrigin::Worker(pubkey);

        System {
            platform,
            dev_mode,
            sealing_path,
            storage_path,
            egress: send_mq.channel(sender, identity_key.clone().0.into()),
            worker_events: recv_mq.subscribe_bound(),
            master_key_launch_events: recv_mq.subscribe_bound(),
            master_key_distribution_events: recv_mq.subscribe_bound(),
            master_key_apply_events: recv_mq.subscribe_bound(),
            identity_key,
            ecdh_key,
            registered: false,
            keyfairy: None,
            block_number: 0,
            now_ms: 0,
            genesis_block: 0,
            args,
        }
    }

    pub fn process_next_message(&mut self, block: &mut BlockDispatchContext) -> anyhow::Result<bool> {
        let ok = ces_mq::select_ignore_errors! {
            (event, origin) = self.worker_events => {
                if !origin.is_pallet() {
                    anyhow::bail!("Invalid WorkerEvent sender: {}", origin);
                }
                self.process_worker_event(block, &event);
            },
            (event, origin) = self.master_key_launch_events => {
                self.process_master_key_launch_event(block, origin, event);
            },
            (event, origin) = self.master_key_distribution_events => {
                self.process_master_key_distribution_event(block, origin, event);
            },
            (event, origin) = self.master_key_apply_events => {
                self.process_master_key_apply_event(block, origin, event);
            },
        };
        Ok(ok.is_none())
    }

    pub fn will_process_block(&mut self, block: &mut BlockDispatchContext) {
        self.block_number = block.block_number;
        self.now_ms = block.now_ms;
    }

    pub fn process_messages(&mut self, block: &mut BlockDispatchContext) {
        loop {
            match self.process_next_message(block) {
                Err(err) => {
                    error!("Error processing message: {:?}", err);
                },
                Ok(no_more) =>
                    if no_more {
                        break
                    },
            }
        }
    }

    pub fn did_process_block(&mut self, _block: &mut BlockDispatchContext) {}

    fn process_worker_event(&mut self, _block: &BlockDispatchContext, event: &WorkerEvent) {
        match event {
            WorkerEvent::Registered(pubkey) => {
                if *pubkey != self.identity_key.public() {
                    return
                }
                info!("System::handle_event: {:?}", event);
                self.registered = true;
            },
        };
    }

    fn process_master_key_launch_event(
        &mut self,
        block: &mut BlockDispatchContext,
        origin: MessageOrigin,
        event: MasterKeyLaunch,
    ) {
        if !origin.is_pallet() {
            error!("Invalid origin {:?} sent a {:?}", origin, event);
            return
        }

        match event {
            MasterKeyLaunch::LaunchRequest(worker_pubkey, _) =>
                self.process_master_key_launch_reqeust(block, origin, worker_pubkey),
            MasterKeyLaunch::OnChainLaunched(master_pubkey) => {
                if let Some(keyfairy) = &mut self.keyfairy {
                    assert_eq!(keyfairy.master_pubkey(), master_pubkey, "local and on-chain master key mismatch");
                }
                info!("master key is launched on chain at block {}", block.block_number);
            },
        }
    }

    fn init_keyfairy(&mut self, block: &mut BlockDispatchContext, master_key: CesealMasterKey) {
        let signer = master_key.sr25519_keypair().clone().into();
        let egress = block.send_mq.channel(MessageOrigin::Keyfairy, signer);
        self.keyfairy = Some(keyfairy::Keyfairy::new(master_key, egress));
        info!("keyfairy inited at block {}", block.block_number);
    }

    /// Generate the master key if this is the first keyfairy
    fn process_master_key_launch_reqeust(
        &mut self,
        block: &mut BlockDispatchContext,
        _origin: MessageOrigin,
        worker_pubkey: WorkerPublicKey,
    ) {
        // ATTENTION: the first keyfairy cannot resume if its original master_key.seal is lost,
        // since there is no tx recorded on-chain that shares the key to itself
        //
        // Solution: always unregister the first keyfairy after the second keyfairy receives the key,
        // thank god we only need to do this once for each blockchain

        assert!(self.keyfairy.is_none(), "Duplicated keyfairy initialization");

        if self.genesis_block != 0 {
            panic!("Keyfairy must be synced start from the first block");
        }

        if self.identity_key.public() != worker_pubkey {
            return
        }

        if is_master_key_exists_on_local(&self.sealing_path) {
            info!("the master key on local, unseal it");
            let master_key =
                CesealMasterKey::from_sealed_file(&self.sealing_path, &self.identity_key.0, &self.platform)
                    .expect("CesealMasterKey from sealed file");
            self.init_keyfairy(block, master_key);
            return
        }

        // double check the first master key holder is valid on chain
        if !block.storage.is_master_key_first_holder(&worker_pubkey) {
            error!("Fatal error: Invalid master key first holder {:?}", worker_pubkey);
            panic!("System state poisoned");
        }

        info!("generate new master key as the first keyfairy");
        let new_master_key = CesealMasterKey::generate().expect("CesealMasterKey generate");
        new_master_key.seal(&self.sealing_path, &self.identity_key.0, &self.platform);

        let master_pubkey = new_master_key.sr25519_public_key();
        // upload the master key on chain via worker egress
        info!("upload master pubkey: {} on chain", hex::encode(master_pubkey));
        let master_pubkey = MasterKeySubmission::MasterPubkey { master_pubkey };
        self.egress.push_message(&master_pubkey);

        self.init_keyfairy(block, new_master_key);
    }

    fn process_master_key_distribution_event(
        &mut self,
        block: &mut BlockDispatchContext,
        origin: MessageOrigin,
        event: MasterKeyDistribution,
    ) {
        match event {
            MasterKeyDistribution::Distribution(event) => {
                if let Err(err) = self.process_master_key_distribution(block, origin, event) {
                    error!("Failed to process master key distribution event: {:?}", err);
                };
            },
        }
    }

    /// Decrypt the key encrypted by `encrypt_key_to()`
    ///
    /// This function could panic a lot, thus should only handle data from other ceseals.
    fn decrypt_key_from(&self, ecdh_pubkey: &EcdhPublicKey, encrypted_key: &[u8], iv: &AeadIV) -> RsaDer {
        let my_ecdh_key = self
            .identity_key
            .derive_ecdh_key()
            .expect("Should never failed with valid identity key; qed.");
        let secret = key_share::decrypt_secret_from(&my_ecdh_key, &ecdh_pubkey.0, encrypted_key, iv)
            .expect("Failed to decrypt dispatched key");
        let secret_data = match secret {
            SecretKey::Rsa(key) => key,
            _ => panic!("Expected rsa key, but got sr25519 key."),
        };
        secret_data
    }

    /// Process encrypted master key from mq
    fn process_master_key_distribution(
        &mut self,
        block: &mut BlockDispatchContext,
        origin: MessageOrigin,
        event: DispatchMasterKeyEvent,
    ) -> Result<()> {
        if !origin.is_keyfairy() {
            error!("Invalid origin {:?} sent a {:?}", origin, event);
            return Err(TransactionError::BadOrigin.into())
        }

        if self.identity_key.public() != event.dest {
            trace!("ignore DispatchMasterKeyEvent that do not belong to you");
            return Ok(())
        }

        if self.keyfairy.is_some() {
            warn!("ignore DispatchMasterKeyEvent as the keyfariy has already inited");
            return Ok(())
        }

        let rsa_der = self.decrypt_key_from(&event.ecdh_pubkey, &event.encrypted_master_key, &event.iv);
        let master_key =
            CesealMasterKey::from_rsa_der(&rsa_der).context("failed build CesealMasterKey from rsa_der")?;
        master_key.seal(&self.sealing_path, &self.identity_key.0, &self.platform);

        self.init_keyfairy(block, master_key);

        Ok(())
    }

    fn process_master_key_apply_event(
        &mut self,
        block: &mut BlockDispatchContext,
        origin: MessageOrigin,
        event: MasterKeyApply,
    ) {
        match event {
            MasterKeyApply::Apply(worker_pubkey, ecdh_pubkey) => {
                if !origin.is_pallet() {
                    error!("Invalid origin {:?} sent a {:?}", origin, event);
                    return
                }

                if worker_pubkey == self.identity_key.public() {
                    trace!("ignore MasterKeyApply event that you send out");
                    return
                }

                // double check the tee-worker is registered on chain
                if !block.storage.is_worker_registered(&worker_pubkey) {
                    error!("Invalid master key apply: the tee-worker no registration {:?}", worker_pubkey);
                    return
                }

                // Share the master key to the newly-registered keyfairy
                // Tick the state if the registered keyfairy is this worker
                if let Some(keyfairy) = &mut self.keyfairy {
                    keyfairy.share_master_key(&worker_pubkey, &ecdh_pubkey, block.block_number);
                }
            },
        }
    }

    pub fn is_registered(&self) -> bool {
        self.registered
    }

    pub fn get_info(&self) -> SystemInfo {
        let master_public_key = self
            .keyfairy
            .as_ref()
            .map_or(String::default(), |kf| hex::encode(kf.master_pubkey()));
        SystemInfo {
            registered: self.is_registered(),
            master_public_key,
            public_key: hex::encode(self.identity_key.public()),
            ecdh_public_key: hex::encode(self.ecdh_key.public()),
            genesis_block: self.genesis_block,
        }
    }
}

impl<P: pal::Platform> System<P> {
    pub fn on_restored(&mut self, _safe_mode_level: u8) -> Result<()> {
        if self.keyfairy.is_some() {
            info!("system.on_restored(), keyfairy was assigned");
        } else {
            warn!("system.on_restored(), keyfairy is not set");
        }
        Ok(())
    }
}

#[derive(Encode, Decode, Debug)]
pub enum Error {
    NotAuthorized,
    TxHashNotFound,
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotAuthorized => write!(f, "not authorized"),
            Error::TxHashNotFound => write!(f, "transaction hash not found"),
            Error::Other(e) => write!(f, "{e}"),
        }
    }
}
