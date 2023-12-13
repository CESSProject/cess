pub mod gk;
mod master_key;

use crate::{benchmark, secret_channel::ecdh_serde, types::BlockInfo};
use anyhow::Result;
use core::fmt;
use runtime::BlockNumber;

use crate::pal;
use chain::pallet_registry::RegistryEvent;
pub use master_key::{gk_master_key_exists, RotatedMasterKey};
use parity_scale_codec::{Decode, Encode};
pub use cestory_api::crpc::{GatekeeperRole, GatekeeperStatus, SystemInfo};
use ces_crypto::{
    ecdh::EcdhKey,
    key_share,
    sr25519::{Persistence, KDF},
};
use ces_mq::{
    traits::MessageChannel, BadOrigin, MessageDispatcher, MessageOrigin, MessageSendQueue,
    SignedMessageChannel, TypedReceiver,
};
use ces_serde_more as more;
use ces_types::{
    messaging::{
        AeadIV, BatchRotateMasterKeyEvent, DispatchMasterKeyEvent, DispatchMasterKeyHistoryEvent,
        GatekeeperChange, GatekeeperLaunch, HeartbeatChallenge, KeyDistribution,
        NewGatekeeperEvent, RemoveGatekeeperEvent, RotateMasterKeyEvent, SystemEvent, WorkerEvent,
        WorkingReportEvent,
    },
    wrap_content_to_sign, EcdhPublicKey, SignedContentType, WorkerPublicKey,
};
use serde::{Deserialize, Serialize};
use sp_core::{hashing::blake2_256, sr25519, Pair, U256};
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
    // for gatekeeper
    NotGatekeeper,
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

#[derive(Debug, Serialize, Deserialize, Clone, ::scale_info::TypeInfo)]
struct BenchState {
    start_block: chain::BlockNumber,
    start_time: u64,
    start_iter: u64,
    duration: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, ::scale_info::TypeInfo)]
enum WorkingState {
    Computing,
    Paused,
}

#[derive(Debug, Serialize, Deserialize, Clone, ::scale_info::TypeInfo)]
struct WorkingInfo {
    session_id: u32,
    state: WorkingState,
    start_time: u64,
    start_iter: u64,
}

// Minimum worker state machine can be reused to replay in GK.
#[derive(Debug, Serialize, Deserialize, Clone, ::scale_info::TypeInfo)]
struct WorkerState {
    #[codec(skip)]
    #[serde(with = "more::pubkey_bytes")]
    pubkey: WorkerPublicKey,
    hashed_id: U256,
    registered: bool,
    bench_state: Option<BenchState>,
    working_state: Option<WorkingInfo>,
}

impl WorkerState {
    pub fn new(pubkey: WorkerPublicKey) -> Self {
        let raw_pubkey: &[u8] = pubkey.as_ref();
        let pkh = blake2_256(raw_pubkey);
        let hashed_id: U256 = pkh.into();
        Self {
            pubkey,
            hashed_id,
            registered: false,
            bench_state: None,
            working_state: None,
        }
    }

    pub fn process_event(
        &mut self,
        block: &BlockInfo,
        event: &SystemEvent,
        callback: &mut impl WorkerStateMachineCallback,
        log_on: bool,
    ) {
        match event {
            SystemEvent::WorkerEvent(evt) => {
                if evt.pubkey != self.pubkey {
                    return;
                }

                use WorkerEvent::*;
                use WorkingState::*;
                if log_on {
                    info!("System::handle_event: {:?}", evt.event);
                }
                match evt.event {
                    Registered(_) => {
                        self.registered = true;
                    }
                    BenchStart { duration } => {
                        self.bench_state = Some(BenchState {
                            start_block: block.block_number,
                            start_time: block.now_ms,
                            start_iter: callback.bench_iterations(),
                            duration,
                        });
                        callback.bench_resume();
                    }
                    BenchScore(score) => {
                        if log_on {
                            info!("My benchmark score is {}", score);
                        }
                    }
                    Started { session_id, .. } => {
                        self.working_state = Some(WorkingInfo {
                            session_id,
                            state: Computing,
                            start_time: block.now_ms,
                            start_iter: callback.bench_iterations(),
                        });
                        callback.bench_resume();
                    }
                    Stopped => {
                        self.working_state = None;
                        if self.need_pause() {
                            callback.bench_pause();
                        }
                    }
                    EnterUnresponsive => {
                        if let Some(info) = &mut self.working_state {
                            if let Computing = info.state {
                                if log_on {
                                    info!("Enter paused");
                                }
                                info.state = Paused;
                                return;
                            }
                        }
                        if log_on {
                            error!(
                                "Unexpected event received: {:?}, working_state= {:?}",
                                evt.event, self.working_state
                            );
                        }
                    }
                    ExitUnresponsive => {
                        if let Some(info) = &mut self.working_state {
                            if let Paused = info.state {
                                if log_on {
                                    info!("Exit paused");
                                }
                                info.state = Computing;
                                return;
                            }
                        }
                        if log_on {
                            error!(
                                "Unexpected event received: {:?}, working_state= {:?}",
                                evt.event, self.working_state
                            );
                        }
                    }
                }
            }
            SystemEvent::HeartbeatChallenge(seed_info) => {
                self.handle_heartbeat_challenge(block, seed_info, callback, log_on);
            }
        };
    }

    fn handle_heartbeat_challenge(
        &mut self,
        block: &BlockInfo,
        seed_info: &HeartbeatChallenge,
        callback: &mut impl WorkerStateMachineCallback,
        log_on: bool,
    ) {
        if log_on {
            debug!(
                "System::handle_heartbeat_challenge({}, {:?}), registered={:?}, working_state={:?}",
                block.block_number, seed_info, self.registered, self.working_state
            );
        }

        if !self.registered {
            return;
        }

        let working_state = if let Some(state) = &mut self.working_state {
            state
        } else {
            return;
        };

        if matches!(working_state.state, WorkingState::Paused) {
            return;
        }

        let x = self.hashed_id ^ seed_info.seed;
        let online_hit = x <= seed_info.online_target;

        // Push queue when necessary
        if online_hit {
            let iterations = callback.bench_iterations() - working_state.start_iter;
            callback.heartbeat(
                working_state.session_id,
                block.block_number,
                block.now_ms,
                iterations,
            );
        }
    }

    fn need_pause(&self) -> bool {
        self.bench_state.is_none() && self.working_state.is_none()
    }

    fn on_block_processed(
        &mut self,
        block: &BlockInfo,
        callback: &mut impl WorkerStateMachineCallback,
    ) {
        // Handle registering benchmark report
        if let Some(BenchState {
            start_block,
            start_time,
            start_iter,
            duration,
        }) = self.bench_state
        {
            if block.block_number - start_block >= duration {
                self.bench_state = None;
                let iterations = callback.bench_iterations() - start_iter;
                callback.bench_report(start_time, iterations);
                if self.need_pause() {
                    callback.bench_pause();
                }
            }
        }
    }
}

trait WorkerStateMachineCallback {
    fn bench_iterations(&self) -> u64 {
        0
    }
    fn bench_resume(&mut self) {}
    fn bench_pause(&mut self) {}
    fn bench_report(&mut self, _start_time: u64, _iterations: u64) {}
    fn heartbeat(
        &mut self,
        _session_id: u32,
        _block_num: chain::BlockNumber,
        _block_time: u64,
        _iterations: u64,
    ) {
    }
}

struct WorkerSMDelegate<'a> {
    egress: &'a SignedMessageChannel,
}

impl WorkerStateMachineCallback for WorkerSMDelegate<'_> {
    fn bench_iterations(&self) -> u64 {
        benchmark::iteration_counter()
    }
    fn bench_resume(&mut self) {
        benchmark::resume();
    }
    fn bench_pause(&mut self) {
        benchmark::pause();
    }
    fn bench_report(&mut self, start_time: u64, iterations: u64) {
        let report = RegistryEvent::BenchReport {
            start_time,
            iterations,
        };
        info!("Reporting benchmark: {:?}", report);
        self.egress.push_message(&report);
    }
    fn heartbeat(
        &mut self,
        session_id: u32,
        challenge_block: chain::BlockNumber,
        challenge_time: u64,
        iterations: u64,
    ) {
        let event = WorkingReportEvent::Heartbeat {
            session_id,
            challenge_block,
            challenge_time,
            iterations,
        };
        info!("System: sending {:?}", event);
        self.egress.push_message(&event);
    }
}

#[derive(
    Serialize, Deserialize, Clone, derive_more::Deref, derive_more::DerefMut, derive_more::From,
)]
#[serde(transparent)]
pub(crate) struct WorkerIdentityKey(#[serde(with = "more::key_bytes")] sr25519::Pair);

// By mocking the public key of the identity key pair, we can pretend to be the first Gatekeeper on Khala
// for "shadow-gk" simulation.
#[cfg(feature = "shadow-gk")]
impl WorkerIdentityKey {
    pub(crate) fn public(&self) -> sr25519::Public {
        // The pubkey of the first GK on khala
        sr25519::Public(hex_literal::hex!(
            "60067697c486c809737e50d30a67480c5f0cede44be181b96f7d59bc2116a850"
        ))
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
    gatekeeper_launch_events: TypedReceiver<GatekeeperLaunch>,
    gatekeeper_change_events: TypedReceiver<GatekeeperChange>,
    key_distribution_events: TypedReceiver<KeyDistribution<chain::BlockNumber>>,
    // Worker
    #[codec(skip)]
    pub(crate) identity_key: WorkerIdentityKey,
    #[codec(skip)]
    #[serde(with = "ecdh_serde")]
    pub(crate) ecdh_key: EcdhKey,
    /// Be careful to use this field, as it is not updated in safe mode.
    worker_state: WorkerState,
    // Gatekeeper
    pub(crate) gatekeeper: Option<gk::Gatekeeper<SignedMessageChannel>>,

    // Cached for query
    /// The block number of the last block that the worker has synced.
    /// Be careful to use this field, as it is not updated in safe mode.
    pub(crate) block_number: BlockNumber,
    /// The timestamp of the last block that the worker has synced.
    /// Be careful to use this field, as it is not updated in safe mode.
    pub(crate) now_ms: u64,

    // If non-zero indicates the block which this worker loaded the chain state from.
    pub(crate) genesis_block: BlockNumber,
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
        _worker_threads: usize,
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
            gatekeeper_launch_events: recv_mq.subscribe_bound(),
            gatekeeper_change_events: recv_mq.subscribe_bound(),
            key_distribution_events: recv_mq.subscribe_bound(),
            identity_key,
            ecdh_key,
            worker_state: WorkerState::new(pubkey),
            gatekeeper: None,
            block_number: 0,
            now_ms: 0,
            genesis_block: 0,
        }
    }

    pub fn registered(&self) -> bool {
        self.worker_state.registered
    }

    pub fn process_next_message(&mut self, block: &mut BlockInfo) -> anyhow::Result<bool> {
        let ok = ces_mq::select_ignore_errors! {
            (event, origin) = self.system_events => {
                if !origin.is_pallet() {
                    anyhow::bail!("Invalid SystemEvent sender: {}", origin);
                }
                self.process_system_event(block, &event);
            },
            (event, origin) = self.gatekeeper_launch_events => {
                self.process_gatekeeper_launch_event(block, origin, event);
            },
            (event, origin) = self.gatekeeper_change_events => {
                self.process_gatekeeper_change_event(block, origin, event);
            },
            (event, origin) = self.key_distribution_events => {
                self.process_key_distribution_event(block, origin, event);
            },
        };
        Ok(ok.is_none())
    }

    pub fn will_process_block(&mut self, block: &mut BlockInfo) {
        self.block_number = block.block_number;
        self.now_ms = block.now_ms;

        if let Some(gatekeeper) = &mut self.gatekeeper {
            gatekeeper.will_process_block(block);
        }
    }

    pub fn process_messages(&mut self, block: &mut BlockInfo) {
        loop {
            match self.process_next_message(block) {
                Err(err) => {
                    error!("Error processing message: {:?}", err);
                }
                Ok(no_more) => {
                    if no_more {
                        break;
                    }
                }
            }
        }
        if let Some(gatekeeper) = &mut self.gatekeeper {
            gatekeeper.process_messages(block);
        }
    }

    pub fn did_process_block(&mut self, block: &mut BlockInfo) {
        if let Some(gatekeeper) = &mut self.gatekeeper {
            gatekeeper.did_process_block(block);
        }

        self.worker_state.on_block_processed(
            block,
            &mut WorkerSMDelegate {
                egress: &self.egress,
            },
        );
    }

    fn process_system_event(&mut self, block: &BlockInfo, event: &SystemEvent) {
        self.worker_state.process_event(
            block,
            event,
            &mut WorkerSMDelegate {
                egress: &self.egress,
            },
            true,
        );
    }

    /// Update local sealed master keys if the received history is longer than existing one.
    ///
    /// Panic if `self.gatekeeper` is None since it implies a need for resync from the start as gk
    fn set_master_key_history(&mut self, master_key_history: Vec<RotatedMasterKey>) {
        if self.gatekeeper.is_none() {
            master_key::seal(
                self.sealing_path.clone(),
                &master_key_history,
                &self.identity_key,
                &self.platform,
            );
            crate::maybe_remove_checkpoints(&self.storage_path);
            panic!("Received master key, please restart ceseal and cifrost to sync as Gatekeeper");
        }

        if self
            .gatekeeper
            .as_mut()
            .expect("checked; qed.")
            .set_master_key_history(&master_key_history)
        {
            master_key::seal(
                self.sealing_path.clone(),
                &master_key_history,
                &self.identity_key,
                &self.platform,
            );
        }
    }

    fn init_gatekeeper(
        &mut self,
        block: &mut BlockInfo,
        master_key_history: Vec<RotatedMasterKey>,
    ) {
        assert!(
            self.gatekeeper.is_none(),
            "Duplicated gatekeeper initialization"
        );
        assert!(
            !master_key_history.is_empty(),
            "Init gatekeeper with no master key"
        );

        if self.genesis_block != 0 {
            panic!("Gatekeeper must be synced start from the first block");
        }

        let master_key = sr25519::Pair::restore_from_secret_key(
            &master_key_history
                .first()
                .expect("empty master key history")
                .secret,
        );
        let gatekeeper = gk::Gatekeeper::new(
            master_key_history,
            block.recv_mq,
            block
                .send_mq
                .channel(MessageOrigin::Gatekeeper, master_key.into()),
        );
        self.gatekeeper = Some(gatekeeper);
    }

    fn process_gatekeeper_launch_event(
        &mut self,
        block: &mut BlockInfo,
        origin: MessageOrigin,
        event: GatekeeperLaunch,
    ) {
        if !origin.is_pallet() {
            error!("Invalid origin {:?} sent a {:?}", origin, event);
            return;
        }

        info!("Incoming gatekeeper launch event: {:?}", event);
        match event {
            GatekeeperLaunch::FirstGatekeeper(event) => {
                self.process_first_gatekeeper_event(block, origin, event)
            }
            GatekeeperLaunch::MasterPubkeyOnChain(event) => {
                info!(
                    "Gatekeeper launches on chain in block {}",
                    block.block_number
                );
                if let Some(gatekeeper) = &mut self.gatekeeper {
                    gatekeeper.master_pubkey_uploaded(event.master_pubkey);
                }
            }
            GatekeeperLaunch::RotateMasterKey(event) => {
                info!(
                    "Master key rotation req round {} in block {}",
                    event.rotation_id, block.block_number
                );
                self.process_master_key_rotation_request(block, origin, event);
            }
            GatekeeperLaunch::MasterPubkeyRotated(event) => {
                info!(
                    "Rotated Master Pubkey {} on chain in block {}",
                    hex::encode(event.master_pubkey),
                    block.block_number
                );
            }
        }
    }

    /// Generate the master key if this is the first gatekeeper
    fn process_first_gatekeeper_event(
        &mut self,
        block: &mut BlockInfo,
        _origin: MessageOrigin,
        event: NewGatekeeperEvent,
    ) {
        // ATTENTION: the first gk cannot resume if its original master_key.seal is lost,
        // since there is no tx recorded on-chain that shares the key to itself
        //
        // Solution: always unregister the first gk after the second gk receives the key,
        // thank god we only need to do this once for each blockchain

        // double check the first gatekeeper is valid on chain
        if !chain_state::is_gatekeeper(&event.pubkey, block.storage) {
            error!(
                "Fatal error: Invalid first gatekeeper registration {:?}",
                event
            );
            panic!("System state poisoned");
        }

        let mut master_key_history = master_key::try_unseal(
            self.sealing_path.clone(),
            &self.identity_key.0,
            &self.platform,
        );
        let my_pubkey = self.identity_key.public();
        if my_pubkey == event.pubkey {
            // if the first gatekeeper reboots, it will possess the master key, and should not re-generate it
            if master_key_history.is_empty() {
                info!("Gatekeeper: generate master key as the first gatekeeper");
                // generate master key as the first gatekeeper, no need to restart
                let master_key = crate::new_sr25519_key();
                master_key_history.push(RotatedMasterKey {
                    rotation_id: 0,
                    block_height: 0,
                    secret: master_key.dump_secret_key(),
                });
                // manually seal the first master key for the first gk
                master_key::seal(
                    self.sealing_path.clone(),
                    &master_key_history,
                    &self.identity_key,
                    &self.platform,
                );
            }

            let master_key = sr25519::Pair::restore_from_secret_key(
                &master_key_history.first().expect("checked; qed.").secret,
            );
            // upload the master key on chain via worker egress
            info!(
                "Gatekeeper: upload master key {} on chain",
                hex::encode(master_key.public())
            );
            let master_pubkey = RegistryEvent::MasterPubkey {
                master_pubkey: master_key.public(),
            };
            self.egress.push_message(&master_pubkey);
        }

        // other gatekeepers will has keys after key sharing and reboot
        // init the gatekeeper if there is any master key to start slient syncing
        if !master_key_history.is_empty() {
            info!("Init gatekeeper in block {}", block.block_number);
            self.init_gatekeeper(block, master_key_history);
        }

        if my_pubkey == event.pubkey {
            self.gatekeeper
                .as_mut()
                .expect("gatekeeper must be initializaed; qed.")
                .register_on_chain();
        }
    }

    /// Rotate the master key
    ///
    /// All the gatekeepers will generate the key, and only one will get published due to the nature of message queue.
    ///
    /// The generated master key will be shared to all the gatekeepers (include this one), and only then will they really
    /// update the master key on-chain.
    fn process_master_key_rotation_request(
        &mut self,
        block: &mut BlockInfo,
        _origin: MessageOrigin,
        event: RotateMasterKeyEvent,
    ) {
        if let Some(gatekeeper) = &mut self.gatekeeper {
            info!("Gatekeeper：Rotate master key");
            gatekeeper.process_master_key_rotation_request(
                block,
                event,
                self.identity_key.0.clone(),
            );
        }
    }

    fn process_gatekeeper_change_event(
        &mut self,
        block: &mut BlockInfo,
        origin: MessageOrigin,
        event: GatekeeperChange,
    ) {
        info!("Incoming gatekeeper change event: {:?}", event);
        match event {
            GatekeeperChange::Registered(event) => {
                self.process_new_gatekeeper_event(block, origin, event)
            }
            GatekeeperChange::Unregistered(event) => {
                self.process_remove_gatekeeper_event(block, origin, event)
            }
        }
    }

    /// Share the master key to the newly-registered gatekeeper
    /// Tick the state if the registered gatekeeper is this worker
    fn process_new_gatekeeper_event(
        &mut self,
        block: &mut BlockInfo,
        origin: MessageOrigin,
        event: NewGatekeeperEvent,
    ) {
        if !origin.is_pallet() {
            error!("Invalid origin {:?} sent a {:?}", origin, event);
            return;
        }

        // double check the registered gatekeeper is valid on chain
        if !chain_state::is_gatekeeper(&event.pubkey, block.storage) {
            error!(
                "Fatal error: Invalid first gatekeeper registration {:?}",
                event
            );
            panic!("System state poisoned");
        }

        if let Some(gatekeeper) = &mut self.gatekeeper {
            gatekeeper.share_master_key(&event.pubkey, &event.ecdh_pubkey, block.block_number);

            let my_pubkey = self.identity_key.public();
            if my_pubkey == event.pubkey {
                gatekeeper.register_on_chain();
            }
        }
    }

    /// Turn gatekeeper to silent syncing. The real cleanup will happen in next key rotation since it will have no chance
    /// to continuce syncing.
    ///
    /// There is no meaning to remove the master_key.seal file
    fn process_remove_gatekeeper_event(
        &mut self,
        _block: &mut BlockInfo,
        origin: MessageOrigin,
        event: RemoveGatekeeperEvent,
    ) {
        if !origin.is_pallet() {
            error!("Invalid origin {:?} sent a {:?}", origin, event);
            return;
        }

        let my_pubkey = self.identity_key.public();
        if my_pubkey == event.pubkey && self.gatekeeper.is_some() {
            self.gatekeeper
                .as_mut()
                .expect("checked; qed.")
                .unregister_on_chain();
        }
    }

    fn process_key_distribution_event(
        &mut self,
        block: &mut BlockInfo,
        origin: MessageOrigin,
        event: KeyDistribution<chain::BlockNumber>,
    ) {
        match event {
            KeyDistribution::MasterKeyDistribution(event) => {
                if let Err(err) = self.process_master_key_distribution(origin, event) {
                    error!("Failed to process master key distribution event: {:?}", err);
                };
            }
            KeyDistribution::MasterKeyRotation(event) => {
                if let Err(err) = self.process_batch_rotate_master_key(block, origin, event) {
                    error!(
                        "Failed to process batch master key rotation event: {:?}",
                        err
                    );
                };
            }
            KeyDistribution::MasterKeyHistory(event) => {
                if let Err(err) = self.process_master_key_history(origin, event) {
                    error!("Failed to process master key history event: {:?}", err);
                };
            }
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
    ) -> sr25519::Pair {
        let my_ecdh_key = self
            .identity_key
            .derive_ecdh_key()
            .expect("Should never failed with valid identity key; qed.");
        let secret =
            key_share::decrypt_secret_from(&my_ecdh_key, &ecdh_pubkey.0, encrypted_key, iv)
                .expect("Failed to decrypt dispatched key");
        sr25519::Pair::restore_from_secret_key(&secret)
    }

    /// Process encrypted master key from mq
    fn process_master_key_distribution(
        &mut self,
        origin: MessageOrigin,
        event: DispatchMasterKeyEvent,
    ) -> Result<(), TransactionError> {
        if !origin.is_gatekeeper() {
            error!("Invalid origin {:?} sent a {:?}", origin, event);
            return Err(TransactionError::BadOrigin);
        }

        let my_pubkey = self.identity_key.public();
        if my_pubkey == event.dest {
            let master_pair =
                self.decrypt_key_from(&event.ecdh_pubkey, &event.encrypted_master_key, &event.iv);
            info!("Gatekeeper: successfully decrypt received master key");
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
        if !origin.is_gatekeeper() {
            error!("Invalid origin {:?} sent a {:?}", origin, event);
            return Err(TransactionError::BadOrigin);
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
    /// The new master key takes effect immediately after the GatekeeperRegistryEvent::RotatedMasterPubkey is sent
    fn process_batch_rotate_master_key(
        &mut self,
        block: &mut BlockInfo,
        origin: MessageOrigin,
        event: BatchRotateMasterKeyEvent,
    ) -> Result<(), TransactionError> {
        // ATTENTION.shelven: There would be a mismatch between on-chain and off-chain master key until the on-chain pubkey
        // is updated, which may cause problem in the future.
        if !origin.is_gatekeeper() {
            error!("Invalid origin {:?} sent a {:?}", origin, event);
            return Err(TransactionError::BadOrigin);
        }

        // check the event sender identity and signature to ensure it's not forged with a leaked master key and really from
        // a gatekeeper
        let data = event.data_be_signed();
        let sig = sp_core::sr25519::Signature::try_from(event.sig.as_slice())
            .or(Err(TransactionError::BadSenderSignature))?;
        let data = wrap_content_to_sign(&data, SignedContentType::MasterKeyRotation);
        if !sp_io::crypto::sr25519_verify(&sig, &data, &event.sender) {
            return Err(TransactionError::BadSenderSignature);
        }
        // valid master key but from a non-gk
        if !chain_state::is_gatekeeper(&event.sender, block.storage) {
            error!("Fatal error: Forged batch master key rotation {:?}", event);
            return Err(TransactionError::MasterKeyLeakage);
        }

        let my_pubkey = self.identity_key.public();
        // for normal worker
        if self.gatekeeper.is_none() {
            if event.secret_keys.contains_key(&my_pubkey) {
                panic!(
                    "Batch rotate master key to a normal worker {:?}",
                    &my_pubkey
                );
            }
            return Ok(());
        }

        // for gatekeeper (both active or unregistered)
        if event.secret_keys.contains_key(&my_pubkey) {
            let encrypted_key = &event.secret_keys[&my_pubkey];
            let new_master_key = self.decrypt_key_from(
                &encrypted_key.ecdh_pubkey,
                &encrypted_key.encrypted_key,
                &encrypted_key.iv,
            );
            info!("Worker: successfully decrypt received rotated master key");
            let gatekeeper = self.gatekeeper.as_mut().expect("checked; qed.");
            if gatekeeper.append_master_key(RotatedMasterKey {
                rotation_id: event.rotation_id,
                block_height: self.block_number,
                secret: new_master_key.dump_secret_key(),
            }) {
                master_key::seal(
                    self.sealing_path.clone(),
                    gatekeeper.master_key_history(),
                    &self.identity_key,
                    &self.platform,
                );
            }
        }

        if self
            .gatekeeper
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
            info!("Worker: master key rotation received, stop unregistered gatekeeper silent syncing and cleanup");
            self.gatekeeper = None;
        }
        Ok(())
    }

    pub fn is_registered(&self) -> bool {
        self.worker_state.registered
    }

    pub fn gatekeeper_status(&self) -> GatekeeperStatus {
        let has_gatekeeper = self.gatekeeper.is_some();
        let active = match &self.gatekeeper {
            Some(gk) => gk.registered_on_chain(),
            None => false,
        };
        let role = match (has_gatekeeper, active) {
            (true, true) => GatekeeperRole::Active,
            (true, false) => GatekeeperRole::Dummy,
            _ => GatekeeperRole::None,
        };
        let master_public_key = self
            .gatekeeper
            .as_ref()
            .map(|gk| hex::encode(gk.master_pubkey()))
            .unwrap_or_default();
        GatekeeperStatus {
            role: role.into(),
            master_public_key,
        }
    }

    pub fn get_info(&self) -> SystemInfo {
        SystemInfo {
            registered: self.is_registered(),
            gatekeeper: Some(self.gatekeeper_status()),
            public_key: hex::encode(self.identity_key.public()),
            ecdh_public_key: hex::encode(self.ecdh_key.public()),
            genesis_block: self.genesis_block,
        }
    }
}

impl<P: pal::Platform> System<P> {
    pub fn on_restored(&mut self, safe_mode_level: u8) -> Result<()> {
        if safe_mode_level > 0 {
            return Ok(());
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

pub mod chain_state {
    use super::*;
    use crate::storage::ChainStorage;

    pub fn is_gatekeeper(pubkey: &WorkerPublicKey, chain_storage: &ChainStorage) -> bool {
        chain_storage.gatekeepers().contains(pubkey)
    }
}
