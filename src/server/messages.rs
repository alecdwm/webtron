pub use incoming::Message as MessageIn;
pub use incoming::MessagePayload as MessageInPayload;
pub use outgoing::Message as MessageOut;

///
/// Server to client messages
///
pub mod outgoing {
    use actix::Message as ActixMessage;
    use chrono::{DateTime, Utc};
    use serde_derive::Serialize;

    use crate::server::{Arena, ArenaUpdates, GameId, NetworkPlayer, PlayerId};

    ///
    /// Outgoing messages
    ///
    #[derive(Debug, Clone, Serialize, ActixMessage)]
    pub enum Message {
        PlayerId(PlayerId),
        TotalGames(usize),

        JoinedGame(GameId),
        PartedGame,

        GamePlayers(Vec<NetworkPlayer>),
        GameStarting(DateTime<Utc>),

        NewGameState(Arena),
        PatchGameState(ArenaUpdates),
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
    use debug_stub_derive::DebugStub;
    use failure::Error;
    use serde_derive::Deserialize;

    use crate::server::{ClientId, Direction, GameId, MessageOut, PlayerColor};

    ///
    /// Incoming messages
    ///
    #[derive(Debug, ActixMessage)]
    pub struct Message {
        pub client_id: ClientId,
        pub payload: MessagePayload,
    }

    #[derive(Debug, Deserialize)]
    pub enum MessagePayload {
        #[serde(skip)]
        Connection(ConnectionMessage),

        Matchmaking(MatchmakingMessage),
        GameInput(GameInputMessage),
    }

    #[derive(DebugStub)]
    pub enum ConnectionMessage {
        Connect(
            Option<String>,
            #[debug_stub = "Recipient<MessageOut>"] Recipient<MessageOut>,
        ),
        Disconnect,
    }

    #[derive(Debug, Deserialize)]
    pub enum MatchmakingMessage {
        ConfigurePlayer { name: String, color: PlayerColor },
        JoinGame(Option<GameId>),
        PartGame,
    }

    #[derive(Debug, Deserialize)]
    pub enum GameInputMessage {
        StartGame,
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
                payload: MessagePayload::Connection(ConnectionMessage::Connect(
                    ip_address, address,
                )),
            }
        }

        pub fn disconnect(client_id: ClientId) -> Self {
            Self {
                client_id,
                payload: MessagePayload::Connection(ConnectionMessage::Disconnect),
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
