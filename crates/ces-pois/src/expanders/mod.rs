pub mod generate_expanders;
pub mod generate_idle_file;
use std::mem;

use num_bigint_dig::BigInt;
use num_traits::{Signed, ToPrimitive};

pub use generate_idle_file::new_hash;
pub type NodeType = i32;

#[derive(Clone, Debug)]
pub struct Expanders {
    pub k: i64,
    pub n: i64,
    pub d: i64,
    pub size: i64,
    pub hash_size: i64,
}

pub struct Node {
    pub index: NodeType,
    pub parents: Vec<NodeType>,
}

impl Expanders {
    pub fn new(k: i64, n: i64, d: i64) -> Self {
        Expanders {
            k,
            n,
            d,
            size: (k + 1) * n,
            hash_size: 64,
        }
    }
}

impl Node {
    pub fn new(idx: NodeType) -> Self {
        Self {
            index: idx,
            parents: Vec::new(),
        }
    }

    pub fn add_parent(&mut self, parent: NodeType) -> bool {
        if self.index == parent {
            return false;
        }
        if self.parents.is_empty() || self.parents.len() >= self.parents.capacity() {
            return false;
        }

        let (i, ok) = self.parent_in_list(parent);
        if ok {
            return false;
        }
        self.parents.push(0);
        let lens = self.parents.len();
        if lens == 1 || i == lens as i32 - 1 {
            self.parents[i as usize] = parent;
            return true;
        }
        self.parents
            .copy_within(i as usize + 1..lens - 1, i as usize);
        self.parents[i as usize] = parent;

        true
    }

    pub fn no_parents(&self) -> bool {
        self.parents.is_empty()
    }

    pub fn parent_in_list(&self, parent: NodeType) -> (i32, bool) {
        if self.no_parents() {
            return (0, false);
        }
        let lens = self.parents.len();
        let (mut l, mut r) = (0, lens - 1);
        while l <= r {
            let mid = (l + r) / 2;
            if self.parents[mid] == parent {
                return (0, true);
            }
            if self.parents[mid] > parent {
                r = mid - 1;
            } else {
                l = mid + 1;
            }
        }
        let mut i = (l + r) / 2;
        if self.parents[i] < parent {
            i += 1;
        }
        (i as i32, false)
    }
}

pub fn get_bytes<T: ToPrimitive>(v: T) -> Vec<u8> {
    let size = mem::size_of::<T>();
    let value = v.to_i64().unwrap();
    let mut bytes = vec![0; size];

    for i in 0..size {
        bytes[size - 1 - i] = ((value >> (8 * i)) & 0xFF) as u8;
    }

    bytes
}

pub fn get_bytes_slice<T: ToPrimitive>(values: &[T]) -> Vec<u8> {
    let size = mem::size_of::<T>();
    let mut bytes = Vec::with_capacity(size * values.len());

    for v in values {
        let value = v.to_i64().unwrap();
        for i in 0..size {
            bytes.push(((value >> (8 * (size - 1 - i))) & 0xFF) as u8);
        }
    }

    bytes
}

pub fn bytes_to_node_value(data: &[u8], max: i64) -> NodeType {
    let value = BigInt::from_bytes_be(num_bigint_dig::Sign::Plus, data);
    let big_max = BigInt::from(max);

    let value = value % &big_max;
    let i_value = value.to_i64().unwrap_or_else(|| {
        if value.is_negative() {
            i64::min_value()
        } else {
            i64::max_value()
        }
    });

    ((i_value + max) % max) as NodeType
}