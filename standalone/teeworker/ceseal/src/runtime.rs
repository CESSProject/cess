use crate::pal_gramine::GraminePlatform;

use anyhow::Result;
use core::sync::atomic::{AtomicU32, Ordering};
use cestory::{benchmark, Ceseal, RpcService};
use tracing::info;

lazy_static::lazy_static! {
    static ref APPLICATION: RpcService<GraminePlatform> = RpcService::new(GraminePlatform);
}

pub fn ecall_handle(req_id: u64, action: u8, input: &[u8]) -> Result<Vec<u8>> {
    let allow_rcu = action == cestory_api::actions::ACTION_GET_INFO;
    let mut factory = APPLICATION.lock_ceseal(allow_rcu, true)?;
    Ok(factory.handle_scale_api(req_id, action, input))
}

pub fn ecall_getinfo() -> String {
    let Ok(guard) = APPLICATION.lock_ceseal(true, true) else {
        return r#"{"error": "Failed to lock Ceseal""#.into();
    };
    let info = guard.get_info();
    serde_json::to_string_pretty(&info).unwrap_or_default()
}

pub fn ecall_sign_http_response(data: &[u8]) -> Option<String> {
    APPLICATION.lock_ceseal(true, true).ok()?.sign_http_response(data)
}

pub fn ecall_init(args: cestory_api::ecall_args::InitArgs) -> Result<()> {
    static INITIALIZED: AtomicU32 = AtomicU32::new(0);
    if INITIALIZED.fetch_add(1, Ordering::SeqCst) != 0 {
        anyhow::bail!("Enclave already initialized.");
    }

    if args.enable_checkpoint {
        match Ceseal::restore_from_checkpoint(&GraminePlatform, &args) {
            Ok(Some(factory)) => {
                info!("Loaded checkpoint");
                **APPLICATION.lock_ceseal(true, true).expect("Failed to lock Ceseal") = factory;
                return Ok(());
            }
            Err(err) => {
                anyhow::bail!("Failed to load checkpoint: {:?}", err);
            }
            Ok(None) => {
                info!("No checkpoint found");
            }
        }
    } else {
        info!("Checkpoint disabled.");
    }

    APPLICATION.lock_ceseal(true, true).unwrap().init(args);

    info!("Enclave init OK");
    Ok(())
}

pub fn ecall_bench_run(index: u32) {
    if !benchmark::paused() {
        info!(index, "Benchmark thread started");
        benchmark::run();
    }
}

pub async fn ecall_crpc_request(req_id: u64, path: &str, data: &[u8], json: bool) -> (u16, Vec<u8>) {
    info!(%path, json, "Handling cRPC request");
    let (code, data) = APPLICATION.dispatch_request(req_id, path, data, json).await;
    info!(code, size = data.len(), "cRPC returned");
    (code, data)
}
