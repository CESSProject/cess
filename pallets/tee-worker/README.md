# Tee Worker Module

Manage meta information of tee-worker.

## Terminology

* **EndPoint:** The ip+port of tee-worker, or the domain name of tee-worker. Selected independently by tee-workers.
* **StashAccount:** Staking account when bond a consensus node.
* **ControllerAccount:** The control account bound when registering the consensus node, used for work transaction signatures.
* **Podr2Pbk:** The only public key shared between tee-workers in the entire network.
* **SgxAttestationReport:** The SGX report certificate applied for tee-worker certification is provided by Inter.

## Extrinsic
* `register()` - The extrinsic used for tee-worker registration requires bond to become a consensus before registration. The report will be verified in Extrinsic to confirm that the registrant is a legitimate sgx.
* `update_whitelist()` - Used to support iterative updates of sgx. When the internal code of sgx is updated, the new identification code needs to be added to the whitelist.
* `exit()` - Tee worker exit function, when the last tee worker exits, the only public key in the entire network will be cleared. However, this function will not affect the unbundling of consensus stashes.
* `update_podr2_pk()` - Method to update root permissions of the only public key in the entire network.
*  `force_register()` - Forced registration of tee workers through root privileges will ignore a series of qualification certifications such as verification reports. After mainnet login, this method will be removed

## Interface

### TeeWorkerHandler

A series of methods for finding consensus scheduling.
 * `contains_scheduler` - Judge whether the controller account exists.
 * `get_controller_list` - Get the list of controller accounts of currently registered tee workers.
 * `get_first_controller` - Get the first consensus in the list.
 * `get_master_publickey` - Get the network-wide unique public key of the tee worker to verify the signature of sgx.
 * `punish_scheduler` - Punish tee workers and deduct credibility points.

#### Usage
in pallet::Config
```rust
pub trait Config:
		frame_system::Config + sp_std::fmt::Debug
    {
        //...
        type TeeWorkerHandler: TeeWorkerHandler<Self::AccountId, BlockNumberFor<Self>>;
        //...
    }
```
in runtime.rs
```rust
impl pallet_audit::Config for Runtime {
    //...
    type TeeWorkerHandler = TeeWorker;
    //...
}
```

## Implementation Details

### Verify SGX report
Use the `verify_miner_cert` method to verify sgx reports. The method uses the third-party library webpki internally to verify the sgx report. At the same time, it will also check whether the custom information of tee worker in the report is legal.

Custom information includes the following:
* `sign` - the certificate signature.
* `cert_der` - certificate.
* `report_json_raw` - the json format string of the report.
* `identity_hash` - some basic registration information of tee worker, which is composed of peer_id, podr2_pbk, end_point, and finally the hash value calculated by sha2_256 algorithm.

