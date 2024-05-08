use super::*;
use sp_runtime::traits::Zero;

impl<Balance: Zero> Default for ReservoirInfo<Balance> {
    fn default() -> Self {
        ReservoirInfo::<Balance> {
            free_balance: Balance::zero(),
            borrow_balance: Balance::zero(),
            store_balance: Balance::zero(),
        }
    }
}

impl<Balance: Zero> Default  for UserHold<Balance> {
    fn default() -> Self {
        UserHold::<Balance> {
            free: Balance::zero(),
            staking: Balance::zero(),
        }
    }
}