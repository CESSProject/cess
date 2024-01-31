#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::boxed::Box;
use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::RuntimeDebug;

#[derive(Copy, Clone, Eq, PartialEq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct BloomFilter(pub [u64; 256]);

#[derive(Debug)]
pub enum BloomError {
    InsertError,
    DeleteError,
    Overflow,
    BinaryError,
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


pub fn binary(data: [u8; 64]) -> Result<Box<[u8; 256]>, BloomError> {
    let mut elem: Box<[u8; 256]> = Box::new([0u8; 256]);
    let mut index: usize = 0;
    for value in data.iter() {
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
            _ => return Err(BloomError::BinaryError),
        };

        elem[index * 4] = binary[0];
        elem[index * 4 + 1] = binary[1];
        elem[index * 4 + 2] = binary[2];
        elem[index * 4 + 3] = binary[3];

        index = index + 1;
    }
    Ok(elem)
}
