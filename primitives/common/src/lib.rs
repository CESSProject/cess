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
		let new_hash = Hash([0u8; 64]);
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
	pub fn binary(&self) -> Result<[u8; 256], HashError> {
		let mut elem: [u8; 256] = [0u8; 256];
		let mut index: usize = 0;
		for value in self.0.iter() {
			let binary = match value {
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

