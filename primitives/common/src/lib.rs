#![cfg_attr(not(feature = "std"), no_std)]

/* use */
use frame_support::{
	RuntimeDebug,
	dispatch::{Decode, Encode},
};
use codec::{
	MaxEncodedLen,
};
use scale_info::TypeInfo;

pub struct Hash(pub [u8; 68]);

impl Default for Hash {
	fn default() -> Hash {
		let new_hash = Hash([0u8; 68]);
		return new_hash
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

