use crate::expert::CesealExpertStub;
use cestory_api::pubkeys::{
    ceseal_pubkeys_provider_server::{
        CesealPubkeysProvider, CesealPubkeysProviderServer as CesealPubkeysProviderServerPb,
    },
    IdentityPubkeyResponse, MasterPubkeyResponse, Podr2PubkeyResponse, Request as InnerReq,
};
use sp_core::{crypto::AccountId32, ByteArray, Pair};
use std::result::Result as StdResult;
use tonic::{Request, Response, Status};

pub type Result<T> = StdResult<Response<T>, Status>;
pub type CesealPubkeysProviderServer = CesealPubkeysProviderServerPb<CesealPubkeysProviderImpl>;

pub struct CesealPubkeysProviderImpl {
    ceseal_expert: CesealExpertStub,
}

pub fn new_pubkeys_provider_server(ceseal_expert: CesealExpertStub) -> CesealPubkeysProviderServer {
    CesealPubkeysProviderServerPb::new(CesealPubkeysProviderImpl { ceseal_expert })
}

async fn is_storage_miner_registered_on_chain(
    ceseal_expert: &CesealExpertStub,
    account_id: &[u8],
) -> StdResult<(), Status> {
    let account_id = AccountId32::from_slice(account_id).map_err(|_| Status::internal("invalid input account"))?;
    let registered = ceseal_expert
        .using_chain_storage(move |opt| {
            if let Some(cs) = opt {
                cs.is_storage_miner_registered_ignore_state(account_id)
            } else {
                false
            }
        })
        .await
        .map_err(|e| Status::internal(format!("internal error: {}", e.to_string())))?;
    if !registered {
        return Err(Status::internal("the storage miner is not registered on the chain"))
    }
    Ok(())
}

#[tonic::async_trait]
impl CesealPubkeysProvider for CesealPubkeysProviderImpl {
    async fn get_identity_pubkey(&self, request: Request<InnerReq>) -> Result<IdentityPubkeyResponse> {
        let request = request.into_inner();
        is_storage_miner_registered_on_chain(&self.ceseal_expert, &request.storage_miner_account_id[..]).await?;
        let now_ts = chrono::Utc::now().timestamp_millis();
        let pubkey = self.ceseal_expert.identify_public_key();
        let sign = self.ceseal_expert.identity_key().sign(&now_ts.to_be_bytes());
        Ok(Response::new(IdentityPubkeyResponse {
            pubkey: pubkey.to_raw_vec(),
            timestamp: now_ts,
            signature: sign.0.to_vec(),
        }))
    }

    async fn get_master_pubkey(&self, request: Request<InnerReq>) -> Result<MasterPubkeyResponse> {
        let request = request.into_inner();
        is_storage_miner_registered_on_chain(&self.ceseal_expert, &request.storage_miner_account_id[..]).await?;
        let now_ts = chrono::Utc::now().timestamp_millis();
        let pubkey = self.ceseal_expert.master_key().public();
        let sign = self.ceseal_expert.master_key().sign(&now_ts.to_be_bytes());
        Ok(Response::new(MasterPubkeyResponse {
            pubkey: pubkey.to_raw_vec(),
            timestamp: now_ts,
            signature: sign.0.to_vec(),
        }))
    }

    async fn get_podr2_pubkey(&self, request: Request<InnerReq>) -> Result<Podr2PubkeyResponse> {
        use rsa::{pkcs1::EncodeRsaPublicKey, Pkcs1v15Sign};

        let request = request.into_inner();
        is_storage_miner_registered_on_chain(&self.ceseal_expert, &request.storage_miner_account_id[..]).await?;
        let now_ts = chrono::Utc::now().timestamp_millis();
        let pubkey = self.ceseal_expert.podr2_key().pkey.clone();
        let pubkey = pubkey
            .to_pkcs1_der()
            .map_err(|e| Status::internal(format!("PKCS#1-encoding Podr2 public key error: {:?}", e)))?
            .to_vec();
        let sign = self
            .ceseal_expert
            .podr2_key()
            .skey
            .sign(Pkcs1v15Sign::new_raw(), &now_ts.to_be_bytes())
            .map_err(|e| Status::internal(format!("Podr2 key sign error: {:?}", e)))?;
        Ok(Response::new(Podr2PubkeyResponse { pubkey, timestamp: now_ts, signature: sign }))
    }
}
