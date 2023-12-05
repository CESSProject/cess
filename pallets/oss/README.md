# Oss Module

Manage meta information of DeOss.

## Overview

DeOss is a server used to upload and download files in the cess network. Process files uploaded by users, distribute them to storage nodes, and put the meta information of the files on the chain.

If you want to make your DeOss public and provide services to others, you need to register in this module and make your information public on the network.

## Terminology

### Authorize Operator

In the CESS network, if users want to upload their own files through DeOss, they need to authorize the designated DeOss account.

After authorization, DeOss will have the authority to perform the following operations for the user:

* Upload Files.
* Delete Files.
* Create Bucket.
* Delete Bucket.

Currently, it supports authorizing multiple DeOss to provide services to users.

## Extrinsic

* `authorize()` - User authorization DeOss function.
* `cancel_authorize()` - The user cancels the function of authorizing a certain DeOss.
* `register()` - DeOss registration function, after registration, users across the entire network will be able to access the service through the endpoint or peer id provided by DeOss.
* `update()` - DeOss updates the current endpoint or peer id information.
* `destroy()` - DeOss logout function.

## Interface

### OssFindAuthor

Interface provided to other modules for detecting permissions

#### Function

* `is_authorized` - Determine whether the user has authorized the DeOss.

#### Usage

in pallet::Config

```rust
pub trait Config: frame_system::Config + sp_std::fmt::Debug {
    // ...
    type OssFindAuthor: OssFindAuthor<Self::AccountId>;
    // ...
}
```

in runtime.rs 
```rust
impl pallet_file_bank::Config for Runtime {
    // ...
    type OssFindAuthor = Oss;
    // ...
}
```
