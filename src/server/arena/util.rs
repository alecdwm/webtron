use super::*;

const SPAWNPOINTS: [(f64, f64, Direction); ARENA_MAX_PLAYERS] = [
    // (
    //     ARENA_WIDTH / 2.0,
    //     ARENA_HEIGHT / 2.0,
    //     Direction::Up,
    // ),
    //
    (
        ARENA_WIDTH / 2.0,
        ARENA_HEIGHT / 2.0 - ARENA_HEIGHT / 4.0,
        Direction::Up,
    ),
    (
        ARENA_WIDTH / 2.0,
        ARENA_HEIGHT / 2.0 + ARENA_HEIGHT / 4.0,
        Direction::Down,
    ),
    //
    (
        ARENA_WIDTH / 2.0 - ARENA_WIDTH / 4.0,
        ARENA_HEIGHT / 2.0,
        Direction::Right,
    ),
    (
        ARENA_WIDTH / 2.0 + ARENA_WIDTH / 4.0,
        ARENA_HEIGHT / 2.0,
        Direction::Left,
    ),
    //
    (
        ARENA_WIDTH / 2.0 - ARENA_WIDTH / 4.0,
        ARENA_HEIGHT / 2.0 - ARENA_HEIGHT / 4.0,
        Direction::Right,
    ),
    (
        ARENA_WIDTH / 2.0 - ARENA_WIDTH / 4.0,
        ARENA_HEIGHT / 2.0 + ARENA_HEIGHT / 4.0,
        Direction::Right,
    ),
    (
        ARENA_WIDTH / 2.0 + ARENA_WIDTH / 4.0,
        ARENA_HEIGHT / 2.0 - ARENA_HEIGHT / 4.0,
        Direction::Left,
    ),
    (
        ARENA_WIDTH / 2.0 + ARENA_WIDTH / 4.0,
        ARENA_HEIGHT / 2.0 + ARENA_HEIGHT / 4.0,
        Direction::Left,
    ),
];

pub fn calculate_spawnpoints(player_ids: Vec<PlayerId>) -> Vec<(PlayerId, ArenaPoint, Direction)> {
    let mut spawnpoints: Vec<(PlayerId, ArenaPoint, Direction)> = Vec::new();
    let mut spawnpoints_used: Vec<usize> = Vec::new();

    for player_id in player_ids {
        // check if no spawnpoints remain
        if spawnpoints_used.len() >= SPAWNPOINTS.len() {
            error!("No spawnpoints remain!");
            break;
        }

        // randomly select an available spawnpoint
        let mut selected_spawnpoint = OsRng.next_u32() as usize % SPAWNPOINTS.len();
        while spawnpoints_used
            .iter()
            .any(|spawnpoint| *spawnpoint == selected_spawnpoint)
        {
            selected_spawnpoint = OsRng.next_u32() as usize % SPAWNPOINTS.len();
        }
        spawnpoints_used.push(selected_spawnpoint);

        // add the selected spawnpoint to our spawnpoints vector
        let selected_spawnpoint = SPAWNPOINTS[selected_spawnpoint];
        spawnpoints.push((
            player_id,
            ArenaPoint::new(selected_spawnpoint.0, selected_spawnpoint.1),
            selected_spawnpoint.2,
        ));
    }

    spawnpoints
}

const MAX_AI_PLAYERS: usize = 4;

pub fn select_ai_quantity(add_ai: bool) -> usize {
    if !add_ai {
        return 0;
    }

    (OsRng.next_u32() as usize % (MAX_AI_PLAYERS - 1)) + 1
}

const AI_PLAYER_POOL: &'static [(&'static str, PlayerColor)] = &[
    ("AI_ABRAXAS", PlayerColor::Green),
    ("AI_ANON", PlayerColor::White),
    ("AI_BARTOK", PlayerColor::Blue),
    ("AI_BECK", PlayerColor::White),
    ("AI_CASTOR", PlayerColor::White),
    ("AI_CLU", PlayerColor::Orange),
    ("AI_CLU_2.0", PlayerColor::Orange),
    ("AI_CROM", PlayerColor::Blue),
    ("AI_DUMONT", PlayerColor::Purple),
    ("AI_DYSON", PlayerColor::Red),
    ("AI_GEM", PlayerColor::White),
    ("AI_ISO_Q", PlayerColor::White),
    ("AI_JARVIS", PlayerColor::Red),
    ("AI_JET", PlayerColor::White),
    ("AI_MCP", PlayerColor::Orange),
    ("AI_MERCURY", PlayerColor::Blue),
    ("AI_RAM", PlayerColor::Blue),
    ("AI_RINZLER", PlayerColor::Red),
    ("AI_SARK", PlayerColor::Red),
    ("AI_TESLER", PlayerColor::Red),
    ("AI_TRON", PlayerColor::Blue),
    ("AI_YORI", PlayerColor::Blue),
    ("AI_ZUSE", PlayerColor::White),
];
const INCOMPATIBLE_AI_SETS: &'static [&'static [&'static str]] = &[
    &["AI_CASTOR", "AI_ZUSE"],
    &["AI_CLU", "AI_CLU_2.0"],
    &["AI_RINZLER", "AI_TRON"],
];

pub fn select_ai_from_pool(quantity: usize) -> impl Iterator<Item = Player> {
    let mut pool_remaining = AI_PLAYER_POOL.to_owned();

    fn remove_incompatible_ai_from_pool(
        ai_name: &str,
        pool_remaining: &mut Vec<(&str, PlayerColor)>,
    ) {
        INCOMPATIBLE_AI_SETS
            .iter()
            .filter(|incompatible_set| incompatible_set.iter().any(|name| *name == ai_name))
            .for_each(|incompatible_set| {
                incompatible_set.iter().for_each(|incompatible_name| {
                    if let Some(index) = pool_remaining
                        .iter()
                        .position(|(name, _)| name == incompatible_name)
                    {
                        pool_remaining.remove(index);
                    }
                })
            });
    }

    (0..quantity)
        .map(move |_| {
            if pool_remaining.len() < 1 {
                return None;
            }

            let selected_ai =
                pool_remaining.remove(OsRng.next_u32() as usize % pool_remaining.len());

            remove_incompatible_ai_from_pool(selected_ai.0, &mut pool_remaining);

            Some(selected_ai)
        })
        .filter_map(|selected_id| selected_id)
        .map(|(name, color)| Player {
            name: name.to_owned(),
            color,
            ai: true,
            ..Default::default()
        })
}
