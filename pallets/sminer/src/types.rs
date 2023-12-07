use super::*;
use frame_support::pallet_prelude::MaxEncodedLen;

/// The custom struct for storing info of storage miners.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct MinerInfo<T: Config> {
	//Income account
	pub(super) beneficiary: AccountOf<T>,
	pub(super) staking_account: AccountOf<T>,
	pub(super) peer_id: PeerId,
	pub(super) collaterals: BalanceOf<T>,
	pub(super) debt: BalanceOf<T>,
	//nomal, exit, frozen, e_frozen
	pub(super) state: BoundedVec<u8, T::ItemLimit>,
	pub(super) declaration_space: u128,
	pub(super) idle_space: u128,
	pub(super) service_space: u128,
	pub(super) lock_space: u128,
	pub(super) space_proof_info: Option<SpaceProofInfo<AccountOf<T>>>,
	pub(super) service_bloom_filter: BloomFilter,
    pub(super) tee_signature: TeeRsaSignature,
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

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct RestoralTargetInfo<Account, Block> {
	pub(super) miner: Account,
	pub(super) service_space: u128,
	pub(super) restored_space: u128,
	pub(super) cooling_block: Block,
}
