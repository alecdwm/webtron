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
/// With `Mutator::Portals`, how likely each number of pairs of portals is,
/// three on average, and how far apart the two of a pair are at least, in
/// layout cells across and up.
const PORTAL_PAIR_COUNTS: [(usize, u32); 5] = [(1, 1), (2, 3), (3, 5), (4, 3), (5, 1)];
const PORTAL_MIN_APART: u8 = 6;
/// The colours pairs of portals can be, each pair a different one: as many as
/// there can be pairs.
const PORTAL_COLORS: [PlayerColor; 5] = [
    PlayerColor::Blue,
    PlayerColor::Green,
    PlayerColor::Orange,
    PlayerColor::Purple,
    PlayerColor::Red,
];
/// How likely each length of portal is, in layout cells.
const PORTAL_LENGTHS: [(u8, u32); 5] = [(1, 3), (2, 3), (3, 2), (4, 2), (5, 1)];
/// How far along its way from its spawn point each lightcycle is kept clear
/// of portals, so that none goes through one at the start of a round.
const SPAWN_RUN: f64 = 120.0;

/// Rows and columns kept clear of obstacles: those the lightcycles spawn in
/// and ride along at the start of a round (see `SPAWNPOINTS`).
const SPAWN_LANES: [u8; 4] = [3, 6, 7, 10];

/// The shape of the arena for a round, made of walled-off layout cells.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

/// How likely each shape is to be rolled for a round, relative to the
/// others: the plain grid about a third of the time.
const SHAPE_WEIGHTS: [(Shape, u32); 6] = [
    (Shape::Grid, 35),
    (Shape::Circle, 13),
    (Shape::Pillars, 13),
    (Shape::Cross, 13),
    (Shape::Core, 13),
    (Shape::Bastions, 13),
];

impl Shape {
    fn roll() -> Self {
        weighted_pick(&SHAPE_WEIGHTS).unwrap_or_default()
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    /// Pairs of single cells joined together: riding into one side of a
    /// portal comes out of the opposite side of its twin, going the same way.
    Portals,
}

/// How likely each mutator is to be rolled for a round, relative to the
/// others.
const MUTATOR_WEIGHTS: [(Mutator, u32); 7] = [
    (Mutator::Shrink, 10),
    (Mutator::Wrap, 10),
    (Mutator::Snake, 10),
    (Mutator::Overdrive, 10),
    (Mutator::Fog, 4),
    (Mutator::Obstacles, 10),
    (Mutator::Portals, 10),
];

/// Pairs of mutators that aren't rolled together. Walls closing in would
/// have to wrap lightcycles round a moving edge, and would cover portals.
const CLASHING_MUTATORS: [(Mutator, Mutator); 2] = [
    (Mutator::Shrink, Mutator::Wrap),
    (Mutator::Shrink, Mutator::Portals),
];

/// The layout and rules of one round, rolled when it starts.
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct Round {
    pub shape: Shape,
    pub mutators: Vec<Mutator>,
    /// The walled-off layout cells, as (column, row) counted from the bottom
    /// left, from both the shape and `Mutator::Obstacles`.
    pub blocked: Vec<(u8, u8)>,
    /// The pairs of portals joined by `Mutator::Portals`.
    pub portals: Vec<PortalPair>,
}

/// Two portals joined together, each coming out of the other, drawn in
/// `color`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PortalPair {
    pub color: PlayerColor,
    pub twins: [Portal; 2],
}

#[cfg(test)]
impl From<[Portal; 2]> for PortalPair {
    fn from(twins: [Portal; 2]) -> Self {
        Self {
            color: PlayerColor::Blue,
            twins,
        }
    }
}

/// Where a portal is on the layout: a line along the edges of cells, from
/// the corner of cells at `x` and `y`, counted like `Round::blocked`, going
/// `length` cells up if it is `upright`, or across. Lightcycles go through it
/// from either side. Both portals of a pair are the same length and way up.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Portal {
    pub x: u8,
    pub y: u8,
    pub length: u8,
    pub upright: bool,
}

impl Portal {
    /// The cells either side of the line, which must be open.
    fn footprint(self) -> Vec<(u8, u8)> {
        (0..self.length)
            .flat_map(|along| {
                if self.upright {
                    [(self.x - 1, self.y + along), (self.x, self.y + along)]
                } else {
                    [(self.x + along, self.y - 1), (self.x + along, self.y)]
                }
            })
            .collect()
    }

    fn center(self) -> (f64, f64) {
        let half = self.length as f64 / 2.0;
        if self.upright {
            (self.x as f64, self.y as f64 + half)
        } else {
            (self.x as f64 + half, self.y as f64)
        }
    }

    /// The ends of the line, the lowest first.
    fn ends(self, width: f64, height: f64) -> (ArenaPoint, ArenaPoint) {
        let (cell_width, cell_height) = cell_size(width, height);
        let start = ArenaPoint::new(self.x as f64 * cell_width, self.y as f64 * cell_height);
        let length = self.length as f64;
        let end = if self.upright {
            start + ArenaVector::new(0.0, length * cell_height)
        } else {
            start + ArenaVector::new(length * cell_width, 0.0)
        };
        (start, end)
    }
}

impl Round {
    /// A shape and up to two mutators, at random, each weighted by how
    /// likely it is (see `SHAPE_WEIGHTS` and `MUTATOR_WEIGHTS`). A quarter of
    /// rounds have no mutators and a quarter have two.
    pub fn roll() -> Self {
        Self::scenario(None, None)
    }

    /// The shape and mutators given, or those left out rolled at random as
    /// by `roll`. Mutators given twice, or that clash with one given before
    /// them, are left out.
    pub fn scenario(shape: Option<Shape>, mutators: Option<Vec<Mutator>>) -> Self {
        let mutators = match mutators {
            Some(given) => given.into_iter().fold(Vec::new(), |mut kept, mutator| {
                if !clashes(&kept, mutator) {
                    kept.push(mutator);
                }
                kept
            }),
            None => {
                let count = match random_below(4) {
                    0 => 0,
                    3 => 2,
                    _ => 1,
                };
                let mut mutators: Vec<Mutator> = Vec::with_capacity(count);
                while mutators.len() < count {
                    let open: Vec<(Mutator, u32)> = MUTATOR_WEIGHTS
                        .into_iter()
                        .filter(|(mutator, _)| !clashes(&mutators, *mutator))
                        .collect();
                    let Some(mutator) = weighted_pick(&open) else {
                        break;
                    };
                    mutators.push(mutator);
                }
                mutators
            }
        };
        Self::new(shape.unwrap_or_else(Shape::roll), mutators)
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
        let portals = if mutators.contains(&Mutator::Portals) {
            place_portals(&blocked)
        } else {
            Vec::new()
        };
        Self {
            shape,
            mutators,
            blocked,
            portals,
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

    /// The ends of each pair of portals, the lowest first.
    pub fn portal_rects(&self, width: f64, height: f64) -> Vec<[(ArenaPoint, ArenaPoint); 2]> {
        self.portals
            .iter()
            .map(|PortalPair { twins: [a, b], .. }| [a.ends(width, height), b.ends(width, height)])
            .collect()
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

/// A random number of pairs of portals (see `PORTAL_PAIR_COUNTS`), as many
/// of them as fit, each a different colour and of a random length and way up
/// that no other pair has, in open cells at
/// random. Each is kept off the edges of the arena and out of the way of
/// lightcycles riding away from their spawn points, and has open cells all
/// round it, so that a lightcycle coming out of it has somewhere to go and
/// can't ride straight from one portal into another.
fn place_portals(blocked: &[(u8, u8)]) -> Vec<PortalPair> {
    let inside = 1..LAYOUT_CELLS - 1;
    let fits = |portal: Portal| {
        let open = portal.footprint().iter().all(|&(x, y)| {
            inside.contains(&x)
                && inside.contains(&y)
                && [(x, y), (x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                    .iter()
                    .all(|cell| !blocked.contains(cell))
        });
        let (start, end) = portal.ends(ARENA_WIDTH, ARENA_HEIGHT);
        let clear = SPAWNPOINTS.iter().all(|&(x, y, direction)| {
            let spawn = ArenaPoint::new(x, y);
            !overlaps(
                start,
                end,
                spawn,
                spawn + direction.as_velocity() * SPAWN_RUN,
            )
        });
        open && clear
    };
    // Portals are kept a cell apart, corners included.
    let clear_of = |placed: &[Portal], portal: Portal| {
        let cells = portal.footprint();
        placed
            .iter()
            .flat_map(|other| other.footprint())
            .all(|(px, py)| {
                cells
                    .iter()
                    .all(|&(x, y)| px.abs_diff(x) > 1 || py.abs_diff(y) > 1)
            })
    };
    let spots = |length: u8, upright: bool| -> Vec<Portal> {
        (1..LAYOUT_CELLS)
            .flat_map(|x| (1..LAYOUT_CELLS).map(move |y| (x, y)))
            .map(|(x, y)| Portal {
                x,
                y,
                length,
                upright,
            })
            .filter(|portal| fits(*portal))
            .collect()
    };

    let count = weighted_pick(&PORTAL_PAIR_COUNTS).unwrap_or(3);
    let mut colors = PORTAL_COLORS.to_vec();
    let mut pairs: Vec<PortalPair> = Vec::with_capacity(count);
    for _ in 0..count * 10 {
        if pairs.len() == count {
            break;
        }
        let placed: Vec<Portal> = pairs.iter().flat_map(|pair| pair.twins).collect();
        // A length and way up that no other pair has, so that each pair can
        // be told by its shape alone.
        let upright = random_below(2) == 0;
        let unused: Vec<(u8, u32)> = PORTAL_LENGTHS
            .into_iter()
            .filter(|&(length, _)| {
                pairs
                    .iter()
                    .all(|pair| (pair.twins[0].length, pair.twins[0].upright) != (length, upright))
            })
            .collect();
        let Some(length) = weighted_pick(&unused) else {
            continue;
        };
        let open: Vec<Portal> = spots(length, upright)
            .into_iter()
            .filter(|portal| clear_of(&placed, *portal))
            .collect();
        let pick = || open[random_below(open.len() as u32) as usize];
        for _ in 0..(if open.is_empty() { 0 } else { 50 }) {
            let (a, b) = (pick(), pick());
            let ((ax, ay), (bx, by)) = (a.center(), b.center());
            let apart = (ax - bx).abs() + (ay - by).abs() >= PORTAL_MIN_APART as f64;
            if apart && clear_of(&[a], b) {
                let color = colors.remove(random_below(colors.len() as u32) as usize);
                pairs.push(PortalPair {
                    color,
                    twins: [a, b],
                });
                break;
            }
        }
    }
    pairs
}

/// Whether the horizontal or vertical segments from `a` to `b` and from `c`
/// to `d` meet.
fn overlaps(a: ArenaPoint, b: ArenaPoint, c: ArenaPoint, d: ArenaPoint) -> bool {
    a.x.min(b.x) <= c.x.max(d.x)
        && c.x.min(d.x) <= a.x.max(b.x)
        && a.y.min(b.y) <= c.y.max(d.y)
        && c.y.min(d.y) <= a.y.max(b.y)
}

/// Whether `mutator` is one of `chosen` or clashes with one of them.
fn clashes(chosen: &[Mutator], mutator: Mutator) -> bool {
    chosen.iter().any(|&other| {
        other == mutator
            || CLASHING_MUTATORS.contains(&(other, mutator))
            || CLASHING_MUTATORS.contains(&(mutator, other))
    })
}

/// One of `choices` at random, each as likely as its weight makes it
/// relative to the others, or `None` if they all weigh nothing.
fn weighted_pick<T: Copy>(choices: &[(T, u32)]) -> Option<T> {
    let total: u32 = choices.iter().map(|(_, weight)| weight).sum();
    if total == 0 {
        return None;
    }
    let mut roll = random_below(total);
    choices.iter().find_map(|&(choice, weight)| {
        if roll < weight {
            return Some(choice);
        }
        roll -= weight;
        None
    })
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
        for (shape, _) in SHAPE_WEIGHTS {
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
    fn portals_have_open_cells_all_round_and_keep_clear_of_spawns() {
        let mut longest = 0;
        for (shape, _) in SHAPE_WEIGHTS {
            for _ in 0..100 {
                let round = Round::new(shape, vec![Mutator::Obstacles, Mutator::Portals]);
                assert!(!round.portals.is_empty(), "{shape:?} has no portals");
                for PortalPair { twins: [a, b], .. } in &round.portals {
                    assert_eq!((a.length, a.upright), (b.length, b.upright));
                }
                let portals: Vec<Portal> =
                    round.portals.iter().flat_map(|pair| pair.twins).collect();
                longest = portals
                    .iter()
                    .map(|portal| portal.length)
                    .fold(longest, u8::max);
                for (index, portal) in portals.iter().enumerate() {
                    let (start, end) = portal.ends(ARENA_WIDTH, ARENA_HEIGHT);
                    for (x, y, direction) in SPAWNPOINTS {
                        let spawn = ArenaPoint::new(x, y);
                        let run = spawn + direction.as_velocity() * SPAWN_RUN;
                        assert!(!overlaps(start, end, spawn, run), "{portal:?}");
                    }
                    for (x, y) in portal.footprint() {
                        for side in [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)] {
                            assert!(!round.blocked.contains(&side), "{shape:?} walls in {x},{y}");
                            for other in &portals[index + 1..] {
                                assert!(!other.footprint().contains(&side));
                            }
                        }
                    }
                }
            }
        }
        assert!(longest >= 4, "{longest}");
    }

    #[test]
    fn portal_rounds_have_three_pairs_of_different_colours_and_shapes_on_average() {
        let mut counts = [0; 6];
        for _ in 0..1000 {
            let round = Round::new(Shape::Grid, vec![Mutator::Portals]);
            counts[round.portals.len()] += 1;
            let shape = |pair: &PortalPair| (pair.twins[0].length, pair.twins[0].upright);
            for (index, pair) in round.portals.iter().enumerate() {
                for other in &round.portals[index + 1..] {
                    assert_ne!(other.color, pair.color);
                    assert_ne!(shape(other), shape(pair));
                }
            }
        }
        assert_eq!(counts[0], 0);
        assert!(counts[3] > counts[2] && counts[3] > counts[4], "{counts:?}");
        assert!(counts[2] > counts[1] && counts[4] > counts[5], "{counts:?}");
    }

    #[test]
    fn fog_is_rolled_less_often_than_other_mutators() {
        let count = |mutator| (0..4000).filter(|_| Round::roll().has(mutator)).count();
        let (fog, snake) = (count(Mutator::Fog), count(Mutator::Snake));
        assert!(fog * 3 < snake * 2, "fog {fog}, snake {snake}");
    }

    #[test]
    fn scenarios_keep_the_mutators_given_that_dont_clash() {
        let round = Round::scenario(
            Some(Shape::Core),
            Some(vec![
                Mutator::Portals,
                Mutator::Shrink,
                Mutator::Wrap,
                Mutator::Portals,
            ]),
        );
        assert_eq!(round.shape, Shape::Core);
        assert_eq!(round.mutators, vec![Mutator::Portals, Mutator::Wrap]);
    }

    #[test]
    fn shrink_and_wrap_are_never_rolled_together() {
        for _ in 0..500 {
            let round = Round::roll();
            assert!(!(round.has(Mutator::Shrink) && round.has(Mutator::Wrap)));
            assert!(!(round.has(Mutator::Shrink) && round.has(Mutator::Portals)));
            assert!(round.mutators.len() <= 2);
        }
    }
}
