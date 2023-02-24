# Cacher Module ( pallet-cacher )

Contain operations related cache miner. Cache miner retrieves the bills in the transaction records and provides file downloading service.

### Terminology

* **Bill:** Contains payment and download information.

## Interface

### Dispatchable Functions
* `register` - A cache miner joins the CDN.
* `update` - Update a cache miner information.
* `logout` - A cache miner exits the CDN.
* `pay` - A retrieval miner pays for downloads.

## Tests
```
cargo test --package pallet-cacher --features runtime-benchmarks
```

## Code Walkthrough
1. A cache miner calls `register` method to join the CDN. Cache miner information will be saved to the `Cachers` StorageMap.



1. Retrieval miners periodically query the `Cachers` StorageMap for all cache miner information.



1. A retrieval miner calls `pay` method to pay cache miners separately for downloading file fragments in batches.



1. A cache miner calls `update` method to change ip or unit price.



1. A cache miner calls `logout` method to exit the CDN.


