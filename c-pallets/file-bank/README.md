# File Bank Module

Contain operations related info of files on multi-direction. Provide storage space purchase, capacity expansion and lease renewal interfaces.

### Terminology

* **Is Public:** Public or private.
* **Backups:** Number of duplicates.
* **Deadline:** Expiration time.

## Interface

### Dispatchable Functions
* `upload_declaration` - Users need to call this method to "place an order" when uploading files.
* `upload` - The method of uploading file meta information can only be called by consensus.
* `upload_filler` - The method of uploading filled files can only be called by consensus.
* `delete_file` - Delete file meta information.
* `buy_package` - Purchase storage package, Package 1 is for free purchase of 10g storage space.
* `upgrade_package` - This method is used to upgrade to a higher level package.
* `renewal_package` - This method is used to renew the current package.
* `clear_invalid_file` - Feedback method after miners clear invalid files.
* `recover_file` - Feedback method after scheduling and restoring files.

