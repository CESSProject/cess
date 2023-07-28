# Segment Book Module
This file is the exclusive pallet of cess and the proof of podr2 adaptation

## OverView

The job of this segment Book pallet is to process the proof of miner's service file and filling file,  and generate random challenges. Call some traits of Smith pallet to punish miners. Call the trail of file bank pallet to obtain random files or files with problems in handling challenges.

### Terminology

* **random_challenge:** The random time trigger initiates a challenge to the random documents.
The miners need to complete the challenge within a limited time and submit the certificates of
the corresponding documents.

* **deadline:** 	Expiration time of challenge, stored in challengeduration.
* **mu:**			Miner generated challenge related information.
* **sigma:**			Miner generated challenge related information.

### Interface

### Dispatchable Functions

* `submit_challange_prove`   Miner submits challenge certificate.
* `verify_proof`             Consensus submission verification challenge proof results.

### Scenarios

#### Punishment

When the verification result of the miner's certificate is false,or the miner fails to complete the challenge on time, the miner will be punished in both cases. Decide whether to reduce power or space according to the file type of punishment.

## Storage Mining
CESS supports to obtain incentives by contributing idle storage with [storage mining tool](https://github.com/CESSProject/storage-mining-tool), click [here](https://github.com/CESSProject/cess/tree/v0.1.1/docs/designs-of-storage-mining.md) to learn more.
