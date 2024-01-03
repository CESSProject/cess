use sha2::{Digest, Sha256, Sha512};

#[derive(Debug)]
pub struct PathProof {
    pub locs: Vec<u8>,
    pub path: Vec<Vec<u8>>,
}

pub const DEFAULT_HASH_SIZE: u32 = 32;

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
        }
        Hasher::SHA512(hash) => {
            let mut hash = hash;
            hash.update(data);

            let result = hash.finalize();
            result.to_vec()
        }
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
            }
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
            }
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