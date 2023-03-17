#![cfg_attr(not(feature = "std"), no_std)]

/* use */
use frame_support::{
	RuntimeDebug,
	dispatch::{Decode, Encode},
};
use codec::{MaxEncodedLen};
use scale_info::TypeInfo;

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Hash(pub [u8; 64]);
pub struct TryFromSliceError(());

impl sp_std::fmt::Debug for TryFromSliceError {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			_ => write!(fmt, "try form slice error!"),
		}
	}
}

impl Default for Hash {
	fn default() -> Hash {
		let new_hash = Hash([0u8; 64]);
		return new_hash
	}
}


impl Hash {
	pub fn slice_to_array_64(slice: &[u8]) -> Result<[u8; 64], TryFromSliceError> {
		// log::info!("slice len: {:?}", slice.len());
		if slice.len() == 64 {
			let ptr: [u8; 64] = (*slice).try_into().map_err(|_e| TryFromSliceError(()))?;
			Ok(ptr)
		} else {
			Err(TryFromSliceError(()))
		}
	}

	pub fn from_shard_id(shard_id: &[u8; 68]) -> Result<Self, TryFromSliceError> {
		let slice = Self::slice_to_array_64(&shard_id[0..64])?;
		let hash = Hash(slice);
		return Ok(hash)
	}
}

pub const M_BYTE: u128 = 1_048_576;
pub const G_BYTE: u128 = 1_048_576 * 1024;
pub const T_BYTE: u128 = 1_048_576 * 1024 * 1024;

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum PackageType {
	Package1,
	Package2,
	Package3,
	Package4,
	Package5,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum DataType {
	File,
	Filler,
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum IpAddress {
	IPV4([u8; 4], u16),
	IPV6([u16; 8], u16),
}

