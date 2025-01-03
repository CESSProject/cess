pub mod tx;

use subxt::dynamic::{self, Value};

pub fn storage_key(pallet: &str, entry: &str) -> Vec<u8> {
    dynamic::storage(pallet, entry, Vec::<Value>::new()).to_root_bytes()
}
