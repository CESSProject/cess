#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	BoundedVec,
	pallet_prelude::ConstU32,
};
use codec::{MaxEncodedLen, Decode, Encode};
use scale_info::TypeInfo;
use sp_core::RuntimeDebug;
use sp_std::prelude::Box;

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialOrd, Ord)]
pub struct Hash(pub [u8; 64]);
pub struct TryFromSliceError(());

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct PoISKey {
	pub g: [u8; 256],
	pub n: [u8; 256],
}

#[derive(PartialEq, Eq, Encode, Decode, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct SpaceProofInfo<AccountId> {
	pub miner: AccountId,
	pub front: u64,
	pub rear: u64,
	pub pois_key: PoISKey,
	pub accumulator: Accumulator,
}

pub enum HashError {
	TryFromSliceError,
	BinaryError,
}

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
	pub fn binary(&self) -> Result<Box<[u8; 256]>, HashError> {
		let mut elem: Box<[u8; 256]> = Box::new([0u8; 256]);
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
pub type TeeRsaSignature = [u8; 256];
pub type EndPoint = BoundedVec<u8, ConstU32<100>>;

pub const M_BYTE: u128 = 1_048_576;
pub const G_BYTE: u128 = 1_048_576 * 1024;
pub const T_BYTE: u128 = 1_048_576 * 1024 * 1024; 
pub const IDLE_SEG_SIZE: u128 = M_BYTE * 64;
pub const SEGMENT_SIZE: u128 = M_BYTE * 32;
pub const FRAGMENT_SIZE: u128 = M_BYTE * 8;
// The number of fragments contained in a segment
pub const FRAGMENT_COUNT: u32 = (SEGMENT_SIZE * 30 / 10 / FRAGMENT_SIZE) as u32;
pub const ASSIGN_MINER_IDEAL_QUANTITY: u32 = FRAGMENT_COUNT;
pub const CHUNK_COUNT: u32 = 1024;

pub const BUCKET_ALLOW_CHAR: [u8; 65] = [
	b'a', b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', b'g', b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't', b'u', b'v', b'w', b'x', b'y', b'z',
	b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'G', b'K', b'L', b'M', b'N', b'O', b'P', b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z',
	b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9',
	b'-', b'_', b'.',
];

pub const NUMBER: [u8; 10] = [b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9'];

pub type Accumulator = [u8; 256];

pub type NodePublicKey = sp_core::ed25519::Public;
pub type NodeSignature = [u8; 64];

pub type ReportSign = BoundedVec<u8, ConstU32<344>>;
pub type Report =  BoundedVec<u8, ConstU32<5000>>;
pub type Cert = BoundedVec<u8, ConstU32<1588>>;

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

