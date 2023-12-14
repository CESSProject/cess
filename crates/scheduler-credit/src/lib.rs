/*!
# Some scheduler credit primitives
*/
#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::dispatch::DispatchResult;
/// API necessary for Scheduler record ops about credit.
pub trait SchedulerCreditCounter<SchedulerCtrlAccountId> {
	fn increase_point_for_tag(scheduler_id: &SchedulerCtrlAccountId, space: u128) -> DispatchResult;

	fn increase_point_for_cert(scheduler_id: &SchedulerCtrlAccountId, space: u128) -> DispatchResult;

	fn increase_point_for_idle_verify(scheduler_id: &SchedulerCtrlAccountId, space: u128) -> DispatchResult;

	fn increase_point_for_service_verify(scheduler_id: &SchedulerCtrlAccountId, space: u128) -> DispatchResult;

	fn increase_point_for_replace(scheduler_id: &SchedulerCtrlAccountId, space: u128) -> DispatchResult;

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
