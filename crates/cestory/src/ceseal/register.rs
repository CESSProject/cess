use super::{
    master_pubkey_q::{self, OnChainMasterPubkeyQuerier},
    ReadyCeseal, TxSigner,
};
use crate::{
    attestation::{create_register_attestation_report, save_attestation, try_load_attestation, AttestationInfo},
    identity_key::PublicKey,
    master_key::is_master_key_exists_on_local,
    unix_now, CesealMasterKey, Config, IdentityKey, RegistrationInfo,
};
use anyhow::{anyhow, Result};
use ces_crypto::{key_share, rsa::RsaDer, sr25519::KDF, SecretKey};
use ces_types::AeadIV;
use cestory_api::chain_client::{
    runtime::{self, runtime_types::pallet_tee_worker::pallet::WorkerInfo},
    AccountId, CesChainClient,
};
use parity_scale_codec::{Decode, Encode};
use sp_core::H256;

pub(crate) struct RegisteredCeseal<Platform> {
    pub config: Config,
    pub chain_client: CesChainClient,
    pub platform: Platform,
    pub tx_signer: TxSigner,
    pub id_key: IdentityKey,
    pub registration_info: RegistrationInfo,
    pub attestation: AttestationInfo,
}

impl<Platform: pal::Platform> RegisteredCeseal<Platform> {
    pub async fn build(
        config: Config,
        chain_client: CesChainClient,
        platform: Platform,
        genesis_hash: H256,
    ) -> Result<RegisteredCeseal<Platform>> {
        let endpoint = config.endpoint.clone().ok_or(anyhow!("config.endpoint is required"))?;
        let tx_signer = {
            use core::str::FromStr;
            use subxt_signer::{sr25519::Keypair, SecretUri};
            Keypair::from_uri(&SecretUri::from_str(&config.mnemonic)?)?
        };

        let id_key = IdentityKey::build(&platform, &config.sealing_path, config.debug_set_key.clone())?;
        if id_key.dev_mode && config.attestation_provider.is_some() {
            return Err(anyhow!("RA is disallowed when debug_set_key is enabled"));
        }

        platform
            .quote_test(config.attestation_provider)
            .map_err(|e| anyhow!("quote test error: {:?}", e))?;

        let id_pubkey = id_key.public_key();
        info!("Identity pubkey: {:?}", hex::encode(id_pubkey));
        let registration_info = RegistrationInfo {
            version: Self::compat_app_version(),
            machine_id: platform.machine_id(),
            pubkey: id_key.public_key(),
            ecdh_pubkey: id_key.ecdh_public_key(),
            genesis_block_hash: genesis_hash,
            features: vec![platform.cpu_core_num(), platform.cpu_feature_level()],
            stash_account: config.stash_account.clone(),
            role: config.role.clone(),
            endpoint,
        };

        info!("Identity pubkey: {:?}", hex::encode(registration_info.pubkey.0.as_ref()));
        info!("ECDH pubkey: {:?}", hex::encode(registration_info.ecdh_pubkey.0.as_ref()));
        info!("CPU cores: {}", registration_info.features[0]);
        info!("CPU feature level: {}", registration_info.features[1]);

        let attestation =
            if let Some(worker_info_on_chain) = Self::get_worker_info_on_chain(&chain_client, &id_pubkey).await? {
                let (attestation, new_attest) = if let Some(a) = try_load_attestation(&platform, &config)? {
                    if a.is_attestation_expired(None) {
                        (create_register_attestation_report(&platform, &registration_info, &config)?, true)
                    } else if Self::need_update_worker_info(worker_info_on_chain, &registration_info) {
                        (create_register_attestation_report(&platform, &registration_info, &config)?, true)
                    } else {
                        (a, false)
                    }
                } else {
                    (create_register_attestation_report(&platform, &registration_info, &config)?, true)
                };
                if new_attest {
                    debug!("Updating TEE Worker info ...");
                    do_register(&chain_client, &tx_signer, &registration_info, &attestation).await?;
                    let _ = save_attestation(&platform, &attestation, &config);
                }
                attestation
            } else {
                debug!("Registering TEE Worker ...");
                let attestation = create_register_attestation_report(&platform, &registration_info, &config)?;
                do_register(&chain_client, &tx_signer, &registration_info, &attestation).await?;
                let _ = save_attestation(&platform, &attestation, &config);
                attestation
            };

        Ok(RegisteredCeseal { config, chain_client, platform, tx_signer, id_key, registration_info, attestation })
    }

    async fn get_worker_info_on_chain(
        chain_client: &CesChainClient,
        id_pubkey: &PublicKey,
    ) -> Result<Option<WorkerInfo<AccountId>>> {
        let q = runtime::storage().tee_worker().workers(&id_pubkey.0);
        let r = chain_client.storage().at_latest().await?.fetch(&q).await?;
        Ok(r)
    }

    fn need_update_worker_info(worker_info_on_chain: WorkerInfo<AccountId>, this_reg_info: &RegistrationInfo) -> bool {
        if worker_info_on_chain.role as u8 != this_reg_info.role.clone() as u8
            || worker_info_on_chain.stash_account != this_reg_info.stash_account
            || worker_info_on_chain.endpoint != this_reg_info.endpoint
            || worker_info_on_chain.version != this_reg_info.version
        {
            return true;
        }
        false
    }

    fn compat_app_version() -> u32 {
        let version = Platform::app_version();
        (version.major << 16) + (version.minor << 8) + version.patch
    }

    pub async fn wait_for_ready(self) -> Result<ReadyCeseal<Platform>> {
        info!("Waiting for Ceseal to be ready ...");
        let mk_pub_on_chain_q = master_pubkey_q::new_default(self.chain_client.clone());
        let mut mk_in_local = self.load_local_master_key();
        let mut applied = false;
        let mut apply_cnt = 0;
        let mut delivery_check_cnt = 0;
        loop {
            if let Some(ref mk_on_chain) = mk_pub_on_chain_q.try_query().await? {
                if let Some(ref local_mk) = mk_in_local {
                    if &local_mk.sr25519_public_key().0 == mk_on_chain {
                        let local_mk = mk_in_local.take().unwrap(); // Should never panic as earlier condition guarantees it
                        return Ok(ReadyCeseal::new_from(self, local_mk));
                    } else if !applied {
                        warn!("Local master key mismatches the on-chain master key");
                    }
                }

                if applied {
                    match self.try_pickup_my_master_key_delivery().await {
                        Ok(ps) => {
                            if let PostationState::Delivered(mk) = ps {
                                return Ok(ReadyCeseal::new_from(self, mk));
                            }
                        },
                        Err(e) => debug!("failed to process master key distribution: {:?}", e),
                    }
                    delivery_check_cnt += 1;
                    if delivery_check_cnt > 5 {
                        applied = false;
                        delivery_check_cnt = 0;
                    }
                } else {
                    if apply_cnt >= 5 {
                        return Err(anyhow!("Failed to apply master key after 5 attempts"));
                    }
                    self.apply_master_key().await?;
                    applied = true;
                    apply_cnt += 1;
                }
                tokio::time::sleep(std::time::Duration::from_secs(6)).await;
            } else {
                // mk not launched
                debug!("Waiting for master key to launch on chain");
                match mk_pub_on_chain_q.is_launching().await? {
                    Some(holder) if holder == self.id_key.public_key().0 => {
                        // We are the holder, so we should generate the master key
                        let mk = self.process_master_key_launch().await?;
                        return Ok(ReadyCeseal::new_from(self, mk));
                    },
                    _ => {},
                }
                tokio::time::sleep(std::time::Duration::from_secs(12)).await;
            }
        }
    }

    async fn apply_master_key(&self) -> Result<()> {
        debug!("Appling master key ...");
        use runtime::tee_worker::calls::types::apply_master_key::Payload;
        let payload = Payload {
            pubkey: self.id_key.public_key().0.clone(),
            ecdh_pubkey: self.id_key.ecdh_public_key().0.clone(),
            signing_time: unix_now(),
        };
        let signature = self.id_key.sign(&payload.encode()).encode();
        let tx = runtime::tx().tee_worker().apply_master_key(payload, signature);
        self.chain_client
            .tx()
            .sign_and_submit_then_watch_default(&tx, &self.tx_signer)
            .await?
            .wait_for_finalized_success()
            .await?;
        Ok(())
    }

    /// Generate the master key if this is the first keyfairy
    async fn process_master_key_launch(&self) -> Result<CesealMasterKey> {
        // TODO: double check the first master key holder is valid on chain
        let new_master_key = {
            use std::{fs::File, io::Read, path::PathBuf};
            let p = PathBuf::from(&self.config.storage_path).join("mk_rsa_der.dat");
            if p.exists() {
                // SHOULD ONLY USE IN DEVELOPMENT/TEST ENV!
                warn!("Use injected rsa-der for Master-Key from {}", p.as_path().display());
                let mut file = File::open(p).map_err(|e| anyhow!("open mk_rsa_der.dat failed: {:?}", e))?;
                let mut rsa_der = Vec::new();
                file.read_to_end(&mut rsa_der)
                    .map_err(|e| anyhow!("read mk_rsa_der.dat failed: {:?}", e))?;
                CesealMasterKey::from_rsa_der(&rsa_der)?
            } else {
                info!("Generate new Master-Key as the launcher");
                CesealMasterKey::generate()?
            }
        };
        new_master_key.seal(&self.config.sealing_path, &self.id_key.key_pair, &self.platform);

        let master_pubkey = new_master_key.sr25519_public_key();
        // upload the master key on chain via worker egress
        info!("Uploading Master-Pubkey: {} on chain", hex::encode(master_pubkey));

        // Claim launched to chain
        use runtime::tee_worker::calls::types::settle_master_key_launch::Payload;
        let payload = Payload { launcher: self.id_key.public_key().0.clone(), master_pubkey: master_pubkey.0.clone() };
        let tx = runtime::tx().tee_worker().settle_master_key_launch(payload, vec![]);
        self.chain_client
            .tx()
            .sign_and_submit_then_watch_default(&tx, &self.tx_signer)
            .await?
            .wait_for_finalized_success()
            .await?;
        Ok(new_master_key)
    }

    fn load_local_master_key(&self) -> Option<CesealMasterKey> {
        if is_master_key_exists_on_local(&self.config.sealing_path) {
            debug!("the master key on local, unseal it");
            let mk =
                CesealMasterKey::from_sealed_file(&self.config.sealing_path, &self.id_key.key_pair, &self.platform)
                    .expect("CesealMasterKey from sealed file");
            Some(mk)
        } else {
            None
        }
    }

    async fn try_pickup_my_master_key_delivery(&self) -> Result<PostationState> {
        let id_pubkey = self.id_key.public_key();
        let q = runtime::storage().tee_worker().master_key_postation(&id_pubkey.0);
        let payload = self.chain_client.storage().at_latest().await?.fetch(&q).await?;
        if payload.is_none() {
            return Ok(PostationState::NoApply);
        }

        if let Some((_, payload)) = payload.flatten() {
            let rsa_der = self.decrypt_key_from(&payload.ecdh_pubkey, &payload.encrypted_master_key, &payload.iv);
            let master_key = CesealMasterKey::from_rsa_der(&rsa_der)
                .map_err(|e| anyhow!("failed build CesealMasterKey from rsa_der: {e}"))?;
            master_key.seal(&self.config.sealing_path, &self.id_key.key_pair, &self.platform);
            Ok(PostationState::Delivered(master_key))
        } else {
            // mk apply has not yet been processed
            Ok(PostationState::Pending)
        }
    }

    /// Decrypt the key encrypted by `encrypt_key_to()`
    ///
    /// This function could panic a lot
    fn decrypt_key_from(&self, raw_ecdh_pubkey: &[u8; 32], encrypted_key: &[u8], iv: &AeadIV) -> RsaDer {
        let my_ecdh_key = self
            .id_key
            .key_pair
            .derive_ecdh_key()
            .expect("Should never failed with valid identity key; qed.");
        trace!("my ecdh pubkey: {}", hex::encode(my_ecdh_key.public()));
        trace!("ecdh pubkey of master key: {}", hex::encode(raw_ecdh_pubkey));
        trace!("encrypted key: {}", hex::encode(encrypted_key));
        trace!("iv: {}", hex::encode(iv));
        let secret = key_share::decrypt_secret_from(&my_ecdh_key, raw_ecdh_pubkey, encrypted_key, iv)
            .expect("Failed to decrypt dispatched key");
        let secret_data = match secret {
            SecretKey::Rsa(key) => key,
            _ => panic!("Expected rsa key, but got sr25519 key."),
        };
        secret_data
    }
}

enum PostationState {
    NoApply,
    Pending,
    Delivered(CesealMasterKey),
}

pub(crate) async fn do_register(
    chain_client: &CesChainClient,
    tx_signer: &TxSigner,
    registration_info: &RegistrationInfo,
    attestation: &AttestationInfo,
) -> Result<()> {
    use runtime::tee_worker::calls::types::register_worker::{Attestation, CesealInfo};
    let reg_info: CesealInfo = Decode::decode(&mut &registration_info.encode()[..])?;
    let attestation: Attestation = Decode::decode(&mut &attestation.encoded_report[..])?;
    let tx = runtime::tx().tee_worker().register_worker(reg_info, attestation);
    let tx_progress = chain_client.tx().sign_and_submit_then_watch_default(&tx, tx_signer).await?;
    debug!("The register tx hash: {:?}", hex::encode(tx_progress.extrinsic_hash()));
    let tx_in_block = tx_progress.wait_for_finalized().await?;
    debug!("The register tx in block: {:?}", hex::encode(tx_in_block.block_hash()));
    tx_in_block.wait_for_success().await?;

    Ok(())
}
