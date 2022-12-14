use super::*;
use frame_support::pallet_prelude::MaxEncodedLen;

// type AccountOf<T> = <T as frame_system::Config>::AccountId;
// type BalanceOf<T> = <<T as pallet::Config>::Currency as Currency<<T as
// frame_system::Config>::AccountId>>::Balance;
type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
/// The custom struct for storing info of storage miners.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct MinerInfo<AccountId, Balance, BoundedString> {
	pub(super) beneficiary: AccountId,
	pub(super) ip: IpAddress,
	pub(super) collaterals: Balance,
	pub(super) debt: Balance,
	//nomal, exit, frozen, e_frozen, debt
	pub(super) state: BoundedString,
	pub(super) idle_space: u128,
	pub(super) service_space: u128,
	pub(super) autonomy_space: u128,
	pub(super) puk: Public,
	pub(super) ias_cert: IasCert,
	pub(super) bloom_filter: BloomCollect,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct RewardOrder<Balance> {
	//Total award for orders
	order_reward: Balance,
	//Number of awards to be distributed each time
	each_share: Balance,
	//Number of order reward distribution
	award_count: u8,
	//Whether the 20% reward immediately paid has been paid
	has_issued: bool,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Reward<Balance> {
	//Total reward for miners
    total_reward: Balance,
	//Rewards issued at present
	reward_issued: Balance,
	//Reward order list, up to 180 reward orders can be accumulated
	order_list: [RewardOrder<Balance>; 180],
}



/// The custom struct for storing info of storage FaucetRecord.
#[derive(PartialEq, Eq, Encode, Default, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct FaucetRecord<BlockNumber> {
	pub(super) last_claim_time: BlockNumber,
}

#[derive(PartialEq, Eq, Encode, Default, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct BloomCollect {
	//for autonomy file
	pub(super) autonomy_filter: BloomFilter,
	//for service file
	pub(super) service_filter: BloomFilter,
	//for filler file
	pub(super) idle_filter: BloomFilter,
}

