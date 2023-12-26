use super::{Call, CallMatcher, Config, IntoH256, OffchainIngress};

use ces_types::messaging::MessageOrigin;
use frame_support::dispatch::DispatchInfo;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::traits::{DispatchInfoOf, Dispatchable, SignedExtension};
use sp_runtime::transaction_validity::{
	InvalidTransaction, TransactionValidity, TransactionValidityError, ValidTransaction,
};
use sp_std::marker::PhantomData;
use sp_std::vec;
use sp_std::vec::Vec;

/// Requires a message queue message must has correct sequence id.
///
/// We only care about `sync_offchain_message` call.
///
/// When a message comes to the transaction pool, we drop it immediately if its sequence is
/// less than the expected one. Otherwise we keep the message in the pool for a while, hoping there
/// will be a sequence of continuous messages to be included in the future block.
#[derive(Encode, Decode, TypeInfo, Clone, Eq, PartialEq)]
#[scale_info(skip_type_params(T))]
pub struct CheckMqSequence<T>(PhantomData<T>);

pub fn tag(sender: &MessageOrigin, seq: u64) -> Vec<u8> {
	("CesMqOffchainMessages", sender, seq).encode()
}

impl<T> Default for CheckMqSequence<T> {
	fn default() -> Self {
		Self(Default::default())
	}
}

impl<T> CheckMqSequence<T> {
	pub fn new() -> Self {
		Default::default()
	}
}

impl<T: Config> sp_std::fmt::Debug for CheckMqSequence<T> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "CheckMqSequence()")
	}

	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		Ok(())
	}
}

impl<T: Config> SignedExtension for CheckMqSequence<T>
where
	T::RuntimeCall: Dispatchable<Info = DispatchInfo>,
	T: Send + Sync,
	T::AccountId: IntoH256,
{
	const IDENTIFIER: &'static str = "CheckMqSequence";
	type AccountId = T::AccountId;
	type Call = T::RuntimeCall;
	type AdditionalSigned = ();
	type Pre = ();

	fn additional_signed(&self) -> sp_std::result::Result<(), TransactionValidityError> {
		Ok(())
	}

	fn pre_dispatch(
		self,
		_who: &Self::AccountId,
		call: &Self::Call,
		_info: &DispatchInfoOf<Self::Call>,
		_len: usize,
	) -> Result<(), TransactionValidityError> {
		let signed_message = match T::CallMatcher::match_call(call) {
			Some(Call::sync_offchain_message { signed_message }) => signed_message,
			_ => return Ok(()),
		};
		let sender = &signed_message.message.sender;
		let sequence = signed_message.sequence;
		let expected_seq = OffchainIngress::<T>::get(sender).unwrap_or(0);
		// Strictly require the message to include must match the expected sequence id
		if sequence != expected_seq {
			return Err(if sequence < expected_seq {
				InvalidTransaction::Stale
			} else {
				InvalidTransaction::Future
			}
			.into());
		}
		Ok(())
	}

	fn validate(
		&self,
		_who: &Self::AccountId,
		call: &Self::Call,
		_info: &DispatchInfoOf<Self::Call>,
		_len: usize,
	) -> TransactionValidity {
		let signed_message = match T::CallMatcher::match_call(call) {
			Some(Call::sync_offchain_message { signed_message }) => signed_message,
			_ => return Ok(ValidTransaction::default()),
		};
		let sender = &signed_message.message.sender;
		let sequence = signed_message.sequence;
		let expected_seq = OffchainIngress::<T>::get(sender).unwrap_or(0);
		// Drop the stale message immediately
		if sequence < expected_seq {
			return InvalidTransaction::Stale.into();
		}

		// Otherwise build a dependency graph based on (sender, sequence), hoping that it can be
		// included later
		let provides = vec![tag(sender, sequence)];
		let requires = if sequence > expected_seq {
			vec![tag(sender, sequence - 1)]
		} else {
			vec![]
		};
		Ok(ValidTransaction {
			provides,
			requires,
			..Default::default()
		})
	}
}
