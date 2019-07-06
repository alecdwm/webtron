use crate::game::Game;
use crate::game::Player;
use actix::{Message, Recipient};
use debug_stub_derive::DebugStub;
use failure::Error;
use serde_derive::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Message)]
pub enum MessageOut {
    PlayerId(Uuid),
    LobbyData {
        games: Vec<Game>,
        players: Vec<Player>,
    },
    // GamesList { games: Vec<Game> },
    // PlayersList { players: Vec<Player> },
    PlayerSpawned {
        player: Player,
        x: f64,
        y: f64,
    },
    // GameState(GameState),
    PlayerDeath(Player),
}

impl MessageOut {
    pub fn to_json(&self) -> Result<String, serde_json::error::Error> {
        serde_json::to_string(self)
    }
}

#[derive(Debug, Message)]
pub struct MessageIn {
    pub(super) client_id: Uuid,
    pub(super) data: MessageInData,
}

#[derive(Debug, Deserialize)]
pub(super) enum MessageInData {
    #[serde(skip)]
    Client(ClientMessageIn),

    Lobby(LobbyMessageIn),
    InGame(InGameMessageIn),
}

#[derive(DebugStub)]
pub(super) enum ClientMessageIn {
    Connect(
        Option<String>,
        #[debug_stub = "Recipient<MessageOut>"] Recipient<MessageOut>,
    ),
    Disconnect,
}

#[derive(Debug, Deserialize)]
pub(super) enum LobbyMessageIn {
    ConfigurePlayer(Player),

    FetchLobbyData,
    CreateGame(String),
    JoinGame(Uuid),
    LeaveGame,
}

#[derive(Debug, Deserialize)]
pub(super) enum InGameMessageIn {
    Spawn,
    Turn(TurnDirection),
}

#[derive(Debug, Deserialize)]
pub(super) enum TurnDirection {
    Left,
    Right,
}

impl MessageIn {
    pub fn connect(
        client_id: Uuid,
        ip_address: Option<String>,
        addr: Recipient<MessageOut>,
    ) -> Self {
        Self {
            client_id,
            data: MessageInData::Client(ClientMessageIn::Connect(ip_address, addr)),
        }
    }

    pub fn disconnect(client_id: Uuid) -> Self {
        Self {
            client_id,
            data: MessageInData::Client(ClientMessageIn::Disconnect),
        }
    }

    pub fn from_json(client_id: Uuid, json: &str) -> Result<Self, Error> {
        Ok(Self {
            client_id,
            data: serde_json::from_str(&json)?,
        })
    }
}
