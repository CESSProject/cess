// miner cpu use 50%.
pub(super) const SERVICE_PROVE_RATE: u128 = 25_165_824;
// miner cpu use 50%.
pub(super) const IDLE_PROVE_RATE: u128 = 71_582_788;

pub(super) const IDLE_VERIFY_RATE: u128 = 2_147_483_648;

pub(super) const IDLE_FAULT_TOLERANT: u8 = 2;

pub(super) const SERVICE_FAULT_TOLERANT: u8 = 2;

pub(super) type SpaceChallengeParam = [u64; 8];
