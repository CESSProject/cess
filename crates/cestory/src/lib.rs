#![warn(unused_imports)]
#![warn(unused_extern_crates)]

#[macro_use]
extern crate log;
extern crate cestory_pal as pal;

use anyhow::{anyhow, Result};
use sp_core::{crypto::Pair, sr25519};
use std::str::FromStr;

mod attestation;
mod ceseal;
// mod handover;
mod identity_key;
mod master_key;
pub mod podr2;
pub mod pois;
pub mod pubkeys;
mod utils;

pub use ceseal::CesealClient;
pub use cestory_api::chain_client::{self, runtime, AccountId, BlockNumber, CesChainClient};
pub use identity_key::IdentityKey;
pub use master_key::CesealMasterKey;
pub use utils::{
    cqh::ChainQueryHelper,
    ext_res_permit::{self, Permitter as ExtResPermitter},
    rpc::{anyhow_to_status, as_status, RpcResult, RpcStatusSource},
};

/// POIS k, n, d parameters
pub type PoisParam = (i64, i64, i64);

#[derive(Default, Clone)]
pub struct Config {
    pub chain_bootnodes: Option<Vec<String>>,

    /// The GK master key sealing path.
    pub sealing_path: String,

    /// The Ceseal persistent data storing path
    pub storage_path: String,

    /// The App version.
    pub version: String,

    /// The git commit hash which this binary was built from.
    pub git_revision: String,
    /// Number of cores used to run ceseal service
    pub cores: u32,

    /// The timeout of getting the attestation report.
    pub ra_timeout: std::time::Duration,

    /// The max retry times of getting the attestation report.
    pub ra_max_retries: u32,

    /// The type of ceseal's remote attestation method, None means epid.
    pub ra_type: Option<String>,

    pub role: ces_types::WorkerRole,

    pub mnemonic: String,

    pub debug_set_key: Option<Vec<u8>>,
    pub attestation_provider: Option<ces_types::AttestationProvider>,
    pub endpoint: String,
    pub stash_account: Option<AccountId>,
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Config")
            .field("chain_bootnodes", &self.chain_bootnodes)
            .field("sealing_path", &self.sealing_path)
            .field("storage_path", &self.storage_path)
            .field("version", &self.version)
            .field("git_revision", &self.git_revision)
            .field("cores", &self.cores)
            .field("ra_timeout", &self.ra_timeout)
            .field("ra_type", &self.ra_type)
            .field("mnemonic", &"***")
            .field("debug_set_key", &self.debug_set_key.as_ref().map(|e| hex::encode(e)))
            .field("attestation_provider", &self.attestation_provider)
            .field("endpoint", &self.endpoint)
            .field("stash_account", &self.stash_account)
            .finish()
    }
}

pub(crate) fn new_sr25519_key() -> sr25519::Pair {
    use ces_crypto::sr25519::SEED_BYTES;
    use rand::RngCore;
    let mut rng = rand::thread_rng();
    let mut seed = [0_u8; SEED_BYTES];
    rng.fill_bytes(&mut seed);
    sr25519::Pair::from_seed(&seed)
}

pub fn unix_now() -> u64 {
    use std::time::SystemTime;
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    now.as_secs()
}

pub fn try_account_from(account_slice: &[u8]) -> Result<AccountId> {
    if account_slice.len() == 32 {
        let mut data = [0; 32];
        data.copy_from_slice(account_slice);
        Ok(subxt::utils::AccountId32(data))
    } else {
        Err(anyhow!("Invalid account length"))
    }
}

pub async fn build<Platform: pal::Platform>(
    config: Config,
    platform: Platform,
) -> Result<(CesealClient, ChainQueryHelper, CesChainClient)> {
    use cestory_api::chain_client::{self, CesRuntimeConfig};
    use sp_core::H256;
    use subxt::{
        client::OnlineClient,
        lightclient::{ChainConfig, LightClient},
    };
    let genesis_hash = H256::from_str(chain_client::GENESIS_HASH)?;
    info!("{:?} chain genesis hash: {:?}", chain_client::CHAIN_NETWORK, genesis_hash);
    let chain_config = if let Some(ref bootnodes) = config.chain_bootnodes {
        ChainConfig::chain_spec(chain_client::CHAIN_SPEC).set_bootnodes(bootnodes)?
    } else {
        ChainConfig::chain_spec(chain_client::CHAIN_SPEC)
    };
    info!("Building light-client ...");
    let (_light_client, light_client_rpc) = LightClient::relay_chain(chain_config)?;
    let chain_client = OnlineClient::<CesRuntimeConfig>::from_rpc_client(light_client_rpc).await?;
    let cqh = ChainQueryHelper::build(chain_client.clone()).await?;
    let ceseal_client = ceseal::build(config, platform, chain_client.clone(), genesis_hash).await?;
    info!("Ceseal client was ready");
    Ok((ceseal_client, cqh, chain_client))
}
