#![cfg_attr(not(feature = "std"), no_std)]

/* use */
use frame_support::{
	RuntimeDebug,
	dispatch::{Decode, Encode},
};
use codec::{MaxEncodedLen};
use scale_info::TypeInfo;

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Hash(pub [u8; 68]);
pub struct TryFromSliceError(());

// impl Encode for Hash {
// 	// fn using_encoded<R, F>(&self, f: F) -> R
// 	// 	where
// 	// 		F: FnOnce(&[u8]) -> R
// 	// {
// 	// 	f(self.0.as_slice())
// 	// }
// 	//
// 	// fn encode(&self) -> Vec<u8> {
// 	// 	self.0.encode()
// 	// }
// 	fn encode_to<T: Output + ?Sized>(&self, dest: &mut T) {
// 		for i in self.0.iter() {
// 			dest.push_byte(*i)
// 		}
// 	}
// }

// impl EncodeLike<Hash> for Hash {}

// impl Decode for Hash {
// 	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
// 		let value: &mut [u8] = Default::default();
// 		let _ = input.read(value)?;
// 		let v = Hash::slice_to_array_68(value).map_err(|_| "convert err")?;
// 		let hash = Hash(v);
// 		return Ok(hash)
// 	}
// }

impl Default for Hash {
	fn default() -> Hash {
		let new_hash = Hash([0u8; 68]);
		return new_hash
	}
}

// impl TypeInfo for Hash {
// 	type Identity = Self;
//
// 	fn type_info() -> scale_info::Type {
// 		scale_info::Type::builder()
// 			.path(scale_info::Path::new("Vote", module_path!()))
// 			.composite(
// 				scale_info::build::Fields::unnamed()
// 					.field(|f| f.ty::<[u8; 68]>()),
// 			)
// 	}
// }

impl Hash {
	fn slice_to_array_68(slice: &[u8]) -> Result<[u8; 68], TryFromSliceError> {
		if slice.len() == 68 {
			let ptr: [u8; 68] = (*slice).try_into().map_err(|_e| TryFromSliceError(()))?;
			Ok(ptr)
		} else {
			Err(TryFromSliceError(()))
		}
	}

	pub fn from_shard_id(shard_id: &[u8; 72]) -> Result<Self, TryFromSliceError> {
		let slice = Self::slice_to_array_68(&shard_id[0..67])?;
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
	IPV4([u8; 4]),
	IPV6([u16; 8]),
}

