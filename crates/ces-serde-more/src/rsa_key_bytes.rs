use alloc::{vec::Vec, string::ToString};
use serde::{de, Deserialize, Deserializer, Serializer, Serialize};
use rsa::pkcs8::{EncodePrivateKey, DecodePrivateKey};

pub fn serialize<S: Serializer>(data: &rsa::RsaPrivateKey, ser: S) -> Result<S::Ok, S::Error> {
    let bytes = data.to_pkcs8_der().unwrap().as_bytes().to_vec();
    bytes.serialize(ser)
}

pub fn deserialize<'de, De: Deserializer<'de>>(der: De) -> Result<rsa::RsaPrivateKey, De::Error> {
    let bytes: Vec<u8> = Deserialize::deserialize(der)?;
    let skey = rsa::RsaPrivateKey::from_pkcs8_der(&bytes).map_err(|e|de::Error::custom(e.to_string()))?;
    Ok(skey)
}
