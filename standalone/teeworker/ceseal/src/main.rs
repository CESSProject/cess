mod handover;
mod pal_gramine;

use anyhow::{anyhow, Result};
use ces_types::{AttestationProvider, WorkerRole};
use cestory::{
    self, chain_client, AccountId, CesealClient, CesealMasterKey, ChainQueryHelper, Config,
    ExtResPermitter, PoisParam,
};
use clap::{crate_version, Parser, Subcommand};
use pal_gramine::GraminePlatform;
use std::{
    env,
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
#[clap(about = "The CESS TEE worker app.", version = VERSION, author)]
struct Args {
    /// Number of CPU cores to be used for PODR2 thread-pool.
    #[arg(short, long)]
    cores: Option<u32>,

    /// Listening IP address of public H2 server
    #[arg(long)]
    listening_ip: Option<String>,

    /// Listening port of public H2 server
    #[arg(long)]
    listening_port: Option<u16>,

    #[arg(long)]
    only_handover_server: bool,

    /// Handover key from another running ceseal instance
    #[arg(long)]
    request_handover_from: Option<String>,

    #[arg(long)]
    ra_type: Option<String>,

    /// The timeout of getting the attestation report. (in seconds)
    #[arg(long, value_parser = humantime::parse_duration, default_value = "8s")]
    ra_timeout: Duration,

    /// The max retry times of getting the attestation report.
    #[arg(long, default_value = "1")]
    ra_max_retries: u32,

    #[arg(long, value_parser = WorkerRole::from_str, default_value = "full")]
    role: WorkerRole,

    /// Custom ceseal data directory in non-SGX environment
    #[arg(long)]
    data_dir: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(
        default_value = "//Alice",
        short = 'm',
        long = "mnemonic",
        help = "Controller SR25519 private key mnemonic, private key seed, or derive path"
    )]
    pub mnemonic: String,

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

    #[arg(
        default_value = "0",
        long,
        help = "The charge transaction payment, unit: balance"
    )]
    pub tip: u128,

    #[arg(
        default_value = "16",
        long,
        help = "The transaction longevity, should be a power of two between 4 and 65536. unit: block"
    )]
    pub longevity: u64,

    /// Attestation provider
    #[arg(long, value_enum, default_value_t = RaOption::Ias)]
    pub attestation_provider: RaOption,

    #[arg(long)]
    chain_bootnodes: Option<Vec<String>>,

    #[arg(long, help = "The stash account for the TEE worker.")]
    pub stash_account: Option<AccountId>,
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum RaOption {
    None,
    Ias,
    Dcap,
}

impl From<RaOption> for Option<AttestationProvider> {
    fn from(other: RaOption) -> Self {
        match other {
            RaOption::None => None,
            RaOption::Ias => Some(AttestationProvider::Ias),
            RaOption::Dcap => Some(AttestationProvider::Dcap),
        }
    }
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
            self.mnemonic = String::from("//Alice");
            self.attestation_provider = RaOption::None;
        }
        if self.longevity > 0 {
            assert!(self.longevity >= 4, "Option --longevity must be 0 or >= 4.");
            assert_eq!(
                self.longevity.count_ones(),
                1,
                "Option --longevity must be power of two."
            );
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

    fn into_config(self, sealing_path: String, storage_path: String) -> Config {
        let debug_set_key = self.debug_set_key();
        Config {
            chain_bootnodes: self.chain_bootnodes,
            sealing_path,
            storage_path,
            version: env!("CARGO_PKG_VERSION").into(),
            git_revision: format!(
                "{}-{}",
                env!("VERGEN_GIT_SHA"),
                env!("VERGEN_BUILD_TIMESTAMP")
            ),
            cores: self.cores.unwrap_or_else(|| num_cpus::get() as _),
            ra_timeout: self.ra_timeout,
            ra_max_retries: self.ra_max_retries,
            ra_type: self.ra_type,
            role: self.role,
            debug_set_key,
            mnemonic: self.mnemonic,
            attestation_provider: self.attestation_provider.into(),
            endpoint: self.public_endpoint,
            stash_account: self.stash_account,
        }
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

            let sgx = pal_gramine::is_gramine();
            pal_gramine::print_target_info();

            rt.block_on(serve(sgx, args))?;
        }
    }
    Ok(())
}

fn prepare_paths(sgx: bool, args: &Args) -> Result<(String, String)> {
    let sealing_path;
    let storage_path;
    if sgx {
        // In gramine, the protected files are configured via manifest file. So we must not allow it to
        // be changed at runtime for security reason. Thus hardcoded it to `/data/protected_files` here.
        // Should keep it the same with the manifest config.
        sealing_path = "/data/protected_files".to_string();
        storage_path = "/data/storage_files".to_string();
    } else {
        use std::{fs, path::Path};
        let data_dir = args.data_dir.as_ref().map_or("./data", |dir| dir.as_str());
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

#[tracing::instrument(name = "main", skip_all)]
async fn serve(sgx: bool, args: Args) -> Result<()> {
    info!(sgx, "Starting ceseal...");
    let (sealing_path, storage_path) = prepare_paths(sgx, &args)?;

    // for handover client side
    if let Some(from) = args.request_handover_from.clone() {
        info!(%from, "Starting handover");
        let config = args.into_config(sealing_path, storage_path);
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
    let config = args.into_config(sealing_path, storage_path);
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
        config: &Config,
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
