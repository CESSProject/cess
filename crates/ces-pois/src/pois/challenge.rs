use crate::expanders::{self, generate_idle_file::get_hash};

pub fn new_challenge_handle(
    miner_id: &[u8],
    tee_id: &[u8],
    chal: &[i64],
    front: i64,
    rear: i64,
    proof_num: i64,
) -> Option<impl FnMut(&[u8], i64, i64) -> bool> {
    let bytes_chal = expanders::get_bytes_slice(chal);
    let front_size = miner_id.len() + tee_id.len() + bytes_chal.len();
    let mut source = Vec::with_capacity(front_size + 64);
    source.extend_from_slice(miner_id);
    source.extend_from_slice(tee_id);
    source.extend_from_slice(&bytes_chal);

    let file_num: i64 = 256;
    let group_size: i64 = 16;

    let start = front / file_num;
    let mut count: i64 = 0;
    let total = (rear - front + front % file_num) / (file_num * group_size);

    if total > proof_num {
        return None;
    }

    Some(move |prior_hash: &[u8], left: i64, right: i64| -> bool {
        if !prior_hash.is_empty() {
            source.extend_from_slice(&prior_hash);
        }
        let hash = get_hash(&source);
        let v = expanders::bytes_to_node_value(&hash, group_size - 1) as i64;
        let mut l = (start + count * group_size + v) * file_num;
        let r = (l / file_num) * file_num + 256;
        if l < front {
            l = front;
        }
        count += 1;
        if l != left || r != right {
            println!("false");
            return false;
        }
        println!("true");
        true
    })
}
