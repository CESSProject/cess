use super::*;

//Miner active state
pub(super) const STATE_POSITIVE: &str = "positive";
//Miner frozen state
pub(super) const STATE_FROZEN: &str = "frozen";
//
pub(super) const STATE_EXIT_FROZEN: &str = "e_frozen";
//Miner exit status
pub(super) const STATE_EXIT: &str = "exit";
//Miner debt status
pub(super) const STATE_DEBT: &str = "debt";
//Amount of water tap hair
pub(super) const FAUCET_VALUE: u128 = 10_000_000_000_000_000;
//Penalty multiplier
pub(super) const DOUBLE: u8 = 2;
//Proportion of computing power in autonomous space
pub(super) const AUTONOMY_MUTI: Perbill = Perbill::from_percent(10);
//Proportion of idle space computing power
pub(super) const IDLE_MUTI: Perbill = Perbill::from_percent(30);
//Proportion of computing power in service space
pub(super) const SERVICE_MUTI: Perbill = Perbill::from_percent(60);