# Segemnt Book Module

Contain operations related proof of storage. 

### Terminology

* **uncid:** 		Necessary parameters for generating proof (unencrypted).
* **sealed_cid:** 	Necessary parameters for generating proof (encrypted).
* **segment_id:**	Allocated segment ID.
* **is_ready:**		Used to know whether to submit a certificate.
* **size_type:**	Segment size.
* **peer_id:**		Miner's ID.

## Interface

### Dispatchable Functions

* `intent_submit` 		Outputs parameters such as segment-id, random number and other parameters required by miners to generate the PoRep, and submit some proof parameters to the unverified pool in advance.
* `intent_submit_po_st` Outputs parameters such as random number and other parameters required by miners to generate the PoSt, and submit some proof parameters to the unverified pool in advance.
* `submit_to_vpa` 		Submits the PoRep to the unverified pool to verify for scheduler node.
* `verify_in_vpa` 		Verifies the PoRep from unverified pool, and submit the results.
* `submit_to_vpb` 		Submits the PoSt to the unverified pool to verify for scheduler node.
* `verify_in_vpb` 		Verifies the PoSt from unverified pool, and submit the results.
* `submit_to_vpc` 		Submits the PoRep to the unverified pool to verify for scheduler node.
* `verify_in_vpc` 		Verifies the PoRep from unverified pool, and submit the results.
* `submit_to_vpd` 		Submits the PoSt to the unverified pool to verify for scheduler node.
* `verify_in_vpd` 		Verifies the PoSt from unverified pool, and submit the results.

## Storage Mining
CESS supports to obtain incentives by contributing idle storage with [storage mining tool](https://github.com/CESSProject/storage-mining-tool), and click [here](https://github.com/CESSProject/cess/tree/v0.1.1/docs/designs-of-storage-mining.md) to learn more.
