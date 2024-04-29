pub mod keyfairy;
mod master_key;

use crate::{pal, secret_channel::ecdh_serde, types::BlockDispatchContext};
use anyhow::Result;
use ces_crypto::{ecdh::EcdhKey, key_share, rsa::Persistence, sr25519::KDF, SecretKey};
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
pub use master_key::{is_master_key_exists_on_local, RotatedMasterKey};
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

        // if let Some(keyfairy) = &mut self.keyfairy {
        //     keyfairy.will_process_block(block);
        // }
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
        // if let Some(keyfairy) = &mut self.keyfairy {
        //     keyfairy.process_messages(block);
        // }
    }

    pub fn did_process_block(&mut self, _block: &mut BlockDispatchContext) {
        // if let Some(keyfairy) = &mut self.keyfairy {
        //     keyfairy.did_process_block(block);
        // }
    }

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

    /// Update local sealed master keys if the received history is longer than existing one.
    ///
    /// Panic if `self.keyfairy` is None since it implies a need for resync from the start as gk
    fn set_master_key_history(&mut self, master_key_history: Vec<RotatedMasterKey>) {
        if self.keyfairy.is_none() {
            master_key::seal(self.sealing_path.clone(), &master_key_history, &self.identity_key, &self.platform);
            crate::maybe_remove_checkpoints(&self.storage_path);
            //TODO: refactor here later
            panic!("Received master key, please restart ceseal and cifrost to sync as Keyfairy");            
        }

        if self
            .keyfairy
            .as_mut()
            .expect("checked; qed.")
            .set_master_key_history(&master_key_history)
        {
            master_key::seal(self.sealing_path.clone(), &master_key_history, &self.identity_key, &self.platform);
        }
    }

    fn init_keyfairy(&mut self, block: &mut BlockDispatchContext, master_key_history: Vec<RotatedMasterKey>) {
        assert!(self.keyfairy.is_none(), "Duplicated keyfairy initialization");
        assert!(!master_key_history.is_empty(), "Init keyfairy with no master key");

        if self.genesis_block != 0 {
            panic!("Keyfairy must be synced start from the first block");
        }
        let master_key = crate::get_sr25519_from_rsa_key(
            rsa::RsaPrivateKey::restore_from_der(&master_key_history.first().expect("empty master key history").secret)
                .expect("Failed restore podr2 key from master_key_history in init_keyfairy"),
        );
        let keyfairy = keyfairy::Keyfairy::new(
            master_key_history,
            block.send_mq.channel(MessageOrigin::Keyfairy, master_key.into()),
        );
        self.keyfairy = Some(keyfairy);
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

        info!("Incoming keyfairy launch event: {:?}", event);
        match event {
            MasterKeyLaunch::LaunchRequest(worker_pubkey, _) =>
                self.process_master_key_launch_reqeust(block, origin, worker_pubkey),
            MasterKeyLaunch::OnChainLaunched(masterkey_pubkey) => {
                info!("Keyfairy launches on chain in block {}", block.block_number);
                if let Some(keyfairy) = &mut self.keyfairy {
                    keyfairy.master_pubkey_uploaded(masterkey_pubkey);
                }
            },
        }
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

        // double check the first master key holder is valid on chain
        if !block.storage.is_master_key_first_holder(&worker_pubkey) {
            error!("Fatal error: Invalid master key first holder {:?}", worker_pubkey);
            panic!("System state poisoned");
        }

        let mut master_key_history =
            master_key::try_unseal(self.sealing_path.clone(), &self.identity_key.0, &self.platform);
        let my_pubkey = self.identity_key.public();
        if my_pubkey == worker_pubkey {
            // if the first keyfairy reboots, it will possess the master key, and should not re-generate it
            if master_key_history.is_empty() {
                info!("Keyfairy: generate master key as the first keyfairy");
                // generate master key as the first keyfairy, no need to restart
                let podr2_key = crate::new_podr2_key();
                master_key_history.push(RotatedMasterKey {
                    rotation_id: 0,
                    block_height: 0,
                    secret: podr2_key.dump_secret_der(),
                });
                // manually seal the first master key for the first gk
                master_key::seal(self.sealing_path.clone(), &master_key_history, &self.identity_key, &self.platform);
            }
            let master_key = crate::get_sr25519_from_rsa_key(
                rsa::RsaPrivateKey::restore_from_der(&master_key_history.first().expect("checked; qed.").secret)
                    .expect("Failed to restore sr25519 from master key history in process_first_keyfairy_event"),
            );
            // upload the master key on chain via worker egress
            info!("Keyfairy: upload master key {} on chain", hex::encode(master_key.public()));
            let master_pubkey = MasterKeySubmission::MasterPubkey { master_pubkey: master_key.public() };
            self.egress.push_message(&master_pubkey);
        }

        // other keyfairys will has keys after key sharing and reboot
        // init the keyfairy if there is any master key to start slient syncing
        if !master_key_history.is_empty() {
            info!("Init keyfairy in block {}", block.block_number);
            self.init_keyfairy(block, master_key_history);
        }
    }

    fn process_master_key_distribution_event(
        &mut self,
        _block: &mut BlockDispatchContext,
        origin: MessageOrigin,
        event: MasterKeyDistribution,
    ) {
        match event {
            MasterKeyDistribution::Distribution(event) => {
                if let Err(err) = self.process_master_key_distribution(origin, event) {
                    error!("Failed to process master key distribution event: {:?}", err);
                };
            },
        }
    }

    /// Decrypt the key encrypted by `encrypt_key_to()`
    ///
    /// This function could panic a lot, thus should only handle data from other ceseals.
    fn decrypt_key_from(
        &self,
        ecdh_pubkey: &EcdhPublicKey,
        encrypted_key: &[u8],
        iv: &AeadIV,
    ) -> (sr25519::Pair, Vec<u8>) {
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
        (
            crate::get_sr25519_from_rsa_key(
                rsa::RsaPrivateKey::restore_from_der(&secret_data)
                    .expect("Failed restore sr25519 from rsa in decrypt_key_from"),
            ),
            secret_data,
        )
    }

    /// Process encrypted master key from mq
    fn process_master_key_distribution(
        &mut self,
        origin: MessageOrigin,
        event: DispatchMasterKeyEvent,
    ) -> Result<(), TransactionError> {
        if !origin.is_keyfairy() {
            error!("Invalid origin {:?} sent a {:?}", origin, event);
            return Err(TransactionError::BadOrigin)
        }

        let my_pubkey = self.identity_key.public();
        if my_pubkey == event.dest {
            let master_pair = self.decrypt_key_from(&event.ecdh_pubkey, &event.encrypted_master_key, &event.iv);
            info!("Keyfairy: successfully decrypt received master key");
            self.set_master_key_history(vec![RotatedMasterKey {
                rotation_id: 0,
                block_height: 0,
                secret: master_pair.1,
            }]);
        }
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
                    info!("ignore self master key apply");
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
    pub fn on_restored(&mut self, safe_mode_level: u8) -> Result<()> {
        if safe_mode_level > 0 {
            return Ok(())
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
