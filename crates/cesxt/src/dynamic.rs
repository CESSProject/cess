pub mod tx;

pub fn storage_key(pallet: &str, entry: &str) -> Vec<u8> {
    ::subxt::dynamic::storage(pallet, entry, Vec::<()>::new()).to_root_bytes()
}
