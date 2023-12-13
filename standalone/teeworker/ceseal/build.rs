use std::{env, error::Error};
use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn Error>> {
	EmitBuilder::builder()
        .all_build()
        .all_git()
        .emit()?;

	let ias_env = env::var("IAS_ENV").unwrap_or_else(|_| "DEV".to_string());
	match ias_env.as_ref() {
		"PROD" => {
			println!("cargo:rustc-env=IAS_HOST=api.trustedservices.intel.com");
			println!("cargo:rustc-env=IAS_SIGRL_ENDPOINT=/sgx/attestation/v4/sigrl/");
			println!("cargo:rustc-env=IAS_REPORT_ENDPOINT=/sgx/attestation/v4/report");
		},
		_ => {
			// DEV by default
			println!("cargo:rustc-env=IAS_HOST=api.trustedservices.intel.com");
			println!("cargo:rustc-env=IAS_SIGRL_ENDPOINT=/sgx/dev/attestation/v4/sigrl/");
			println!("cargo:rustc-env=IAS_REPORT_ENDPOINT=/sgx/dev/attestation/v4/report");
		},
	}
	if env::var("IAS_API_KEY").is_err() {
		println!("cargo:rustc-env=IAS_API_KEY=''");
	}
	println!("cargo:rerun-if-env-changed=IAS_API_KEY");

	Ok(())
}
