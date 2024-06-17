use super::*;

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct OwnedSpaceDetails<T: Config> {
	pub(super) total_space: u128,
	pub(super) used_space: u128,
	pub(super) locked_space: u128,
	pub(super) remaining_space: u128,
	pub(super) start: BlockNumberFor<T>,
	pub(super) deadline: BlockNumberFor<T>,
	pub(super) state: BoundedVec<u8, T::StateStringMax>,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct OrderInfo<T: Config> {
	pub(super) pay: BalanceOf<T>,
	pub(super) gib_count: u32,
	pub(super) days: u32,
	pub(super) expired: BlockNumberFor<T>,	
	pub(super) target_acc: AccountOf<T>,
	pub(super) order_type: OrderType,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum OrderType {
	Buy,
	Expansion,
	Renewal,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct TerritoryInfo<T: Config> {
	pub(super) token: H256,
    pub(super) total_space: u128,
	pub(super) used_space: u128,
	pub(super) locked_space: u128,
	pub(super) remaining_space: u128,
	pub(super) start: BlockNumberFor<T>,
	pub(super) deadline: BlockNumberFor<T>,
	pub(super) state: TerritoryState,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct ConsignmentInfo<T: Config> {
	pub(super) user: AccountOf<T>,
    pub(super) price: BalanceOf<T>,
    pub(super) buyers: Option<AccountOf<T>>,
    pub(super) exec: Option<BlockNumberFor<T>>,
	pub(super) locked: bool,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum TerritoryState {
	Active,
	Frozen,
	Expired,
	OnConsignment,
}