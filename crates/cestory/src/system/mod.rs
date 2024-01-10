pub mod keyfairy;
mod master_key;

use crate::{
    expert::CesealExpertStub,
    podr2,
    secret_channel::ecdh_serde,
    types::{BlockDispatchContext, ExternalServiceMadeSender},
};
use anyhow::Result;
use core::fmt;
use runtime::BlockNumber;

use crate::pal;
use ces_crypto::{
    ecdh::EcdhKey,
    key_share,
    sr25519::{Persistence, KDF},
};
use ces_mq::{
    traits::MessageChannel, BadOrigin, MessageDispatcher, MessageOrigin, MessageSendQueue, SignedMessageChannel,
    TypedReceiver,
};
use ces_serde_more as more;
use ces_types::{
    messaging::{
        AeadIV, BatchRotateMasterKeyEvent, DispatchMasterKeyEvent, DispatchMasterKeyHistoryEvent, KeyDistribution,
        KeyfairyChange, KeyfairyLaunch, NewKeyfairyEvent, RemoveKeyfairyEvent, RotateMasterKeyEvent, SystemEvent,
        WorkerEvent,
    },
    wrap_content_to_sign, EcdhPublicKey, SignedContentType,
};
pub use cestory_api::crpc::{KeyfairyRole, KeyfairyStatus, SystemInfo};
pub use master_key::{gk_master_key_exists, RotatedMasterKey};
use pallet_tee_worker::RegistryEvent;
use parity_scale_codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair};
use std::convert::TryFrom;
use tracing::{error, info};

#[derive(Encode, Decode, Debug, Clone, thiserror::Error)]
#[error("TransactionError: {:?}", self)]
pub enum TransactionError {
    BadInput,
    BadOrigin,
    Other(String),
    // general
    InsufficientBalance,
    NoBalance,
    UnknownError,
    BadCommand,
    SymbolExist,
    AssetIdNotFound,
    NotAssetOwner,
    BadSecret,
    BadMachineId,
    FailedToSign,
    BadDecimal,
    DestroyNotAllowed,
    ChannelError,
    // for keyfairy
    NotKeyfairy,
    MasterKeyLeakage,
    BadSenderSignature,
    // for pdiem
    BadAccountInfo,
    BadLedgerInfo,
    BadTrustedStateData,
    BadEpochChangedProofData,
    BadTrustedState,
    InvalidAccount,
    BadTransactionWithProof,
    FailedToVerify,
    FailedToGetTransaction,
    FailedToCalculateBalance,
    BadChainId,
    TransferringNotAllowed,
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
pub(crate) struct WorkerIdentityKey(#[serde(with = "more::key_bytes")] sr25519::Pair);

// By mocking the public key of the identity key pair, we can pretend to be the first Keyfairy on Khala
// for "shadow-gk" simulation.
#[cfg(feature = "shadow-gk")]
impl WorkerIdentityKey {
    pub(crate) fn public(&self) -> sr25519::Public {
        // The pubkey of the first GK on khala
        sr25519::Public(hex_literal::hex!("60067697c486c809737e50d30a67480c5f0cede44be181b96f7d59bc2116a850"))
    }
}

#[derive(Serialize, Deserialize, Clone, ::scale_info::TypeInfo)]
pub struct System<Platform> {
    platform: Platform,
    // Configuration
    dev_mode: bool,
    pub(crate) sealing_path: String,
    pub(crate) storage_path: String,
    // Messageing
    egress: SignedMessageChannel,
    system_events: TypedReceiver<SystemEvent>,
    keyfairy_launch_events: TypedReceiver<KeyfairyLaunch>,
    keyfairy_change_events: TypedReceiver<KeyfairyChange>,
    key_distribution_events: TypedReceiver<KeyDistribution<chain::BlockNumber>>,
    // Worker
    #[codec(skip)]
    pub(crate) identity_key: WorkerIdentityKey,
    #[codec(skip)]
    #[serde(with = "ecdh_serde")]
    pub(crate) ecdh_key: EcdhKey,
    registered: bool,

    // Keyfairy
    keyfairy: Option<keyfairy::Keyfairy<SignedMessageChannel>>,

    // Cached for query
    /// The block number of the last block that the worker has synced.
    /// Be careful to use this field, as it is not updated in safe mode.
    block_number: BlockNumber,
    /// The timestamp of the last block that the worker has synced.
    /// Be careful to use this field, as it is not updated in safe mode.
    pub(crate) now_ms: u64,

    // If non-zero indicates the block which this worker loaded the chain state from.
    pub(crate) genesis_block: BlockNumber,

    #[codec(skip)]
    #[serde(skip)]
    ext_srvs_made_sender: Option<ExternalServiceMadeSender>,
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
        ext_srvs_made_sender: ExternalServiceMadeSender,
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
            system_events: recv_mq.subscribe_bound(),
            keyfairy_launch_events: recv_mq.subscribe_bound(),
            keyfairy_change_events: recv_mq.subscribe_bound(),
            key_distribution_events: recv_mq.subscribe_bound(),
            identity_key,
            ecdh_key,
            registered: false,
            keyfairy: None,
            block_number: 0,
            now_ms: 0,
            genesis_block: 0,
            ext_srvs_made_sender: Some(ext_srvs_made_sender),
        }
    }

    pub fn process_next_message(&mut self, block: &mut BlockDispatchContext) -> anyhow::Result<bool> {
        let ok = ces_mq::select_ignore_errors! {
            (event, origin) = self.system_events => {
                if !origin.is_pallet() {
                    anyhow::bail!("Invalid SystemEvent sender: {}", origin);
                }
                self.process_system_event(block, &event);
            },
            (event, origin) = self.keyfairy_launch_events => {
                self.process_keyfairy_launch_event(block, origin, event);
            },
            (event, origin) = self.keyfairy_change_events => {
                self.process_keyfairy_change_event(block, origin, event);
            },
            (event, origin) = self.key_distribution_events => {
                self.process_key_distribution_event(block, origin, event);
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

    fn process_system_event(&mut self, _block: &BlockDispatchContext, event: &SystemEvent) {
        match event {
            SystemEvent::WorkerEvent(evt) => {
                if evt.pubkey != self.identity_key.public() {
                    return
                }

                use WorkerEvent::*;
                info!("System::handle_event: {:?}", evt.event);
                match evt.event {
                    Registered(_) => {
                        self.registered = true;
                    },
                }
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

        let master_key = sr25519::Pair::restore_from_secret_key(
            &master_key_history.first().expect("empty master key history").secret,
        );
        let keyfairy = keyfairy::Keyfairy::new(
            master_key_history,
            block.send_mq.channel(MessageOrigin::Keyfairy, master_key.into()),
        );
        self.keyfairy = Some(keyfairy);

        self.init_external_services();
    }

    fn init_external_services(&mut self) {
        let (ce, rx) = CesealExpertStub::new();
        //TODO! TO BE REFACOTER HERE!
        let podr2_key = ces_pdp::gen_key(2048);
        let s1 = podr2::new_podr2_api_server(podr2_key.clone(), ce.clone());
        let s2 = podr2::new_podr2_verifier_api_server(podr2_key, ce);
        self.ext_srvs_made_sender
            .as_ref()
            .expect("ext_srvs_made_sender must be set")
            .send((rx, s1, s2))
            .expect("must to be sent");
    }

    fn process_keyfairy_launch_event(
        &mut self,
        block: &mut BlockDispatchContext,
        origin: MessageOrigin,
        event: KeyfairyLaunch,
    ) {
        if !origin.is_pallet() {
            error!("Invalid origin {:?} sent a {:?}", origin, event);
            return
        }

        info!("Incoming keyfairy launch event: {:?}", event);
        match event {
            KeyfairyLaunch::FirstKeyfairy(event) => self.process_first_keyfairy_event(block, origin, event),
            KeyfairyLaunch::MasterPubkeyOnChain(event) => {
                info!("Keyfairy launches on chain in block {}", block.block_number);
                if let Some(keyfairy) = &mut self.keyfairy {
                    keyfairy.master_pubkey_uploaded(event.master_pubkey);
                }
            },
            KeyfairyLaunch::RotateMasterKey(event) => {
                info!("Master key rotation req round {} in block {}", event.rotation_id, block.block_number);
                self.process_master_key_rotation_request(block, origin, event);
            },
            KeyfairyLaunch::MasterPubkeyRotated(event) => {
                info!(
                    "Rotated Master Pubkey {} on chain in block {}",
                    hex::encode(event.master_pubkey),
                    block.block_number
                );
            },
        }
    }

    /// Generate the master key if this is the first keyfairy
    fn process_first_keyfairy_event(
        &mut self,
        block: &mut BlockDispatchContext,
        _origin: MessageOrigin,
        event: NewKeyfairyEvent,
    ) {
        // ATTENTION: the first keyfairy cannot resume if its original master_key.seal is lost,
        // since there is no tx recorded on-chain that shares the key to itself
        //
        // Solution: always unregister the first keyfairy after the second keyfairy receives the key,
        // thank god we only need to do this once for each blockchain

        // double check the first keyfairy is valid on chain
        if !block.storage.is_keyfairy(&event.pubkey) {
            error!("Fatal error: Invalid first keyfairy registration {:?}", event);
            panic!("System state poisoned");
        }

        let mut master_key_history =
            master_key::try_unseal(self.sealing_path.clone(), &self.identity_key.0, &self.platform);
        let my_pubkey = self.identity_key.public();
        if my_pubkey == event.pubkey {
            // if the first keyfairy reboots, it will possess the master key, and should not re-generate it
            if master_key_history.is_empty() {
                info!("Keyfairy: generate master key as the first keyfairy");
                // generate master key as the first keyfairy, no need to restart
                let master_key = crate::new_sr25519_key();
                master_key_history.push(RotatedMasterKey {
                    rotation_id: 0,
                    block_height: 0,
                    secret: master_key.dump_secret_key(),
                });
                // manually seal the first master key for the first gk
                master_key::seal(self.sealing_path.clone(), &master_key_history, &self.identity_key, &self.platform);
            }

            let master_key =
                sr25519::Pair::restore_from_secret_key(&master_key_history.first().expect("checked; qed.").secret);
            // upload the master key on chain via worker egress
            info!("Keyfairy: upload master key {} on chain", hex::encode(master_key.public()));
            let master_pubkey = RegistryEvent::MasterPubkey { master_pubkey: master_key.public() };
            self.egress.push_message(&master_pubkey);
        }

        // other keyfairys will has keys after key sharing and reboot
        // init the keyfairy if there is any master key to start slient syncing
        if !master_key_history.is_empty() {
            info!("Init keyfairy in block {}", block.block_number);
            self.init_keyfairy(block, master_key_history);
        }

        if my_pubkey == event.pubkey {
            self.keyfairy
                .as_mut()
                .expect("keyfairy must be initializaed; qed.")
                .register_on_chain();
        }
    }

    /// Rotate the master key
    ///
    /// All the keyfairys will generate the key, and only one will get published due to the nature of message queue.
    ///
    /// The generated master key will be shared to all the keyfairys (include this one), and only then will they really
    /// update the master key on-chain.
    fn process_master_key_rotation_request(
        &mut self,
        block: &mut BlockDispatchContext,
        _origin: MessageOrigin,
        event: RotateMasterKeyEvent,
    ) {
        if let Some(keyfairy) = &mut self.keyfairy {
            info!("Keyfairy：Rotate master key");
            keyfairy.process_master_key_rotation_request(block, event, self.identity_key.0.clone());
        }
    }

    fn process_keyfairy_change_event(
        &mut self,
        block: &mut BlockDispatchContext,
        origin: MessageOrigin,
        event: KeyfairyChange,
    ) {
        info!("Incoming keyfairy change event: {:?}", event);
        match event {
            KeyfairyChange::Registered(event) => self.process_new_keyfairy_event(block, origin, event),
            KeyfairyChange::Unregistered(event) => self.process_remove_keyfairy_event(block, origin, event),
        }
    }

    /// Share the master key to the newly-registered keyfairy
    /// Tick the state if the registered keyfairy is this worker
    fn process_new_keyfairy_event(
        &mut self,
        block: &mut BlockDispatchContext,
        origin: MessageOrigin,
        event: NewKeyfairyEvent,
    ) {
        if !origin.is_pallet() {
            error!("Invalid origin {:?} sent a {:?}", origin, event);
            return
        }

        // double check the registered keyfairy is valid on chain
        if !block.storage.is_keyfairy(&event.pubkey) {
            error!("Fatal error: Invalid first keyfairy registration {:?}", event);
            panic!("System state poisoned");
        }

        if let Some(keyfairy) = &mut self.keyfairy {
            keyfairy.share_master_key(&event.pubkey, &event.ecdh_pubkey, block.block_number);

            let my_pubkey = self.identity_key.public();
            if my_pubkey == event.pubkey {
                keyfairy.register_on_chain();
            }
        }
    }

    /// Turn keyfairy to silent syncing. The real cleanup will happen in next key rotation since it will have no chance
    /// to continuce syncing.
    ///
    /// There is no meaning to remove the master_key.seal file
    fn process_remove_keyfairy_event(
        &mut self,
        _block: &mut BlockDispatchContext,
        origin: MessageOrigin,
        event: RemoveKeyfairyEvent,
    ) {
        if !origin.is_pallet() {
            error!("Invalid origin {:?} sent a {:?}", origin, event);
            return
        }

        let my_pubkey = self.identity_key.public();
        if my_pubkey == event.pubkey && self.keyfairy.is_some() {
            self.keyfairy.as_mut().expect("checked; qed.").unregister_on_chain();
        }
    }

    fn process_key_distribution_event(
        &mut self,
        block: &mut BlockDispatchContext,
        origin: MessageOrigin,
        event: KeyDistribution<chain::BlockNumber>,
    ) {
        match event {
            KeyDistribution::MasterKeyDistribution(event) => {
                if let Err(err) = self.process_master_key_distribution(origin, event) {
                    error!("Failed to process master key distribution event: {:?}", err);
                };
            },
            KeyDistribution::MasterKeyRotation(event) => {
                if let Err(err) = self.process_batch_rotate_master_key(block, origin, event) {
                    error!("Failed to process batch master key rotation event: {:?}", err);
                };
            },
            KeyDistribution::MasterKeyHistory(event) => {
                if let Err(err) = self.process_master_key_history(origin, event) {
                    error!("Failed to process master key history event: {:?}", err);
                };
            },
        }
    }

    /// Decrypt the key encrypted by `encrypt_key_to()`
    ///
    /// This function could panic a lot, thus should only handle data from other ceseals.
    fn decrypt_key_from(&self, ecdh_pubkey: &EcdhPublicKey, encrypted_key: &[u8], iv: &AeadIV) -> sr25519::Pair {
        let my_ecdh_key = self
            .identity_key
            .derive_ecdh_key()
            .expect("Should never failed with valid identity key; qed.");
        let secret = key_share::decrypt_secret_from(&my_ecdh_key, &ecdh_pubkey.0, encrypted_key, iv)
            .expect("Failed to decrypt dispatched key");
        sr25519::Pair::restore_from_secret_key(&secret)
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
                secret: master_pair.dump_secret_key(),
            }]);
        }
        Ok(())
    }

    fn process_master_key_history(
        &mut self,
        origin: MessageOrigin,
        event: DispatchMasterKeyHistoryEvent<chain::BlockNumber>,
    ) -> Result<(), TransactionError> {
        if !origin.is_keyfairy() {
            error!("Invalid origin {:?} sent a {:?}", origin, event);
            return Err(TransactionError::BadOrigin)
        }

        let my_pubkey = self.identity_key.public();
        if my_pubkey == event.dest {
            let master_key_history: Vec<RotatedMasterKey> = event
                .encrypted_master_key_history
                .iter()
                .map(|(rotation_id, block_height, key)| RotatedMasterKey {
                    rotation_id: *rotation_id,
                    block_height: *block_height,
                    secret: self
                        .decrypt_key_from(&key.ecdh_pubkey, &key.encrypted_key, &key.iv)
                        .dump_secret_key(),
                })
                .collect();
            self.set_master_key_history(master_key_history);
        }
        Ok(())
    }

    /// Decrypt the rotated master key
    ///
    /// The new master key takes effect immediately after the KeyfairyRegistryEvent::RotatedMasterPubkey is sent
    fn process_batch_rotate_master_key(
        &mut self,
        block: &mut BlockDispatchContext,
        origin: MessageOrigin,
        event: BatchRotateMasterKeyEvent,
    ) -> Result<(), TransactionError> {
        // ATTENTION.shelven: There would be a mismatch between on-chain and off-chain master key until the on-chain
        // pubkey is updated, which may cause problem in the future.
        if !origin.is_keyfairy() {
            error!("Invalid origin {:?} sent a {:?}", origin, event);
            return Err(TransactionError::BadOrigin)
        }

        // check the event sender identity and signature to ensure it's not forged with a leaked master key and really
        // from a keyfairy
        let data = event.data_be_signed();
        let sig = sp_core::sr25519::Signature::try_from(event.sig.as_slice())
            .or(Err(TransactionError::BadSenderSignature))?;
        let data = wrap_content_to_sign(&data, SignedContentType::MasterKeyRotation);
        if !sp_io::crypto::sr25519_verify(&sig, &data, &event.sender) {
            return Err(TransactionError::BadSenderSignature)
        }
        // valid master key but from a non-gk
        if !block.storage.is_keyfairy(&event.sender) {
            error!("Fatal error: Forged batch master key rotation {:?}", event);
            return Err(TransactionError::MasterKeyLeakage)
        }

        let my_pubkey = self.identity_key.public();
        // for normal worker
        if self.keyfairy.is_none() {
            if event.secret_keys.contains_key(&my_pubkey) {
                panic!("Batch rotate master key to a normal worker {:?}", &my_pubkey);
            }
            return Ok(())
        }

        // for keyfairy (both active or unregistered)
        if event.secret_keys.contains_key(&my_pubkey) {
            let encrypted_key = &event.secret_keys[&my_pubkey];
            let new_master_key =
                self.decrypt_key_from(&encrypted_key.ecdh_pubkey, &encrypted_key.encrypted_key, &encrypted_key.iv);
            info!("Worker: successfully decrypt received rotated master key");
            let keyfairy = self.keyfairy.as_mut().expect("checked; qed.");
            if keyfairy.append_master_key(RotatedMasterKey {
                rotation_id: event.rotation_id,
                block_height: self.block_number,
                secret: new_master_key.dump_secret_key(),
            }) {
                master_key::seal(
                    self.sealing_path.clone(),
                    keyfairy.master_key_history(),
                    &self.identity_key,
                    &self.platform,
                );
            }
        }

        if self
            .keyfairy
            .as_mut()
            .expect("checked; qed.")
            .switch_master_key(event.rotation_id, self.block_number)
        {
            // This is a valid GK in syncing, the needed master key should already be dispatched before the restart this
            // ceseal.
            info!("Worker: rotate master key with received master key history");
        } else {
            // This is an unregistered GK whose master key is not outdated yet, it 's still sliently syncing. It cannot
            // do silent syncing anymore since it does not know the rotated key.
            info!("Worker: master key rotation received, stop unregistered keyfairy silent syncing and cleanup");
            self.keyfairy = None;
        }
        Ok(())
    }

    pub fn is_registered(&self) -> bool {
        self.registered
    }

    pub fn keyfairy_status(&self) -> KeyfairyStatus {
        let has_keyfairy = self.keyfairy.is_some();
        let active = match &self.keyfairy {
            Some(kf) => kf.registered_on_chain(),
            None => false,
        };
        let role = match (has_keyfairy, active) {
            (true, true) => KeyfairyRole::Active,
            (true, false) => KeyfairyRole::Dummy,
            _ => KeyfairyRole::None,
        };
        let master_public_key = self
            .keyfairy
            .as_ref()
            .map(|kf| hex::encode(kf.master_pubkey()))
            .unwrap_or_default();
        KeyfairyStatus { role: role.into(), master_public_key }
    }

    pub fn get_info(&self) -> SystemInfo {
        SystemInfo {
            registered: self.is_registered(),
            keyfairy: Some(self.keyfairy_status()),
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
