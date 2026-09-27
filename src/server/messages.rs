pub use incoming::Message as MessageIn;
pub use incoming::MessagePayload as MessageInPayload;
pub use outgoing::Message as MessageOut;

///
/// Server to client messages
///
pub mod outgoing {
    use chrono::serde::ts_milliseconds;
    use chrono::{DateTime, Utc};
    use serde_derive::Serialize;

    use crate::server::{
        Arena, ArenaId, ArenaOverview, ArenaUpdate, LeaderboardStandings, PlayerId,
    };

    ///
    /// Outgoing messages
    ///
    #[derive(Debug, Clone, Serialize)]
    pub enum Message {
        ArenaList(Vec<ArenaOverview>),
        ArenaJoined(ArenaId, PlayerId),
        /// Another connected client has claimed the player's name.
        NameInUse,

        /// The whole arena, as it was at `time`.
        ArenaState {
            #[serde(with = "ts_milliseconds")]
            time: DateTime<Utc>,
            state: Box<Arena>,
        },
        /// The arena's changes in the update that ran at `time`.
        ArenaStatePatch {
            #[serde(with = "ts_milliseconds")]
            time: DateTime<Utc>,
            updates: Vec<ArenaUpdate>,
        },

        Leaderboard(LeaderboardStandings),
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

    use crate::server::{ArenaId, ClientId, Direction, Emote, MessageOut, Player};

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

        /// Claims a player name for this client, as soon as it connects.
        ClaimName(String),
        GetArenaList,
        Join {
            player: Player,
            arena_id: Option<ArenaId>,
        },

        Start,
        Turn(Direction),
        /// The player pressed (true) or let go of (false) the brake.
        Brake(bool),
        Emote(Emote),
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
                payload: serde_json::from_str(json)?,
            })
        }
    }
}
