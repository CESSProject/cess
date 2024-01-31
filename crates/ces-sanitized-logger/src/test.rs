use super::*;

#[test]
fn whitelist_works() {
    let allowed: Vec<_> = include_str!("all-log-targets.txt")
        .split('\n')
        .filter(|t| target_allowed(t))
        .collect();
    assert_eq!(
        allowed,
        [
            "cestory",
            "cestory::benchmark",
            "cestory::light_validation",
            "cestory::light_validation::justification::communication",
            "cestory::crpc_service",
            "cestory::storage::storage_ext",
            "cestory::system",
            "cestory::system::gk",
            "cestory::system::master_key",
            "cestory_api::storage_sync",
            "ces_mq",
            "cess_node_runtime",
            "crpc_measuring",
            "ceseal",
            "ceseal::api_server",
            "ceseal::ias",
            "ceseal::pal_gramine",
            "ceseal::runtime",
            "rocket::launch",
            "rocket::launch_",
            "rocket::server",
        ]
    );
}

#[test]
fn see_log() {
    use log::info;

    init_env_logger(true);
    info!(target: "cestory", "target cestory");
    info!(target: "other", "target other");
}
