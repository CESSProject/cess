use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use anyhow::Result;
use log::info;

use crate::crpc::{
    client::{Error as ClientError, RequestClient},
    ceseal_api_client::CesealApiClient,
    server::ProtoError as ServerError,
    Message,
};

pub type CesealClient = CesealApiClient<RpcRequest>;

pub fn new_ceseal_client(base_url: String) -> CesealApiClient<RpcRequest> {
    CesealApiClient::new(RpcRequest::new(base_url))
}

pub fn new_ceseal_client_no_log(base_url: String) -> CesealApiClient<RpcRequest> {
    CesealApiClient::new(RpcRequest::new(base_url).disable_log())
}

pub struct RpcRequest {
    base_url: String,
    disable_log: bool,
}

impl RpcRequest {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            disable_log: false,
        }
    }

    pub fn disable_log(mut self) -> Self {
        self.disable_log = true;
        self
    }
}

#[async_trait::async_trait]
impl RequestClient for RpcRequest {
    async fn request(&self, path: &str, body: Vec<u8>) -> Result<Vec<u8>, ClientError> {
        fn from_display(err: impl core::fmt::Display) -> ClientError {
            ClientError::RpcError(err.to_string())
        }

        let url = alloc::format!("{}/crpc/{path}", self.base_url);
        let res = reqwest::Client::new()
            .post(url)
            .header("Connection", "close")
            .body(body)
            .send()
            .await
            .map_err(from_display)?;

        if !self.disable_log {
            info!("{path}: {}", res.status());
        }
        let status = res.status();
        let body = res.bytes().await.map_err(from_display)?;
        if status.is_success() {
            Ok(body.as_ref().to_vec())
        } else {
            let err: ServerError = Message::decode(body.as_ref())?;
            Err(ClientError::ServerError(err))
        }
    }
}
