# Sminer Module

Contain operations related storage miners.

### Terminology

* **Collateral:** The Staking amount when registering storage miner.
* **Earnings:** Store the storage miner's earnings during mining.
* **Locked:** Store the locked amount of the storage miner during mining.

## Interface

### Dispatchable Functions

* `regnstk` - Staking and register for storage miner.
* `increase_collateral` - Additional pledge method for miners.
* `update_beneficiary` -Miner replacement income account.
* `update_ip` - Miner changes IP endpoint address.
* `timing_storage_space` - A scheduled task for computing power trend data of the entire network.
* `timing_storage_space_thirty_days` - Generate power trend data for the first 30 days.
* `timed_increase_rewards` - Add reward orders.
* `timing_task_increase_power_rewards` - Added timed tasks for reward orders.
* `timed_user_receive_award1` - Users receive rewards for scheduled tasks.
* `faucet_top_up` - Obtain transaction token from faucet.
* `faucet` - Users receive money through the faucet.
* `increase_collateral` - Increase additional deposit for miners.
* `exit_miner` - Storage Miner exit system.
* `withdraw` - Miners redeem deposit.
