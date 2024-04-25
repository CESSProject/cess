use sha2::{Digest, Sha512};

use super::{Expanders, Node, NodeType};

pub fn construct_stacked_expanders(k: i64, n: i64, d: i64) -> Expanders {
    Expanders::new(k, n, d)
}

pub fn calc_parents(
    expanders: &Expanders,
    node: &mut Node,
    miner_id: &[u8],
    count: i64,
    rlayer: i64,
) {
    if node.parents.capacity() != (expanders.d + 1) as usize {
        return;
    }

    let layer = node.index as i64 / expanders.n;
    if layer == 0 {
        return;
    }

    let group_size = expanders.n / expanders.d;
    let offset = group_size / 256;

    let mut hash = Sha512::new();
    hash.update(miner_id);
    hash.update(count.to_be_bytes());
    hash.update(rlayer.to_be_bytes()); // add real layer
    hash.update((node.index as i64).to_be_bytes());
    let res = hash.clone().finalize();
    let mut res = res.to_vec();
    if expanders.d > 64 {
        hash.reset();
        hash.update(res.clone());
        let result = hash.finalize();
        res.append(&mut result.to_vec());
    }
    res = res[..expanders.d as usize].to_vec();

    let parent = node.index - expanders.n as NodeType;
    node.add_parent(parent);
    for i in 0..res.len() as i64 {
        let index = (layer - 1) * expanders.n
            + i * group_size
            + res[i as usize] as i64 * offset
            + res[i as usize] as i64 % offset;
        match index {
            i if i == parent as i64 => {
                node.add_parent((i + 1) as i32);
            }
            i if i < parent as i64 => {
                node.add_parent((i + expanders.n) as i32);
            }
            _ => {
                node.add_parent(index as i32);
            }
        }
    }
}