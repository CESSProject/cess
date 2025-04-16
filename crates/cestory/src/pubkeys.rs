use crate::{anyhow_to_status as a2s, try_account_from, CesealClient, ChainQueryHelper, RpcResult};
use cestory_api::pubkeys::{
    ceseal_pubkeys_provider_server::{
        CesealPubkeysProvider, CesealPubkeysProviderServer as CesealPubkeysProviderServerPb,
    },
    IdentityPubkeyResponse, MasterPubkeyResponse, Podr2PubkeyResponse, Request as InnerReq,
};
use tonic::{Request, Response, Status};

pub type CesealPubkeysProviderServer = CesealPubkeysProviderServerPb<CesealPubkeysProviderImpl>;

pub struct CesealPubkeysProviderImpl {
    ceseal_client: CesealClient,
    chainq_helper: ChainQueryHelper,
}

pub fn new_pubkeys_provider_server(
    ceseal_client: CesealClient,
    chainq_helper: ChainQueryHelper,
) -> CesealPubkeysProviderServer {
    CesealPubkeysProviderServerPb::new(CesealPubkeysProviderImpl { ceseal_client, chainq_helper })
}

impl CesealPubkeysProviderImpl {
    async fn is_storage_miner_registered_on_chain(&self, account_id: &[u8]) -> Result<(), Status> {
        let account_id = try_account_from(account_id).map_err(|_| Status::internal("invalid input account"))?;
        let registered = self
            .chainq_helper
            .is_storage_miner_registered_ignore_state(&account_id)
            .await
            .map_err(|e| Status::internal(format!("internal error: {}", e.to_string())))?;
        if !registered {
            return Err(Status::internal("the storage miner is not registered on the chain"));
        }
        Ok(())
    }
}

#[tonic::async_trait]
impl CesealPubkeysProvider for CesealPubkeysProviderImpl {
    async fn get_identity_pubkey(&self, request: Request<InnerReq>) -> RpcResult<IdentityPubkeyResponse> {
        let request = request.into_inner();
        self.is_storage_miner_registered_on_chain(&request.storage_miner_account_id[..])
            .await?;
        let now_ts = chrono::Utc::now().timestamp_millis();
        let (signature, pubkey) = self
            .ceseal_client
            .sign_use_identity_key(&now_ts.to_be_bytes())
            .await
            .map_err(a2s)?;
        Ok(Response::new(IdentityPubkeyResponse { pubkey, timestamp: now_ts, signature }))
    }

    async fn get_master_pubkey(&self, request: Request<InnerReq>) -> RpcResult<MasterPubkeyResponse> {
        let request = request.into_inner();
        self.is_storage_miner_registered_on_chain(&request.storage_miner_account_id[..])
            .await?;
        let now_ts = chrono::Utc::now().timestamp_millis();
        let (signature, pubkey) = self
            .ceseal_client
            .sign_use_master_sr25519_key(&now_ts.to_be_bytes())
            .await
            .map_err(a2s)?;
        Ok(Response::new(MasterPubkeyResponse { pubkey, timestamp: now_ts, signature }))
    }

    async fn get_podr2_pubkey(&self, request: Request<InnerReq>) -> RpcResult<Podr2PubkeyResponse> {
        let request = request.into_inner();
        self.is_storage_miner_registered_on_chain(&request.storage_miner_account_id[..])
            .await?;
        let now_ts = chrono::Utc::now().timestamp_millis();
        let (signature, pubkey) = self
            .ceseal_client
            .sign_use_master_rsa_key(&now_ts.to_be_bytes())
            .await
            .map_err(a2s)?;
        Ok(Response::new(Podr2PubkeyResponse { pubkey, timestamp: now_ts, signature }))
    }
}
