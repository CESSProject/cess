use super::*;

pub(super) const STATE_NOT_READY: &str = "not ready";

pub(super) const STATE_POSITIVE: &str = "positive";

pub(super) const STATE_FROZEN: &str = "frozen";

pub(super) const STATE_EXIT: &str = "exit";

pub(super) const STATE_LOCK: &str = "lock";

pub(super) const STATE_OFFLINE: &str = "offline";

pub(super) const FAUCET_VALUE: u128 = 10_000_000_000_000_000_000_000;

pub(super) const IDLE_MUTI: Perbill = Perbill::from_percent(30);

pub(super) const SERVICE_MUTI: Perbill = Perbill::from_percent(70);

pub(super) const RELEASE_NUMBER: u8 = 90;

pub(super) const AOIR_PERCENT: Perbill = Perbill::from_percent(50);

pub(super) const IDLE_PUNI_MUTI: Perbill = Perbill::from_percent(10);

pub(super) const SERVICE_PUNI_MUTI: Perbill = Perbill::from_percent(5);

pub(super) const BASE_UNIT: u128 = 4_000_000_000_000_000_000_000;