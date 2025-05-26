use super::{
    master_pubkey_q::{self, OnChainMasterPubkeyQuerier},
    register::{self, RegisteredCeseal},
    RegistrationInfo, TxSigner,
};
use crate::{attestation::AttestationInfo, unix_now, BlockNumber, CesealMasterKey, Config, IdentityKey};
use anyhow::{anyhow, Result};
use ces_crypto::{
    aead, key_share,
    sr25519::{Persistence, KDF},
    SecretKey,
};
use ces_types::{EcdhPublicKey, EncryptedKey};
use cestory_api::chain_client::{runtime, CesChainClient};
use futures::{stream::StreamExt, FutureExt};
use parity_scale_codec::Encode;
use sp_core::{hashing, sr25519};
use tokio::{
    sync::{mpsc, oneshot},
    time::{self, Duration},
};
use tokio_stream::wrappers::{IntervalStream, UnboundedReceiverStream};

pub struct ReadyCeseal<Platform> {
    config: Config,
    chain_client: CesChainClient,
    platform: Platform,
    tx_signer: TxSigner,
    id_key: IdentityKey,
    registration_info: RegistrationInfo,
    attestation: AttestationInfo,
    master_key: CesealMasterKey,
    iv_seq: u64,
}

#[derive(Debug)]
enum Message {
    GetIdentityKey { sender: oneshot::Sender<IdentityKey> },
    GetMasterKey { sender: oneshot::Sender<CesealMasterKey> },
}

/// A handle to communicate with the background task.
#[derive(Clone, Debug)]
pub struct BackgroundTaskHandle {
    to_backend: mpsc::UnboundedSender<Message>,
}

impl BackgroundTaskHandle {
    pub async fn get_identity_key(&self) -> Result<IdentityKey> {
        let (tx, rx) = oneshot::channel();
        self.to_backend.send(Message::GetIdentityKey { sender: tx })?;
        rx.await.map_err(|_| anyhow!("Failed to receive identity key"))
    }

    pub async fn get_master_key(&self) -> Result<CesealMasterKey> {
        let (tx, rx) = oneshot::channel();
        self.to_backend.send(Message::GetMasterKey { sender: tx })?;
        rx.await.map_err(|_| anyhow!("Failed to receive master key"))
    }
}

pub struct BackgroundTask<Platform> {
    channels: BackgroundTaskChannels,
    data: ReadyCeseal<Platform>,
}

impl<Platform: pal::Platform> BackgroundTask<Platform> {
    pub(crate) async fn new(
        ready_ceseal: ReadyCeseal<Platform>,
    ) -> Result<(BackgroundTask<Platform>, BackgroundTaskHandle)> {
        let (tx, rx) = mpsc::unbounded_channel();

        let bg_task = BackgroundTask {
            channels: BackgroundTaskChannels {
                from_front: UnboundedReceiverStream::new(rx),
                from_interval: IntervalStream::new(time::interval(Duration::from_secs(6))),
            },
            data: ready_ceseal,
        };

        let bg_handle = BackgroundTaskHandle { to_backend: tx };

        Ok((bg_task, bg_handle))
    }

    /// Run the background task, which:
    /// - Forwards messages/subscription requests to Smoldot from the front end.
    /// - Forwards responses back from Smoldot to the front end.
    pub async fn run(self) {
        let mut channels = self.channels;
        let mut data = self.data;
        loop {
            tokio::pin! {
                let from_front_fut = channels.from_front.next().fuse();
                let from_interval_fut = channels.from_interval.next().fuse();
            }

            futures::select! {
                front_message = from_front_fut => {
                    let Some(message) = front_message else {
                        trace!("Ceseal channel closed");
                        break;
                    };
                    trace!("Received register message {message:?}");

                    data.handle_requests(message).await;
                },
                _ = from_interval_fut => {
                    data.handle_chain_tick().await;
                }
            }
        }
        trace!("Task closed");
    }
}

struct BackgroundTaskChannels {
    from_front: UnboundedReceiverStream<Message>,
    from_interval: IntervalStream,
}

impl<Platform: pal::Platform> ReadyCeseal<Platform> {
    pub(crate) fn new_from(registered: RegisteredCeseal<Platform>, master_key: CesealMasterKey) -> Self {
        let RegisteredCeseal {
            config, chain_client, platform, tx_signer, id_key, registration_info, attestation, ..
        } = registered;

        Self {
            config,
            chain_client,
            platform,
            tx_signer,
            id_key,
            registration_info,
            attestation,
            master_key,
            iv_seq: 0,
        }
    }

    async fn handle_requests(&mut self, message: Message) {
        match message {
            Message::GetIdentityKey { sender } => {
                let _ = sender.send(self.id_key.clone());
            },
            Message::GetMasterKey { sender } => {
                let _ = sender.send(self.master_key.clone());
            },
        }
    }

    async fn handle_chain_tick(&mut self) {
        {
            let mk_pub_on_chain_q = master_pubkey_q::new_default(self.chain_client.clone());
            match mk_pub_on_chain_q.is_launching().await {
                Ok(Some(_)) => {
                    info!("The MasterKey is rolling, exit and restart it");
                    std::process::exit(1);
                },
                Ok(None) => {},
                Err(e) => {
                    warn!("Failed to check MasterKey: {:?}", e);
                },
            }
        }
        if let Err(e) = self.try_process_master_key_apply().await {
            warn!("Try process master key apply return error: {:?}", e);
        }
        if let Err(e) = self.try_process_attestation_update().await {
            warn!("Try process atttestation update return error: {:?}", e);
        }
    }

    async fn try_process_master_key_apply(&mut self) -> Result<()> {
        let id_pubkey = self.id_key.public_key();
        let q = runtime::storage().tee_worker().master_key_distribute_notify(&id_pubkey.0);
        let Some(applier) = self.chain_client.storage().at_latest().await?.fetch(&q).await? else {
            trace!("not found mk distribute notify for you");
            return Ok(());
        };

        if applier == id_pubkey.0 {
            warn!("BUG: must not be distribute to self");
            return Ok(());
        }

        // Share the master key
        let block_number = self.chain_client.blocks().at_latest().await?.number();
        // Use the identity public key for the ecdh public key as that their are the same thing right now
        let ecdh_pubkey = applier.clone();
        let master_key = &self.master_key;
        const MASTER_KEY_SHARING_SALT: &[u8] = b"master_key_sharing";
        info!("Sharing master key at block: {block_number} for applier: {}", hex::encode(&ecdh_pubkey));
        let encrypted_key = Self::encrypt_key_to(
            master_key,
            &[MASTER_KEY_SHARING_SALT],
            &ecdh_pubkey.into(),
            block_number,
            self.iv_seq,
        );
        self.iv_seq += 1;

        use runtime::tee_worker::calls::types::distribute_master_key::Payload;
        let payload = Payload {
            distributor: id_pubkey.0,
            target: applier,
            ecdh_pubkey: encrypted_key.ecdh_pubkey.0,
            encrypted_master_key: encrypted_key.encrypted_key,
            iv: encrypted_key.iv,
            signing_time: unix_now(),
        };
        let signature = self.id_key.sign(&payload.encode()).encode();
        let tx = runtime::tx().tee_worker().distribute_master_key(payload, signature);
        self.chain_client
            .tx()
            .sign_and_submit_then_watch_default(&tx, &self.tx_signer)
            .await?
            .wait_for_finalized_success()
            .await?;
        Ok(())
    }

    /// Manually encrypt the secret key for sharing
    fn encrypt_key_to(
        master_key: &CesealMasterKey,
        key_derive_info: &[&[u8]],
        ecdh_pubkey: &EcdhPublicKey,
        block_number: BlockNumber,
        iv_seq: u64,
    ) -> EncryptedKey {
        let iv = Self::generate_iv(master_key, block_number, iv_seq);
        let rsa_secret_der = master_key.dump_rsa_secret_der();
        let (ecdh_pubkey, encrypted_key) = key_share::encrypt_secret_to(
            master_key.sr25519_keypair(),
            key_derive_info,
            &ecdh_pubkey.0,
            &SecretKey::Rsa(rsa_secret_der),
            &iv,
        )
        .expect("should never fail with valid master key; qed.");
        EncryptedKey { ecdh_pubkey: sr25519::Public::from_raw(ecdh_pubkey), encrypted_key, iv }
    }

    fn generate_iv(master_key: &CesealMasterKey, block_number: BlockNumber, iv_seq: u64) -> aead::IV {
        let derived_key = master_key
            .sr25519_keypair()
            .derive_sr25519_pair(&[b"iv_generator"])
            .expect("should not fail with valid info");

        let mut buf: Vec<u8> = Vec::new();
        buf.extend(derived_key.dump_secret_key().iter().copied());
        buf.extend(block_number.to_be_bytes().iter().copied());
        buf.extend(iv_seq.to_be_bytes().iter().copied());

        let hash = hashing::blake2_256(buf.as_ref());
        hash[0..12].try_into().expect("should never fail given correct length; qed.")
    }

    async fn try_process_attestation_update(&mut self) -> Result<()> {
        if self.attestation.is_attestation_expired(None) {
            debug!("Updating TEE Worker attestation ...");
            self.attestation =
                register::create_register_attestation_report(&self.platform, &self.registration_info, &self.config)?;
            register::save_attestation(&self.platform, &self.attestation, &self.config)?;
            register::do_register(&self.chain_client, &self.tx_signer, &self.registration_info, &self.attestation)
                .await?;
        }
        Ok(())
    }
}
