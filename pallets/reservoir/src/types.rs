use super::*;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ReservoirInfo<Balance> {
    pub(super) free_balance: Balance,
    pub(super) borrow_balance: Balance,
    pub(super) store_balance: Balance,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct BorrowInfo<T: Config> {
    pub(super) free: BalanceOf<T>,
    pub(super) lender: AccountOf<T>,
    pub(super) staking: BalanceOf<T>,
    pub(super) deadline: BlockNumberFor<T>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct UserHold<Balance> {
    pub(super) free: Balance,
    pub(super) staking: Balance,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct EventInfo<T: Config> {
    pub(super) quota: u32,
    pub(super) deadline: BlockNumberFor<T>,
    pub(super) unit_amount: BalanceOf<T>,
    pub(super) borrow_period: BlockNumberFor<T>,
    pub(super) use_type: UseType,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum UseType {
    MinerStaking,
}