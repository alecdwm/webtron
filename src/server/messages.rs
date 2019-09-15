pub use incoming::Message as MessageIn;
pub use incoming::MessageHandler as MessageInHandler;
pub use outgoing::Message as MessageOut;

///
/// Server to client messages
///
pub mod outgoing {
    use actix::Message as ActixMessage;
    use chrono::{DateTime, Utc};
    use serde_derive::Serialize;
    use uuid::Uuid;

    use crate::server::{GameState, Player};

    ///
    /// Outgoing messages
    ///
    #[derive(Debug, Serialize, ActixMessage)]
    pub enum Message {
        PlayerId(Uuid),
        TotalGames(u64),

        JoinedGame(Uuid),
        PartedGame,

        GamePlayers(Vec<Player>),
        GameStarting(DateTime<Utc>),

        NewGameState(GameState),
        PatchGameState(GameState),
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
    use uuid::Uuid;

    use crate::server::{Direction, MessageOut, Player};

    ///
    /// Incoming messages
    ///
    #[derive(Debug, ActixMessage)]
    pub struct Message {
        pub client_id: Uuid,
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
        ConfigurePlayer(Player),

        JoinGame(Option<Uuid>),
        PartGame,
    }

    #[derive(Debug, Deserialize)]
    pub enum GameInputMessage {
        StartGame,
        Turn(Direction),
    }

    impl Message {
        pub fn connect(
            client_id: Uuid,
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

        pub fn disconnect(client_id: Uuid) -> Self {
            Self {
                client_id,
                payload: MessagePayload::Connection(ConnectionMessage::Disconnect),
            }
        }

        pub fn from_json(client_id: Uuid, json: &str) -> Result<Self, Error> {
            Ok(Self {
                client_id,
                payload: serde_json::from_str(&json)?,
            })
        }
    }

    pub trait MessageHandler {
        fn handle_connection_message(
            &mut self,
            client_id: Uuid,
            payload: ConnectionMessage,
        ) -> Result<(), Error>;

        fn handle_matchmaking_message(
            &mut self,
            client_id: Uuid,
            payload: MatchmakingMessage,
        ) -> Result<(), Error>;

        fn handle_game_input_message(
            &mut self,
            client_id: Uuid,
            payload: GameInputMessage,
        ) -> Result<(), Error>;
    }

    impl Message {
        pub fn handle_with(self, handler: &mut impl MessageHandler) -> Result<(), Error> {
            let client_id = self.client_id;

            match self.payload {
                MessagePayload::Connection(message) => {
                    handler.handle_connection_message(client_id, message)
                }
                MessagePayload::Matchmaking(message) => {
                    handler.handle_matchmaking_message(client_id, message)
                }
                MessagePayload::GameInput(message) => {
                    handler.handle_game_input_message(client_id, message)
                }
            }
        }
    }
}
