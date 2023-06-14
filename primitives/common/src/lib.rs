#![cfg_attr(not(feature = "std"), no_std)]

/* use */
use frame_support::{
	RuntimeDebug,
	dispatch::{Decode, Encode},
};
use frame_support::{
	BoundedVec,
	pallet_prelude::ConstU32,
};
use codec::{MaxEncodedLen};
use scale_info::TypeInfo;

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialOrd, Ord)]
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

pub type Mrenclave = [u8; 32];
pub type PeerId = [u8; 38];
pub type Podr2Key = [u8; 270];

pub const M_BYTE: u128 = 1_048_576;
pub const G_BYTE: u128 = 1_048_576 * 1024;
pub const T_BYTE: u128 = 1_048_576 * 1024 * 1024;

pub const SEGMENT_SIZE: u128 = M_BYTE * 16;
pub const FRAGMENT_SIZE: u128 = M_BYTE * 8;
pub const CHUNK_COUNT: u32 = 1024;

pub type NodePublicKey = sp_core::ed25519::Public;
pub type NodeSignature = [u8; 64];

pub type ReportSign = BoundedVec<u8, ConstU32<344>>;
pub type Report =  BoundedVec<u8, ConstU32<1354>>;
pub type Cert = BoundedVec<u8, ConstU32<1588>> ;

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

