use super::*;

pub(super) const STATE_POSITIVE: &str = "positive";

pub(super) const STATE_FROZEN: &str = "frozen";

pub(super) const STATE_EXIT: &str = "exit";

pub(super) const STATE_LOCK: &str = "lock";

pub(super) const STATE_OFFLINE: &str = "offline";

pub(super) const FAUCET_VALUE: u128 = 10_000_000_000_000_000;

pub(super) const IDLE_MUTI: Perbill = Perbill::from_percent(30);

pub(super) const SERVICE_MUTI: Perbill = Perbill::from_percent(70);

pub(super) const ISSUE_MUTI: Perbill = Perbill::from_percent(20);

pub(super) const EACH_SHARE_MUTI: Perbill = Perbill::from_percent(80);

pub(super) const RELEASE_NUMBER: u8 = 180;

pub(super) const IDLE_PUNI_MUTI: Perbill = Perbill::from_percent(10);

pub(super) const SERVICE_PUNI_MUTI: Perbill = Perbill::from_percent(25);

pub(super) const BASE_LIMIT: u128 = 2_000_000_000_000_000;