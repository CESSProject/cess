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
        help = "ceseal home",
        default_value = "/opt/ceseal/data"
    )]
    pub ceseal_data_path: String,

    #[arg(
        long,
        help = "Ceseal log path for detect the status of previous ceseal",
        default_value = "/tmp/pre_ceseal.log"
    )]
    pub previous_ceseal_log_path: String,

    #[arg(
        long,
        help = "Ceseal log path for detect the status of new ceseal",
        default_value = "/tmp/new_ceseal.log"
    )]
    pub new_ceseal_log_path: String,

    #[arg(
        long,
        help = "The relative path where each version of ceseal stores protected files",
        default_value = "data/protected_files"
    )]
    pub ceseal_protected_files_path: String,

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
    pub previous_ceseal_port: u64,

    #[arg(
        long,
        help = "remote attestation type",
    )]
    pub ra_type: String,
}