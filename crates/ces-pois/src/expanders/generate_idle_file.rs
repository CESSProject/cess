use sha2::{Digest, Sha256, Sha512};

pub const HASH_SIZE: i32 = 64;

pub enum Hasher {
    SHA256(Sha256),
    SHA512(Sha512),
}

pub fn new_hash() -> Hasher {
    match HASH_SIZE {
        32 => Hasher::SHA256(Sha256::new()),
        64 => Hasher::SHA512(Sha512::new()),
        _ => Hasher::SHA512(Sha512::new()),
    }
}

pub fn get_hash(data: &[u8]) -> Vec<u8> {
    let hash = new_hash();
    let mut data = data;
    if data.is_empty() {
        data = b"none";
    }

    match hash {
        Hasher::SHA256(hash) => {
            let mut hash = hash;
            hash.update(data);
            let result = hash.finalize();
            result.to_vec()
        }
        Hasher::SHA512(hash) => {
            let mut hash = hash;
            hash.update(data);
            let result = hash.finalize();
            result.to_vec()
        }
    }
}
