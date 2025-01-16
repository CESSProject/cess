use super::*;
use frame_support::pallet_prelude::MaxEncodedLen;

/// The custom struct for storing info of storage miners.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct MinerInfo<T: Config> {
	//Income account
	pub beneficiary: AccountOf<T>,
	pub staking_account: AccountOf<T>,
	pub endpoint: EndPoint,
	pub collaterals: BalanceOf<T>,
	pub debt: BalanceOf<T>,
	//nomal, exit, frozen, e_frozen
	pub state: BoundedVec<u8, T::ItemLimit>,
	pub declaration_space: u128,
	pub idle_space: u128,
	pub service_space: u128,
	pub lock_space: u128,
	pub space_proof_info: Option<SpaceProofInfo<AccountOf<T>>>,
	pub service_bloom_filter: BloomFilter,
    pub tee_signature: TeeSig,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct Reward<T: pallet::Config> {
	//Total reward for miners
    pub(super) total_reward: BalanceOf<T>,
	//Rewards issued at present
	pub(super) reward_issued: BalanceOf<T>,
	//Reward order list, up to 180 reward orders can be accumulated
	pub(super) order_list: BoundedVec<RewardOrder<BalanceOf<T>, BlockNumberFor<T>>, ConstU32<{RELEASE_NUMBER as u32}>>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct RewardOrder<Balance, Block> {
	pub(super) receive_count: u8,
	pub(super) max_count: u8,
	pub(super) atonce: bool,
	pub(super) order_reward: Balance,
	pub(super) each_amount: Balance,
	pub(super) last_receive_block: Block,
}

/// The custom struct for storing info of storage FaucetRecord.
#[derive(PartialEq, Eq, Encode, Default, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct FaucetRecord<BlockNumber> {
	pub(super) last_claim_time: BlockNumber,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct RestoralTargetInfo<Account, Block> {
	pub(super) miner: Account,
	pub(super) service_space: u128,
	pub(super) restored_space: u128,
	pub(super) cooling_block: Block,
}

/// audit -> sminer -> cess-treasury
/// way 1:
/// sminer.round_snapshot  key u128
/// {
/// 	pub total_power: u128
/// 	pub count: u32
/// }
/// sminer.snapshot  key account, value vec
/// {
/// 	pub finsh_block: Block
/// 	pub power: u128
/// 	pub issued: bool
/// }
/// cess-treasury.record  key u128
/// {
/// 	reward: Balance
/// }
/// 1. round_reward = now / one_day  
///	2. round = now / one_day
/// 3. finsh_block / one_day = which_day && which_day * one_day < now;
/// 4. according which_day, calculate reward. change vec[index].issued = true.
/// 5. delete sminer.snapshot vec[index].issued = true.
#[derive(Default, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct CompleteInfo {
	pub(super) miner_count: u32,
	pub(super) total_power: u128,
}

#[derive(Default, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct MinerCompleteInfo<Block> {
	pub(super) era_index: u32,
	pub(super) issued: bool,
	pub(super) finsh_block: Block,
	pub(super) power: u128,
}