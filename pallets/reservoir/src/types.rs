use super::*;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct ReserviorInfo<Balance> {
    pub(super) free_balance: Balance,
    pub(super) borrow_balance: Balance,
    pub(super) lock_balance: Balance,
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