use super::*;
use frame_support::pallet_prelude::MaxEncodedLen;

/// The custom struct for storing info of storage miners.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct MinerInfo<AccountId, Balance, BoundedString> {
	//Income account
	pub(super) beneficiary: AccountId,
	pub(super) peer_id: [u8; 52],
	pub(super) collaterals: Balance,
	pub(super) debt: Balance,
	//nomal, exit, frozen, e_frozen
	pub(super) state: BoundedString,
	pub(super) idle_space: u128,
	pub(super) service_space: u128,
	pub(super) lock_space: u128,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct Reward<T: pallet::Config> {
	//Total reward for miners
    pub(super) total_reward: BalanceOf<T>,
	//Rewards issued at present
	pub(super) reward_issued: BalanceOf<T>,
	//Currently available reward
	pub(super) currently_available_reward: BalanceOf<T>,
	//Reward order list, up to 180 reward orders can be accumulated
	pub(super) order_list: BoundedVec<RewardOrder<BalanceOf<T>>, ConstU32<{RELEASE_NUMBER as u32}>>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct RewardOrder<Balance> {
	pub(super) order_reward: Balance,
	pub(super) each_share: Balance,
	pub(super) award_count: u8,
	pub(super) has_issued: bool,
}

/// The custom struct for storing info of storage FaucetRecord.
#[derive(PartialEq, Eq, Encode, Default, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct FaucetRecord<BlockNumber> {
	pub(super) last_claim_time: BlockNumber,
}
