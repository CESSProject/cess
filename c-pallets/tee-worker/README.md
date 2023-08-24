# File Map Module

Store scheduling related information

### Terminology

* **Ip:** Scheduled IP address.


## Interface
### Trait
#### TeeWorkerHandler

A series of methods for finding consensus scheduling.
 * `contains_scheduler` - Judge whether the controller account exists.
 * `get_controller_acc` - Obtain controller account through stash account.
 * `get_first_controller` - Get the first consensus in the list.
### Dispatchable Functions

* `registration_scheduler` - The interface for scheduling registration has no special restrictions at present.
* `update_scheduler` - Consensus Method for Updating IP Endpoints.
* `init_public_key` - Initialize the public key related to the certificate.
