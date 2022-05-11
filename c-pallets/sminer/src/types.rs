use super::*;
use frame_support::pallet_prelude::MaxEncodedLen;

// type AccountOf<T> = <T as frame_system::Config>::AccountId;
// type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
/// The custom struct for storing info of storage MinerInfo.
#[derive(PartialEq, Eq, Default, Encode, Decode, Clone, MaxEncodedLen, RuntimeDebug, TypeInfo)]
pub struct MinerInfo<BoundedString> {
	pub(super) peerid: u64,
	pub(super) ip: BoundedString,
	pub(super) power: u128,	
	pub(super) space: u128,
}
/// The custom struct for storing info of storage miners.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Mr<AccountId, Balance, BoundedString> {
	pub(super) peerid: u64,
	//Income account
	pub(super) beneficiary: AccountId,
	pub(super) ip: BoundedString,
	pub(super) collaterals: Balance,
	pub(super) earnings: Balance,
	pub(super) locked: Balance,
	//nomal, exit, frozen, e_frozen
	pub(super) state: BoundedString,
	pub(super) temp_power: u128,
	pub(super) power: u128,
	pub(super) space: u128,
}
/// The custom struct for storing index of segment, miner's current power and space.
#[derive(PartialEq, Eq, Default, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct SegmentInfo {
	pub(super) segment_index: u64,
}
/// The custom struct for storing info of storage StorageInfo.
#[derive(PartialEq, Eq, Default, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct StorageInfo {
	pub(super) used_storage: u128,
	pub(super) available_storage: u128,
	pub(super) time: u128,
}
/// The custom struct for miner table of block explorer.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct TableInfo<AccountId, Balance> {
	pub(super) address: AccountId,
	pub(super) beneficiary: AccountId,
	pub(super) total_storage: u128,
	pub(super) average_daily_data_traffic_in: u64,
	pub(super) average_daily_data_traffic_out: u64,
	pub(super) mining_reward: Balance,
}
/// The custom struct for miner detail of block explorer.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct MinerDetailInfo<AccountId, Balance> {
	pub(super) address: AccountId,
	pub(super) beneficiary: AccountId,
	pub(super) temp_power: u128,
	pub(super) power: u128,
	pub(super) space: u128,
	pub(super) total_reward: Balance, 
	pub(super) total_rewards_currently_available: Balance,
	pub(super) totald_not_receive: Balance,
	pub(super) collaterals: Balance,
}
/// The custom struct for miner detail of block explorer.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct MinerStatInfo<Balance> {
	pub(super) total_miners: u64,
	pub(super) active_miners: u64,
	pub(super) staking: Balance,
	pub(super) miner_reward: Balance,
	pub(super) sum_files: u128, 
}
/// The custom struct for storing info of storage CalculateRewardOrder.
#[derive(PartialEq, Eq, Encode, Default, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct CalculateRewardOrder <T: pallet::Config>{
	pub(super) calculate_reward:u128,
	pub(super) start_t: BlockNumberOf<T>,
	pub(super) deadline: BlockNumberOf<T>,
}
/// The custom struct for storing info of storage RewardClaim.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct RewardClaim <AccountId, Balance>{
	pub(super) beneficiary: AccountId,
	pub(super) total_reward: Balance,
	pub(super) have_to_receive: Balance,
	pub(super) current_availability: Balance,
	pub(super) total_not_receive: Balance,
}
/// The custom struct for storing info of storage FaucetRecord.
#[derive(PartialEq, Eq, Encode, Default, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct FaucetRecord <BlockNumber>{
	pub(super) last_claim_time: BlockNumber,
}