use super::*;

const SPAWNPOINTS: [(isize, isize, Direction); ARENA_MAX_PLAYERS] = [
    // (
    //     ARENA_WIDTH as isize / 2,
    //     ARENA_HEIGHT as isize / 2,
    //     Direction::Up,
    // ),
    //
    (
        ARENA_WIDTH as isize / 2,
        ARENA_HEIGHT as isize / 2 - ARENA_HEIGHT as isize / 4,
        Direction::Down,
    ),
    (
        ARENA_WIDTH as isize / 2,
        ARENA_HEIGHT as isize / 2 + ARENA_HEIGHT as isize / 4,
        Direction::Up,
    ),
    //
    (
        ARENA_WIDTH as isize / 2 - ARENA_WIDTH as isize / 4,
        ARENA_HEIGHT as isize / 2,
        Direction::Right,
    ),
    (
        ARENA_WIDTH as isize / 2 + ARENA_WIDTH as isize / 4,
        ARENA_HEIGHT as isize / 2,
        Direction::Left,
    ),
    //
    (
        ARENA_WIDTH as isize / 2 - ARENA_WIDTH as isize / 4,
        ARENA_HEIGHT as isize / 2 - ARENA_HEIGHT as isize / 4,
        Direction::Right,
    ),
    (
        ARENA_WIDTH as isize / 2 - ARENA_WIDTH as isize / 4,
        ARENA_HEIGHT as isize / 2 + ARENA_HEIGHT as isize / 4,
        Direction::Right,
    ),
    (
        ARENA_WIDTH as isize / 2 + ARENA_WIDTH as isize / 4,
        ARENA_HEIGHT as isize / 2 - ARENA_HEIGHT as isize / 4,
        Direction::Left,
    ),
    (
        ARENA_WIDTH as isize / 2 + ARENA_WIDTH as isize / 4,
        ARENA_HEIGHT as isize / 2 + ARENA_HEIGHT as isize / 4,
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
        let mut selected_spawnpoint = OsRng.next_u64() as usize % SPAWNPOINTS.len();
        while spawnpoints_used
            .iter()
            .any(|spawnpoint| *spawnpoint == selected_spawnpoint)
        {
            selected_spawnpoint = OsRng.next_u64() as usize % SPAWNPOINTS.len();
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

pub fn is_point_on_line_2d(point: ArenaPoint, line: Line) -> bool {
    let start = line.0;
    let end = line.1;

    if start.x == end.x {
        if point.x != start.x {
            return false;
        }

        return is_point_on_line_1d(point.y, (start.y, end.y));
    }

    if start.y == end.y {
        if point.y != start.y {
            return false;
        }

        return is_point_on_line_1d(point.x, (start.x, end.x));
    }

    false
}

pub fn is_point_on_line_1d(point: isize, line: (isize, isize)) -> bool {
    let (low, high) = if line.0 <= line.1 {
        (line.0, line.1)
    } else {
        (line.1, line.0)
    };

    if point < low || high < point {
        return false;
    }

    true
}
