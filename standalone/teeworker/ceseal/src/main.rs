mod handover;
mod ias;
mod pal_gramine;

use anyhow::Result;
use ces_sanitized_logger as logger;
use cestory::run_ceseal_server;
use cestory_api::ecall_args::InitArgs;
use clap::Parser;
use pal_gramine::GraminePlatform;
use std::{env, time::Duration};
use tracing::info;

#[derive(Parser, Debug, Clone)]
#[clap(about = "The CESS TEE worker app.", version, author)]
struct Args {
    /// Number of CPU cores to be used for mining.
    #[arg(short, long)]
    cores: Option<u32>,

    /// Run benchmark at startup.
    #[arg(long)]
    init_bench: bool,

    /// Allow CORS for HTTP
    #[arg(long)]
    allow_cors: bool,

    /// Turn on /kick API
    #[arg(long)]
    enable_kick_api: bool,

    /// Listening IP address of HTTP
    #[arg(long)]
    address: Option<String>,

    /// Listening port of HTTP
    #[arg(long)]
    port: Option<u16>,

    /// Listening port of HTTP (with access control)
    #[arg(long)]
    public_port: Option<u16>,

    /// Disable checkpoint
    #[arg(long)]
    disable_checkpoint: bool,

    /// Checkpoint interval in seconds, default to 30 minutes
    #[arg(long)]
    #[arg(default_value_t = 1800)]
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
}

#[tokio::main]
async fn main() -> Result<()> {
    pal_gramine::print_target_info();

    let sgx = pal_gramine::is_gramine();
    logger::init_subscriber(sgx);
    serve(sgx).await
}

#[tracing::instrument(name = "main", skip_all)]
async fn serve(sgx: bool) -> Result<()> {
    let args = Args::parse();

    info!(sgx, "Starting ceseal...");

    let sealing_path;
    let storage_path;
    if sgx {
        // In gramine, the protected files are configured via manifest file. So we must not allow it to
        // be changed at runtime for security reason. Thus hardcoded it to `/data/protected_files` here.
        // Should keep it the same with the manifest config.
        sealing_path = "/data/protected_files";
        storage_path = "/data/storage_files";
    } else {
        sealing_path = "./data/protected_files";
        storage_path = "./data/storage_files";

        fn mkdir(dir: &str) {
            if let Err(err) = std::fs::create_dir_all(dir) {
                panic!("Failed to create {dir}: {err:?}");
            }
        }
        mkdir(sealing_path);
        mkdir(storage_path);
    }

    let listener_addr = {
        let ip = args.address.as_ref().map_or("0.0.0.0", String::as_str);
        let port = args.port.unwrap_or(8000);
        format!("{ip}:{port}").parse().unwrap()
    };
    let public_listener_addr = {
        let ip = args.address.as_ref().map_or("0.0.0.0", String::as_str);
        let port = args.public_port.unwrap_or(19999);
        format!("{ip}:{port}").parse().unwrap()
    };

    let cores: u32 = args.cores.unwrap_or_else(|| num_cpus::get() as _);
    info!(bench_cores = cores);

    let init_args = {
        let args = args.clone();
        InitArgs {
            sealing_path: sealing_path.into(),
            storage_path: storage_path.into(),
            init_bench: args.init_bench,
            version: env!("CARGO_PKG_VERSION").into(),
            git_revision: format!(
                "{}-{}",
                env!("VERGEN_GIT_DESCRIBE"),
                env!("VERGEN_BUILD_TIMESTAMP")
            ),
            enable_checkpoint: !args.disable_checkpoint,
            checkpoint_interval: args.checkpoint_interval,
            remove_corrupted_checkpoint: args.remove_corrupted_checkpoint,
            max_checkpoint_files: args.max_checkpoint_files,
            cores,
            public_port: args.public_port,
            safe_mode_level: args.safe_mode_level,
            no_rcu: args.no_rcu,
            ra_timeout: args.ra_timeout,
            ra_max_retries: args.ra_max_retries,
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

    run_ceseal_server(
        init_args,
        GraminePlatform,
        listener_addr,
        public_listener_addr,
    )
    .await?;

    Ok(())
}
