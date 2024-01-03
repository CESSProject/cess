use num_bigint_dig::BigUint;
use rand::Rng;
use serde::{Deserialize, Serialize};

use super::{generate_acc, hash_2_prime::h_prime, RsaKey};

const DEFAULT_LEVEL: i32 = 3;
const DEFAULT_ELEMS_NUM: i32 = 256;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct WitnessNode {
    pub elem: Vec<u8>,
    pub wit: Vec<u8>,
    pub acc: Option<Box<WitnessNode>>,
}

pub fn verify_insert_update(
    key: RsaKey,
    exist: Option<Box<WitnessNode>>,
    elems: Vec<Vec<u8>>,
    accs: Vec<Vec<u8>>,
    acc: Vec<u8>,
) -> bool {
    if exist.is_none() || elems.is_empty() || accs.len() < DEFAULT_LEVEL as usize {
        println!("acc wit chain is empty");
        return false;
    }

    let mut p = exist.clone().unwrap().as_ref().clone();
    while p.acc.is_some() && p.acc.as_ref().unwrap().elem == p.wit {
        p = p.acc.unwrap().as_ref().clone();
    }

    // Proof of the witness of accumulator elements,
    // when the element's accumulator does not exist, recursively verify its parent accumulator
    if !verify_mutilevel_acc(&key, Some(&mut p.clone()), &acc) {
        println!("verify muti-level acc error");
        return false;
    }

    // Verify that the newly generated accumulators after inserting elements
    // is calculated based on the original accumulators
    let sub_acc = generate_acc(&key, &exist.as_ref().unwrap().elem, elems);
    if !sub_acc.eq(&Some(accs[0].clone())) {
        println!("Verify that the newly generated accumulators after inserting elements is calculated based on the original accumulators error");
        return false;
    }

    let mut count = 1;
    let mut p = *exist.unwrap();
    let mut sub_acc;

    while p.acc.is_some() {
        sub_acc = generate_acc(&key, &p.wit, vec![accs[count - 1].clone()]);

        if !sub_acc.eq(&Some(accs[count].to_vec())) {
            println!("verify sub acc error");
            return false;
        }
        p = *p.acc.unwrap();
        count += 1;
    }

    true
}

fn verify_acc(key: &RsaKey, acc: &[u8], u: &[u8], wit: &[u8]) -> bool {
    let e = h_prime(&BigUint::from_bytes_be(u));
    let dash = BigUint::from_bytes_be(wit).modpow(&e, &key.n);
    dash == BigUint::from_bytes_be(acc)
}

pub fn verify_mutilevel_acc(key: &RsaKey, wits: Option<&mut WitnessNode>, acc: &[u8]) -> bool {
    let mut current_wit = wits.unwrap();
    while let Some(acc_node) = &mut current_wit.acc {
        if !verify_acc(key, &acc_node.elem, &current_wit.elem, &current_wit.wit) {
            return false;
        }
        current_wit = acc_node;
    }
    current_wit.elem.eq(acc)
}

pub fn verify_mutilevel_acc_for_batch(key: &RsaKey, base_idx: i64, wits: Vec<WitnessNode>, acc: &[u8]) -> bool {
    let mut sub_acc: Option<Vec<u8>> = None;
    let default_elems_num = DEFAULT_ELEMS_NUM as i64;
    for (i, witness) in wits.iter().enumerate() {
        if let Some(sa) = &sub_acc {
            if witness.acc.clone().unwrap().elem != *sa {
                return false;
            }
        }

        if (i as i64 + base_idx) % default_elems_num == 0 || i == wits.len() - 1 {
            if !verify_mutilevel_acc(key, Some(&mut witness.clone()), acc) {
                return false;
            }
            sub_acc = None;
            continue;
        }

        let mut rng = rand::thread_rng();
        if rng.gen_range(0..100) < 25
            && !verify_acc(key, &witness.acc.clone().unwrap().elem, &witness.elem, &witness.wit)
        {
            return false;
        }

        sub_acc = Some(witness.acc.clone().unwrap().elem.clone());
    }
    true
}

pub fn verify_delete_update(
    key: RsaKey,
    exist: &mut WitnessNode,
    elems: Vec<Vec<u8>>,
    accs: Vec<Vec<u8>>,
    acc: &[u8],
) -> bool {
    if elems.is_empty() || accs.len() < DEFAULT_LEVEL as usize {
        return false;
    }
    if !verify_mutilevel_acc(&key, Some(exist), acc) {
        return false;
    }

    let mut sub_acc = generate_acc(&key, &accs[0], elems);
    if sub_acc.eq(&Some(exist.elem.clone())) {
        return false;
    }
    let mut p = exist;
    let mut count = 1;
    while p.acc.is_some() {
        if accs[count - 1].eq(&key.g.to_bytes_be()) {
            sub_acc = generate_acc(&key, &p.wit, vec![accs[count - 1].clone()]);
        } else {
            sub_acc = Some(p.wit.clone());
        }
        if !sub_acc.eq(&Some(accs[count].to_vec())) {
            return false;
        }
        p = p.acc.as_mut().unwrap();
        count += 1;
    }

    true
}
