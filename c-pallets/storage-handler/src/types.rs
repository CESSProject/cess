use super::*;

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
#[scale_info(skip_type_params(T))]
#[codec(mel_bound())]
pub struct OwnedSpaceDetails<T: Config> {
	pub(super) total_space: u128,
	pub(super) used_space: u128,
	pub(super) remaining_space: u128,
	pub(super) start: BlockNumberOf<T>,
	pub(super) deadline: BlockNumberOf<T>,
	pub(super) state: BoundedVec<u8, T::StateStringMax>,
}