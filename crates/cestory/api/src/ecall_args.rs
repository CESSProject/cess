use alloc::string::String;
use ces_types::WorkerRole;
use core::time::Duration;
use parity_scale_codec::{Decode, Encode};

#[derive(Debug, Encode, Decode, Default, Clone)]
pub struct InitArgs {
    /// The GK master key sealing path.
    pub sealing_path: String,

    /// The Ceseal persistent data storing path
    pub storage_path: String,

    /// The App version.
    pub version: String,

    /// The git commit hash which this binary was built from.
    pub git_revision: String,

    /// Enable checkpoint
    pub enable_checkpoint: bool,

    /// Checkpoint interval in seconds
    pub checkpoint_interval: u64,

    /// Remove corrupted checkpoint so that ceseal can restart to continue to load others.
    pub remove_corrupted_checkpoint: bool,

    /// Max number of checkpoint files kept
    pub max_checkpoint_files: u32,

    /// Number of cores used to run ceseal service
    pub cores: u32,

    /// Listening IP address of H2 server    
    pub ip_address: Option<String>,

    /// The public rpc port with acl enabled
    pub public_port: Option<u16>,

    /// Only sync blocks into ceseal without dispatching messages.
    pub safe_mode_level: u8,

    /// Disable the RCU policy to update the Ceseal state.
    pub no_rcu: bool,

    /// The timeout of getting the attestation report.
    pub ra_timeout: Duration,

    /// The max retry times of getting the attestation report.
    pub ra_max_retries: u32,

    /// The type of ceseal's remote attestation method,None means epid.
    pub ra_type: Option<String>,

    pub role: WorkerRole,
}