#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::RuntimeDebug;

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct BloomFilter(pub [u64; 256]);

pub enum BloomError {
    InsertError,
    DeleteError,
    Overflow,
}

impl Default for BloomFilter {
    fn default() -> Self {
        let value: [u64; 256] = [0u64; 256];
        BloomFilter(value)
    }
}

impl BloomFilter {
    pub fn insert(&mut self, elem: [u8; 256]) -> Result<(), BloomError> {
        let mut index: usize = 0;
        for value in elem {
            if value != 1 && value != 0 {
                return Err(BloomError::InsertError);
            }
            self.0[index] = self.0[index] + value as u64;
            index = index + 1;
        }

        Ok(())
    }

    pub fn delete(&mut self, elem: [u8; 256]) -> Result<(), BloomError> {
        let mut index: usize = 0;
        for value in elem {
            if value != 1 && value != 0 {
                return Err(BloomError::DeleteError);
            }
            self.0[index] = self.0[index] - value as u64;
            index = index + 1;
        }

        Ok(())
    }
}