pub use incoming::Message as MessageIn;
pub use incoming::MessagePayload as MessageInPayload;
pub use outgoing::Message as MessageOut;

///
/// Server to client messages
///
pub mod outgoing {
    use serde_derive::Serialize;

    use crate::server::{Arena, ArenaId, ArenaOverview, ArenaUpdate, PlayerId};

    ///
    /// Outgoing messages
    ///
    #[derive(Debug, Clone, Serialize)]
    pub enum Message {
        ArenaList(Vec<ArenaOverview>),
        ArenaJoined(ArenaId, PlayerId),

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
    use anyhow::Error;
    use serde_derive::Deserialize;
    use tokio::sync::mpsc::Sender;

    use crate::server::{ArenaId, ClientId, Direction, MessageOut, Player};

    ///
    /// Incoming messages
    ///
    #[derive(Debug)]
    pub struct Message {
        pub client_id: ClientId,
        pub payload: MessagePayload,
    }

    #[derive(Debug, Deserialize)]
    pub enum MessagePayload {
        #[serde(skip)]
        Connect(Option<String>, Sender<MessageOut>),
        #[serde(skip)]
        Disconnect,

        GetArenaList,
        Join {
            player: Player,
            arena_id: Option<ArenaId>,
        },

        Start,
        Turn(Direction),
    }

    impl Message {
        pub fn connect(
            client_id: ClientId,
            ip_address: Option<String>,
            tx: Sender<MessageOut>,
        ) -> Self {
            Self {
                client_id,
                payload: MessagePayload::Connect(ip_address, tx),
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
