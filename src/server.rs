mod arena;
mod leaderboard;
mod messages;
mod primitives;

use anyhow::{Context as ResultContext, Error, anyhow, bail};
use chrono::Utc;
use log::{debug, error, info, warn};
use std::collections::HashMap;
use std::mem;
use std::time::Duration;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::time::{self, Instant, MissedTickBehavior};

pub use arena::{Arena, ArenaInput, ArenaOverview, ArenaUpdate, DeathCause};
pub use leaderboard::{Leaderboard, LeaderboardStandings, Stat};
pub use messages::{MessageIn, MessageOut};
pub use primitives::*;

use crate::{get_error_chain, truncate_for_log};
use leaderboard::normalize_name;
use messages::MessageInPayload;

const UPDATE_RATE_MILLISECONDS: u64 = 50; // 1000 / 50 = 20 updates per second
/// Longest time one update covers, in seconds, however long it has been
/// since the last one, so that a late update doesn't move lightcycles
/// unusually far in one step.
const MAX_UPDATE_SECONDS: f64 = 0.1;
/// Most messages from clients handled in one update. The rest wait for the
/// next update.
const MAX_MESSAGES_PER_UPDATE: usize = 512;
/// Most arenas open at once.
const MAX_ARENAS: usize = 64;
/// Longest player name, in characters, the same as the client's name input.
const MAX_PLAYER_NAME_LENGTH: usize = 10;

#[derive(Debug)]
pub struct Server {
    message_queue: Receiver<MessageIn>,
    clients: HashMap<ClientId, Client>,
    arenas: HashMap<ArenaId, Arena>,
    leaderboard: Leaderboard,
    /// Clients whose queue of outgoing messages is full, to be disconnected
    /// at the end of the update.
    lagging: Vec<ClientId>,
}

impl Server {
    pub fn new(message_queue: Receiver<MessageIn>, leaderboard: Leaderboard) -> Self {
        Self {
            message_queue,
            clients: Default::default(),
            arenas: Default::default(),
            leaderboard,
            lagging: Default::default(),
        }
    }

    pub async fn start(mut self) {
        let mut interval = time::interval(Duration::from_millis(UPDATE_RATE_MILLISECONDS));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let mut last_update = Instant::now();
        loop {
            interval.tick().await;
            let now = Instant::now();
            let delta_time = (now - last_update).as_secs_f64().min(MAX_UPDATE_SECONDS);
            last_update = now;

            self.tick(delta_time);
        }
    }

    /// Handles the messages that have arrived, updates every arena by
    /// `delta_time` seconds and sends clients the changes.
    pub fn tick(&mut self, delta_time: f64) {
        self.process_messages();
        self.update(delta_time);
        self.send_updates();
        self.disconnect_lagging_clients();
    }

    pub fn process_messages(&mut self) {
        for _ in 0..MAX_MESSAGES_PER_UPDATE {
            let Ok(message) = self.message_queue.try_recv() else {
                break;
            };
            self.handle_message(message.client_id, message.payload)
                .unwrap_or_else(|error| {
                    warn!(
                        "Failed to process incoming message: {}",
                        get_error_chain(error)
                    )
                })
        }
    }

    pub fn update(&mut self, delta_time: f64) {
        self.arenas.retain(|_, arena| {
            arena.update(delta_time);

            // discard arena if all human players have left
            arena.human_player_ids().next().is_some()
        });

        let ids: Vec<ArenaId> = self.arenas.keys().copied().collect();
        for arena_id in ids {
            self.record_leaderboard_events(arena_id);
        }
    }

    /// Adds this update's rounds, wins, kills and deaths of human players to
    /// the leaderboard. If the round was won, sends the leaderboard to everyone
    /// in the arena. The leaderboard file is used off the game loop.
    fn record_leaderboard_events(&mut self, arena_id: ArenaId) {
        let Some(arena) = self.arenas.get_mut(&arena_id) else {
            return;
        };
        let winner = arena.unrecorded_winner.take();
        let deaths = mem::take(&mut arena.unrecorded_deaths);
        let round_players = mem::take(&mut arena.unrecorded_round_players);
        let arena = &*arena;

        let human = |id: &PlayerId| human_player(arena, id);

        let mut events: Vec<(String, Stat)> = Vec::new();
        for player in round_players {
            if let Some(player) = human(&player) {
                events.push((player.name.clone(), Stat::RoundPlayed));
            }
        }
        for (victim, cause) in deaths {
            events.extend(death_events(arena, victim, cause));
        }
        if let Some(player) = winner.and_then(|winner| human(&winner)) {
            events.push((player.name.clone(), Stat::Win(player.color)));
        }

        if events.is_empty() && winner.is_none() {
            return;
        }

        // Only a won round shows the leaderboard.
        let recipients: Vec<(Sender<MessageOut>, Option<String>)> = match winner {
            Some(_) => self
                .clients
                .values()
                .filter(|client| client.arena == Some(arena.id))
                .map(|client| {
                    let name = client
                        .player
                        .and_then(|id| arena.players.get(&id))
                        .map(|player| player.name.clone());
                    (client.tx.clone(), name)
                })
                .collect(),
            None => Vec::new(),
        };

        let leaderboard = self.leaderboard.clone();
        tokio::task::spawn_blocking(move || {
            let stats = match leaderboard.record(&events) {
                Ok(stats) => stats,
                Err(error) => {
                    return error!("Failed to update leaderboard: {}", get_error_chain(error));
                }
            };

            for (tx, name) in recipients {
                let standings = stats.standings(name.as_deref());
                if let Err(error) = tx.try_send(MessageOut::Leaderboard(standings)) {
                    warn!("Failed to send Leaderboard to client: {}", error);
                }
            }
        });
    }

    /// Sends each client the full state of its arena if it hasn't had it yet,
    /// otherwise the changes made this update, then clears the changes.
    pub fn send_updates(&mut self) {
        let Self {
            clients,
            arenas,
            lagging,
            ..
        } = self;

        for client in clients.values_mut() {
            let Some(arena) = client.arena.and_then(|id| arenas.get(&id)) else {
                continue;
            };

            if !client.synced {
                client.synced = send(client, MessageOut::ServerTime(Utc::now()), lagging)
                    && send(
                        client,
                        MessageOut::ArenaState(Box::new(arena.clone())),
                        lagging,
                    );
            } else if !arena.updates.is_empty() {
                send(
                    client,
                    MessageOut::ArenaStatePatch(arena.updates.clone()),
                    lagging,
                );
            }
        }

        for arena in arenas.values_mut() {
            arena.clear_updates();
        }
    }

    /// Disconnects the clients that aren't keeping up with their messages.
    /// Dropping a client's sender closes its websocket.
    fn disconnect_lagging_clients(&mut self) {
        for client_id in mem::take(&mut self.lagging) {
            warn!(
                "Disconnecting client {} that isn't keeping up with its messages",
                client_id
            );
            self.remove_client(client_id);
        }
    }

    fn remove_client(&mut self, client_id: ClientId) {
        self.client_part_arena(client_id);
        self.clients.remove(&client_id);
    }
}

/// Queues a message for a client, noting a client whose queue is full as
/// lagging. Returns whether the message was queued.
fn send(client: &Client, message: MessageOut, lagging: &mut Vec<ClientId>) -> bool {
    match client.tx.try_send(message) {
        Ok(()) => true,
        Err(TrySendError::Full(_)) => {
            if !lagging.contains(&client.id) {
                lagging.push(client.id);
            }
            false
        }
        // The websocket has closed, and its disconnect message is queued.
        Err(TrySendError::Closed(_)) => false,
    }
}

/// Puts a player's name in the form the client's name input produces,
/// refusing a name that is then empty, longer than `MAX_PLAYER_NAME_LENGTH`
/// characters or has control characters in it.
fn validate_name(name: &str) -> Result<String, Error> {
    let normalized = normalize_name(name);
    let length = normalized.chars().count();
    if length == 0 || length > MAX_PLAYER_NAME_LENGTH || normalized.chars().any(char::is_control) {
        bail!("Invalid player name {:?}", truncate_for_log(name));
    }
    Ok(normalized)
}

impl Server {
    /// Claims `name` for the client, unless another connected client has
    /// already claimed it, in which case the client is told so. `name` must
    /// be in the form `validate_name` gives. Returns whether the claim
    /// succeeded.
    fn claim_name(&mut self, client_id: ClientId, name: &str) -> Result<bool, Error> {
        let in_use = self
            .clients
            .values()
            .any(|client| client.id != client_id && client.name.as_deref() == Some(name));

        let client = self
            .clients
            .get_mut(&client_id)
            .ok_or_else(|| anyhow!("Client {} not found", client_id))?;

        if in_use {
            send(client, MessageOut::NameInUse, &mut self.lagging);
            info!("Client {} tried to claim a name in use", client_id);
            return Ok(false);
        }

        client.name = Some(name.to_string());
        Ok(true)
    }

    pub fn new_arena(&mut self, name: &str) -> ArenaId {
        let arena = Arena::with_name(name);
        let id = arena.id;

        self.arenas.insert(id, arena);

        id
    }

    pub fn client_input(&mut self, client_id: ClientId, input: ArenaInput) -> Result<(), Error> {
        let client = self
            .clients
            .get(&client_id)
            .with_context(|| anyhow!("Client {} not found", client_id))?;

        let arena_id = client
            .arena
            .with_context(|| anyhow!("Client {} not in an arena", client_id))?;

        let arena = self
            .arenas
            .get_mut(&arena_id)
            .with_context(|| anyhow!("Arena {} not found", arena_id))?;

        let player_id = client
            .player
            .with_context(|| anyhow!("Client {} has no player", client_id))?;

        arena.process_input(player_id, input);

        Ok(())
    }

    /// Takes the client out of its arena, if it is in one.
    pub fn client_part_arena(&mut self, client_id: ClientId) {
        let Some(client) = self.clients.get_mut(&client_id) else {
            return;
        };
        client.synced = false;
        let (Some(arena_id), Some(player_id)) = (client.arena.take(), client.player.take()) else {
            return;
        };
        if let Some(arena) = self.arenas.get_mut(&arena_id) {
            arena.remove_player(player_id);
        }
    }
}

impl Server {
    pub fn handle_message(
        &mut self,
        client_id: ClientId,
        payload: MessageInPayload,
    ) -> Result<(), Error> {
        match payload {
            MessageInPayload::Connect(ip_address, tx) => {
                info!(
                    "Client {} connected from {}",
                    client_id,
                    ip_address.as_deref().unwrap_or("an unknown address")
                );
                let client = Client {
                    id: client_id,
                    tx,
                    player: None,
                    arena: None,
                    synced: false,
                    name: None,
                };
                send(
                    &client,
                    MessageOut::ServerTime(Utc::now()),
                    &mut self.lagging,
                );
                self.clients.insert(client_id, client);
            }
            MessageInPayload::Disconnect => {
                // A client that fell behind has been removed already.
                if self.clients.contains_key(&client_id) {
                    info!("Client disconnected: {}", client_id);
                    self.remove_client(client_id);
                } else {
                    debug!("Client disconnected after removal: {}", client_id);
                }
            }

            MessageInPayload::ClaimName(name) => {
                let client = self
                    .clients
                    .get(&client_id)
                    .ok_or_else(|| anyhow!("Client {} not found", client_id))?;
                // The name stays the player's name until they leave the arena.
                if client.player.is_some() {
                    bail!("Client {} can't claim a name while in an arena", client_id);
                }
                let name = validate_name(&name)?;
                self.claim_name(client_id, &name)?;
            }
            MessageInPayload::GetArenaList => {
                let arena_list: Vec<ArenaOverview> =
                    self.arenas.values().map(ArenaOverview::from).collect();

                let client = self
                    .clients
                    .get(&client_id)
                    .ok_or_else(|| anyhow!("Client {} not found", client_id))?;

                send(client, MessageOut::ArenaList(arena_list), &mut self.lagging);
            }
            MessageInPayload::Join {
                mut player,
                arena_id,
            } => {
                player.name = validate_name(&player.name)?;
                if !self.claim_name(client_id, &player.name)? {
                    return Ok(());
                }

                let current_arena = self.clients.get(&client_id).and_then(|client| client.arena);
                let arena_id = arena_id.filter(|id| self.arenas.contains_key(id));
                match arena_id.and_then(|id| self.arenas.get(&id)) {
                    Some(arena) => {
                        // The client's own place is free once it leaves.
                        let leaving = usize::from(current_arena == Some(arena.id));
                        if arena.human_player_ids().count().saturating_sub(leaving)
                            >= arena.max_players
                        {
                            bail!("Arena {} is full", arena.id);
                        }
                    }
                    None => {
                        if self.arenas.len() >= MAX_ARENAS {
                            bail!(
                                "Client {} can't open an arena: {} are open already",
                                client_id,
                                self.arenas.len()
                            );
                        }
                    }
                }

                self.client_part_arena(client_id);

                let arena_id = match arena_id {
                    Some(arena_id) => arena_id,
                    None => self.new_arena(&player.name),
                };

                let arena = self
                    .arenas
                    .get_mut(&arena_id)
                    .ok_or_else(|| anyhow!("Arena {} not found", arena_id))?;

                let player_id = player.id;

                let client = self
                    .clients
                    .get_mut(&client_id)
                    .ok_or_else(|| anyhow!("Client {} not found", client_id))?;

                arena.add_player(player);
                client.player = Some(player_id);
                client.arena = Some(arena_id);

                send(
                    client,
                    MessageOut::ArenaJoined(arena.id, player_id),
                    &mut self.lagging,
                );

                info!("Player {} joined arena {}", player_id, arena_id);
            }

            MessageInPayload::Start => {
                self.client_input(client_id, ArenaInput::Start)?;
            }
            MessageInPayload::Turn(direction) => {
                self.client_input(client_id, ArenaInput::Turn(direction))?;
            }
        }
        Ok(())
    }
}

/// A human player in the arena, or one who left during the round, or `None`
/// for an NPC.
fn human_player<'a>(arena: &'a Arena, id: &PlayerId) -> Option<&'a Player> {
    arena
        .players
        .get(id)
        .filter(|_| !arena.npc_states.contains_key(id))
        .or_else(|| arena.departed.get(id))
}

/// The name of a human player in the arena, or one who left during the
/// round, or `None` for an NPC.
fn human_name(arena: &Arena, id: &PlayerId) -> Option<String> {
    human_player(arena, id).map(|player| player.name.clone())
}

/// The leaderboard events for a lightcycle's death: a kill for the human
/// player whose trail it hit, and the kind of death for a human victim.
fn death_events(arena: &Arena, victim: PlayerId, cause: DeathCause) -> Vec<(String, Stat)> {
    let victim_name = human_name(arena, &victim);
    let owner = match cause {
        DeathCause::Boundary => {
            return victim_name
                .map(|name| (name, Stat::ArenaDeath))
                .into_iter()
                .collect();
        }
        DeathCause::Lightcycle(_) => {
            return victim_name
                .map(|name| (name, Stat::HeadonCrash))
                .into_iter()
                .collect();
        }
        DeathCause::Trail(owner) => owner,
    };
    if owner == victim {
        return victim_name
            .map(|name| (name, Stat::Suicide))
            .into_iter()
            .collect();
    }

    let mut events = Vec::new();
    if let Some(name) = human_name(arena, &owner) {
        let kill = if arena.npc_states.contains_key(&victim) {
            Stat::NpcKill
        } else {
            Stat::PlayerKill
        };
        events.push((name, kill));
    }
    if let Some(name) = victim_name {
        let death = if arena.npc_states.contains_key(&owner) {
            Stat::NpcDeath
        } else {
            Stat::PlayerDeath
        };
        events.push((name, death));
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;
    use arena::Lightcycle;
    use tokio::sync::mpsc;

    fn player(arena: &mut Arena, name: &str, npc: bool) -> PlayerId {
        let id = PlayerId::default();
        arena.players.insert(
            id,
            Player {
                id,
                name: name.to_string(),
                npc,
                ..Default::default()
            },
        );
        if npc {
            arena.npc_states.insert(id, Default::default());
        }
        id
    }

    fn events(list: &[(&str, Stat)]) -> Vec<(String, Stat)> {
        list.iter()
            .map(|(name, stat)| (name.to_string(), *stat))
            .collect()
    }

    #[test]
    fn a_player_who_leaves_mid_round_still_has_their_crash_counted() {
        let mut arena = Arena::default();
        let alec = player(&mut arena, "alec", false);
        let clu = player(&mut arena, "CLU", true);
        for id in [alec, clu] {
            arena.lightcycles.insert(id, Lightcycle::default());
        }

        arena.remove_player(alec);
        assert!(!arena.players.contains_key(&alec));
        // The riderless lightcycle doesn't hold up the next round.
        assert!(!arena.any_human_alive());
        assert_eq!(
            death_events(&arena, alec, DeathCause::Trail(clu)),
            events(&[("alec", Stat::NpcDeath)])
        );
    }

    /// A server with no clients, whose leaderboard file is never written.
    fn server() -> Server {
        let (_, message_queue) = mpsc::channel(1);
        let path =
            std::env::temp_dir().join(format!("webtron-server-{}.jsonl", ClientId::default()));
        Server::new(message_queue, Leaderboard::open(&path).unwrap())
    }

    /// Connects a client, returning its id and the messages it is sent.
    fn connect(server: &mut Server) -> (ClientId, mpsc::Receiver<MessageOut>) {
        let id = ClientId::default();
        let (tx, rx) = mpsc::channel(100);
        server
            .handle_message(id, MessageInPayload::Connect(None, tx))
            .unwrap();
        (id, rx)
    }

    fn join(
        server: &mut Server,
        client_id: ClientId,
        name: &str,
        arena_id: Option<ArenaId>,
    ) -> Result<(), Error> {
        let player = Player {
            name: name.to_string(),
            ..Default::default()
        };
        server.handle_message(client_id, MessageInPayload::Join { player, arena_id })
    }

    fn received(rx: &mut mpsc::Receiver<MessageOut>) -> Vec<MessageOut> {
        let mut messages = Vec::new();
        while let Ok(message) = rx.try_recv() {
            messages.push(message);
        }
        messages
    }

    fn arena_of(server: &Server, client_id: ClientId) -> Option<ArenaId> {
        server.clients[&client_id].arena
    }

    #[tokio::test]
    async fn clients_get_the_full_state_once_then_each_updates_changes() {
        let mut server = server();
        let (alec, mut alec_rx) = connect(&mut server);
        assert!(matches!(
            received(&mut alec_rx)[..],
            [MessageOut::ServerTime(_)]
        ));

        join(&mut server, alec, "alec", None).unwrap();
        let arena_id = arena_of(&server, alec).unwrap();
        server.tick(0.05);
        assert!(matches!(
            received(&mut alec_rx)[..],
            [
                MessageOut::ArenaJoined(..),
                MessageOut::ServerTime(_),
                MessageOut::ArenaState(_)
            ]
        ));
        assert!(server.arenas[&arena_id].updates.is_empty());

        server
            .handle_message(alec, MessageInPayload::Start)
            .unwrap();
        server.tick(0.05);
        assert!(matches!(
            received(&mut alec_rx)[..],
            [MessageOut::ArenaStatePatch(_)]
        ));
        assert!(server.arenas[&arena_id].updates.is_empty());

        // A client joining later gets everything so far in its full state.
        let (bob, mut bob_rx) = connect(&mut server);
        join(&mut server, bob, "bob", Some(arena_id)).unwrap();
        server.tick(0.05);
        let messages = received(&mut bob_rx);
        let Some(MessageOut::ArenaState(state)) = messages.last() else {
            panic!("no ArenaState in {:?}", messages);
        };
        assert!(matches!(
            messages[messages.len() - 2],
            MessageOut::ServerTime(_)
        ));
        let alec_player = server.clients[&alec].player.unwrap();
        assert!(state.lightcycles.contains_key(&alec_player));
        assert_eq!(state.human_player_ids().count(), 2);
        let Some(MessageOut::ArenaStatePatch(patch)) = received(&mut alec_rx).pop() else {
            panic!("alec wasn't sent bob joining");
        };
        assert!(matches!(patch[..], [ArenaUpdate::AddPlayer(..)]));

        // Nothing changes before the round starts, so nothing is sent.
        server.tick(0.05);
        assert!(received(&mut alec_rx).is_empty());
        assert!(received(&mut bob_rx).is_empty());
    }

    #[test]
    fn arenas_take_no_more_than_their_players_joining_at_once() {
        let mut server = server();
        let (first, _first_rx) = connect(&mut server);
        join(&mut server, first, "p0", None).unwrap();
        let arena_id = arena_of(&server, first).unwrap();
        let max_players = server.arenas[&arena_id].max_players;

        let mut receivers = Vec::new();
        let mut joined = 1;
        for index in 1..max_players + 2 {
            let (client, rx) = connect(&mut server);
            receivers.push(rx);
            if join(&mut server, client, &format!("p{index}"), Some(arena_id)).is_ok() {
                joined += 1;
            }
        }

        assert_eq!(joined, max_players);
        assert_eq!(
            server.arenas[&arena_id].human_player_ids().count(),
            max_players
        );
    }

    #[tokio::test]
    async fn leaving_an_arena_detaches_the_client_from_it() {
        let mut server = server();
        let (alec, _alec_rx) = connect(&mut server);
        let (bob, _bob_rx) = connect(&mut server);
        join(&mut server, alec, "alec", None).unwrap();
        let first_arena = arena_of(&server, alec).unwrap();
        join(&mut server, bob, "bob", Some(first_arena)).unwrap();
        server
            .handle_message(alec, MessageInPayload::Start)
            .unwrap();
        server.tick(0.05);
        let old_player = server.clients[&alec].player.unwrap();

        // Alec moves to an arena of his own.
        join(&mut server, alec, "alec", None).unwrap();
        let second_arena = arena_of(&server, alec).unwrap();
        assert_ne!(second_arena, first_arena);
        let arena = &server.arenas[&first_arena];
        assert!(!arena.players.contains_key(&old_player));
        assert!(arena.departed.contains_key(&old_player));
        assert!(!arena.lightcycles[&old_player].dead);

        // Leaving altogether detaches the client, and its turns go nowhere.
        server.client_part_arena(alec);
        assert_eq!(server.clients[&alec].arena, None);
        assert_eq!(server.clients[&alec].player, None);
        assert!(
            server
                .handle_message(alec, MessageInPayload::Turn(Direction::Left))
                .is_err()
        );

        // A full arena is refused before the client leaves its own.
        join(&mut server, alec, "alec", None).unwrap();
        let own_arena = arena_of(&server, alec).unwrap();
        let full = server.arenas.get_mut(&first_arena).unwrap();
        for index in 0..full.max_players {
            full.add_player(Player {
                id: PlayerId::default(),
                name: format!("p{index}"),
                ..Default::default()
            });
        }
        assert!(join(&mut server, alec, "alec", Some(first_arena)).is_err());
        assert_eq!(arena_of(&server, alec), Some(own_arena));
    }

    #[test]
    fn names_cant_be_claimed_while_in_an_arena() {
        let mut server = server();
        let (alec, _alec_rx) = connect(&mut server);
        server
            .handle_message(alec, MessageInPayload::ClaimName("alec".to_string()))
            .unwrap();
        join(&mut server, alec, "alec", None).unwrap();

        assert!(
            server
                .handle_message(alec, MessageInPayload::ClaimName("x".to_string()))
                .is_err()
        );
        assert_eq!(server.clients[&alec].name.as_deref(), Some("alec"));

        // Someone else still can't take it.
        let (bob, mut bob_rx) = connect(&mut server);
        join(&mut server, bob, "Alec", None).unwrap();
        assert!(matches!(
            received(&mut bob_rx).last(),
            Some(MessageOut::NameInUse)
        ));
        assert_eq!(arena_of(&server, bob), None);
    }

    #[test]
    fn names_are_checked() {
        assert_eq!(validate_name("  Big Al ").unwrap(), "big_al");
        assert!(validate_name("").is_err());
        assert!(validate_name(" \t ").is_err());
        assert!(validate_name("elevenchars").is_err());
        assert!(validate_name("nul\u{0}").is_err());
    }

    #[test]
    fn clients_that_fall_behind_are_disconnected() {
        let mut server = server();
        let id = ClientId::default();
        let (tx, mut rx) = mpsc::channel(1);
        server
            .handle_message(id, MessageInPayload::Connect(None, tx))
            .unwrap();
        server
            .handle_message(id, MessageInPayload::GetArenaList)
            .unwrap();
        server.disconnect_lagging_clients();

        assert!(!server.clients.contains_key(&id));
        // The queued messages are still delivered before the channel closes.
        assert!(matches!(rx.try_recv(), Ok(MessageOut::ServerTime(_))));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn crashing_into_your_own_trail_is_only_a_suicide() {
        let mut arena = Arena::default();
        let alec = player(&mut arena, "alec", false);
        assert_eq!(
            death_events(&arena, alec, DeathCause::Trail(alec)),
            events(&[("alec", Stat::Suicide)])
        );
    }

    #[test]
    fn crashes_into_other_trails_count_for_both_players() {
        let mut arena = Arena::default();
        let alec = player(&mut arena, "alec", false);
        let bob = player(&mut arena, "bob", false);
        let clu = player(&mut arena, "CLU", true);

        assert_eq!(
            death_events(&arena, bob, DeathCause::Trail(alec)),
            events(&[("alec", Stat::PlayerKill), ("bob", Stat::PlayerDeath)])
        );
        assert_eq!(
            death_events(&arena, clu, DeathCause::Trail(alec)),
            events(&[("alec", Stat::NpcKill)])
        );
        assert_eq!(
            death_events(&arena, alec, DeathCause::Trail(clu)),
            events(&[("alec", Stat::NpcDeath)])
        );
        assert_eq!(death_events(&arena, clu, DeathCause::Trail(clu)), vec![]);
        assert_eq!(
            death_events(&arena, alec, DeathCause::Boundary),
            events(&[("alec", Stat::ArenaDeath)])
        );
        assert_eq!(
            death_events(&arena, bob, DeathCause::Lightcycle(alec)),
            events(&[("bob", Stat::HeadonCrash)])
        );
    }
}
