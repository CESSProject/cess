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

https://github.com/CESSProject/cess/blob/1acaa2de1a7dcf0c6ec676f1ffa4d605cf43c830/c-pallets/cacher/src/lib.rs#L109-L117

2. Retrieval miners periodically query the `Cachers` StorageMap for all cache miner information.

https://github.com/CESSProject/cess/blob/1acaa2de1a7dcf0c6ec676f1ffa4d605cf43c830/c-pallets/cacher/src/lib.rs#L95

3. A retrieval miner calls `pay` method to pay cache miners separately for downloading file fragments in batches.

https://github.com/CESSProject/cess/blob/1acaa2de1a7dcf0c6ec676f1ffa4d605cf43c830/c-pallets/cacher/src/lib.rs#L157-L172

4. A cache miner calls `update` method to change ip or unit price.

https://github.com/CESSProject/cess/blob/1acaa2de1a7dcf0c6ec676f1ffa4d605cf43c830/c-pallets/cacher/src/lib.rs#L124-L137

5. A cache miner calls `logout` method to exit the CDN.

https://github.com/CESSProject/cess/blob/1acaa2de1a7dcf0c6ec676f1ffa4d605cf43c830/c-pallets/cacher/src/lib.rs#L141-L150
