#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::dispatch::DispatchResult;
/// API necessary for Scheduler record ops about credit.
pub trait SchedulerCreditCounter<SchedulerCtrlAccountId> {

	fn record_proceed_block_size(scheduler_id: &SchedulerCtrlAccountId, block_size: u64) -> DispatchResult;

	fn record_punishment(scheduler_id: &SchedulerCtrlAccountId) -> DispatchResult;
}

pub trait SchedulerStashAccountFinder<AccountId> {
	fn find_stash_account_id(ctrl_account_id: &AccountId) -> Option<AccountId>;
}
