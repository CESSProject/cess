use ic_verify_bls_signature::*;
use rand::Rng;

fn test_bls_signature(
    expected_result: bool,
    sig: &'static str,
    msg: &'static str,
    key: &'static str,
) {
    let sig = hex::decode(sig).expect("Invalid hex");
    let msg = hex::decode(msg).expect("Invalid hex");
    let key = hex::decode(key).expect("Invalid hex");

    let result = verify_bls_signature(&sig, &msg, &key);

    assert_eq!(expected_result, result.is_ok());
}

#[test]
fn verify_valid() {
    // derived from agent-rs tests
    test_bls_signature(
        true,
        "ace9fcdd9bc977e05d6328f889dc4e7c99114c737a494653cb27a1f55c06f4555e0f160980af5ead098acc195010b2f7",
        "0d69632d73746174652d726f6f74e6c01e909b4923345ce5970962bcfe3004bfd8474a21dae28f50692502f46d90",
        "814c0e6ec71fab583b08bd81373c255c3c371b2e84863c98a4f1e08b74235d14fb5d9c0cd546d9685f913a0c0b2cc5341583bf4b4392e467db96d65b9bb4cb717112f8472e0d5a4d14505ffd7484b01291091c5f87b98883463f98091a0baaae");

    test_bls_signature(
        true,
        "89a2be21b5fa8ac9fab1527e041327ce899d7da971436a1f2165393947b4d942365bfe5488710e61a619ba48388a21b1",
        "0d69632d73746174652d726f6f74b294b418b11ebe5dd7dd1dcb099e4e0372b9a42aef7a7a37fb4f25667d705ea9",
        "9933e1f89e8a3c4d7fdcccdbd518089e2bd4d8180a261f18d9c247a52768ebce98dc7328a39814a8f911086a1dd50cbe015e2a53b7bf78b55288893daa15c346640e8831d72a12bdedd979d28470c34823b8d1c3f4795d9c3984a247132e94fe");
}

#[test]
fn reject_invalid() {
    // derived from agent-rs tests
    test_bls_signature(
        false,
        "89a2be21b5fa8ac9fab1527e041327ce899d7da971436a1f2165393947b4d942365bfe5488710e61a619ba48388a21b1",
        "0d69632d73746174652d726f6f74e6c01e909b4923345ce5970962bcfe3004bfd8474a21dae28f50692502f46d90",
        "814c0e6ec71fab583b08bd81373c255c3c371b2e84863c98a4f1e08b74235d14fb5d9c0cd546d9685f913a0c0b2cc5341583bf4b4392e467db96d65b9bb4cb717112f8472e0d5a4d14505ffd7484b01291091c5f87b98883463f98091a0baaae");

    test_bls_signature(
        false,
        "ace9fcdd9bc977e05d6328f889dc4e7c99114c737a494653cb27a1f55c06f4555e0f160980af5ead098acc195010b2f7",
        "0d69632d73746174652d726f6f74b294b418b11ebe5dd7dd1dcb099e4e0372b9a42aef7a7a37fb4f25667d705ea9",
        "9933e1f89e8a3c4d7fdcccdbd518089e2bd4d8180a261f18d9c247a52768ebce98dc7328a39814a8f911086a1dd50cbe015e2a53b7bf78b55288893daa15c346640e8831d72a12bdedd979d28470c34823b8d1c3f4795d9c3984a247132e94fe");
}

#[test]
fn reject_invalid_sig() {
    // sig is not a valid point
    test_bls_signature(
        false,
        "ace9fcdd9bc977e05d6328f889dc4e7c99114c737a494653cb27a1f55c06f4555e0f160980af5ead098acc195010b2f8",
        "0d69632d73746174652d726f6f74e6c01e909b4923345ce5970962bcfe3004bfd8474a21dae28f50692502f46d90",
        "814c0e6ec71fab583b08bd81373c255c3c371b2e84863c98a4f1e08b74235d14fb5d9c0cd546d9685f913a0c0b2cc5341583bf4b4392e467db96d65b9bb4cb717112f8472e0d5a4d14505ffd7484b01291091c5f87b98883463f98091a0baaae");
}

#[test]
fn reject_invalid_key() {
    // key is not a valid point
    test_bls_signature(
        false,
        "ace9fcdd9bc977e05d6328f889dc4e7c99114c737a494653cb27a1f55c06f4555e0f160980af5ead098acc195010b2f7",
        "0d69632d73746174652d726f6f74e6c01e909b4923345ce5970962bcfe3004bfd8474a21dae28f50692502f46d90",
        "814c0e6ec71fab583b08bd81373c255c3c371b2e84863c98a4f1e08b74235d14fb5d9c0cd546d9685f913a0c0b2cc5341583bf4b4392e467db96d65b9bb4cb717112f8472e0d5a4d14505ffd7484b01291091c5f87b98883463f98091a0baaad");
}

#[test]
fn accepts_generated_signatures() {
    let mut rng = rand::thread_rng();

    for _trial in 0..30 {
        let sk = PrivateKey::random();
        let pk = sk.public_key();
        let msg = rng.gen::<[u8; 24]>();
        let sig = sk.sign(&msg);
        assert!(verify_bls_signature(&sig.serialize(), &msg, &pk.serialize()).is_ok());

        assert_eq!(sig, Signature::deserialize(&sig.serialize()).unwrap());
        assert_eq!(sk, PrivateKey::deserialize(&sk.serialize()).unwrap());
        assert_eq!(pk, PublicKey::deserialize(&pk.serialize()).unwrap());
    }
}

#[test]
fn accepts_known_good_signature() {
    // Generated using the threshold signature implementation in IC repo

    let public_key = hex::decode("87033f48fd8f327ff5d164e85af31433c6a8c73fc5a65bad5d472127205c73c5168a45e862f5af6d0da5676df45d0a5f1293a530d5498f812a34a280f6bef869e4ca9b7c275554456d8770733d72ac4006777382fa541873fe002adb12184268").unwrap();
    let message = hex::decode("e751fdb69185002b13c8d2954c7d0c39546402ecdde9c2a9a2c624293535a5ca2f560a582f705580448fbe1ccdc0e86af3ba4c487a7f73bc9c312556").unwrap();
    let signature = hex::decode("98733cc2b312d5787cd4dba6ea0e19a1f1850b9e8c6d5112f12e12db8e7413a4ecb4096c23730566c67d9b2694e4e179").unwrap();

    assert!(verify_bls_signature(&signature, &message, &public_key).is_ok());
}

#[test]
fn generates_expected_signature() {
    // Generated using the threshold signature implementation in IC repo

    let secret_key =
        hex::decode("6f3977f6051e184b2c412daa1b5c0115ef7ab347cac8d808ffa2c26bd0658243").unwrap();
    let message = hex::decode("50484522ad8aede64ec7f86b9273b7ed3940481acf93cdd40a2b77f2be2734a14012b2492b6363b12adaeaf055c573e4611b085d2e0fe2153d72453a95eaebf350ac3ba6a26ba0bc79f4c0bf5664dfdf5865f69f7fc6b58ba7d068e8").unwrap();
    let expected_signature = "8f7ad830632657f7b3eae17fd4c3d9ff5c13365eea8d33fd0a1a6d8fbebc5152e066bb0ad61ab64e8a8541c8e3f96de9";

    let sk = PrivateKey::deserialize(&secret_key).unwrap();
    let generated_sig = sk.sign(&message);

    assert_eq!(hex::encode(generated_sig.serialize()), expected_signature);
}
