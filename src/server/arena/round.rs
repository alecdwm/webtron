use super::*;

/// The arena's layout is walled off in square cells, this many to a side.
pub const LAYOUT_CELLS: u8 = 14;

/// With `Mutator::Shrink`, the walls start closing in this long after the
/// round starts, at `SHRINK_RATE` arena units per second on every side,
/// until the arena is `SHRINK_MIN_SIZE` across.
pub const SHRINK_DELAY_SECONDS: f64 = 10.0;
pub const SHRINK_RATE: f64 = 4.0;
pub const SHRINK_MIN_SIZE: f64 = 160.0;
/// With `Mutator::Snake`, the longest a living lightcycle's trail can be.
pub const SNAKE_TRAIL_LENGTH: f64 = 420.0;
/// With `Mutator::Overdrive`, how much faster every lightcycle goes.
pub const OVERDRIVE_SPEED_SCALE: f64 = 1.6;
/// With `Mutator::Obstacles`, how many single cells are walled off.
const OBSTACLE_COUNT: usize = 10;

/// Rows and columns kept clear of obstacles: those the lightcycles spawn in
/// and ride along at the start of a round (see `SPAWNPOINTS`).
const SPAWN_LANES: [u8; 4] = [3, 6, 7, 10];

/// The shape of the arena for a round, made of walled-off layout cells.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize)]
pub enum Shape {
    /// No walls: the whole square.
    #[default]
    Grid,
    /// Corners walled off in steps, leaving a round arena.
    Circle,
    /// A pillar in each quarter of the arena, between the spawn lanes.
    Pillars,
    /// Each corner walled off, leaving a plus-shaped arena.
    Cross,
    /// A block in the middle, which the lightcycles start out facing.
    Core,
    /// A block jutting in from the middle of each side.
    Bastions,
}

const SHAPES: [Shape; 5] = [
    Shape::Circle,
    Shape::Pillars,
    Shape::Cross,
    Shape::Core,
    Shape::Bastions,
];

impl Shape {
    /// The shape for a new round: the plain grid about a third of the time.
    fn roll() -> Self {
        if random_below(100) < 35 {
            Shape::Grid
        } else {
            SHAPES[random_below(SHAPES.len() as u32) as usize]
        }
    }

    fn cells(self) -> Vec<(u8, u8)> {
        let all = (0..LAYOUT_CELLS).flat_map(|x| (0..LAYOUT_CELLS).map(move |y| (x, y)));
        let blocks = |ranges: &[(std::ops::Range<u8>, std::ops::Range<u8>)], (x, y): (u8, u8)| {
            ranges
                .iter()
                .any(|(xs, ys)| xs.contains(&x) && ys.contains(&y))
        };
        match self {
            Shape::Grid => Vec::new(),
            Shape::Circle => {
                let center = LAYOUT_CELLS as f64 / 2.0;
                all.filter(|&(x, y)| {
                    let dx = x as f64 + 0.5 - center;
                    let dy = y as f64 + 0.5 - center;
                    (dx * dx + dy * dy).sqrt() > center
                })
                .collect()
            }
            Shape::Pillars => all
                .filter(|&cell| {
                    blocks(
                        &[(4..6, 4..6), (8..10, 4..6), (4..6, 8..10), (8..10, 8..10)],
                        cell,
                    )
                })
                .collect(),
            Shape::Cross => all
                .filter(|&cell| {
                    blocks(
                        &[
                            (0..3, 0..3),
                            (11..14, 0..3),
                            (0..3, 11..14),
                            (11..14, 11..14),
                        ],
                        cell,
                    )
                })
                .collect(),
            Shape::Core => all.filter(|&cell| blocks(&[(6..8, 6..8)], cell)).collect(),
            Shape::Bastions => all
                .filter(|&cell| {
                    blocks(
                        &[(0..3, 6..8), (11..14, 6..8), (6..8, 0..3), (6..8, 11..14)],
                        cell,
                    )
                })
                .collect(),
        }
    }
}

/// A rule that changes how a round plays.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub enum Mutator {
    /// The walls close in, after `SHRINK_DELAY_SECONDS`.
    Shrink,
    /// Riding off one edge of the arena comes back in at the opposite edge.
    Wrap,
    /// Trails are no longer than `SNAKE_TRAIL_LENGTH`.
    Snake,
    /// Every lightcycle is `OVERDRIVE_SPEED_SCALE` times as fast.
    Overdrive,
    /// Players see only the arena near their own lightcycle. The server
    /// doesn't hide anything: clients draw the fog.
    Fog,
    /// Single cells walled off at random.
    Obstacles,
}

const MUTATORS: [Mutator; 6] = [
    Mutator::Shrink,
    Mutator::Wrap,
    Mutator::Snake,
    Mutator::Overdrive,
    Mutator::Fog,
    Mutator::Obstacles,
];

/// Pairs of mutators that aren't rolled together. Walls closing in would
/// have to wrap lightcycles round a moving edge.
const CLASHING_MUTATORS: [(Mutator, Mutator); 1] = [(Mutator::Shrink, Mutator::Wrap)];

/// The layout and rules of one round, rolled when it starts.
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct Round {
    pub shape: Shape,
    pub mutators: Vec<Mutator>,
    /// The walled-off layout cells, as (column, row) counted from the bottom
    /// left, from both the shape and `Mutator::Obstacles`.
    pub blocked: Vec<(u8, u8)>,
}

impl Round {
    /// A shape and up to two mutators, at random. A quarter of rounds have
    /// no mutators and a quarter have two.
    pub fn roll() -> Self {
        let count = match random_below(4) {
            0 => 0,
            3 => 2,
            _ => 1,
        };
        let mut mutators: Vec<Mutator> = Vec::with_capacity(count);
        while mutators.len() < count {
            let mutator = MUTATORS[random_below(MUTATORS.len() as u32) as usize];
            let clashes = mutators.iter().any(|&other| {
                other == mutator
                    || CLASHING_MUTATORS.contains(&(other, mutator))
                    || CLASHING_MUTATORS.contains(&(mutator, other))
            });
            if !clashes {
                mutators.push(mutator);
            }
        }
        Self::new(Shape::roll(), mutators)
    }

    pub fn new(shape: Shape, mutators: Vec<Mutator>) -> Self {
        let mut blocked = shape.cells();
        if mutators.contains(&Mutator::Obstacles) {
            let mut placed = 0;
            while placed < OBSTACLE_COUNT {
                let cell = (
                    random_below(LAYOUT_CELLS as u32) as u8,
                    random_below(LAYOUT_CELLS as u32) as u8,
                );
                if SPAWN_LANES.contains(&cell.0) || SPAWN_LANES.contains(&cell.1) {
                    continue;
                }
                if !blocked.contains(&cell) {
                    blocked.push(cell);
                }
                // Counted even when already walled off, so a shape that
                // covers most free cells can't keep this looking forever.
                placed += 1;
            }
        }
        Self {
            shape,
            mutators,
            blocked,
        }
    }

    pub fn has(&self, mutator: Mutator) -> bool {
        self.mutators.contains(&mutator)
    }

    pub fn speed_scale(&self) -> f64 {
        if self.has(Mutator::Overdrive) {
            OVERDRIVE_SPEED_SCALE
        } else {
            1.0
        }
    }

    /// The corners of the part of the arena still open `elapsed` seconds
    /// into the round, the lowest first.
    pub fn bounds(&self, width: f64, height: f64, elapsed: f64) -> (ArenaPoint, ArenaPoint) {
        let margin = if self.has(Mutator::Shrink) {
            let max_margin = ((width.min(height) - SHRINK_MIN_SIZE) / 2.0).max(0.0);
            ((elapsed - SHRINK_DELAY_SECONDS) * SHRINK_RATE).clamp(0.0, max_margin)
        } else {
            0.0
        };
        (
            ArenaPoint::new(margin, margin),
            ArenaPoint::new(width - margin, height - margin),
        )
    }

    /// The corners of each walled-off cell, the lowest first.
    pub fn blocked_rects(&self, width: f64, height: f64) -> Vec<(ArenaPoint, ArenaPoint)> {
        let (cell_width, cell_height) = cell_size(width, height);
        self.blocked
            .iter()
            .map(|&(x, y)| {
                let min = ArenaPoint::new(x as f64 * cell_width, y as f64 * cell_height);
                (min, min + ArenaVector::new(cell_width, cell_height))
            })
            .collect()
    }

    /// The sides of walled-off cells that face open cells, which lightcycles
    /// crash into. Sides on the edge of the arena are left out.
    pub fn walls(&self, width: f64, height: f64) -> Vec<(ArenaPoint, ArenaPoint)> {
        let (cell_width, cell_height) = cell_size(width, height);
        let is_blocked = |x: i16, y: i16| {
            (0..LAYOUT_CELLS as i16).contains(&x)
                && (0..LAYOUT_CELLS as i16).contains(&y)
                && self.blocked.contains(&(x as u8, y as u8))
        };
        let in_arena = |x: i16, y: i16| {
            (0..LAYOUT_CELLS as i16).contains(&x) && (0..LAYOUT_CELLS as i16).contains(&y)
        };
        let point = |x: i16, y: i16| ArenaPoint::new(x as f64 * cell_width, y as f64 * cell_height);

        let mut walls = Vec::new();
        for &(x, y) in &self.blocked {
            let (x, y) = (x as i16, y as i16);
            // Each side as the neighbouring cell it faces and its two ends.
            let sides = [
                ((x - 1, y), point(x, y), point(x, y + 1)),
                ((x + 1, y), point(x + 1, y), point(x + 1, y + 1)),
                ((x, y - 1), point(x, y), point(x + 1, y)),
                ((x, y + 1), point(x, y + 1), point(x + 1, y + 1)),
            ];
            for ((nx, ny), a, b) in sides {
                if in_arena(nx, ny) && !is_blocked(nx, ny) {
                    walls.push((a, b));
                }
            }
        }
        walls
    }

    /// Whether `point` is in a walled-off cell or on its edge.
    pub fn is_blocked_at(&self, point: ArenaPoint, width: f64, height: f64) -> bool {
        let (cell_width, cell_height) = cell_size(width, height);
        let last = LAYOUT_CELLS as f64 - 1.0;
        // A point on a cell's edge is in the cells on either side of it.
        let columns = [
            (point.x / cell_width).floor().clamp(0.0, last),
            (point.x / cell_width).ceil().clamp(1.0, last + 1.0) - 1.0,
        ];
        let rows = [
            (point.y / cell_height).floor().clamp(0.0, last),
            (point.y / cell_height).ceil().clamp(1.0, last + 1.0) - 1.0,
        ];
        columns.iter().any(|&x| {
            rows.iter()
                .any(|&y| self.blocked.contains(&(x as u8, y as u8)))
        })
    }
}

fn cell_size(width: f64, height: f64) -> (f64, f64) {
    (width / LAYOUT_CELLS as f64, height / LAYOUT_CELLS as f64)
}

fn random_below(n: u32) -> u32 {
    UnwrapErr(SysRng).next_u32() % n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawnpoints_and_the_lanes_ahead_of_them_are_open_in_every_shape() {
        for shape in SHAPES {
            let round = Round::new(shape, vec![Mutator::Obstacles]);
            for (x, y, direction) in SPAWNPOINTS {
                let spawn = ArenaPoint::new(x, y);
                // A second at the start of the round, with time to turn.
                for step in 0..=40 {
                    let point = spawn + direction.as_velocity() * step as f64;
                    assert!(
                        !round.is_blocked_at(point, ARENA_WIDTH, ARENA_HEIGHT),
                        "{shape:?} blocks {point:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn walls_face_open_cells_only() {
        let round = Round::new(Shape::Core, vec![]);
        // The 2 by 2 block has two sides facing out on each side.
        assert_eq!(round.walls(ARENA_WIDTH, ARENA_HEIGHT).len(), 8);
    }

    #[test]
    fn shrinking_walls_stop_at_the_smallest_size() {
        let round = Round::new(Shape::Grid, vec![Mutator::Shrink]);
        let (min, max) = round.bounds(ARENA_WIDTH, ARENA_HEIGHT, SHRINK_DELAY_SECONDS);
        assert_eq!((min.x, max.x), (0.0, ARENA_WIDTH));
        let (min, max) = round.bounds(ARENA_WIDTH, ARENA_HEIGHT, 1000.0);
        assert_eq!(max.x - min.x, SHRINK_MIN_SIZE);
    }

    #[test]
    fn shrink_and_wrap_are_never_rolled_together() {
        for _ in 0..500 {
            let round = Round::roll();
            assert!(!(round.has(Mutator::Shrink) && round.has(Mutator::Wrap)));
            assert!(round.mutators.len() <= 2);
        }
    }
}
