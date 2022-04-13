#![cfg_attr(not(feature = "std"), no_std)]

use ink_env::Environment;
use ink_lang as ink;
use ink_prelude::vec::Vec;
use ink_prelude::string::String;
use ink_storage::traits::{
    PackedLayout,
    SpreadAllocate,
    SpreadLayout,
    StorageLayout,
};
use core::alloc::Layout;
/// This is an example of how an ink! contract may call the Substrate
/// runtime function `RandomnessCollectiveFlip::random_seed`. See the
/// file `runtime/chain-extension-example.rs` for that implementation.
///
/// Here we define the operations to interact with the Substrate runtime.

#[ink::chain_extension]
pub trait FetchRandom {
    type ErrorCode = RandomReadErr;

    /// Note: this gives the operation a corresponding `func_id` (1101 in this case),
    /// and the chain-side chain extension will get the `func_id` to do further operations.
    #[ink(extension = 1101, returns_result = false)]
    fn fetch_random(subject: ([u8; 32], [u8; 32])) -> ([u8; 32], [u8; 32]);

    #[ink(extension = 1102, returns_result = false)]
    fn fetch_struct(subject: FileInfo) -> FileInfo;

    #[ink(extension = 1103, returns_result = false)]
    fn fetch_file(value: (<ink_env::DefaultEnvironment as Environment>::AccountId, [u8; 32], [u8; 32], u32) );
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum RandomReadErr {
    FailGetRandomSource,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, scale::Encode, scale::Decode, SpreadLayout, PackedLayout, SpreadAllocate)]
#[cfg_attr(
    feature = "std",
    derive(
        scale_info::TypeInfo,
        ink_storage::traits::StorageLayout
    )
)]
pub struct FileInfo {
    pub filename: [u8; 32],
    pub filesize: u32,
}

impl ink_env::chain_extension::FromStatusCode for RandomReadErr {
    fn from_status_code(status_code: u32) -> Result<(), Self> {
        match status_code {
            0 => Ok(()),
            1 => Err(Self::FailGetRandomSource),
            _ => panic!("encountered unknown status code"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum CustomEnvironment {}


impl Environment for CustomEnvironment {
    const MAX_EVENT_TOPICS: usize =
        <ink_env::DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;

    type AccountId = <ink_env::DefaultEnvironment as Environment>::AccountId;
    type Balance = <ink_env::DefaultEnvironment as Environment>::Balance;
    type Hash = <ink_env::DefaultEnvironment as Environment>::Hash;
    type BlockNumber = <ink_env::DefaultEnvironment as Environment>::BlockNumber;
    type Timestamp = <ink_env::DefaultEnvironment as Environment>::Timestamp;

    type ChainExtension = FetchRandom;
}

#[ink::contract(env = crate::CustomEnvironment)]
mod rand_extension {
    use super::RandomReadErr;
    use crate::Vec;
    use crate::String;
    use crate::FileInfo;
    use ink_storage::traits::{
        PackedLayout,
        SpreadAllocate,
        SpreadLayout,
        StorageLayout,
    };
    use crate::Layout;

    
    /// Defines the storage of our contract.
    ///
    /// Here we store the random seed fetched from the chain.
    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct RandExtension {
        /// Stores a single `bool` value on the storage.
        value1: Vec<u8>,

        value2: Vec<u8>,

        file: FileInfo,
    }

    #[ink(event)]
    pub struct RandomUpdated {
        #[ink(topic)]
        new: Vec<u8>,
    }

    impl RandExtension {
        /// Constructor that initializes the `bool` value to the given `init_value`.


        /// Constructor that initializes the `bool` value to `false`.
        ///
        /// Constructors may delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            let v1: Vec<u8> = Default::default();
            let v2: Vec<u8> = Default::default();
            let file: FileInfo = FileInfo {
                filename: Default::default(),
                filesize: 0,
            };
            RandExtension {
                value1: v1,
                value2: v2,
                file: file,
            }
        }

        /// Seed a random value by passing some known argument `subject` to the runtime's
        /// random source. Then, update the current `value` stored in this contract with the
        /// new random value.
        #[ink(message)]
        pub fn update_random(&mut self, subject: [u8; 32], subject2: [u8; 32]) -> Result<(), RandomReadErr> {
            // Get the on-chain random seed
            let (new_random1, new_random2) = self.env().extension().fetch_random((subject, subject2))?;
            let v1 = new_random1.to_vec();
            let v2 = new_random2.to_vec();
            self.value1 = v1.clone();
            self.value2 = v2.clone();
            // Emit the `RandomUpdated` event when the random seed
            // is successfully fetched.
            self.env().emit_event(RandomUpdated { new: v1 });
            Ok(())
        }

        #[ink(message)]
        pub fn update_struct(&mut self, subject: FileInfo) -> Result<(), RandomReadErr> {
            // Get the on-chain random seed
            let file = self.env().extension().fetch_struct(subject)?;  
            self.file = file;
            // Emit the `RandomUpdated` event when the random seed
            // is successfully fetched.
            Ok(())
        }

        #[ink(message)]
        pub fn update_file(&mut self, owner: AccountId, fileid: [u8; 32], filename: [u8; 32], filesize: u32) -> Result<(), RandomReadErr> {
            // Get the on-chain random seed
            self.env().extension().fetch_file((owner, fileid, filename, filesize))?;  
            // Emit the `RandomUpdated` event when the random seed
            // is successfully fetched.
            Ok(())
        }


        /// Simply returns the current value.
        #[ink(message)]
        pub fn get_value1(&self) -> String {
            String::from_utf8(self.value1.clone()).unwrap()
        }

        #[ink(message)]
        pub fn get_value2(&self) -> String {
            String::from_utf8(self.value2.clone()).unwrap()
        }


        #[ink(message)]
        pub fn get_file(&self) -> FileInfo{
            self.file
        }
    }
}

//     mod tests {
//         /// Imports all the definitions from the outer scope so we can use them here.
//         use super::*;
//         use ink_lang as ink;

//         /// We test if the default constructor does its job.
//         #[ink::test]
//         fn default_works() {
//             let rand_extension = RandExtension::default();
//             assert_eq!(rand_extension.get(), [0; 32]);
//         }

//         #[ink::test]
//         fn chain_extension_works() {
//             // given
//             struct MockedExtension;
//             impl ink_env::test::ChainExtension for MockedExtension {
//                 /// The static function id of the chain extension.
//                 fn func_id(&self) -> u32 {
//                     1101
//                 }

//                 /// The chain extension is called with the given input.
//                 ///
//                 /// Returns an error code and may fill the `output` buffer with a
//                 /// SCALE encoded result. The error code is taken from the
//                 /// `ink_env::chain_extension::FromStatusCode` implementation for
//                 /// `RandomReadErr`.
//                 fn call(&mut self, _input: &[u8], output: &mut Vec<u8>) -> u32 {
//                     let ret: [u8; 32] = [1; 32];
//                     scale::Encode::encode_to(&ret, output);
//                     0
//                 }
//             }
//             ink_env::test::register_chain_extension(MockedExtension);
//             let mut rand_extension = RandExtension::default();
//             assert_eq!(rand_extension.get(), [0; 32]);

//             // when
//             rand_extension.update([0_u8; 32]).expect("update must work");

//             // then
//             assert_eq!(rand_extension.get(), [1; 32]);
//         }
//     }
// }