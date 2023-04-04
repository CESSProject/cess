use super::*;
use frame_support::pallet_prelude::MaxEncodedLen;

pub type AccountOf<T> = <T as frame_system::Config>::AccountId;
/// The balance type of this pallet.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// The custom struct for cacher info.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct CacherInfo<AccountId, Balance> {
	pub payee: AccountId,
	pub ip: IpAddress,
	pub byte_price: Balance,
}

/// The custom struct for bill info.
#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Bill<AccountId, Balance, Hash> {
	pub id: [u8; 16],
	pub to: AccountId,
	pub amount: Balance,
	// Hash of the file to download
	pub file_hash: Hash,
	// Hash of the file slice to download
	pub slice_hash: Hash,
	pub expiration_time: u64,
}
