use std::vec;

use anyhow::Result;
use sha2::{Digest, Sha256, Sha512};

pub type LightMHT = Vec<u8>;

#[derive(Debug, Default)]
pub struct PathProof {
    pub locs: Vec<u8>,
    pub path: Vec<Vec<u8>>,
}

pub const DEFAULT_HASH_SIZE: u32 = 32;

pub fn get_light_mht(e_len: i64) -> LightMHT {
    vec![0u8; (e_len as u32 * DEFAULT_HASH_SIZE) as usize]
}

// CalcLightMhtWithBytes calc light weight mht whit fixed size elements data
pub fn calc_light_mht_with_bytes(mht: &mut LightMHT, data: &[u8], size: i64) {
    let mut hasher = Sha256::new();
    for i in 0..(data.len() as i64 / size) as usize {
        hasher.update(&data[i * size as usize..(i + 1) * size as usize]);
        mht[i * DEFAULT_HASH_SIZE as usize..(i + 1) * DEFAULT_HASH_SIZE as usize]
            .copy_from_slice(&hasher.finalize_reset());
    }
    calc_light_mht(mht);
}

pub fn calc_light_mht_with_aux(mht: &mut LightMHT, aux: &[u8]) {
    mht.copy_from_slice(aux);
    calc_light_mht(mht);
}
pub fn calc_light_mht(mht: &mut LightMHT) {
    let lens = mht.len() as i64;
    let mut p = lens / 2;
    let mut src = mht.clone();
    let size = DEFAULT_HASH_SIZE as i64;

    for i in 0..((lens as f64) / (size as f64)).log2() as i64 + 1 {
        let num = lens / (1 << (i + 1));
        let target = &mut mht[p as usize..p as usize + num as usize];

        let mut j = (num / size) - 1;
        let mut k = num * 2 / size - 2;
        while j >= 0 && k >= 0 {
            let mut hash = Sha256::new();
            hash.update(&src[(k * size) as usize..((k + 2) * size) as usize]);

            target[(j * size) as usize..((j + 1) * size) as usize].copy_from_slice(&hash.finalize());
            j = j - 1;
            k = k - 2;
        }

        p /= 2;
        src.truncate(target.len());
        src.as_mut_slice().clone_from_slice(&target);
    }
}

pub fn get_root(mht: &LightMHT) -> Vec<u8> {
    if mht.len() < (DEFAULT_HASH_SIZE * 2) as usize {
        return Vec::new();
    }
    let mut root = vec![0u8; DEFAULT_HASH_SIZE as usize];
    root.copy_from_slice(&mht[DEFAULT_HASH_SIZE as usize..(DEFAULT_HASH_SIZE * 2) as usize]);

    root
}

pub fn get_path_proof(mht: &LightMHT, data: &[u8], index: i64, size: i64, hashed: bool) -> Result<PathProof> {
    let mut size = size;
    let mut index = index;
    let deep = f64::log2(data.len() as f64 / size as f64) as i64;
    let mut proof = PathProof { locs: vec![0u8; deep as usize], path: vec![Vec::new(); deep as usize] };
    let mut num = mht.len();
    let mut p = mht.len();
    let mut data = data.to_vec().clone();

    for i in 0..deep {
        let (d, loc) = if (index + 1) % 2 == 0 {
            (data[((index - 1) * size) as usize..(index * size) as usize].to_vec(), 0)
        } else {
            (data[((index + 1) * size) as usize..((index + 2) * size) as usize].to_vec(), 1)
        };
        if i == 0 && (size != DEFAULT_HASH_SIZE as i64 || !hashed) {
            let mut hasher = Sha256::new();
            hasher.update(&d);
            proof.path[i as usize] = hasher.finalize().to_vec();
            size = DEFAULT_HASH_SIZE as i64;
        } else {
            proof.path[i as usize] = vec![0u8; size as usize];
            proof.path[i as usize].copy_from_slice(&d);
        }
        proof.locs[i as usize] = loc;
        num = num / 2;
        index = index / 2;
        p -= num;
        data = mht[p..p + num].to_vec();
    }
    Ok(proof)
}

pub fn get_path_proof_with_aux(data: &Vec<u8>, aux: &mut Vec<u8>, index: usize, size: usize) -> Result<PathProof> {
    let mut proof = PathProof::default();
    let aux_size = aux.len() / (DEFAULT_HASH_SIZE as usize);
    let plate_size = data.len() / size / aux_size;
    let mut mht = vec![0u8; plate_size * DEFAULT_HASH_SIZE as usize];
    let left = index / plate_size;
    let data = &mut data[left * plate_size * size..(left + 1) * plate_size * size].to_vec().clone();

    for i in 0..plate_size {
        let mut hasher = Sha256::new();
        hasher.update(&data[i * size..(i + 1) * size]);
        mht[i * DEFAULT_HASH_SIZE as usize..(i + 1) * DEFAULT_HASH_SIZE as usize]
            .copy_from_slice(hasher.finalize().as_slice());
    }

    calc_light_mht(&mut mht);
    let mut sub_proof = get_path_proof(&mht, data, (index % plate_size) as i64, size as i64, false)?;

    mht = vec![0u8; aux.len()];
    mht.copy_from_slice(&aux);
    calc_light_mht(&mut mht);
    let top_proof = get_path_proof(&mht, aux, left as i64, DEFAULT_HASH_SIZE as i64, true)?;

    sub_proof.locs.extend_from_slice(&top_proof.locs);
    sub_proof.path.extend_from_slice(&top_proof.path);
    proof.locs = sub_proof.locs;
    proof.path = sub_proof.path;

    Ok(proof)
}

pub fn verify_path_proof(root: &[u8], data: &[u8], proof: PathProof) -> bool {
    if proof.locs.len() != proof.path.len() {
        return false;
    }

    let hash = Hasher::SHA256(Sha256::new());
    let mut data = match hash {
        // TODO: write a generic function for the below task.
        Hasher::SHA256(hash) => {
            let mut hash = hash;
            hash.update(data);

            let result = hash.finalize();
            result.to_vec()
        },
        Hasher::SHA512(hash) => {
            let mut hash = hash;
            hash.update(data);

            let result = hash.finalize();
            result.to_vec()
        },
    };

    if data.len() != root.len() {
        return false;
    }
    for i in 0..proof.path.len() {
        let hash = Hasher::SHA256(Sha256::new());
        data = match hash {
            // TODO: write a generic function for the below task.
            Hasher::SHA256(hash) => {
                let mut hash = hash;

                if proof.locs[i] == 0 {
                    let mut proof_path_local = proof.path[i].to_owned();
                    proof_path_local.extend_from_slice(&data);
                    hash.update(proof_path_local);
                } else {
                    let mut proof_path_local = Vec::new();
                    proof_path_local.extend(data);
                    proof_path_local.extend(&proof.path[i]);
                    hash.update(proof_path_local);
                }
                let result = hash.finalize();
                result.to_vec()
            },
            Hasher::SHA512(hash) => {
                let mut hash = hash;
                if proof.locs[i] == 0 {
                    let mut proof_path_local = proof.path[i].to_owned();
                    proof_path_local.extend_from_slice(&data);
                    hash.update(proof_path_local);
                } else {
                    let mut proof_path_local = Vec::new();
                    proof_path_local.extend(data);
                    proof_path_local.extend(&proof.path[i]);
                    hash.update(proof_path_local);
                }
                let result = hash.finalize();
                result.to_vec()
            },
        };
    }
    root.eq(&data)
}

pub fn check_index_path(index: i64, locs: &[u8]) -> bool {
    let mut index = index;
    for v in locs {
        if (index + 1) % 2 == 0 {
            if *v != 0 {
                return false;
            }
        } else if *v != 1 {
            return false;
        }
        index /= 2;
    }
    true
}

pub enum Hasher {
    SHA256(Sha256),
    SHA512(Sha512),
}
