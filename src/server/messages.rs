pub use incoming::Message as MessageIn;
pub use incoming::MessagePayload as MessageInPayload;
pub use outgoing::Message as MessageOut;

///
/// Server to client messages
///
pub mod outgoing {
    use actix::Message as ActixMessage;
    use serde_derive::Serialize;

    use crate::server::{Arena, ArenaId, ArenaOverview, ArenaUpdate};

    ///
    /// Outgoing messages
    ///
    #[derive(Debug, Clone, Serialize, ActixMessage)]
    #[rtype(result = "()")]
    pub enum Message {
        ArenaList(Vec<ArenaOverview>),
        ArenaJoined(ArenaId),

        ArenaState(Box<Arena>),
        ArenaStatePatch(Vec<ArenaUpdate>),
    }

    impl Message {
        pub fn to_json(&self) -> Result<String, serde_json::error::Error> {
            serde_json::to_string(self)
        }
    }
}

///
/// Client to server messages
///
pub mod incoming {
    use actix::{Message as ActixMessage, Recipient};
    use anyhow::Error;
    use serde_derive::Deserialize;

    use crate::server::{ArenaId, ClientId, Direction, MessageOut, Player};

    ///
    /// Incoming messages
    ///
    #[derive(Debug, ActixMessage)]
    #[rtype(result = "()")]
    pub struct Message {
        pub client_id: ClientId,
        pub payload: MessagePayload,
    }

    #[derive(Debug, Deserialize)]
    pub enum MessagePayload {
        #[serde(skip)]
        Connect(Option<String>, Recipient<MessageOut>),
        #[serde(skip)]
        Disconnect,

        GetArenaList,
        Join(Player, Option<ArenaId>),

        Start,
        Turn(Direction),
    }

    impl Message {
        pub fn connect(
            client_id: ClientId,
            ip_address: Option<String>,
            address: Recipient<MessageOut>,
        ) -> Self {
            Self {
                client_id,
                payload: MessagePayload::Connect(ip_address, address),
            }
        }

        pub fn disconnect(client_id: ClientId) -> Self {
            Self {
                client_id,
                payload: MessagePayload::Disconnect,
            }
        }

        pub fn from_json(client_id: ClientId, json: &str) -> Result<Self, Error> {
            Ok(Self {
                client_id,
                payload: serde_json::from_str(&json)?,
            })
        }
    }
}
