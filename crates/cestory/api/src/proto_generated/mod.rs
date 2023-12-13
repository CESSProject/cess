#[allow(clippy::derive_partial_eq_without_eq, clippy::let_unit_value)]
mod ceseal_rpc;
mod protos_codec_extensions;

pub use ceseal_rpc::*;
pub use protos_codec_extensions::*;

pub const PROTO_DEF: &str = include_str!("../../proto/ceseal_rpc.proto");

/// Helper struct used to compat the output of `get_info` for logging.
#[derive(Debug)]
pub struct Info<'a> {
    pub reg: bool,
    pub hdr: u32,
    pub blk: u32,
    pub dev: bool,
    pub msgs: u64,
    pub ver: &'a str,
    pub git: &'a str,
    pub rmem: u64,
    pub mpeak: u64,
    pub rpeak: u64,
    pub rspike: u64,
    pub mfree: u64,
    pub gblk: u32,
}

impl CesealInfo {
    pub fn debug_info(&self) -> Info {
        let mem = self.memory_usage.clone().unwrap_or_default();
        Info {
            reg: self.system.as_ref().map(|s| s.registered).unwrap_or(false),
            hdr: self.headernum,
            blk: self.blocknum,
            dev: self.dev_mode,
            msgs: self.pending_messages,
            ver: &self.version,
            git: &self.git_revision[0..8],
            rmem: mem.rust_used,
            rpeak: mem.rust_peak_used,
            rspike: mem.rust_spike,
            mpeak: mem.total_peak_used,
            mfree: mem.free,
            gblk: self.system.as_ref().map(|s| s.genesis_block).unwrap_or(0),
        }
    }

    pub fn public_key(&self) -> Option<&str> {
        self.system.as_ref().map(|s| s.public_key.as_str())
    }
}

#[cfg(test)]
mod tests;
