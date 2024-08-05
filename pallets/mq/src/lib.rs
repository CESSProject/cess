#![cfg_attr(not(feature = "std"), no_std)]

//! The message queue to connect components in the network

extern crate alloc;

/// Provides `SignedExtension` to check message sequence.
mod check_seq;

pub use self::pallet::*;
pub use check_seq::{tag, CheckMqSequence};
pub use frame_support::storage::generator::StorageMap as StorageMapTrait;

#[frame_support::pallet]
pub mod pallet {
	use ces_types::{
		messaging::{BindTopic, Message, MessageOrigin, Path, SignedMessage},
		wrap_content_to_sign, MasterPublicKey, SignedContentType, WorkerPublicKey,
	};
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::{PalletInfo, StorageVersion},
	};
	use frame_system::pallet_prelude::*;
	use primitive_types::H256;
	use sp_core::sr25519;
	use sp_std::vec::Vec;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type QueueNotifyConfig: QueueNotifyConfig;
		type CallMatcher: CallMatcher<Self>;
		type MasterPubkeySupplier: MasterPubkeySupplier;
	}

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// The next expected sequence of a ingress message coming from a certain sender (origin)
	#[pallet::storage]
	pub type OffchainIngress<T> = StorageMap<_, Twox64Concat, MessageOrigin, u64>;

	#[pallet::storage]
	pub type QueuedOutboundMessage<T> = StorageValue<_, Vec<Message>>;

	/// Outbound messages at the current block.
	///
	/// It will be cleared at the beginning of every block.
	#[pallet::storage]
	#[pallet::getter(fn messages)]
	pub type OutboundMessages<T> = StorageValue<_, Vec<Message>, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		BadSender,
		BadSequence,
		BadDestination,
		CannotHandleUnknownMessage,
		MalformedSignature,
		InvalidSignatureLength,
		InvalidSignature,
		MasterKeyUninitialized,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: IntoH256,
	{
		/// Syncs an unverified offchain message to the message queue
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(10_000u64, 0) + T::DbWeight::get().writes(1u64))]
		pub fn sync_offchain_message(origin: OriginFor<T>, signed_message: SignedMessage) -> DispatchResult {
			ensure_signed(origin)?;

			// Check sender
			let sender = &signed_message.message.sender;
			ensure!(sender.is_offchain(), Error::<T>::BadSender);

			// Check destination
			ensure!(signed_message.message.destination.is_valid(), Error::<T>::BadDestination);

			// Check ingress sequence
			let expected_seq = OffchainIngress::<T>::get(sender).unwrap_or(0);
			ensure!(signed_message.sequence == expected_seq, Error::<T>::BadSequence);
			// Validate signature
			Self::check_message(&signed_message)?;
			// Update ingress
			OffchainIngress::<T>::insert(sender.clone(), expected_seq + 1);
			// Call dispatch_message
			Self::dispatch_message(signed_message.message);
			Ok(())
		}

		// Messaging API for end user.
		// TODO: confirm the weight
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(10_000u64, 0) + T::DbWeight::get().writes(1u64))]
		pub fn push_message(origin: OriginFor<T>, destination: Vec<u8>, payload: Vec<u8>) -> DispatchResult {
			let origin = ensure_signed(origin)?;
			let sender = MessageOrigin::AccountId(origin.into_h256());
			let message = Message::new(sender, destination, payload);
			Self::dispatch_message(message);
			Ok(())
		}

		// Force push a from-pallet message.
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(10_000u64, 0) + T::DbWeight::get().writes(1u64))]
		pub fn force_push_pallet_message(
			origin: OriginFor<T>,
			destination: Vec<u8>,
			payload: Vec<u8>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let sender = MessageOrigin::Pallet(b"ForcePushed".to_vec());
			let message = Message::new(sender, destination, payload);
			Self::dispatch_message(message);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn check_message(message: &SignedMessage) -> DispatchResult {
			let pubkey_copy: sr25519::Public;
			let pubkey = match &message.message.sender {
				MessageOrigin::Worker(pubkey) => pubkey,
				MessageOrigin::Keyfairy => {
					// MasterPublicKey should not be None
					pubkey_copy = T::MasterPubkeySupplier::try_get().ok_or(Error::<T>::MasterKeyUninitialized)?;
					&pubkey_copy
				},
				_ => return Err(Error::<T>::CannotHandleUnknownMessage.into()),
			};
			Self::verify_signature(pubkey, message)
		}

		fn verify_signature(pubkey: &WorkerPublicKey, message: &SignedMessage) -> DispatchResult {
			let raw_sig = &message.signature;
			ensure!(raw_sig.len() == 64, Error::<T>::InvalidSignatureLength);
			let sig =
				sp_core::sr25519::Signature::try_from(raw_sig.as_slice()).or(Err(Error::<T>::MalformedSignature))?;
			let data = message.data_be_signed();
			let data = wrap_content_to_sign(&data, SignedContentType::MqMessage);
			ensure!(sp_io::crypto::sr25519_verify(&sig, &data, pubkey), Error::<T>::InvalidSignature);
			Ok(())
		}

		/// Push a validated message to the queue
		pub fn dispatch_message(message: Message) {
			// Notify subscribers
			if let Err(_err) = T::QueueNotifyConfig::on_message_received(&message) {
				// TODO: Consider to emit a message as warning. We can't stop dispatching message in any situation.
			}
			// Notify the off-chain components
			if T::QueueNotifyConfig::should_push_message(&message) {
				OutboundMessages::<T>::append(message);
			}
		}

		pub fn push_message_to<M: Encode>(topic: impl Into<Path>, sender: MessageOrigin, payload: M) {
			let message = Message::new(sender, topic, payload.encode());
			Self::dispatch_message(message);
		}

		pub fn push_bound_message<M: Encode + BindTopic>(sender: MessageOrigin, payload: M) {
			Self::push_message_to(M::topic(), sender, payload)
		}

		pub fn queue_bound_message<M: Encode + BindTopic>(sender: MessageOrigin, payload: M) {
			let message = Message::new(sender, M::topic(), payload.encode());
			QueuedOutboundMessage::<T>::append(message);
		}

		pub fn offchain_ingress(sender: &MessageOrigin) -> Option<u64> {
			OffchainIngress::<T>::get(sender)
		}

		pub fn reset_ingress_channel_seq(sender: MessageOrigin) {
			OffchainIngress::<T>::remove(sender);
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_now: BlockNumberFor<T>) -> Weight {
			// Clear the previously pushed offchain messages
			OutboundMessages::<T>::kill();

			// Send out queued message from the previous block
			if let Some(msgs) = QueuedOutboundMessage::<T>::take() {
				for message in msgs.into_iter() {
					Self::dispatch_message(message);
				}
			}

			Weight::zero()
		}
	}

	/// Defines the behavior of received messages.
	pub trait QueueNotifyConfig {
		/// If true, the message queue push this message to the subscribers
		fn should_push_message(message: &Message) -> bool {
			message.destination.is_offchain()
		}
		/// Handles an incoming message
		fn on_message_received(_message: &Message) -> DispatchResult {
			Ok(())
		}
	}
	impl QueueNotifyConfig for () {}

	/// Needs an external helper struct to extract MqCall from all callables
	pub trait CallMatcher<T: Config> {
		fn match_call(call: &T::RuntimeCall) -> Option<&Call<T>>
		where
			<T as frame_system::Config>::AccountId: IntoH256;
	}

	pub trait IntoH256 {
		fn into_h256(self) -> H256;
	}

	impl IntoH256 for u32 {
		fn into_h256(self) -> H256 {
			H256::from_low_u64_be(self as _)
		}
	}

	impl IntoH256 for u64 {
		fn into_h256(self) -> H256 {
			H256::from_low_u64_be(self)
		}
	}

	impl IntoH256 for sp_runtime::AccountId32 {
		fn into_h256(self) -> H256 {
			let bytes: [u8; 32] = *self.as_ref();
			bytes.into()
		}
	}

	pub trait MessageOriginInfo: Sized + 'static {
		type Config: Config;

		fn message_origin() -> MessageOrigin {
			let name = <<Self as MessageOriginInfo>::Config as frame_system::Config>::PalletInfo::name::<Self>()
				.expect("Pallet should have a name");
			MessageOrigin::Pallet(name.as_bytes().to_vec())
		}

		fn push_message(payload: impl Encode + BindTopic) {
			Pallet::<Self::Config>::push_bound_message(Self::message_origin(), payload);
		}

		fn push_message_to(topic: impl Into<Path>, payload: impl Encode) {
			Pallet::<Self::Config>::push_message_to(topic, Self::message_origin(), payload);
		}

		/// Enqueues a message to push in the beginning of the next block
		fn queue_message(payload: impl Encode + BindTopic) {
			Pallet::<Self::Config>::queue_bound_message(Self::message_origin(), payload);
		}

		fn reset_ingress_channel_seq(sender: MessageOrigin) {
			Pallet::<Self::Config>::reset_ingress_channel_seq(sender);
		}
	}

	pub trait MasterPubkeySupplier {
		fn try_get() -> Option<MasterPublicKey> {
			None
		}
	}
	impl MasterPubkeySupplier for () {}
}
