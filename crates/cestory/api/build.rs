fn main() {
    let mut builder = tonic_build::configure()
        .out_dir("./src/proto_generated")
        .disable_package_emission()
        .build_server(true)
        .build_client(false);

    #[cfg(feature = "ceseal-client")]
    {
        builder = builder.build_client(true);
    }

    builder = builder.type_attribute(
        ".ceseal_rpc",
        "#[derive(::serde::Serialize, ::serde::Deserialize)]",
    );
    for name in [
        "AttestationReport",
        "InitRuntimeResponse",
        "Attestation",
        "NetworkConfig",
    ] {
        builder = builder.type_attribute(name, "#[derive(::scale_info::TypeInfo)]");
    }
    builder = builder.field_attribute("InitRuntimeResponse.attestation", "#[serde(skip, default)]");
    for field in [
        "HttpFetch.body",
        "HttpFetch.headers",
    ] {
        builder = builder.field_attribute(field, "#[serde(default)]");
    }
    builder
        .compile(&["ceseal_rpc.proto"], &["proto"])
        .unwrap();

    tonic_build::compile_protos("proto/pois-api.proto").unwrap();
    tonic_build::compile_protos("proto/podr2-api.proto").unwrap();
}
