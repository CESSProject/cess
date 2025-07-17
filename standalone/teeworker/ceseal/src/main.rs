mod handover;
mod pal_gramine;

use anyhow::{anyhow, Result};
use ces_types::{AttestationProvider, WorkerRole};
use cestory::{
    self, chain_client, AccountId, CesealClient, CesealMasterKey, ChainQueryHelper,
    Config as CestoryConfig, ExtResPermitter, PoisParam,
};
use clap::{crate_version, Parser, Subcommand};
use pal_gramine::GraminePlatform;
use std::{
    env,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tonic::{service::RoutesBuilder, transport::Server};
use tracing::{info, warn};

const VERSION: &str = const_str::format!(
    "ceseal {}-{} {}",
    crate_version!(),
    env!("VERGEN_GIT_SHA"),
    env!("VERGEN_BUILD_TIMESTAMP")
);

#[derive(Parser, Debug, Clone)]
#[command(about = "The CESS TEE worker app.", version = VERSION, author)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    /// The path to the Ceseal configuration file.
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[arg(long)]
    only_handover_server: bool,

    /// Handover key from another running ceseal instance
    #[arg(long)]
    request_handover_from: Option<String>,

    /// Number of CPU cores to be used for PODR2 thread-pool.
    #[arg(long)]
    cores: Option<u32>,

    /// Listening IP address of public H2 server
    #[arg(long)]
    listening_ip: Option<String>,

    /// Listening port of public H2 server
    #[arg(long)]
    listening_port: Option<u16>,

    /// The timeout of getting the attestation report. (in seconds)
    #[arg(long, value_parser = humantime::parse_duration)]
    ra_timeout: Option<Duration>,

    /// The max retry times of getting the attestation report.
    #[arg(long)]
    ra_max_retries: Option<u32>,

    #[arg(long, value_parser = WorkerRole::from_str)]
    role: Option<WorkerRole>,

    /// Custom ceseal data directory in non-SGX environment
    #[arg(long)]
    data_dir: Option<String>,

    #[arg(
        short = 'm',
        long = "mnemonic",
        help = "Controller SR25519 private key mnemonic, private key seed, or derive path"
    )]
    pub mnemonic: Option<String>,

    #[arg(
        long,
        help = "The http endpoint where Ceseal provides services to the outside world"
    )]
    pub public_endpoint: Option<String>,

    #[arg(
        long,
        help = "Dev mode (equivalent to `--use-dev-key --mnemonic='//Alice'`)"
    )]
    pub dev: bool,

    #[arg(
        long,
        help = "Inject dev key (0x1) to Ceseal. Cannot be used with remote attestation enabled."
    )]
    pub use_dev_key: bool,

    #[arg(
        default_value = "",
        long = "inject-key",
        help = "Inject key to Ceseal."
    )]
    pub inject_key: String,

    #[arg(long, help = "The charge transaction payment, unit: balance")]
    pub tip: Option<u64>,

    #[arg(
        long,
        help = "The transaction longevity, should be a power of two between 4 and 65536. unit: block"
    )]
    pub longevity: Option<u64>,

    /// Attestation provider
    #[arg(long, value_enum)]
    pub attestation_provider: Option<AttestationProvider>,

    #[arg(long)]
    chain_bootnodes: Option<Vec<String>>,

    #[arg(long, help = "The stash account for the TEE worker.")]
    pub stash_account: Option<AccountId>,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Show Ceseal version details
    Version,
    /// Show Ceseal target information
    TargetInfo,
}

impl Args {
    fn validate_on_serve(&mut self) {
        if self.dev {
            self.use_dev_key = true;
            self.mnemonic = Some(String::from("//Alice"));
            self.attestation_provider = None;
        }
        self.fix_bootnode_if_absent_for_dev();
    }

    fn debug_set_key(&self) -> Option<Vec<u8>> {
        const DEV_KEY: &str = "0000000000000000000000000000000000000000000000000000000000000001";
        if !self.inject_key.is_empty() {
            if self.inject_key.len() != 64 {
                panic!("inject-key must be 32 bytes hex");
            } else {
                info!("Inject key {}", self.inject_key);
                Some(hex::decode(&self.inject_key).expect("Invalid dev key"))
            }
        } else if self.use_dev_key {
            info!("Inject key {}", DEV_KEY);
            Some(hex::decode(DEV_KEY).expect("Invalid dev key"))
        } else {
            None
        }
    }

    fn fix_bootnode_if_absent_for_dev(&mut self) {
        if matches!(chain_client::CHAIN_NETWORK, ces_types::ChainNetwork::Dev)
            && self.chain_bootnodes.is_none()
        {
            let default_dev_bootnode =
                "/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp";
            warn!("Use default chain bootnode for dev: {default_dev_bootnode}");
            self.chain_bootnodes = Some(vec![default_dev_bootnode.to_string()]);
        }
    }

    fn prepare_paths(&self) -> Result<(String, String)> {
        let sealing_path;
        let storage_path;
        if pal_gramine::is_gramine() {
            // In gramine, the protected files are configured via manifest file. So we must not allow it to
            // be changed at runtime for security reason. Thus hardcoded it to `/data/protected_files` here.
            // Should keep it the same with the manifest config.
            sealing_path = "/data/protected_files".to_string();
            storage_path = "/data/storage_files".to_string();
        } else {
            use std::{fs, path::Path};
            let data_dir = self.data_dir.as_ref().map_or("./data", |dir| dir.as_str());
            {
                let p = Path::new(data_dir).join("protected_files");
                sealing_path = p.to_str().unwrap().to_string();
                fs::create_dir_all(p)?;
            }
            {
                let p = Path::new(data_dir).join("storage_files");
                storage_path = p.to_str().unwrap().to_string();
                fs::create_dir_all(p)?;
            }
        }
        Ok((sealing_path, storage_path))
    }

    fn into_config(self) -> Result<CestoryConfig> {
        use config::{Config, ConfigError, Environment, File};
        let (sealing_path, storage_path) = self.prepare_paths()?;
        let defaults = Config::try_from(&CestoryConfig::default())?;
        let cfg = {
            let builder = Config::builder().add_source(defaults);
            if let Some(ref config_path) = self.config {
                builder.add_source(File::with_name(config_path.to_str().unwrap()).required(true))
            } else {
                builder
            }
            .add_source(Environment::with_prefix("CESEAL"))
            .build()?
        };
        let mut cfg = cfg
            .try_deserialize::<CestoryConfig>()
            .map_err(|e: ConfigError| anyhow!("Failed to deserialize config: {e}"))?;

        cfg.sealing_path = sealing_path;
        cfg.storage_path = storage_path;
        cfg.version = env!("CARGO_PKG_VERSION").to_string();
        cfg.git_revision = format!(
            "{}-{}",
            env!("VERGEN_GIT_SHA"),
            env!("VERGEN_BUILD_TIMESTAMP")
        );
        cfg.handover_serving = self.only_handover_server;
        if let Some(dsk) = self.debug_set_key() {
            cfg.debug_set_key = Some(dsk);
        }
        if let Some(cores) = self.cores {
            cfg.cores = cores;
        } else {
            cfg.cores = num_cpus::get() as u32;
        }
        if let Some(ra_timeout) = self.ra_timeout {
            cfg.ra_timeout = ra_timeout;
        }
        if let Some(ra_max_retries) = self.ra_max_retries {
            cfg.ra_max_retries = ra_max_retries;
        }
        if self.chain_bootnodes.is_some() {
            cfg.chain_bootnodes = self.chain_bootnodes;
        }
        if self.public_endpoint.is_some() {
            cfg.endpoint = self.public_endpoint;
        }
        if self.stash_account.is_some() {
            cfg.stash_account = self.stash_account;
        }
        if let Some(mnemonic) = self.mnemonic {
            cfg.mnemonic = mnemonic;
        }
        if let Some(attestation_provider) = self.attestation_provider {
            cfg.attestation_provider = Some(attestation_provider);
        }
        if let Some(role) = self.role {
            cfg.role = role;
        }
        if let Some(tip) = self.tip {
            cfg.tip = tip;
        }
        if let Some(longevity) = self.longevity {
            cfg.longevity = longevity;
        }
        Ok(cfg)
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let mut args = Args::parse();
    match args.command {
        Some(Commands::Version) => {
            if let Some(em) = pal_gramine::get_extend_measurement().unwrap() {
                println!("{} {:?}", VERSION, em.measurement_hash());
            } else {
                println!("{} [No measurement in non-SGX environments]", VERSION);
            }
        }
        Some(Commands::TargetInfo) => {
            pal_gramine::print_target_info();
        }
        None => {
            args.validate_on_serve();

            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;

            pal_gramine::print_target_info();

            rt.block_on(serve(args))?;
        }
    }
    Ok(())
}

#[tracing::instrument(name = "main", skip_all)]
async fn serve(args: Args) -> Result<()> {
    info!(sgx = pal_gramine::is_gramine(), "Starting ceseal...");

    // for handover client side
    if let Some(from) = args.request_handover_from.clone() {
        info!(%from, "Starting handover");
        let config = args.into_config()?;
        info!("Ceseal config: {:#?}", config);
        handover::handover_from(config, GraminePlatform, &from)
            .await
            .expect("Handover failed");
        info!("Handover done");
        return Ok(());
    }

    let listener_addr = {
        let ip = args.listening_ip.as_ref().map_or("0.0.0.0", String::as_str);
        let port = args.listening_port.unwrap_or(19999);
        format!("{ip}:{port}").parse().unwrap()
    };
    let only_handover_server = args.only_handover_server;
    let config = args.into_config()?;
    info!("Ceseal config: {:#?}", config);
    let chain_client = cestory::build_light_client(&config).await?;
    let cqh = ChainQueryHelper::build(chain_client.clone()).await?;
    let ceseal_client =
        cestory::build_ceseal_client(config.clone(), GraminePlatform, chain_client).await?;

    let mut routes_builder = RoutesBuilder::default();
    if only_handover_server {
        let svc = cestory::handover::new_handover_server(ceseal_client, cqh)
            .await
            .map_err(|e| anyhow!("Failed to create handover server: {e}"))?;
        routes_builder.add_service(svc);
        info!(
            "The ceseal server will listening on {}, only for handover",
            listener_addr
        );
    } else {
        let svc_params = ServiceBuildParams::make(&ceseal_client, &config, &cqh).await?;
        routes_builder.add_service(cestory::pubkeys::new_pubkeys_provider_server(
            ceseal_client,
            cqh,
        ));
        match config.role {
            ces_types::WorkerRole::Verifier => {
                for_verifier_routes(&mut routes_builder, &svc_params)
            }
            ces_types::WorkerRole::Marker => for_marker_routes(&mut routes_builder, &svc_params),
            ces_types::WorkerRole::Full => {
                for_verifier_routes(&mut routes_builder, &svc_params);
                for_marker_routes(&mut routes_builder, &svc_params);
            }
        };
        info!(
            "The ceseal server will listening on {} run with {:?} role",
            listener_addr, config.role
        );
    }
    let result = Server::builder()
        .add_routes(routes_builder.routes())
        .serve(listener_addr)
        .await
        .map_err(|e| anyhow!("Start server failed: {e}"))?;
    Ok::<(), anyhow::Error>(result)
}

const MAX_ENCODED_MSG_SIZE: usize = 104857600; // 100MiB
const MAX_DECODED_MSG_SIZE: usize = MAX_ENCODED_MSG_SIZE;

struct ServiceBuildParams {
    identity_pubkey: [u8; 32],
    master_key: CesealMasterKey,
    res_permitter: ExtResPermitter,
    podr2_thread_pool: Arc<Mutex<threadpool::ThreadPool>>,
    pois_param: PoisParam,
    cqh: ChainQueryHelper,
}

impl ServiceBuildParams {
    async fn make(
        ceseal_client: &CesealClient,
        config: &CestoryConfig,
        cqh: &ChainQueryHelper,
    ) -> Result<Self> {
        let identity_pubkey = ceseal_client.identity_public().await?.0;
        let master_key = ceseal_client.master_key().await?;
        let pois_param = cqh.pois_param().clone();
        let thread_pool_cap = config.cores.saturating_sub(1).max(1);
        let podr2_thread_pool = threadpool::ThreadPool::new(thread_pool_cap as usize);
        info!(
            "PODR2 compute thread pool capacity: {}",
            podr2_thread_pool.max_count()
        );
        let podr2_thread_pool = Arc::new(Mutex::new(podr2_thread_pool));
        let res_permitter = ExtResPermitter::new(config.role.clone());
        Ok(Self {
            identity_pubkey,
            master_key,
            res_permitter,
            podr2_thread_pool,
            pois_param,
            cqh: cqh.clone(),
        })
    }
}

fn for_verifier_routes(builder: &mut RoutesBuilder, svc_params: &ServiceBuildParams) {
    use cestory::{podr2, pois};
    let podr2_svc = podr2::new_podr2_verifier_api_server(
        svc_params.identity_pubkey.clone(),
        svc_params.master_key.clone(),
        svc_params.res_permitter.clone(),
        svc_params.podr2_thread_pool.clone(),
    )
    .max_decoding_message_size(MAX_DECODED_MSG_SIZE)
    .max_encoding_message_size(MAX_ENCODED_MSG_SIZE);
    let poisv_svc = pois::new_pois_verifier_api_server(
        svc_params.identity_pubkey.clone(),
        svc_params.master_key.clone(),
        svc_params.res_permitter.clone(),
        svc_params.pois_param.clone(),
    )
    .max_decoding_message_size(MAX_DECODED_MSG_SIZE)
    .max_encoding_message_size(MAX_ENCODED_MSG_SIZE);
    builder.add_service(podr2_svc).add_service(poisv_svc);
}

fn for_marker_routes(builder: &mut RoutesBuilder, svc_params: &ServiceBuildParams) {
    use cestory::{podr2, pois};
    let podr2_svc = podr2::new_podr2_api_server(
        svc_params.identity_pubkey.clone(),
        svc_params.master_key.clone(),
        svc_params.res_permitter.clone(),
        svc_params.podr2_thread_pool.clone(),
    )
    .max_decoding_message_size(MAX_DECODED_MSG_SIZE)
    .max_encoding_message_size(MAX_ENCODED_MSG_SIZE);
    let pois_svc = pois::new_pois_certifier_api_server(
        svc_params.cqh.clone(),
        svc_params.identity_pubkey.clone(),
        svc_params.master_key.clone(),
        svc_params.res_permitter.clone(),
        svc_params.pois_param.clone(),
    )
    .max_decoding_message_size(MAX_DECODED_MSG_SIZE)
    .max_encoding_message_size(MAX_ENCODED_MSG_SIZE);
    builder.add_service(podr2_svc).add_service(pois_svc);
}
