use std::fs;

use crate::acc::{self, RsaKey};
use anyhow::Result;
use num_bigint_dig::BigUint;

pub fn copy_data(target: &mut [u8], src: &[&[u8]]) {
    let mut count = 0;
    let lens = target.len();

    for d in src {
        let l = d.len();
        if l == 0 || l + count > lens {
            continue;
        }
        target[count..count + l].copy_from_slice(d);
        count += l;
    }
}

pub fn parse_key(path: &str) -> Result<RsaKey> {
    let data = fs::read(path)?;
    Ok(get_key_from_bytes(&data))
}

fn get_key_from_bytes(data: &[u8]) -> RsaKey {
    if data.len() < 8 {
        return acc::rsa_keygen(2048);
    }
    let nl = u64::from_be_bytes(data[..8].try_into().unwrap());
    let gl = u64::from_be_bytes(data[8..16].try_into().unwrap());
    if nl == 0 || gl == 0 || data.len() - 16 != (nl + gl) as usize {
        return acc::rsa_keygen(2048);
    }
    let n = BigUint::from_bytes_be(&data[16..16 + nl as usize]);
    let g = BigUint::from_bytes_be(&data[16 + nl as usize..]);
    RsaKey::new(n, g)
}