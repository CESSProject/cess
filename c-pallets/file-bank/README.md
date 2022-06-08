# File Bank Module

Contain operations related info of files on multi-direction.

### Terminology

* **Is Public:** Public or private.
* **Backups:** Number of duplicates.
* **Deadline:** Expiration time.

## Interface

### Dispatchable Functions

* `upload` - Upload info of stored file.
* `buyfile` - buy specific file pay on download fee.
* `update_dupl` - Update the meta information of file backup and related fragments.
* `update_file_state` - Update file status to prevent multiple scheduling operations on the same file.
* `delete_file` - Delete file meta information.
* `buy_space` - Buy storage space.
* `receive_free_space` - New CESS register users have qulified to get 1GB storage space free.
* `upload_filler` - File upload of filler type.
* `update_price` - Modified pricing transactions.
* `clear_invalid_file` - Feedback method after miners clear invalid files.
* `recover_file` - Feedback method after scheduling and restoring files.

