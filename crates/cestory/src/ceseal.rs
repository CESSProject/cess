mod ready;
mod register;

use crate::{attestation::AttestationInfo, master_key::CesealMasterKey, AccountId, Config, IdentityKey};
use anyhow::Result;
use ces_types::{WorkerPublicKey, WorkerRegistrationInfo};
use cestory_api::chain_client::CesChainClient;
use ready::{BackgroundTask, BackgroundTaskHandle};
use register::RegisteredCeseal;
use sp_core::{sr25519, ByteArray, Pair, H256};

pub type TxSigner = subxt_signer::sr25519::Keypair;
pub type RegistrationInfo = WorkerRegistrationInfo<AccountId>;
pub type RawPublicKey = Vec<u8>;
pub type RawSignature = Vec<u8>;

pub use ready::ReadyCeseal;

pub async fn build<Platform: pal::Platform>(
    config: Config,
    platform: Platform,
    chain_client: CesChainClient,
    genesis_hash: H256,
) -> Result<CesealClient> {
    let registered = RegisteredCeseal::build(config, chain_client, platform, genesis_hash.clone()).await?;
    let ready = registered.wait_for_ready().await?;
    let (bg_task, bg_handle) = BackgroundTask::new(ready).await?;
    tokio::spawn(async move { bg_task.run().await });
    Ok(CesealClient { genesis_hash, handle: bg_handle })
}

#[derive(Debug, Clone)]
pub struct CesealClient {
    genesis_hash: H256,
    handle: BackgroundTaskHandle,
}

impl CesealClient { 
    pub async fn identity_key(&self) -> Result<IdentityKey> {
        self.handle.get_identity_key().await
    }

    pub async fn identity_public(&self) -> Result<WorkerPublicKey> {
        self.handle.get_identity_key().await.map(|ik| ik.public_key())
    }

    pub async fn master_key_sr25519_public(&self) -> Result<WorkerPublicKey> {
        self.handle.get_master_key().await.map(|mk| mk.sr25519_public_key())
    }

    pub async fn master_key_rsa_public_der(&self) -> Result<Vec<u8>> {
        self.handle
            .get_master_key()
            .await
            .map(|mk| mk.rsa_public_key_as_pkcs1_der().map_err(anyhow::Error::new))?
    }

    pub async fn master_key(&self) -> Result<CesealMasterKey> {
        self.handle.get_master_key().await
    }

    pub async fn master_key_sr25519_keypair(&self) -> Result<sr25519::Pair> {
        self.handle.get_master_key().await.map(|mk| mk.sr25519_keypair().clone())
    }

    pub async fn sign_use_identity_key(&self, data: &[u8]) -> Result<(RawSignature, RawPublicKey)> {
        let identity_key = self.handle.get_identity_key().await?;
        let signature = identity_key.key_pair.sign(data).to_raw_vec();
        let public_key = identity_key.public_key().to_vec();
        Ok((signature, public_key))
    }

    pub async fn sign_use_master_sr25519_key(&self, data: &[u8]) -> Result<(RawSignature, RawPublicKey)> {
        let master_key = self.handle.get_master_key().await?;
        let signature = master_key.sr25519_keypair().sign(data).to_raw_vec();
        let public_key = master_key.sr25519_public_key().to_vec();
        Ok((signature, public_key))
    }

    pub async fn sign_use_master_rsa_key(&self, data: &[u8]) -> Result<(RawSignature, RawPublicKey)> {
        let master_key = self.handle.get_master_key().await?;
        let signature = master_key
            .rsa_private_key()
            .sign(rsa::Pkcs1v15Sign::new_raw(), data)
            .map_err(anyhow::Error::new)?;
        let public_key = master_key.rsa_public_key_as_pkcs1_der().map_err(anyhow::Error::new)?;
        Ok((signature, public_key))
    }

    pub fn genesis_hash(&self) -> &H256 {
        &self.genesis_hash
    }

    pub async fn get_config(&self) -> Result<Config> {
        //self.handle.get_config().await
        todo!()
    }

    pub async fn is_sgx_env(&self) -> Result<bool> {
        //self.handle.is_sgx_env().await
        todo!()
    }

    pub async fn create_attestation_report(&self, data: &[u8]) -> Result<AttestationInfo> {
        todo!()
    }
}
