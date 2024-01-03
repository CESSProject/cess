extern crate alloc;

pub mod blocks;
pub mod crypto;
pub mod ecall_args;
pub mod endpoints;
pub mod crpc;
pub mod storage_sync;

mod proto_generated;

#[cfg(feature = "ceseal-client")]
pub mod ceseal_client {
    use crate::crpc::ceseal_api_client::CesealApiClient;
    pub type CesealClient<T> = CesealApiClient<T>;
}

pub mod pois {
    tonic::include_proto!("pois");
}

pub mod podr2 {
    tonic::include_proto!("podr2");
}