use super::*;
use frame_support::pallet_prelude::MaxEncodedLen;

// type AccountOf<T> = <T as frame_system::Config>::AccountId;
// type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as
// frame_system::Config>::AccountId>>::Balance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
/// The custom struct for storing info of storage miners.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct MinerInfo<AccountId, Balance, BoundedString> {
	pub(super) peer_id: u64,
	//Income account
	pub(super) beneficiary: AccountId,
	pub(super) ip: IpAddress,
	pub(super) collaterals: Balance,
	//nomal, exit, frozen, e_frozen
	pub(super) state: BoundedString,
	pub(super) power: u128,
	pub(super) space: u128,
	pub(super) reward_info: RewardInfo<Balance>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct RewardInfo<Balance> {
	pub(super) total_reward: Balance,
	pub(super) total_rewards_currently_available: Balance,
	pub(super) total_not_receive: Balance,
}
/// The custom struct for storing info of storage CalculateRewardOrder.
#[derive(PartialEq, Eq, Encode, Default, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct CalculateRewardOrder<T: pallet::Config> {
	pub(super) calculate_reward: u128,
	pub(super) start_t: BlockNumberOf<T>,
	pub(super) deadline: BlockNumberOf<T>,
}
/// The custom struct for storing info of storage RewardClaim.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct RewardClaim<AccountId, Balance> {
	pub(super) beneficiary: AccountId,
	pub(super) total_reward: Balance,
	pub(super) have_to_receive: Balance,
	pub(super) current_availability: Balance,
	pub(super) total_not_receive: Balance,
}
/// The custom struct for storing info of storage FaucetRecord.
#[derive(PartialEq, Eq, Encode, Default, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct FaucetRecord<BlockNumber> {
	pub(super) last_claim_time: BlockNumber,
}
