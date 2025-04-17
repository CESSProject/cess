use substrate_build_script_utils::{generate_cargo_keys, rerun_if_git_head_changed};

fn main() {
	build_constants();
	generate_cargo_keys();
	rerun_if_git_head_changed();
}

fn build_constants() {
	use std::{env, fs::File, io::Write, path::Path};

	let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
	let none_attestation_enabled = !(env::var("OA").unwrap_or(String::from("1")) == "1");
	let ceseal_verify_required = env::var("VC").unwrap_or(String::from("1")) == "1";
	let mut constants_file =
		File::create(&Path::new(&out_dir).join("constants.rs")).expect("Failed to create constants.rs");
	writeln!(
		constants_file,
		r#"
pub mod constants {{
	pub const NONE_ATTESTATION_ENABLED: bool = {none_attestation_enabled};
	pub const CESEAL_VERIFY_REQUIRED: bool = {ceseal_verify_required};	
}}
		"#
	)
	.expect("Failed to write to constants.rs");
}
