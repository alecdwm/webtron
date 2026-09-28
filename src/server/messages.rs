pub use incoming::Message as MessageIn;
pub use incoming::MessagePayload as MessageInPayload;
pub use outgoing::Message as MessageOut;

///
/// Server to client messages
///
pub mod outgoing {
    use std::sync::{Arc, OnceLock};

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
        /// The arena's changes in the update that ran at `time`. One patch
        /// goes to every client in the arena, and its clones share `updates`
        /// and `json`, which holds the patch as JSON once the first client's
        /// copy of it has been serialized, for the rest to reuse.
        ArenaStatePatch {
            #[serde(with = "ts_milliseconds")]
            time: DateTime<Utc>,
            updates: Arc<Vec<ArenaUpdate>>,
            #[serde(skip)]
            json: Arc<OnceLock<String>>,
        },

        Leaderboard(LeaderboardStandings),
    }

    impl Message {
        /// A patch of the changes in `updates`, made in the update that ran
        /// at `time`.
        pub fn patch(time: DateTime<Utc>, updates: Vec<ArenaUpdate>) -> Self {
            Message::ArenaStatePatch {
                time,
                updates: Arc::new(updates),
                json: Default::default(),
            }
        }

        pub fn to_json(&self) -> Result<String, serde_json::error::Error> {
            let Message::ArenaStatePatch { json, .. } = self else {
                return serde_json::to_string(self);
            };
            if let Some(text) = json.get() {
                return Ok(text.clone());
            }
            let text = serde_json::to_string(self)?;
            // Another client's copy may have been serialized meanwhile, to
            // the same JSON.
            let _ = json.set(text.clone());
            Ok(text)
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

    use crate::server::{ArenaId, ClientId, Direction, Emote, MessageOut, Player, Scenario};

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
        /// Starts the next round set up for testing. Nothing in the client
        /// points to it.
        StartScenario(Scenario),
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

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::MessageOut;
    use crate::server::ArenaUpdate;

    #[test]
    fn a_patch_and_its_clones_serialize_the_same() {
        let patch = MessageOut::patch(Utc::now(), vec![ArenaUpdate::End]);
        let clone = patch.clone();
        let fresh = serde_json::to_string(&patch).unwrap();
        assert_eq!(patch.to_json().unwrap(), fresh);
        assert_eq!(clone.to_json().unwrap(), fresh);
    }
}
