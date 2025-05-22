pub mod file_manager;
pub mod hash_2_prime;
pub mod multi_level_acc;

use byteorder::{BigEndian, ByteOrder};
use std::sync::Arc;
use tokio::task;

use multi_level_acc::AccNode;
use num_bigint_dig::{BigUint, RandBigInt};
use num_integer::Integer;
use num_traits::One;
use rsa::{PublicKeyParts, RsaPrivateKey};
use tokio::sync::RwLock;

use hash_2_prime::h_prime;

#[derive(Clone, Default, Debug)]
pub struct RsaKey {
    pub n: BigUint, //Z/nZ
    pub g: BigUint, // Generator
}

impl RsaKey {
    pub(crate) fn new(n: BigUint, g: BigUint) -> RsaKey {
        Self { n, g }
    }
}

// Generate N and G
// lamda is the bit size of N(preferably 2048 bit)
// Note that the primes factors of N are not exposed for security reason
pub fn rsa_keygen(lambda: usize) -> RsaKey {
    let mut rng = rand::thread_rng();
    let pk = RsaPrivateKey::new(&mut rng, lambda).expect("Failed to generate RSA key");

    let n = pk.n();
    let mut f: BigUint;
    loop {
        f = rng.gen_biguint(lambda);
        if f.gcd(n) == BigUint::one() {
            break;
        }
    }
    let g = f.modpow(&BigUint::from(2u32), &n.clone());

    RsaKey { n: n.clone(), g }
}

pub fn get_key_from_bytes(data: Vec<u8>) -> RsaKey {
    if data.len() < 16 {
        return rsa_keygen(2048);
    }
    let nl = BigEndian::read_u64(&data[0..8]);
    let gl = BigEndian::read_u64(&data[8..16]);

    if nl == 0 || gl == 0 || data.len() - 16 != (nl + gl) as usize {
        return rsa_keygen(2048);
    }

    let n_bytes = &data[16..16 + nl as usize];
    let g_bytes = &data[16 + nl as usize..];

    let n = BigUint::from_bytes_be(n_bytes);
    let g = BigUint::from_bytes_be(g_bytes);

    RsaKey { n, g }
}

pub fn generate_acc(key: &RsaKey, acc: &[u8], elems: Vec<Vec<u8>>) -> Option<Vec<u8>> {
    if acc.is_empty() {
        return None;
    }

    let mut g = BigUint::from_bytes_be(acc);
    for elem in elems {
        let prime = h_prime(&BigUint::from_bytes_be(&elem));
        g = g.modpow(&prime, &key.n);
    }

    Some(g.to_bytes_be())
}

async fn generate_witness(g: BigUint, n: BigUint, us: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
    if us.is_empty() {
        return vec![];
    }
    if us.len() == 1 {
        return vec![g.to_bytes_be()];
    }
    let (left, right) = us.split_at(us.len() / 2);

    let left_cloned = left.to_vec();
    let right_cloned = right.to_vec();

    let g1 = Arc::new(tokio::sync::RwLock::new(g.clone()));
    let g2 = Arc::new(tokio::sync::RwLock::new(g.clone()));

    let g1_clone = Arc::clone(&g1);
    let g2_clone = Arc::clone(&g2);

    let n_clone = n.clone();
    let right_task = task::spawn(async move {
        for u in right_cloned {
            let e = h_prime(&BigUint::from_bytes_be(&u));
            let mut g1_locked = g1_clone.write().await;
            *g1_locked = g1_locked.modpow(&e, &n_clone);
        }
    });

    let n_clone = n.clone();
    let left_task = task::spawn(async move {
        for u in left_cloned {
            let e = h_prime(&BigUint::from_bytes_be(&u));
            let mut g2_locked = g2_clone.write().await;
            *g2_locked = g2_locked.modpow(&e, &n_clone);
        }
    });

    // Wait for both tasks to complete
    let _ = tokio::try_join!(right_task, left_task);

    let u1 = Box::pin(generate_witness(g1.read().await.clone(), n.clone(), left.to_vec())).await;
    let u2 = Box::pin(generate_witness(g2.read().await.clone(), n.clone(), right.to_vec())).await;

    let mut result = u1;
    result.extend(u2);
    result
}

pub async fn gen_wits_for_acc_nodes(g: &BigUint, n: &BigUint, elems: &mut Vec<Arc<RwLock<AccNode>>>) {
    let lens = elems.len();
    if lens == 0 {
        return;
    }
    if lens == 1 {
        elems[0].write().await.wit = g.to_bytes_be();
        return;
    }
    let left = &mut elems[0..lens / 2].to_vec();
    let right = &mut elems[lens / 2..].to_vec();
    let g1 = g;
    let g2 = g;

    //todo:use tokio::spawn to speed up
    for u in right.clone() {
        let e = h_prime(&BigUint::from_bytes_be(&u.read().await.value));
        g1.modpow(&e, &n);
    }

    for u in left.clone() {
        let e = h_prime(&BigUint::from_bytes_be(&u.read().await.value));
        g2.modpow(&e, &n);
    }
    Box::pin(gen_wits_for_acc_nodes(g1, n, left)).await;
    Box::pin(gen_wits_for_acc_nodes(g2, n, right)).await;
}
// pub async fn gen_wits_for_acc_nodes(g: &BigUint, n: &BigUint, elems: &mut Vec<Arc<RwLock<AccNode>>>) {
//     let lens = elems.len();
//     if lens == 0 {
//         return;
//     }
//     if lens == 1 {
//         elems[0].write().await.wit = g.to_bytes_be();
//         return;
//     }

//     let (left, right) = elems.split_at_mut(lens / 2);

//     let g1 = g.clone(); // Clone the g value for use in the right part
//     let g2 = g.clone(); // Clone the g value for use in the left part

//     let n_clone = n.clone();
//     // Spawn tasks to process the right and left parts concurrently
//     let right_task = task::spawn(async move {
//         for u in right {
//             let e = h_prime(&BigUint::from_bytes_be(&u.read().await.value));
//             let mut g1_locked = g1.clone(); // Clone the value of g1 for use in the operation
//             g1_locked.modpow(&e, &n_clone); // Perform the modpow operation for the right part
//         }
//     });

//     let n_clone = n.clone();
//     let left_task = task::spawn(async move {
//         for u in left {
//             let e = h_prime(&BigUint::from_bytes_be(&u.read().await.value));
//             let mut g2_locked = g2.clone(); // Clone the value of g2 for use in the operation
//             g2_locked.modpow(&e, &n_clone); // Perform the modpow operation for the left part
//         }
//     });

//     // Wait for both tasks to complete
//     let _ = tokio::try_join!(right_task, left_task);

//     // Recursively generate witnesses for the left and right parts
//     gen_wits_for_acc_nodes(&g1, n, &mut left.to_vec()).await;
//     gen_wits_for_acc_nodes(&g2, n, &mut right.to_vec()).await;
// }
