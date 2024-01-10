# Audit Module

Module for handling random challenges.

## OverView

The Audit module will regularly randomly select a miner to challenge the proof of idle data and service data, requiring the miner to submit proof within a specified time, otherwise he will be punished. At the same time, when the miner's challenge is passed, rewards will be issued to the miner.

## Terminology

* **total_prove_hash** - In order to ensure that the miner's proof when submitting and verifying are the same proof, the miner needs to first submit a hash value calculated based on all proof data.

* **snap_shot** - Save the miner's status and the network's spatial status when the challenge is triggered as a snapshot as the basis for miners' challenges and rewards.

* **service_bloom_filter** - In order to ensure that the service data calculated by the storage miner is not missing, a simple Bloom filter is recorded to check whether the data is consistent.

## Storage

* `VerifySlip` - The time node for verification and liquidation is stored, as well as the corresponding miner.

* `CountedServiceFailed` - Stores the number of consecutive failures in verification of the current miner's service proof. When the miner succeeds in the challenge, the record will be reset to 0.

* `CountedClear` - Stores the number of consecutive unsubmitted proofs for the miner's current challenge. When the miner submits the proof, the record will be reset to 0.

* `ChallengeSnapShot` - Stores snapshots of the current round of miners’ challenges, as well as relevant information generated during the miners’ challenge.

* `ChallengeSlip` - It records the deadline for miners to challenge and submit proofs.

## Hook

In the Audit pallet, Hooks are used for **challenge generation**, **challenge clearing**, and **verification clearing**.

### Challenge Generation

The triggering frequency of the challenge will be adjusted based on the number of miners on the entire network. The current frequency is that the challenge is triggered every 10 blocks. When triggered, a miner will be randomly selected as the challenge object. If the miner's computing power is 0 or is currently in the challenge, the challenge generation process will be ended directly. On the contrary, the status of the miner and the network will be recorded and saved as a snapshot, and the expiration time of this challenge will be calculated based on the size of the miner's space, and finally this challenge will be started.

### Challenge Clearing

In blocks, check whether there are expired challenges in the current block. Check the miner's submission of proof. If the proof of service is not submitted, the miner will be **clear punished**. There will be no penalty for idle proof.

### Verification Clearing

In order to prevent the randomly selected tee from the chain from being unable to connect or experiencing some abnormal conditions. During verification and liquidation, miners who have not completed verification will re-allocate a new tee to their proof for verification. A proof (idle/service) of one miner can be redistributed at most twice. After more than two attempts, the remaining data will be cleared directly to prepare for the next round of challenges.

## Extrinsic

**submit_idle_proof()**

An interface for miners to submit idle proofs. After submitting the idle proof, the value of the `idle_prove` field will change to `Some()`, and the current miner challenge status will be detected. If both proofs are submitted, the value of `CountedClear` will be reset. The chain will randomly assign a tee to be responsible for verification work.

**submit_service_proof()**

An interface for miners to submit proof of service. After submitting the service proof, the value of the `service_prove` field will change to `Some()`, and the current miner challenge status will be detected. If both proofs are submitted, the value of `CountedClear` will be reset. The chain will randomly assign a tee to be responsible for verification work.

**submit_verify_idle_result()**

The miner submits the verification result of the idle proof. The signature submitted by tee will be verified to prevent cheating by miners. At the same time, if the verification results of both proofs are passed, the miners will be **rewarded**. For details, please view the CESS Network’s [Reward Mechanism](https://docs.cess.cloud/core/storage-miner/reward).

**submit_verify_service_result()**

An interface for miners to submit proof of service verification results. To ensure that the miner is not cheating, the chain verifies the tee's signature and compares it to Bloom filters. When the verification result is false, the data of `CountedServiceFailed` will be updated and accumulated by 1. If the number of times >= 2, the miner will receive **service punish**. When it is true, it will detect whether both verification results are true, and if so, the miner will be rewarded.

## Technical Details

### Clear Punish

When a miner fails to submit a proof within the specified time, the penalty imposed is called **clear punishment**. The punishment is divided into two situations, one is for miners who have not received rewards, and the other is for miners who have received rewards. In the first case, 500 CESS will be deducted from the miner's deposit each time. If the miner is kicked out of the network three times in a row, the deposit will be immediately returned. In the second case, 5% of the miner's deposit requirement will be deducted each time, and the miner will be kicked out of the network for three consecutive times. The deposit will not be returned until the end of the staking period. Unreceived rewards will be returned immediately.

### Service Punish

When the service verification result submitted by the miner is false, the `CountedServiceFailed` value will be updated. When the number >= 2, **service punish** will be carried out on the miners. Miners are penalized 5% of their deposit requirements each time.