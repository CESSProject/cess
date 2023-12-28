pub use crate::proto_generated::*;
use alloc::vec::Vec;
use ces_types::messaging::{MessageOrigin, SignedMessage};
pub type EgressMessages = Vec<(MessageOrigin, Vec<SignedMessage>)>;
