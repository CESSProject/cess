use std::{
    env,
    fs::{self, File},
    io::Write,
    path::Path,
    process::Command,
    str::FromStr,
};

fn main() {
    build_runtime_client();
    build_protos();
    println!("cargo:rerun-if-changed=build.rs");
}

fn build_runtime_client() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let profile = env::var("PROFILE").expect("PROFILE not set");
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_dir = Path::new(&manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .expect("Failed to get workspace dir");

    // build genesis hash and chain-spec
    {
        use ces_types::ChainNetwork;
        let chain_network = ChainNetwork::from_str(&env::var("CHAIN_NETWORK").unwrap_or_else(|_| {
            println!("cargo::warning=The `CHAIN_NETWORK` enviroment variable not found, use Dev for default");
            "dev".to_string()
        }))
        .unwrap();
        println!("cargo::warning=building for Chain-Network: {:?}", chain_network);
        let node_crate_name = "cess-node";
        let node_crate_src_path = workspace_dir.join("standalone").join("chain").join("node").join("src");
        let fixed_specs_dir = workspace_dir.join("standalone").join("chain").join("node").join("ccg");
        let spec_path = match chain_network {
            ChainNetwork::Dev => {
                // Generate the `dev` chain-spec
                let dev_spec_path = Path::new(&out_dir).join("dev-spec.json");
                let node_bin = workspace_dir.join("target").join(&profile).join(node_crate_name);
                if !node_bin.exists() {
                    panic!("{node_crate_name} binary missing: {}. Build '{node_crate_name}' crate first.", node_bin.display());
                }
                let chain_spec_output = Command::new(&node_bin)
                    .args(["build-spec", "--chain", "dev", "--raw", "--disable-default-bootnode"])
                    .output()
                    .expect(format!("Failed to run {} build-spec", node_crate_name).as_str());

                if !chain_spec_output.status.success() {
                    eprintln!("Error running {node_crate_name}: {:?}", chain_spec_output.stderr);
                    panic!("{node_crate_name} build-spec failed");
                }
                fs::write(&dev_spec_path, &chain_spec_output.stdout).expect("Failed to write dev-spec.json");
                dev_spec_path
            },
            ChainNetwork::Devnet => fixed_specs_dir.join(format!("cess-develop-spec-raw.json")),
            ChainNetwork::Testnet => fixed_specs_dir.join(format!("cess-testnet-spec-raw.json")),
        };

        let genesis_hash_path = Path::new(&out_dir).join("genesis_hash.rs");
        let mut genesis_hash_file = File::create(&genesis_hash_path).expect("Failed to create genesis_hash.rs");
        if !spec_path.exists() {
            panic!("Chain spec file: {:?} not found", spec_path)
        }
        let spec_content = fs::read_to_string(&spec_path).expect("Failed to read chain spec");
        let genesis_hash = build_genesis_hash(&spec_content);
        writeln!(
            genesis_hash_file,
            r#"
            pub const CHAIN_NETWORK: ces_types::ChainNetwork = ces_types::ChainNetwork::{:?};
            pub const CHAIN_SPEC: &str = include_str!({:?});
            pub const GENESIS_HASH: &str = "{}";
            "#,
            chain_network, spec_path, genesis_hash
        )
        .expect("Failed to write to genesis_hash.rs");

        println!("cargo:rerun-if-changed={}", node_crate_src_path.display());
        println!("cargo:rerun-if-changed={}", fixed_specs_dir.display());
    }

    // build the runtime WASM file
    {
        let runtime_name = env::var("RUNTIME_NAME").unwrap_or("cess-node-runtime".to_string());
        let runtime_crate_src_path = workspace_dir.join("standalone").join("chain").join("runtime").join("src");
        let wasm_path = workspace_dir
            .join("target")
            .join(&profile)
            .join("wbuild")
            .join(&runtime_name)
            .join(format!("{}.wasm", runtime_name.replace("-", "_")));

        if !wasm_path.exists() {
            eprintln!("Error: WASM file does not exist: {}", wasm_path.display());
            println!(
                "cargo:warning=Error: WASM file does not exist: {}. Please build the 'runtime' crate first.",
                wasm_path.display()
            );
            panic!("WASM file missing. Ensure 'runtime' crate is built with the correct profile.");
        }

        let dest_path = Path::new(&out_dir).join("runtime_path.rs");
        let mut file = File::create(&dest_path).expect("Failed to create runtime_path.rs");
        writeln!(
            file,
            r#"
        #[subxt::subxt(runtime_path = "{}")]
        pub mod runtime {{}}
        "#,
            wasm_path.to_string_lossy().replace("\\", "/"),
        )
        .expect("Failed to write to runtime_path.rs");
        println!("cargo:rerun-if-changed={}", wasm_path.display());
        println!("cargo:rerun-if-changed={}", runtime_crate_src_path.display());
    }
    println!("cargo:rerun-if-env-changed=PROFILE");
}

fn build_genesis_hash(spec_json: &str) -> String {
    use smoldot::{chain_spec, header};
    let chain_spec = chain_spec::ChainSpec::from_json_bytes(spec_json).expect("Failed parse chain spec");
    let genesis_chain_information = chain_spec
        .to_chain_information()
        .map(|(ci, _)| ci)
        .expect("Failed to get genesis chain information");
    let header = genesis_chain_information.as_ref().finalized_block_header;
    let genesis_block_header = header.scale_encoding_vec(usize::from(chain_spec.block_number_bytes()));
    let genesis_block_hash = header::hash_from_scale_encoded_header(&genesis_block_header);
    hex::encode(genesis_block_hash)
}

fn build_protos() {
    let builder = tonic_build::configure()
        .build_server(true)
        .build_client(if cfg!(feature = "api-client") { true } else { false });

    builder
        .compile_protos(&["handover-api.proto", "pois-api.proto", "podr2-api.proto", "pubkeys.proto"], &["proto"])
        .unwrap();
}
