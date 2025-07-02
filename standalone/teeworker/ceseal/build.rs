use std::{env, error::Error};
use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn Error>> {
    EmitBuilder::builder()
        .all_build()
        .all_git()
        .git_sha(true)
        .emit()?;

    let sgx_profile = env::var("SGX_PROFILE").unwrap_or_else(|_| "DEV".to_string());
    match sgx_profile.as_ref() {
        "PROD" => {
            println!(
                "cargo:rustc-env=DCAP_PCCS_URL=https://dcap.cess.network/sgx/certification/v4/"
            );
        }
        _ => {
            // DEV by default
            println!(
                "cargo:rustc-env=DCAP_PCCS_URL=https://dcap.cess.network/sgx/certification/v4/"
            );
        }
    }

    Ok(())
}
