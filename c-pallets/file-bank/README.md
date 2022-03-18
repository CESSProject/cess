# File Bank Module

Contain operations related info of files on multi-direction.

### Terminology

* **Is Public:** Public or private.
* **Backups:** Number of duplicates.
* **Deadline:** Expiration time.

## Interface

### Dispatchable Functions

* `upload` - Upload info of stored file.
* `buyfile` - Buy file with download fee.
* `update_dupl` - Update the meta information of file backup and related fragments.
* `update_file_state` - Update file status to prevent multiple scheduling operations on the same file.
* `delete_file` - Delete file meta information.
* `buy_space` - Buy storage.
* `receive_free_space` - New users receive free 1GB of space.
