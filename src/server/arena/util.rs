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
