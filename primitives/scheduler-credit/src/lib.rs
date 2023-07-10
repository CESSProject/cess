/*!
# Some scheduler credit primitives.
*/
#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::dispatch::DispatchResult;
/// API necessary for Scheduler record ops about credit.
pub trait SchedulerCreditCounter<SchedulerCtrlAccountId> {

  /// Records the number of file bytes processed by the scheduler
	fn record_proceed_block_size(scheduler_id: &SchedulerCtrlAccountId, block_size: u64) -> DispatchResult;
  
  /// Record the number of times the scheduler has been punished
	fn record_punishment(scheduler_id: &SchedulerCtrlAccountId) -> DispatchResult;

}

/// Stash account finder, used to find the corresponding Stash account according to the Controller account
pub trait SchedulerStashAccountFinder<AccountId> {
	/// find the corresponding Stash account according to the Controller account
	fn find_stash_account_id(ctrl_account_id: &AccountId) -> Option<AccountId>;
}
