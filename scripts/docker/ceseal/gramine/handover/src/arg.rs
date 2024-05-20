use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    about = "xxx",
    version,
    author
)]
pub struct Args {
    #[arg(
        long,
        help = "The backup path of the each version of ceseal",
        default_value = "/opt/ceseal/backups"
    )]
    pub previous_version_ceseal_path: String,

    #[arg(
        long,
        help = "The backup path of the current version of ceseal",
        default_value = "/opt/ceseal/releases/current"
    )]
    pub current_version_ceseal_path: String,

    #[arg(
        long,
        help = "ceseal log path for detect the status of ceseal",
        default_value = "/tmp/ceseal.log"
    )]
    pub ceseal_log_path: String,

    #[arg(
        long,
        help = "the relative path where each version of ceseal stores sealed runtime_data",
        default_value = "data/protected_files/runtime-data.seal"
    )]
    pub ceseal_runtime_data_seal_path: String,

    #[arg(
        long,
        help = "the relative path where each version of ceseal stores checkpoint file",
        default_value = "data/storage_files"
    )]
    pub ceseal_storage_files_path: String,

    #[arg(
        long,
        help = "old ceseal start on this port",
        default_value = "1888"
    )]
    pub previous_ceseal_port: String,
}