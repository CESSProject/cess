mod handover;
mod ias;
mod pal_gramine;

use anyhow::{bail, Result};
use ces_sanitized_logger as logger;
use ces_types::WorkerRole;
use cestory::run_ceseal_server;
use cestory_api::ecall_args::InitArgs;
use clap::{crate_version, Parser, Subcommand};
use pal_gramine::GraminePlatform;
use std::{env, time::Duration};
use tracing::info;

const VERSION: &str = const_str::format!(
    "ceseal {}-{} {}",
    crate_version!(),
    env!("VERGEN_GIT_SHA"),
    env!("VERGEN_BUILD_TIMESTAMP")
);

#[derive(Parser, Debug, Clone)]
#[clap(about = "The CESS TEE worker app.", version, author)]
struct Args {
    /// Number of CPU cores to be used for PODR2 thread-pool.
    #[arg(short, long)]
    cores: Option<u32>,

    /// Listening IP address of internal H2 server
    #[arg(long)]
    address: Option<String>,

    /// Listening port of internal H2 server
    #[arg(long)]
    port: Option<u16>,

    /// Listening port of public H2 server
    #[arg(long)]
    public_port: Option<u16>,

    /// Disable checkpoint
    #[arg(long)]
    disable_checkpoint: bool,

    /// Checkpoint interval in seconds, default to 15 minutes
    #[arg(long)]
    #[arg(default_value_t = 900)]
    checkpoint_interval: u64,

    /// Remove corrupted checkpoint so that ceseal can restart to continue to load others.
    #[arg(long)]
    remove_corrupted_checkpoint: bool,

    /// Max number of checkpoint files kept
    #[arg(long)]
    #[arg(default_value_t = 5)]
    max_checkpoint_files: u32,

    /// Handover key from another running ceseal instance
    #[arg(long)]
    request_handover_from: Option<String>,

    /// Safe mode level
    ///
    /// - 0, All features enabled.
    /// - 1, Sync blocks without dispatching messages.
    /// - 2, Sync blocks without storing the trie proofs and dispatching messages.
    ///     In this mode, it is needed to invoke crpc.LoadStorageProof to load the necessary values
    ///     before accepting key handover request.
    #[arg(long)]
    #[arg(default_value_t = 0)]
    safe_mode_level: u8,

    /// Disable the RCU policy to update the Ceseal state.
    #[arg(long)]
    no_rcu: bool,

    /// The timeout of getting the attestation report. (in seconds)
    #[arg(long, value_parser = humantime::parse_duration, default_value = "8s")]
    ra_timeout: Duration,

    /// The max retry times of getting the attestation report.
    #[arg(long, default_value = "1")]
    ra_max_retries: u32,

    #[arg(long, value_parser = parse_worker_role, default_value = "full")]
    role: WorkerRole,

    /// Custom ceseal data directory in non-SGX environment
    #[arg(long)]
    data_dir: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Show Ceseal version details
    Version,
    /// Show Ceseal target information
    TargetInfo,
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Some(Commands::Version) => {
            use hex_fmt::HexFmt;
            if let Some(em) = pal_gramine::get_extend_measurement().unwrap() {
                println!("{} {:?}", VERSION, HexFmt(em.measurement()));
            } else {
                println!("{} [No measurement in non-SGX environments]", VERSION);
            }
        }
        Some(Commands::TargetInfo) => {
            pal_gramine::print_target_info();
        }
        None => {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?;
            let sgx = pal_gramine::is_gramine();
            logger::init_subscriber(sgx);
            pal_gramine::print_target_info();
            rt.block_on(serve(sgx, args))?;
        }
    }
    Ok(())
}

#[tracing::instrument(name = "main", skip_all)]
async fn serve(sgx: bool, args: Args) -> Result<()> {
    info!(sgx, "Starting ceseal...");

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

    let listener_addr = {
        let ip = args.address.as_ref().map_or("0.0.0.0", String::as_str);
        let port = args.port.unwrap_or(8000);
        format!("{ip}:{port}").parse().unwrap()
    };

    let cores: u32 = args.cores.unwrap_or_else(|| num_cpus::get() as _);
    info!(cores = cores);

    let init_args = {
        let args = args.clone();
        InitArgs {
            sealing_path: sealing_path.into(),
            storage_path: storage_path.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            git_revision: format!(
                "{}-{}",
                env!("VERGEN_GIT_SHA"),
                env!("VERGEN_BUILD_TIMESTAMP")
            ),
            enable_checkpoint: !args.disable_checkpoint,
            checkpoint_interval: args.checkpoint_interval,
            remove_corrupted_checkpoint: args.remove_corrupted_checkpoint,
            max_checkpoint_files: args.max_checkpoint_files,
            cores,
            ip_address: args.address,
            public_port: args.public_port,
            safe_mode_level: args.safe_mode_level,
            no_rcu: args.no_rcu,
            ra_timeout: args.ra_timeout,
            ra_max_retries: args.ra_max_retries,
            role: args.role,
        }
    };
    info!("init_args: {:#?}", init_args);
    if let Some(from) = args.request_handover_from {
        info!(%from, "Starting handover");
        handover::handover_from(&from, init_args)
            .await
            .expect("Handover failed");
        info!("Handover done");
        return Ok(());
    }

    run_ceseal_server(init_args, GraminePlatform, listener_addr).await?;

    Ok(())
}

fn parse_worker_role(s: &str) -> Result<WorkerRole> {
    match s.to_lowercase().as_str() {
        "full" => Ok(WorkerRole::Full),
        "verifier" => Ok(WorkerRole::Verifier),
        "marker" => Ok(WorkerRole::Marker),
        _ => bail!("invalid WorkerRole value"),
    }
}
