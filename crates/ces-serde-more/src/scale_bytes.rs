use alloc::vec::Vec;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use parity_scale_codec::{Encode, Decode};

pub fn serialize<S: Serializer, T: Encode>(data: &T, ser: S) -> Result<S::Ok, S::Error> {
    data.encode().serialize(ser)
}

pub fn deserialize<'de, De: Deserializer<'de>, T: Decode>(der: De) -> Result<T, De::Error> {
    let bytes = <Vec<u8>>::deserialize(der)?;
    T::decode(&mut bytes.as_slice()).map_err(de::Error::custom)
}
