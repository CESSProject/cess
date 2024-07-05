# Storage Handler Module

Maintain various types of storage space data in the CESS network

## Overview

This module records the usage of each space in the CESS network, such as the storage space purchased by users, the total idle space provided by storage nodes, and the current service space for stored data in the entire network. And provide some related interfaces for users to purchase storage space.

## Terminology

* **Territory:** Users can mint an unlimited number of territories, and each territory will be bound to a unique token. Territories can be used for trading, transfer, or file storage. Users can also customize the names of their territories, but they cannot be repeated.
* **PayOrder:** Users can purchase storage space for others. In the process, a payment order will be created waiting for payment.
* **IdleSpace:** The idle space provided by the storage user is filled with generated random data, waiting for the data uploaded by the storage user.
* **ServiceSpace:** The space used for data uploaded by users is called service space.

## Storage

* `Territory` - It is used to store detailed information about the territories owned by the user, and to specify the query territory through the user address and the user-defined territory name.

* `TerritoryKey` - Ensure unique record of territory token.

* `Consignment` - Stores the details of the current consignment order, using the territory token as the primary key.

* `TerritoryFrozen` - It is used to store some data related to executing territory freezing. The primary key block height represents the block height of the execution task, and the stored territory id represents the territory that needs to be frozen.

* `TerritoryFrozenCounter` - This storage is a counter set up to prevent too many freezing tasks from being executed within a block height.

* `TerritoryExpired` - This storage is similar to `TerritoryFrozen` and is used to store records related to executing territory expiration tasks.

* `UnitPrice` - Current unit price of storage space (Gib/days).

* `TotalIdleSpace` - The total idle space provided by the storage nodes in the entire network.

* `TotalService` - The space currently used by service data stored in storage nodes across the entire network.

* `PurchasedSpace` - The space currently purchased by users across the entire network.

* `PayOrder` - Used to store payment orders for purchasing space for other users.

## Extrinsic

**mint_territory(origin: OriginFor<T>, gib_count: u32, territory_name: TerrName):**  

Used for transactions where users mint territories. `gib_count` indicates the size of the territory the user wants to mint, in gib. The default expiration time for a territory is 30 days. Through the `territory_name` field, users can customize the name of the territory they mint, but the name of a territory owned by a user cannot be repeated.

**expanding_territory(origin: OriginFor<T>, territory_name: TerrName, gib_count: u32):**

Transactions for expanding the user's territory can be called when the user expands the territory. The payment amount is determined by the remaining expiration time and the size of the territory the user needs to expand. Use the territory name to specify which territory you want to expand. (Less than one day will be counted as one day)

**renewal_territory(origin: OriginFor<T>, days: u32, territory_name: TerrName):**

Used for user renewal of territory transactions. After the user has minted a territory, this transaction can be called to extend the expiration date of a certain territory. `days` indicates the number of days for renewal. The fee paid depends on the renewal time and the space size of the current territory. Use the territory name to specify which territory.

**reactivate_territory(origin: OriginFor<T>, territory_name: TerrName, days: u32)**

When a user's territory expires, this transaction can be called to reactivate the territory. The territory status must be Expired to be reactivated. If it is in Frozen status, the territory renewal transaction should be called. 

**territory_consignment(origin: OriginFor<T>, territory_name: TerrName, price: BalanceOf<T>)**

Through this transaction, users can consign their own territories, customize the price and then display it to other users. The consigned territory needs to be empty, and the status of the territory needs to be Active, and the remaining expiration time is greater than one day.

**buy_consignment(origin: OriginFor<T>, token: TokenId, rename: TerrName)**

Users can purchase the territories on consignment through transactions. After calling this transaction, the corresponding consignment order will be locked. During the lock-in period, users can cancel the purchase. When the lock-in period ends, the subsequent transaction content will be automatically executed.

**exec_consignment(origin: OriginFor<T>, token: TokenId, territory_name: TerrName)**

The method for executing a consignment transaction. This transaction cannot be actively called by the user. It is a transaction automatically called by the system after the consignment transaction lock-up period ends.

**cancel_consignment(origin: OriginFor<T>, territory_name: TerrName)**

This transaction is used to remove the consignments that the current user is currently consigning, but the consignment needs to be unlocked.

**cancel_purchase_action(origin: OriginFor<T>, token: TokenId)**

After a buyer locks a commission, if he wants to cancel the purchase during the lock-in period, he can call this transaction. The locked cess will then be returned to the buyer's wallet free balance.

**territory_grants(origin: OriginFor<T>, territory_name: TerrName, receiver: AccountOf<T>)**

The purpose of this transaction is that a user can transfer his territory to another user, but the premise is that there are no files stored in this territory and the status is Active.

**territory_rename(origin: OriginFor<T>, old_name: TerrName, new_name: TerrName)**

It is used by users to rename a piece of their territory, and it is required that no files are stored in the territory.

**update_price(origin: OriginFor<T>):**

The interface during testing can only be called by the root user. Used to change the space unit price.

**update_user_life(origin: OriginFor<T>, user: AccountOf<T>, deadline: BlockNumberFor<T>):**

The interface during testing can only be called by the root user. Used to change the expiration time of the space purchased by the user.

**create_order(origin: OriginFor<T>, target_acc: AccountOf<T>, territory_name: TerrName, order_type: OrderType, gib_count: u32, days: u32):**

Create a transaction for a payment order. Create a payment order to purchase territory for `target_acc`. Specify the space size and time of the territory to be paid for by `gib_count` and `days`. `order_type` specifies whether the order type is purchase, expansion or renewal.

**exec_order(origin: OriginFor<T>, order_id: BoundedVec<u8, ConstU32<32>>):**

Transaction used to execute payment orders. The signing user will pay for the order with id `order_id`.

## Interface

### StorageHandle

Provide methods for updating the status of various types of spaces for other pallets, including methods for maintaining the status of spaces purchased by users.

#### Function

* `check_territry_owner` - Check if the user is the owner of the corresponding territory.

* `add_territory_used_space` - Increase the use of territory.

* `sub_territory_used_space` - Reduce the use of territory.

* `add_total_idle_space` - Increase total idle space.

* `sub_total_idle_space` - Reduce total idle space.

* `add_total_service_space` - Increase total service space.

* `sub_total_service_space` - Reduce total service space.

* `get_total_idle_space` - Get the current total idle space.

* `get_total_service_space` - Get the current total service space.

* `get_avail_space` - Get the space that can be purchased on the current chain.

* `lock_user_space` - Locking a user's space in a certain territory.

* `unlock_user_space` - Unlock space for a certain territory for users.

* `unlock_and_used_user_space` - Unlock space in a territory for the user and then increase the used space.

* `get_user_avail_space` - Get the available space of a user's territory.

* `frozen_task` - Method used to perform the task of polling for territory expiration.