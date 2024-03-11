use super::{
    bytes_to_node_value, generate_idle_file::get_hash, get_bytes_slice, Expanders, Node, NodeType,
};
use crate::{expanders::get_bytes, util::copy_data};

pub fn construct_stacked_expanders(k: i64, n: i64, d: i64) -> Expanders {
    Expanders::new(k, n, d)
}

pub fn calc_parents(expanders: &Expanders, node: &mut Node, miner_id: &[u8], count: &[i64]) {
    if node.parents.capacity() != (expanders.d + 1) as usize {
        return;
    }

    let layer = node.index as i64 / expanders.n;
    if layer == 0 {
        return;
    }

    let base_parent = ((layer - 1) * expanders.n) as NodeType;
    let lens = miner_id.len() + 8 * 17 + count.len() * 8;
    let mut content: Vec<u8> = vec![0; lens];
    copy_data(
        &mut content,
        &[miner_id, &get_bytes_slice(count), &layer.to_be_bytes()],
    );
    node.add_parent(node.index - expanders.n as NodeType);
    let mut plate = vec![vec![]; 16];

    for i in (0..expanders.d).step_by(16) {
        // Add index to plate
        for (j, v) in plate.iter_mut().enumerate().take(16) {
            *v = get_bytes(i + j as i64);
        }

        copy_data(
            &mut content[lens - 8 * 16..],
            &plate.iter().map(|v| &v[..]).collect::<Vec<&[u8]>>(),
        );
        let hash = get_hash(&content);
        let mut s = 0;
        let mut p = 0;

        let mut j = 0;
        while j < 16 {
            if s < 4 && j < 15 {
                p = bytes_to_node_value(&hash[j * 4 + s..(j + 1) * 4 + s], expanders.n);
                p = p % expanders.n as NodeType + base_parent;
            } else {
                s = 0;
                loop {
                    p = p + 1 % expanders.n as NodeType + base_parent;

                    if p <= node.index - expanders.n as NodeType {
                        let (_, ok) = node.parent_in_list(p + expanders.n as NodeType);
                        if !ok && p != node.index - expanders.n as NodeType {
                            break;
                        }
                    }

                    let (_, ok) = node.parent_in_list(p);
                    if !ok {
                        break;
                    }
                }
            }

            if p < node.index - expanders.n as NodeType {
                p += expanders.n as NodeType;
            }

            if node.add_parent(p) {
                j += 1;
                s = 0;
                continue;
            }

            s += 1;
        }
    }
}