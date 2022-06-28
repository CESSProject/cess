# Sminer Module

Contain operations related storage miners.

### Terminology

* **Collateral:** The Staking amount when registering storage miner.
* **Earnings:** Store the storage miner's earnings during mining.
* **Locked:** Store the locked amount of the storage miner during mining.

## Interface

### Dispatchable Functions

* `regnstk` - Staking and register for storage miner.
* `redeem` - Redeem for storage miner.
* `claim` - Storage miner claim rewards from earnings.
* `initi` - Miner information initialization.
* `timing_storage_space` - A scheduled task for computing power trend data of the entire network.
* `timing_storage_space_thirty_days` - Generate power trend data for the first 30 days.
* `timed_increase_rewards` - Add reward orders.
* `timing_task_increase_power_rewards` - Added timed tasks for reward orders.
* `timed_user_receive_award1` - Users receive rewards for scheduled tasks.
* `faucet_top_up` - The faucet top up.
* `faucet` - Users receive money through the faucet.
* `increase_collateral` - Additional deposit for miners.
* `exit_miner` - Miner exit.
* `withdraw` - Miners redeem deposit.
