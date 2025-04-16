#[cfg(feature = "api-client")]
#[tokio::test]
async fn fetch_pubkeys() {
    use cestory_api::pubkeys::{ceseal_pubkeys_provider_client::CesealPubkeysProviderClient, Request as InnerReq};
    use sp_core::{crypto::AccountId32, ByteArray};
    use std::str::FromStr;
    use tonic::Request;

    let miner_id = AccountId32::from_str("cXjEPD6CAnjupMaRrxq9AEKCA3HRCumSxkSxWVKcL3pMeEyFi").unwrap();
    let mut client = CesealPubkeysProviderClient::connect("http://45.195.74.39:19999").await.unwrap();

    {
        let resp = client
            .get_identity_pubkey(Request::new(InnerReq { storage_miner_account_id: miner_id.to_raw_vec() }))
            .await
            .expect("identity key must be fetch");
        // assert!(resp);
        println!("identity pubkey response: {resp:?}");
    }

    {
        let resp = client
            .get_master_pubkey(Request::new(InnerReq { storage_miner_account_id: miner_id.to_raw_vec() }))
            .await
            .expect("master pubkey must be fetch");
        println!("master pubkey response: {resp:?}");
    }

    {
        let resp = client
            .get_podr2_pubkey(Request::new(InnerReq { storage_miner_account_id: miner_id.to_raw_vec() }))
            .await
            .expect("podr2 pubkey must be fetch");
        println!("podr2 pubkey response: {resp:?}");
    }
}
