# File Map Module

Store scheduling related information

### Terminology

* **Ip:** Scheduled IP address.


## Interface
ScheduleFind
    A series of methods for finding consensus scheduling.  
    * `contains_scheduler` - Judge whether the controller account exists.
    * `get_controller_acc` - Obtain controller account through stash account.
### Dispatchable Functions

* `registration_scheduler` - The interface for scheduling registration has no special restrictions at present.
* `scheduler_exception_report` - Error information feedback report of consensus. If one consensus finds that another consensus is not online, it will be reported through this method.
* `init_public_key` - Initialize the public key related to the certificate.
