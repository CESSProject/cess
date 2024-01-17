# Storage Handler Module

Maintain various types of storage space data in the CESS network

## Overview

This module records the usage of each space in the CESS network, such as the storage space purchased by users, the total idle space provided by storage nodes, and the current service space for stored data in the entire network. And provide some related interfaces for users to purchase storage space.

## Terminology

* **PayOrder:** Users can purchase storage space for others. In the process, a payment order will be created waiting for payment.
* **IdleSpace:** The idle space provided by the storage user is filled with generated random data, waiting for the data uploaded by the storage user.
* **ServiceSpace:** The space used for data uploaded by users is called service space.

## Storage

* `UserOwnedSpace` - Used to store the details of users after purchasing space, recording expiration time, used space, total space and other related data.

* `UnitPrice` - Current unit price of storage space (Gib/days).

* `TotalIdleSpace` - The total idle space provided by the storage nodes in the entire network.

* `TotalService` - The space currently used by service data stored in storage nodes across the entire network.

* `PurchasedSpace` - The space currently purchased by users across the entire network.

* `PayOrder` - Used to store payment orders for purchasing space for other users.

## Extrinsic

**buy_space(origin: OriginFor<T>, gib_count: u32):**  

Used for users to purchase space transactions, `gib_count` represents the size of the space the user wants to purchase, in gib. Users who have purchased space and the space has not expired cannot invoke this transaction. The default expiration time of space is 30 days.

**expansion_space(origin: OriginFor<T>, gib_count: u32):**

Transactions used to expand space for users can be called when the user has purchased space, and the amount paid is determined by the remaining expiration time and the amount of space the user needs to expand.(Any less than one day will be counted as one day.)

**renewal_space(origin: OriginFor<T>, days: u32):**

Used for user renewal space transactions. After the user purchases space, this transaction can be called to extend the expiration time of the space. `days` represents the number of days to extend. The fee paid is based on the lease renewal time and the space currently held.

**update_price(origin: OriginFor<T>):**

The interface during testing can only be called by the root user. Used to change the space unit price.

**update_user_life(origin: OriginFor<T>, user: AccountOf<T>, deadline: BlockNumberFor<T>):**

The interface during testing can only be called by the root user. Used to change the expiration time of the space purchased by the user.

**create_order(origin: OriginFor<T>,target_acc: AccountOf<T>,order_type: OrderType,gib_count: u32, days: u32):**

Transaction that creates a payment order. Create a payment order to purchase space for `target_acc`. Specify the size and time of the payment space through `gib_count` and `days`. `order_type` specifies whether the order type is purchase, expansion, or lease renewal.

**exec_order(origin: OriginFor<T>, order_id: BoundedVec<u8, ConstU32<32>>):**

Transaction used to execute payment orders. The signing user will pay for the order with id `order_id`.

## Interface

### StorageHandle

Provide methods for updating the status of various types of spaces for other pallets, including methods for maintaining the status of spaces purchased by users.

#### Function

* `update_user_space` - Update the space used by the user.  

* `add_total_idle_space` - Increase total idle space.

* `sub_total_idle_space` - Reduce total idle space.

* `add_total_service_space` - Increase total service space.

* `sub_total_service_space` - Reduce total service space.

* `get_total_idle_space` - Get total idle space.

* `get_total_service_space` - Get total service space.

* `add_purchased_space` - Increase purchased space.

* `sub_purchased_space` - Reduce purchased space.

* `get_total_space` - Get the total space on the current chain, that is, idle space + service space.

* `lock_user_space` - Lock user space.

* `unlock_and_used_user_space` - Update the user's locked space to used space.

* `get_user_avail_space` - Get the user's available space.

* `frozen_task` - Perform freeze detection and freeze the user's expired space.

* `delete_user_space_storage` - Clear space held by user.