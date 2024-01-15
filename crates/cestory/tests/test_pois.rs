use cestory_api::pois::{pois_certifier_api_client::PoisCertifierApiClient, RequestMinerInitParam};
use sp_core::{crypto::AccountId32, ByteArray};
use std::str::FromStr;
use tonic::Request;

#[tokio::test]
async fn get_pois_key_with_no_exist_miner() {
    let miner_id = AccountId32::from_str("cXjEPD6CAnjupMaRrxq9AEKCA3HRCumSxkSxWVKcL3pMeEyFi").unwrap();
    let mut client = PoisCertifierApiClient::connect("http://127.0.0.1:19999").await.unwrap();
    let result = client
        .request_miner_get_new_key(Request::new(RequestMinerInitParam { miner_id: miner_id.to_raw_vec() }))
        .await;
    // println!("response: {:?}", result);
    assert!(result.is_err());
    if let Err(e) = result {
        assert_eq!("the miner not exists", e.message());
    } else {
        panic!("use a exist miner account id")
    }
}
