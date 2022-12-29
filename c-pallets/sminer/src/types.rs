use super::*;
use frame_support::pallet_prelude::MaxEncodedLen;

// type AccountOf<T> = <T as frame_system::Config>::AccountId;
// type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as
// frame_system::Config>::AccountId>>::Balance;
/// The custom struct for storing info of storage miners.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct MinerInfo<AccountId, Balance, BoundedString> {
	// Income account
	pub(super) beneficiary: AccountId,
	// ip
	pub(super) ip: IpAddress,
	// Pledge amount
	pub(super) collaterals: Balance,
	// Amount of debt
	pub(super) debt: Balance,
	//nomal, exit, frozen, e_frozen, debt
	pub(super) state: BoundedString,
	// Idle file space
	pub(super) idle_space: u128,
	// Service file space
	pub(super) service_space: u128,
	// Autonomous file space
	pub(super) autonomy_space: u128,
	// miner public 
	pub(super) puk: Public,
	// Ias certificate
	pub(super) ias_cert: BoundedString,
	// Bloon filter for three file types
	pub(super) bloom_filter: BloomCollect,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct RewardOrder<Balance> {
	//Total award for orders
	pub(super) order_reward: Balance,
	//Number of awards to be distributed each time
	pub(super) each_share: Balance,
	//Number of order reward distribution
	pub(super) award_count: u8,
	//Whether the 20% reward immediately paid has been paid
	pub(super) has_issued: bool,
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
	pub(super) order_list: BoundedVec<RewardOrder<BalanceOf<T>>, T::OrderLimit>,
}



/// The custom struct for storing info of storage FaucetRecord.
#[derive(PartialEq, Eq, Encode, Default, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct FaucetRecord<BlockNumber> {
	pub(super) last_claim_time: BlockNumber,
}

#[derive(PartialEq, Eq, Encode, Default, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct BloomCollect {
	//for autonomy file
	pub autonomy_filter: BloomFilter,
	//for service file
	pub service_filter: BloomFilter,
	//for filler file
	pub idle_filter: BloomFilter,
}

