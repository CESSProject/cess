use codec::Decode;
use frame_support::dispatch::DispatchResult;
use sp_runtime::DispatchError;
use ces_types::messaging::{BindTopic, DecodedMessage, Message};

pub struct MessageRouteConfig;

fn try_dispatch<Msg, Func>(func: Func, message: &Message) -> DispatchResult
where
    Msg: Decode + BindTopic,
    Func: Fn(DecodedMessage<Msg>) -> DispatchResult,
{
    if message.destination.path() == &Msg::topic() {
        let msg: DecodedMessage<Msg> = message
            .decode()
            .ok_or(DispatchError::Other("MessageCodecError"))?;
        return (func)(msg);
    }
    Ok(())
}

impl ces_pallet_mq::QueueNotifyConfig for MessageRouteConfig {
    /// Handles an incoming message
    fn on_message_received(message: &Message) -> DispatchResult {
        use super::*;
        macro_rules! route_handlers {
            ($($handler: path,)+) => {
                $(try_dispatch($handler, message)?;)+
            }
        }

        route_handlers! {
            TeeWorker::on_message_received,
        };
        Ok(())
    }
}
