fn main() {
    use tera::{Context, Tera};

    let tera = Tera::new("proto/*.proto").unwrap();

    let tmpdir = tempdir::TempDir::new("rendered_proto").unwrap();
    let render_dir = tmpdir.path();

    for tmpl in tera.templates.keys() {
        println!("cargo:rerun-if-changed=proto/{tmpl}");
        let render_output = std::fs::File::create(render_dir.join(tmpl)).unwrap();
        tera.render_to(tmpl, &Context::new(), render_output)
            .unwrap();
    }

    let out_dir = "./src/proto_generated";

    let mut builder = crpc_build::configure()
        .out_dir(out_dir)
        .mod_prefix("crate::crpc::")
        .disable_package_emission();
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
    builder = builder.field_attribute("InitRuntimeResponse.attestation", "#[serde(skip,default)]");
    for field in [
        "HttpFetch.body",
        "HttpFetch.headers",
    ] {
        builder = builder.field_attribute(field, "#[serde(default)]");
    }
    builder
        .compile(&["ceseal_rpc.proto"], &[render_dir])
        .unwrap();
}
