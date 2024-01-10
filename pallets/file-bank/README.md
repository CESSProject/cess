# File Bank Module

FileBank is a module that manages file data. Provide users with an interface to upload files and delete files. And there are also functions related to file recovery. In this module, the meta-information of user files across the entire network is also recorded.

## Terminology

* `tee_sig` - The signature performed by the tee worker will not be verified by the chain, but the information will be stored for verification in the subsequent work of the tee.

* `tee_sig_need_verify` - A signature performed by a tee worker. The chain verifies the signature to ensure that the work was performed within the tee environment.

* `tag` - Files need to be sliced redundant. The redundant slices are called `fragment`, and for each fragment, the storage node will calculate a tag for storage proof.

## Storage

* `DealMap` - Stores file metainformation during the upload process and stores it temporarily.

* `File` - Stores the meta information of the file, the minimum unit is `fragment`.

* `UserHoldFileList` - Records all files owned by each user.

* `Bucket` - Information about the bucket container.

* `UserBucketList` - Stores a list of all buckets created by each user.

* `RestoralOrder` - The recovery order of the file is stored as a public announcement for all miners on the network to claim and recover it.

* `ClearUserList` - This is an auxiliary storage that divides the work into multiple blocks to prevent too much work in one block. Record which user's files should be cleaned next.

## Extrinsic

**upload_declaration():**

This is the file upload statement interface. Before the file is uploaded, you need to declare it to inform the file meta -information structure corresponding to the chain. The chain will lock the user's space until the upload of the file is successful.

**ownership_transfer():**

File ownership transfer method. Users can transfer files they currently hold to another user. The premise is that the target user has enough space.

**transfer_report()**

Used by miners to report a file to complete the storage transaction. After a file is declared, a certain number of miners need to report it to finally represent the file. A file will be divided into several parts, and each miner is responsible for storing a part of it, and uses `index` to identify it when reporting.

**calculate_report()**

Used to calculate transactions completed by tag reporting. For subsequent storage proof, the miner needs to calculate the fragment's tag. After completing the calculation, the miner needs to report to the chain. After reporting, the computing power will be increased for miners, and this part of the data will be included in the challenge.

**replace_idle_space()**

Transactions to replace idle space. After the miner stores the service data, there will be a part of the idle space to be replaced, and the miner completes the update of the relevant data of the idle space through this transaction.

**delete_file()**

A transaction in which a user deletes a file. A file can have multiple holders. When the file holder is 0, the meta-information of the file on the chain will be deleted. Otherwise, only the user who called this transaction will be removed from the holder list of the specified file.

**cert_idle_space()**

Authenticate transactions for idle space. The miner's certification of idle space requires the tee to challenge it. When successful, the miner can update the new space proof data on the chain.

**create_bucket()**

The transaction that creates the bucket. The files uploaded by the user must be located in a certain bucket, and the bucket name can only consist of numbers, letters, and special characters (_ - .).

**delete_bucket()**

Transaction to delete bucket. The prerequisite for deleting a bucket is that there are no files in the bucket before the bucket can be deleted.

**generate_restoral_order()**

Create a transaction to restore the order. When a miner's file cannot be restored, the data segment can be declared to create a recovery order. Wait for other miners to claim the order for recovery.

**claim_restoral_order()**

Claim the transaction that restored the order. A recovery order can only be claimed by one miner. During the recovery period, other miners will no longer be able to claim the order.

**claim_restoral_noexist_order()**

Claim a transaction for a restoration order that does not currently exist. The prerequisite for claiming this type of order is that the miner exits, and all files belonging to the miner can be claimed and restored. Then the remaining miners claim it through this transaction.

**restoral_order_complete()**

Restore the order to complete the transaction. When the miner completes the storage and calculation of tags for this fragment, it calls the transaction to report. The chain will update its state to point this data to that miner.

**root_clear_file()**

Test the dedicated interface. It can only be called with root privileges and directly clears file metadata. It needs to be called together with other interfaces to ensure that the data is correct.



