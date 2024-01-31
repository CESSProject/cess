use cestory_api::podr2::{podr2_api_client::Podr2ApiClient, EchoMessage};
use tonic::Request;

#[tokio::test]
async fn test_echo() {
    let mut client = Podr2ApiClient::connect("http://127.0.0.1:19999").await.unwrap();
    let resp = client
        .echo(Request::new(EchoMessage { echo_msg: vec![60, 61, 62] }))
        .await
        .unwrap();
    // assert!(resp);
    println!("response: {:?}", resp);
}
