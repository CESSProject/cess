use super::*;

type AccountOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
/// The custom struct for storing info of storage MinerInfo.
#[derive(PartialEq, Eq, Default, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
pub struct MinerInfo {
	pub(super) peerid: u64,
	pub(super) ip: Vec<u8>,
	
	pub(super) power: u128,	
	pub(super) space: u128,
}
/// The custom struct for storing info of storage miners.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct Mr<T: pallet::Config> {
	pub(super) peerid: u64,
	//Income account
	pub(super) beneficiary: AccountOf<T>,
	pub(super) ip: Vec<u8>,
	pub(super) collaterals: BalanceOf<T>,
	pub(super) earnings: BalanceOf<T>,
	pub(super) locked: BalanceOf<T>,
	//nomal, exit, frozen, e_frozen
	pub(super) state: Vec<u8>,
	
	pub(super) power: u128,
	pub(super) space: u128,
}
/// The custom struct for storing index of segment, miner's current power and space.
#[derive(PartialEq, Eq, Default, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
pub struct SegmentInfo {
	pub(super) segment_index: u64,
}
/// The custom struct for storing info of storage StorageInfo.
#[derive(PartialEq, Eq, Default, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
pub struct StorageInfo {
	pub(super) used_storage: u128,
	pub(super) available_storage: u128,
	pub(super) time: u128,
}
/// The custom struct for miner table of block explorer.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct TableInfo<T: pallet::Config> {
	pub(super) address: AccountOf<T>,
	pub(super) beneficiary: AccountOf<T>,
	pub(super) total_storage: u128,
	pub(super) average_daily_data_traffic_in: u64,
	pub(super) average_daily_data_traffic_out: u64,
	pub(super) mining_reward: BalanceOf<T>,
}
/// The custom struct for miner detail of block explorer.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct MinerDetailInfo<T: pallet::Config> {
	pub(super) address: AccountOf<T>,
	pub(super) beneficiary: AccountOf<T>,
	pub(super) power: u128,
	pub(super) space: u128,
	pub(super) total_reward: BalanceOf<T>, 
	pub(super) total_rewards_currently_available: BalanceOf<T>,
	pub(super) totald_not_receive: BalanceOf<T>,
	pub(super) collaterals: BalanceOf<T>,
}
/// The custom struct for miner detail of block explorer.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct MinerStatInfo<T: pallet::Config> {
	pub(super) total_miners: u64,
	pub(super) active_miners: u64,
	pub(super) staking: BalanceOf<T>,
	pub(super) miner_reward: BalanceOf<T>,
	pub(super) sum_files: u128, 
}
/// The custom struct for storing info of storage CalculateRewardOrder.
#[derive(PartialEq, Eq, Encode, Default, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct CalculateRewardOrder <T: pallet::Config>{
	pub(super) calculate_reward:u128,
	pub(super) start_t: BlockNumberOf<T>,
	pub(super) deadline: BlockNumberOf<T>,
}
/// The custom struct for storing info of storage RewardClaim.
#[derive(PartialEq, Eq, Encode, Default, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct RewardClaim <T: pallet::Config>{
	pub(super) beneficiary: AccountOf<T>,
	pub(super) total_reward: BalanceOf<T>,
	pub(super) total_rewards_currently_available: BalanceOf<T>,
	pub(super) have_to_receive: BalanceOf<T>,
	pub(super) current_availability: BalanceOf<T>,
	pub(super) total_not_receive: BalanceOf<T>,
}
/// The custom struct for storing info of storage FaucetRecord.
#[derive(PartialEq, Eq, Encode, Default, Decode, Clone, RuntimeDebug, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct FaucetRecord <T: pallet::Config>{
	pub(super) last_claim_time: BlockNumberOf<T>,
}