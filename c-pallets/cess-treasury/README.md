# Cess Treasury Module

Manage meta information of cess-treasury.

## Overview

There are three accounts in the CESS treasury that store funds for different purposes. At the same time, in order to adjust the inflation rate of network tokens, this module provides corresponding methods for destroying funds. Users with root authority can control the funds of two of the accounts, and the reward pool of the storage node does not have any authority to operate.

## Terminology

* **PunishTreasuryId:** Used to collect tokens that have been punished by storage nodes. The tokens in their accounts can be controlled by root privileges.

* **SpaceTreasuryId:** Used to collect tokens spent by users to purchase space. This account can control funds with root authority.

* **MinerRewardId:** Collect tokens used to reward storage nodes. Each era is issued tokens to the account by the `staking pallet`. Root privileges do not have the right to control the account's token.

## Extrinsic

* `send_funds_to_pid()` - Can be called with any permissions. Send any number of tokens to **PunishTreasuryId** account.
* `send_funds_to_sid()` - Can be called with any permissions. Send any number of tokens to **SpaceTreasuryId** account.
* `pid_burn_funds()` - Can only be called with root privileges. Destroy any number of tokens in the **PunishTreasuryId** account.
* `sid_burn_funds()` - Can only be called with root privileges. Destroy any number of tokens in the **SpaceTreasuryId** account.
* `pid_send_funds()` Can only be called with root privileges. **PunishTreasuryId** account transfers any number of tokens to the designated account.
* `sid_send_funds()` Can only be called with root privileges. **SpaceTreasuryId** account transfers any number of tokens to the designated account.

## Interface

### RewardPool

The interface used to operate the reward account and perform addition, modification and check on the funds in the reward account. However, it cannot directly manipulate the balance and can only record changes to `CurrencyReward`.

#### Function

* `get_reward()` - Get the current reward amount.
* `get_reward_128()` - Get the current u128 type reward amount.
* `add_reward()` - Increase reward amount. It should be noted that the total amount of rewards must remain unchanged and can only be used in some special circumstances and cannot be directly increased. Otherwise, the amount of bonus pool rewards and the balance in the account will not correspond.
* `sub_reward()` - Reduce reward amount.

#### Usage

in pallet::Config

```rust
pub trait Config: frame_system::Config + sp_std::fmt::Debug {
    // ...
    type RewardPool: RewardPool<AccountOf<Self>, BalanceOf<Self>>;
    // ...
}
```

in runtime.rs 
```rust
impl pallet_sminer::Config for Runtime {
    // ...
    type RewardPool = CessTreasury;
    // ...
}
```

### TreasuryHandle

Provides an interface for collecting tokens, through which other pallets transfer tokens to designated accounts.

#### Function

* `send_to_pid()` - Send any number of tokens to **PunishTreasuryId**.
* `send_to_sid()` - Send any number of tokens to **SpaceTreasuryId**.

#### Usage

in pallet::Config

```rust
pub trait Config: frame_system::Config + sp_std::fmt::Debug {
    // ...
    type CessTreasuryHandle: TreasuryHandle<AccountOf<Self>, BalanceOf<Self>>;
    // ...
}
```

in runtime.rs 
```rust
impl pallet_sminer::Config for Runtime {
    // ...
    type CessTreasuryHandle = CessTreasury;
    // ...
}
```

## Implementation Details


### Miner Reward

The total issuance of storage node rewards in the first year is 477,000,000 CESS, with an average of 326,488 CESS per era (One era every six hours). 
Starting in the second year, the annual reward decreases linearly with an annual decay rate of approximately 0.841, halving every 4 years. For details, please view the CESS Network’s [Reward Mechanism](https://docs.cess.cloud/core/storage-miner/reward).

* **MinerRewardId:** The wallet account of the miner reward pool, in which balance is the real bonus. 
* **CurrencyReward:** It is a `StorageValue` type storage pool. The current reward records how many available rewards are available in the bonus account. Change it by issuing rewards each time, or recycling rewards.
