use super::*;

pub(super) const STATE_POSITIVE: &str = "positive";

pub(super) const STATE_FROZEN: &str = "frozen";

pub(super) const STATE_EXIT_FROZEN: &str = "e_frozen";

pub(super) const STATE_EXIT: &str = "exit";

pub(super) const FAUCET_VALUE: u128 = 10000000000000000;

pub(super) const DOUBLE: u8 = 2;

pub(super) const IDLE_MUTI: Perbill = Perbill::from_percent(30);

pub(super) const SERVICE_MUTI: Perbill = Perbill::from_percent(70);

pub(super) const ISSUE_MUTI: Perbill = Perbill::from_percent(20);

pub(super) const EACH_SHARE_MUTI: Perbill = Perbill::from_percent(80);

pub(super) const RELEASE_NUMBER: u8 = 180;
