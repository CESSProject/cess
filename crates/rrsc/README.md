# RRSC (Random Rotational Selection Consensus)

## What is Consensus in Blockchain?

A consensus is a method used by nodes within a network to come into agreement. All nodes should come into agreement on a single state of the network at a given time. The nodes in a decentralized network stay synced with each other by adhering to the consensus mechanism. In the blockchain network, the consensus method helps generate new blocks and maintain the state of the network.

## Substrate Consensus

### Block Production: BABE

Blind Assignment for Blockchain Extension (BABE) is a block production mechanism that runs on validator nodes and determines the authors of new blocks.

Follow the following links to read more about BABE:

● https://wiki.polkadot.network/docs/learn-consensus#block-production-babe

● https://research.web3.foundation/en/latest/polkadot/block-production/Babe.html

BABE uses VRF for determining the next validator in every slot.

Slots are discrete units of time six seconds in length. Each slot can contain a block, but may not. Slots make up epochs - on Polkadot, 2400 slots make one epoch, which makes epochs four hours long.

In every slot, each validator "rolls a die". They execute a function (the VRF) that takes as input the following:

● The "secret key", a key specifically made for these die rolls.

● An epoch randomness value, which is the hash of VRF values from the blocks in the epoch before last (N-2), so past randomness affects the current pending randomness (N).

● The slot number.

# ![Figure 1](https://raw.githubusercontent.com/CESSProject/W3F-illustration/main/rrsc/image.png)

The output is two values: a RESULT (the random value) and a PROOF (a proof that the random value was generated correctly).

The RESULT is then compared to a threshold defined in the implementation of the protocol (specifically, in the Polkadot Host). If the value is less than the threshold, then the validator who rolled this number is a viable block production candidate for that slot. The validator then attempts to create a block and submits this block into the network along with the previously obtained PROOF and RESULT. Under VRF, every validator rolls a number for themselves, checks it against a threshold, and produces a block if the random roll is under that threshold.

The astute reader will notice that due to the way this works, some slots may have no validators as block producer candidates because all validator candidates rolled too high and missed the threshold. We clarify how we resolve this issue and make sure that Polkadot block times remain near constant-time in the wiki page on consensus.

## Random Rotational Selection(R²S)

### 1.1 Definitions

- **Slot**: Each slot will generate a new block, that is, the block out time. In the cess test network, 1 slot = 6 seconds.

- **Epoch**: A collection of fixed length slots. Each epoch will update the set of rotation nodes, but not every epoch will trigger the election. The election will be triggered in the first block from the sixth epoch of era. 1 epoch = 1 hour in the cess testnet.

- **Era**: A collection of multiple epochs. Era starts the new rotation node set in the first block, and settles the consensus award of the previous era. In the cess testnet, 1 era = 6 epoch, that is, 6 hours.

### 1.2 Overall process

R²S is an important part of CESS protocol. Compared with the polkadot consensus mechanism, R²S pays more attention to the process of node election and block generation. The following is the overall process:

1. The node has become a consensus node through pledge and registration, and the current pledge amount is 1 million.

2. In each round of era, the validators are rotated. The rotation rule is score ranking. The 11 nodes with the highest scores (4 in the cess testnet) are selected as the validators of era.

3. The final score is determined by reputation score and random score, that is, final score = `(reputation score * 80%) + (random score * 20%)`.

4. Please refer to the next section for credit score calculation, and the random score is determined by VRF.

5. The selected validators generate blocks in sequence.

6. The current way of confirming blocks is the same as GRANDPA.

7. The last epoch of each era starts the validators election of the next era.

<img src="https://raw.githubusercontent.com/CESSProject/W3F-illustration/main/rrsc/image1.png" width = "600" alt="name" align=center />

### 1.3 Reputation model

As each consensus node that joins the cess network needs to maintain the network state, it also needs to undertake the work of storage data scheduling. In order to encourage consensus nodes to do more such work, we designed a reputation model. In our model, each consensus node has a reputation score, which is directly determined by the workload of the scheduler. Specifically, it includes the following items:

1. Total bytes of idle segment processed.

2. Total of bytes of service segment processed.

3. Number of penalties for random challenge timeout of verification file.

Reputation value calculation as follow:

`Scheduler reputation value = 1000 * processing bytes ratio - (10 * penalty times)`

### 1.4 Code Walkthrough

#### 1. The election

final_score = `random_score` * 20% + `credit` * 80%

The final score of the node is composed of two parts, random score accounting for 20% of the weight and reputation score accounting for 80% of the weight. Arrange according to the scores from large to small, and select no more than 11 nodes with the highest scores as the rotation nodes.

https://github.com/CESSProject/substrate/blob/6f338348a5488f56fd338ab678d57e30f456e802/frame/rrsc/src/vrf_solver.rs#L46-L48

https://github.com/CESSProject/substrate/blob/6f338348a5488f56fd338ab678d57e30f456e802/frame/rrsc/src/vrf_solver.rs#L61-L62

#### 2. The random scores

- Use the `currentblockrandomness` random function to calculate a random hash value for each node.

https://github.com/CESSProject/substrate/blob/6f338348a5488f56fd338ab678d57e30f456e802/frame/rrsc/src/vrf_solver.rs#L87-L99

- Convert random hash value to U32 type value.

- Modulo the full score of reputation score with U32 type value to obtain random score.

- During the production of each block, the vrfoutput obtained by executing the VRF function is written into the block header.

https://github.com/CESSProject/substrate/blob/6f338348a5488f56fd338ab678d57e30f456e802/client/consensus/rrsc/src/authorship.rs#L185-L199

- When each block is initialized, vrfoutput is taken from the block header, converted into randomness, and stored in the authorvrrandomness of pallet rrsc as the seed for running the random function in the current block

https://github.com/CESSProject/substrate/blob/6f338348a5488f56fd338ab678d57e30f456e802/frame/rrsc/src/lib.rs#L749

- Randomness stored in authorvrrandomness is used as seed, and random hash value is obtained through byte array inversion, splicing and hash operation.

https://github.com/CESSProject/substrate/blob/6f338348a5488f56fd338ab678d57e30f456e802/frame/rrsc/src/randomness.rs#L136-L148

#### 3. Reputation scores

Obtain reputation scores of all candidate nodes during election

https://github.com/CESSProject/substrate/blob/6f338348a5488f56fd338ab678d57e30f456e802/frame/rrsc/src/vrf_solver.rs#L33

#### 4. Block generation

- Rotation nodes take turns to output blocks.

- Modulo the number of rotation nodes with the slot serial number, and take the modulo value as the node taken from the subscript of the rotation node list, which is the block out node of this slot. Slot numbers are cumulative, so the out of block nodes take turns.

https://github.com/CESSProject/substrate/blob/6f338348a5488f56fd338ab678d57e30f456e802/client/consensus/rrsc/src/authorship.rs#L180-L181

### 1.5 Future work

Consensus is the foundation of the blockchain network, and it is necessary for us to continuously optimize it. At this stage, we have completed the realization of the core functions. But this is far from enough, and there are still many points that need to be improved in the future.

1. Adjust consensus-related RPC interfaces to adapt to applications such as block explorers.

2. According to the actual test situation, the reputation model is continuously iterated to make it more perfect.

3. A more concrete theoretical security analysis will be conducted.
