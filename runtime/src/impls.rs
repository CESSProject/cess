use crate::{AccountId, Assets, Authorship, Balances, NegativeImbalance, Runtime};
use frame_support::traits::{
	fungibles::{Balanced, CreditOf},
	Currency, OnUnbalanced,
};
use pallet_asset_tx_payment::HandleCredit;
use pallet_cess_staking::Pallet as StakingPallet;

pub struct Author;
impl OnUnbalanced<NegativeImbalance> for Author {
	fn on_nonzero_unbalanced(amount: NegativeImbalance) {
		if let Some(author) = Authorship::author() {
			Balances::resolve_creating(&author, amount);
		}
	}
}

/// A `HandleCredit` implementation that naively transfers the fees to the block author .
/// Will drop and burn the assets in case the transfer fails.
pub struct CreditToBlockAuthor;
impl HandleCredit<AccountId, Assets> for CreditToBlockAuthor {
	fn handle_credit(credit: CreditOf<AccountId, Assets>) {
		if let Some(author) = pallet_authorship::Pallet::<Runtime>::author() {
			// Drop the result which will trigger the `OnDrop` of the imbalance in case of error.
			let _ = Assets::resolve(&author, credit);
		}
	}
}

pub struct SchedulerStashAccountFinder;

impl cp_scheduler_credit::SchedulerStashAccountFinder<AccountId> for SchedulerStashAccountFinder {
	fn find_stash_account_id(ctrl_account_id: &AccountId) -> Option<AccountId> {
		if let Some(staking_ledger) = StakingPallet::<Runtime>::ledger(ctrl_account_id) {
			Some(staking_ledger.stash)
		} else {
			None
		}
	}
}
