use sha2::{Digest, Sha512};

use super::{Expanders, Node, NodeType};

pub fn construct_stacked_expanders(k: i64, n: i64, d: i64) -> Expanders {
    Expanders::new(k, n, d)
}

pub fn calc_parents(expanders: &Expanders, node: &mut Node, miner_id: &[u8], count: i64, rlayer: i64) {
    if node.parents.capacity() != (expanders.d + 1) as usize {
        return;
    }

    let layer = node.index as i64 / expanders.n;
    if layer == 0 {
        return;
    }

    let group_size = expanders.n / expanders.d;
    let offset = group_size / 256;

    let mut hasher = Sha512::new();
    hasher.update(miner_id);
    hasher.update(&count.to_be_bytes());
    hasher.update(&rlayer.to_be_bytes());
    hasher.update((node.index as i64).to_be_bytes());

    let mut res = hasher.finalize().to_vec();

    if expanders.d > 64 {
        let mut hasher2 = Sha512::new();
        hasher2.update(&res);
        let extra = hasher2.finalize().to_vec();
        res.extend_from_slice(&extra);
    }

    res.truncate(expanders.d as usize);

    let parent = (node.index - expanders.n as i32) as NodeType;
    node.add_parent(parent);

    for (i, &byte_val) in res.iter().enumerate() {
        let byte_i64 = byte_val as i64;
        let calc_index = (layer - 1) * expanders.n + i as i64 * group_size + byte_i64 * offset + (byte_i64 % offset);

        let index = calc_index as NodeType;

        if index == parent {
            node.add_parent(index + 1);
        } else if index < parent {
            node.add_parent(index + expanders.n as i32);
        } else {
            node.add_parent(index);
        }
    }
}
