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
pub enum HashError {
	TryFromSliceError,
	BinaryError,
}

impl sp_std::fmt::Debug for HashError {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match self {
			TryFromSliceError => write!(fmt, "try form slice error!"),
			BinaryError => write!(fmt, "convert binary error!"),
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
	pub fn binary(&self) -> Result<[u8; 256], HashError> {
		let mut elem: [u8; 256] = [0u8; 256];
		let mut index: usize = 0;
		for value in self.0.iter() {
			let binary = match value {
				b'0' => [0, 0, 0, 0],
				b'1' => [0, 0, 0, 1],
				b'2' => [0, 0, 1, 0],
				b'3' => [0, 0, 1, 1],
				b'4' => [0, 1, 0, 0],
				b'5' => [0, 1, 0, 1],
				b'6' => [0, 1, 1, 0],
				b'7' => [0, 1, 1, 1],
				b'8' => [1, 0, 0, 0],
				b'9' => [1, 0, 0, 1],
				b'a' => [1, 0, 1, 0],
				b'b' => [1, 0, 1, 1],
				b'c' => [1, 1, 0, 0],
				b'd' => [1, 1, 0, 1],
				b'e' => [1, 1, 1, 0],
				b'f' => [1, 1, 1, 1],
				_ 	 => return Err(HashError::BinaryError),
			};

			elem[index * 4] = binary[0];
			elem[index * 4 + 1] = binary[1];
			elem[index * 4 + 2] = binary[2];
			elem[index * 4 + 3] = binary[3];

			index = index + 1;
		}
		Ok(elem)

	}

	pub fn new(slice: &[u8]) -> Result<Self, HashError> {
		let array = Self::slice_to_array_64(slice)?;
		Ok(Hash(array))
	}

	pub fn slice_to_array_64(slice: &[u8]) -> Result<[u8; 64], HashError> {
		// log::info!("slice len: {:?}", slice.len());
		if slice.len() == 64 {
			let ptr: [u8; 64] = (*slice).try_into().map_err(|_e| HashError::TryFromSliceError)?;
			Ok(ptr)
		} else {
			Err(HashError::TryFromSliceError)
		}
	}

	pub fn from_shard_id(shard_id: &[u8; 68]) -> Result<Self, HashError> {
		let slice = Self::slice_to_array_64(&shard_id[0..64])?;
		let hash = Hash(slice);
		return Ok(hash)
	}
}



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

pub type Mrenclave = [u8; 32];

pub const M_BYTE: u128 = 1_048_576;

pub const G_BYTE: u128 = 1_048_576 * 1024;

pub const T_BYTE: u128 = 1_048_576 * 1024 * 1024;

pub const SLICE_DEFAULT_SIZE_MUTI: u128 = 512;

pub const SLICE_DEFAULT_BYTE: u128 = SLICE_DEFAULT_SIZE_MUTI * M_BYTE;