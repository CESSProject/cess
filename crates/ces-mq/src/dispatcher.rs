use core::marker::PhantomData;

use alloc::vec::Vec;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

use crate::simple_mpsc::{channel, ReceiveError, Receiver as RawReceiver, Sender, Seq};
use crate::types::{Message, Path};
use crate::{BindTopic, MessageOrigin};
use derive_more::Display;
use parity_scale_codec::{Decode, Error as CodecError};

impl Seq for (u64, Message) {
    fn seq(&self) -> u64 {
        self.0
    }
}

#[derive(Default, Clone)]
pub struct MessageDispatcher {
    subscribers: im::OrdMap<Path, Vec<Sender<(u64, Message)>>>,
    local_index: u64,
    //match_subscribers: Vec<Matcher, Vec<Sender<Message>>>,
}

#[derive(Clone)]
pub struct Receiver<T> {
    inner: RawReceiver<(u64, T)>,
    topic: Vec<u8>,
}

#[derive(::scale_info::TypeInfo)]
#[allow(dead_code)]
pub struct ReceiverTypeInfo {
    inner: (),
    topic: Vec<u8>,
}

impl<T> scale_info::TypeInfo for Receiver<T> {
    type Identity = <ReceiverTypeInfo as TypeInfo>::Identity;

    fn type_info() -> scale_info::Type {
        <ReceiverTypeInfo as TypeInfo>::type_info()
    }
}

impl core::ops::Deref for Receiver<Message> {
    type Target = RawReceiver<(u64, Message)>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl core::ops::DerefMut for Receiver<Message> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl MessageDispatcher {
    pub fn new() -> Self {
        MessageDispatcher {
            subscribers: Default::default(),
            local_index: 0,
        }
    }

    /// Subscribe messages which are sent to `path`.
    /// Returns a Receiver channel end.
    pub fn subscribe(&mut self, path: impl Into<Path>) -> Receiver<Message> {
        let path = path.into();
        let (rx, tx) = channel();
        let entry = self.subscribers.entry(path.clone()).or_default();
        entry.push(tx);
        Receiver {
            inner: rx,
            topic: path,
        }
    }

    /// Subscribe messages which implementing BindTopic
    /// Returns a TypedReceiver channel end.
    pub fn subscribe_bound<T: Decode + BindTopic>(&mut self) -> TypedReceiver<T> {
        self.subscribe(<T as BindTopic>::topic()).into()
    }

    /// Dispatch a message.
    /// Returns number of receivers dispatched to.
    pub fn dispatch(&mut self, message: Message) -> usize {
        let mut count = 0;
        let sn = self.local_index;
        self.local_index += 1;
        if let Some(receivers) = self.subscribers.get_mut(message.destination.path()) {
            receivers.retain(|receiver| {
                if let Err(error) = receiver.send((sn, message.clone())) {
                    use crate::simple_mpsc::SendError::*;
                    match error {
                        ReceiverGone => {
                            let dst = String::from_utf8_lossy(message.destination.path());
                            tracing::warn!(%dst, "ReceiverGone");
                            false
                        }
                    }
                } else {
                    count += 1;
                    true
                }
            });
        }
        count
    }

    pub fn reset_local_index(&mut self) {
        self.local_index = 0;
    }

    /// Drop all unhandled messages.
    pub fn clear(&mut self) -> usize {
        let mut count = 0;
        for subscriber in self.subscribers.values().flatten() {
            count += subscriber.clear();
        }
        count
    }
}

#[derive(Display, Debug)]
pub enum TypedReceiveError {
    #[display("All senders of the channel have gone")]
    SenderGone,
    #[display("Decode message failed: {_0}")]
    CodecError(CodecError),
}

impl TypedReceiveError {
    pub fn is_sender_gone(&self) -> bool {
        matches!(self, Self::SenderGone)
    }
}

impl From<CodecError> for TypedReceiveError {
    fn from(e: CodecError) -> Self {
        Self::CodecError(e)
    }
}

#[derive(Serialize, Deserialize, ::scale_info::TypeInfo)]
pub struct TypedReceiver<T> {
    queue: Receiver<Message>,
    #[codec(skip)]
    #[serde(skip)]
    _t: PhantomData<T>,
}

impl<T> Clone for TypedReceiver<T> {
    fn clone(&self) -> Self {
        Self {
            queue: self.queue.clone(),
            _t: self._t,
        }
    }
}

impl<T: Decode> TypedReceiver<T> {
    pub fn try_next(&mut self) -> Result<Option<(u64, T, MessageOrigin)>, TypedReceiveError> {
        let message = self.queue.try_next().map_err(|e| match e {
            ReceiveError::SenderGone => TypedReceiveError::SenderGone,
        })?;
        let (sn, msg) = match message {
            None => return Ok(None),
            Some(m) => m,
        };
        let typed = match Decode::decode(&mut &msg.payload[..]) {
            Ok(msg) => msg,
            Err(err) => {
                if msg.sender.always_well_formed() {
                    panic!(
                        "Failed to decode critical mq message (dest={:?}), please upgrade the ceseal client",
                        msg.destination
                    );
                } else {
                    return Err(TypedReceiveError::CodecError(err));
                }
            }
        };
        Ok(Some((sn, typed, msg.sender)))
    }

    pub fn peek_ind(&self) -> Result<Option<u64>, ReceiveError> {
        self.queue.peek_ind()
    }
}

impl<T: Decode> From<Receiver<Message>> for TypedReceiver<T> {
    fn from(queue: Receiver<Message>) -> Self {
        Self {
            queue,
            _t: Default::default(),
        }
    }
}

#[cfg(feature = "checkpoint")]
const _: () = {
    use crate::checkpoint_helper::subscribe_default;
    use serde::Serializer;

    impl Serialize for Receiver<Message> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            self.topic.serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Receiver<Message> {
        fn deserialize<D>(de: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let topic: Vec<u8> = Deserialize::deserialize(de)?;
            Ok(subscribe_default(topic))
        }
    }
};

#[macro_export]
macro_rules! select {
    (
        $( $bind:pat = $mq:expr => $block:expr, )+
    ) => {{
        let mut min = None;
        let mut min_ind = 0;
        let mut ind = 0;
        $({
            match $mq.peek_ind() {
                Ok(Some(sn)) => match min {
                    None => { min = Some(sn); min_ind = ind; }
                    Some(old) if sn < old => { min = Some(sn); min_ind = ind; }
                    _ => (),
                },
                Err(_) => { min = Some(0); min_ind = ind; }
                _ => (),
            }
            ind += 1;
        })+

        let mut ind = 0;
        let mut rv = None;
        if min.is_some() {
            $({
                if min_ind == ind {
                    let msg = $mq.try_next().transpose();
                    rv = match msg {
                        Some($bind) => Some($block),
                        None => None
                    };
                }
                ind += 1;
            })+
        }
        rv
    }};
}

#[macro_export]
macro_rules! function {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        &name[..name.len() - 3]
    }};
}

#[macro_export]
macro_rules! select_ignore_errors {
    (
        $( $bind:pat = $mq:expr => $block:expr, )+
    ) => {{
        $crate::select! {
            $(
                message = $mq => match message {
                    Ok(msg) => {
                        let $bind = (msg.1, msg.2);
                        {
                            $block
                        }
                    }
                    Err(err) if err.is_sender_gone() => {
                        log::warn!("[{}] mq error: {:?}", $crate::function!(), err);
                        panic!("mq error: {:?}", err);
                    }
                    Err(err) => {
                        log::warn!("[{}] mq ignored error: {:?}", $crate::function!(), err);
                    }
                },
            )+
        }
    }}
}
