# File Bank Module

Contain operations related info of files on multi-direction. Provide storage space purchase, capacity expansion and lease renewal interfaces.

### Terminology

* **Is Public:** Public or private.
* **Backups:** Number of duplicates.
* **Deadline:** Expiration time.

## Interface

### Dispatchable Functions
* `upload_declaration` - Users need to call this method to "place an order" when uploading files.
* `update_price` - Update the unit price of storage space with root privileges.
* `upload` - The method of uploading file meta information can only be called by consensus.
* `upload_filler` - The method of uploading filled files can only be called by consensus.
* `delete_file` - Delete file meta information.
* `buy_space` - Purchase storage package, Package 1 is for free purchase of 10g storage space.
* `clear_invalid_file` - Feedback method after miners clear invalid files.
* `recover_file` - Feedback method after scheduling and restoring files.

