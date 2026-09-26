use super::*;

/// Side length of a planning grid cell, in arena units.
const CELL_SIZE: f64 = 4.0;
/// How far ahead, in cells, to plan. Space further away than this is left
/// out of every measurement.
const PLANNING_DEPTH: u16 = 45;
/// Differences in space smaller than this many cells are treated as noise.
const SCORE_SLACK: f64 = 10.0;

/// A coarse occupancy grid of the arena, used to measure how much space each
/// move leaves a lightcycle. It has a border of blocked cells one cell wide,
/// so neighbouring cells can be found without bounds checks.
struct Grid {
    /// Width including the border.
    stride: usize,
    columns: usize,
    rows: usize,
    blocked: Vec<bool>,
}

impl Grid {
    fn new(arena: &Arena) -> Self {
        let columns = (arena.width / CELL_SIZE).ceil().max(1.0) as usize;
        let rows = (arena.height / CELL_SIZE).ceil().max(1.0) as usize;
        let stride = columns + 2;
        let mut blocked = vec![false; stride * (rows + 2)];
        for x in 0..stride {
            blocked[x] = true;
            blocked[(rows + 1) * stride + x] = true;
        }
        for y in 0..rows + 2 {
            blocked[y * stride] = true;
            blocked[y * stride + columns + 1] = true;
        }

        let mut grid = Self {
            stride,
            columns,
            rows,
            blocked,
        };

        for (id, lightribbon) in arena.lightribbons.iter() {
            for (from, to) in lightribbon.segments() {
                grid.block_segment(from, to);
            }
            if let (Some(last_point), Some(lightcycle)) =
                (lightribbon.last_point(), arena.lightcycles.get(id))
            {
                if !lightcycle.dead {
                    grid.block_segment(last_point, lightcycle.position);
                }
            }
        }

        grid
    }

    fn block_segment(&mut self, from: ArenaPoint, to: ArenaPoint) {
        let steps = ((to - from).length() / (CELL_SIZE * 0.5)).ceil().max(1.0) as usize;
        for step in 0..=steps {
            if let Some(cell) = self.cell(from.lerp(to, step as f64 / steps as f64)) {
                self.blocked[cell] = true;
            }
        }
    }

    fn cell(&self, point: ArenaPoint) -> Option<usize> {
        if point.x < 0.0 || point.y < 0.0 {
            return None;
        }
        let x = (point.x / CELL_SIZE) as usize;
        let y = (point.y / CELL_SIZE) as usize;
        // A point on the far edge of the arena is still inside it.
        let edge_x = self.columns as f64 * CELL_SIZE;
        let edge_y = self.rows as f64 * CELL_SIZE;
        let x = if x == self.columns && point.x <= edge_x {
            x - 1
        } else {
            x
        };
        let y = if y == self.rows && point.y <= edge_y {
            y - 1
        } else {
            y
        };
        (x < self.columns && y < self.rows).then_some((y + 1) * self.stride + x + 1)
    }

    fn neighbours(&self, cell: usize) -> [usize; 4] {
        [cell - 1, cell + 1, cell - self.stride, cell + self.stride]
    }

    /// Spreads breadth-first from `start` through unblocked cells, entering
    /// each cell that `enter` accepts at its distance from `start`. The start
    /// cell itself is always entered.
    fn distances(&self, start: usize, mut enter: impl FnMut(usize, u16) -> bool) {
        let mut distances = vec![u16::MAX; self.blocked.len()];
        let mut queue = Vec::new();
        distances[start] = 0;
        queue.push(start);

        let mut next = 0;
        while let Some(&cell) = queue.get(next) {
            next += 1;
            let distance = distances[cell] + 1;
            for neighbour in self.neighbours(cell) {
                if !self.blocked[neighbour]
                    && distances[neighbour] == u16::MAX
                    && enter(neighbour, distance)
                {
                    distances[neighbour] = distance;
                    queue.push(neighbour);
                }
            }
        }
    }
}

/// One of the three directions an NPC can take next, and what it leads to.
#[derive(Debug, Clone, Copy)]
pub struct Evaluation {
    pub direction: Direction,
    /// Distance to the nearest obstacle in this direction.
    pub ray: f64,
    /// Number of nearby cells this lightcycle could reach.
    pub space: f64,
    /// Number of nearby cells this lightcycle could reach before any other.
    pub territory: f64,
}

impl Evaluation {
    fn score(&self, caution: Caution) -> f64 {
        match caution {
            Caution::Contested => self.territory,
            Caution::Bold => self.space,
        }
    }
}

/// How a move is judged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Caution {
    /// By the space the NPC would reach before anyone else, which steers it
    /// away from space that other (maybe faster) lightcycles are closing off.
    Contested,
    /// Only by the free space a move leads to, so the NPC will ride into
    /// contested space to attack, though still not into a dead end.
    Bold,
}

/// Shared per-update planning data for every NPC in an arena.
pub struct Planner<'a> {
    arena: &'a Arena,
    grid: Grid,
    /// The live lightcycles, indexed by `first_owner`.
    lightcycles: Vec<PlayerId>,
    /// For each cell, the earliest time any live lightcycle can reach it, in
    /// cells per unit of speed, and the index of that lightcycle.
    first: Vec<f32>,
    first_owner: Vec<u8>,
    /// For each cell, the earliest time any other lightcycle can reach it.
    second: Vec<f32>,
}

impl<'a> Planner<'a> {
    pub fn new(arena: &'a Arena) -> Self {
        let grid = Grid::new(arena);
        let mut planner = Self {
            arena,
            lightcycles: Vec::new(),
            first: vec![f32::INFINITY; grid.blocked.len()],
            first_owner: vec![u8::MAX; grid.blocked.len()],
            second: vec![f32::INFINITY; grid.blocked.len()],
            grid,
        };

        for (id, lightcycle) in arena.lightcycles.iter() {
            if lightcycle.dead {
                continue;
            }
            let Some(start) = planner.grid.cell(lightcycle.position) else {
                continue;
            };

            let index = planner.lightcycles.len() as u8;
            planner.lightcycles.push(*id);
            let speed = lightcycle.speed as f32;
            let Self {
                grid,
                first,
                first_owner,
                second,
                ..
            } = &mut planner;
            let mut arrive = |cell: usize, distance: u16| {
                let time = distance as f32 / speed;
                if time < first[cell] {
                    second[cell] = first[cell];
                    first[cell] = time;
                    first_owner[cell] = index;
                } else if time < second[cell] {
                    second[cell] = time;
                }
            };
            arrive(start, 0);
            grid.distances(start, |cell, distance| {
                let within = distance <= PLANNING_DEPTH;
                if within {
                    arrive(cell, distance);
                }
                within
            });
        }

        planner
    }

    /// Evaluates going straight and turning either way.
    pub fn options(&self, npc_id: PlayerId, lightcycle: &Lightcycle, delta_time: f64) -> Options {
        let [left, right] = lightcycle.direction.perpendicular_directions();
        Options {
            straight: self.evaluate(npc_id, lightcycle, lightcycle.direction),
            turns: [
                self.evaluate(npc_id, lightcycle, left),
                self.evaluate(npc_id, lightcycle, right),
            ],
            // Far enough to survive the next update at the current speed.
            min_clearance: lightcycle.speed * delta_time * 1.5 + 1.0,
        }
    }

    fn evaluate(
        &self,
        npc_id: PlayerId,
        lightcycle: &Lightcycle,
        direction: Direction,
    ) -> Evaluation {
        let ray = look_ahead_distance(self.arena, lightcycle.position, direction);
        let velocity = direction.as_velocity();

        // Start from the first free cell ahead. Where a trail runs through
        // the cells right ahead, start in the nearest one anyway.
        let ahead = |steps: f64| {
            self.grid
                .cell(lightcycle.position + velocity * CELL_SIZE * steps)
        };
        let start = (1..=3)
            .map(|steps| steps as f64)
            .take_while(|steps| steps * CELL_SIZE < ray)
            .filter_map(ahead)
            .find(|cell| !self.grid.blocked[*cell])
            .or_else(|| ahead(1.0));

        // Count the nearby cells this lightcycle can reach, and which of them
        // it reaches before any other.
        let own_index = self.lightcycles.iter().position(|id| *id == npc_id);
        let speed = lightcycle.speed as f32;
        let mut space = 0.0;
        let mut territory = 0.0;
        if let Some(start) = start {
            self.grid.distances(start, |cell, distance| {
                let others = match own_index {
                    Some(index) if self.first_owner[cell] as usize == index => self.second[cell],
                    _ => self.first[cell],
                };
                if distance > PLANNING_DEPTH {
                    return false;
                }
                space += 1.0;
                if (distance + 1) as f32 / speed < others {
                    territory += 1.0;
                }
                true
            });
        }

        Evaluation {
            direction,
            ray,
            space,
            territory,
        }
    }
}

/// The evaluated moves open to an NPC this update.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub straight: Evaluation,
    pub turns: [Evaluation; 2],
    min_clearance: f64,
}

impl Options {
    #[cfg(test)]
    pub fn get(&self, direction: Direction) -> Option<&Evaluation> {
        self.all().find(|option| option.direction == direction)
    }

    fn all(&self) -> impl Iterator<Item = &Evaluation> {
        std::iter::once(&self.straight).chain(self.turns.iter())
    }

    pub fn is_safe(&self, option: &Evaluation) -> bool {
        option.ray >= self.min_clearance
    }

    fn is_acceptable(&self, option: &Evaluation, caution: Caution, tolerance: f64) -> bool {
        let best = self
            .all()
            .filter(|option| self.is_safe(option))
            .map(|option| option.score(caution))
            .fold(0.0, f64::max);

        self.is_safe(option) && option.score(caution) >= best * (1.0 - tolerance) - SCORE_SLACK
    }

    /// The safe move with the most territory, preferring to go straight.
    pub fn best(&self, caution: Caution) -> Direction {
        self.all()
            .filter(|option| self.is_safe(option))
            .fold(None::<&Evaluation>, |best, option| match best {
                Some(best) if best.score(caution) >= option.score(caution) => Some(best),
                _ => Some(option),
            })
            .or_else(|| self.all().max_by(|a, b| a.ray.total_cmp(&b.ray)))
            .map(|option| option.direction)
            .unwrap_or(self.straight.direction)
    }

    /// Checks the direction a behaviour asked for (`None` for straight ahead)
    /// against the space it leaves. Returns the direction to take, and whether
    /// the request was overruled.
    pub fn vet(
        &self,
        request: Option<Direction>,
        personality: &Personality,
        caution: Caution,
    ) -> (Direction, bool) {
        let turn = request.and_then(|direction| {
            self.turns
                .iter()
                .find(|option| option.direction == direction)
        });

        match turn {
            Some(turn) if self.is_acceptable(turn, caution, personality.tolerance) => {
                (turn.direction, false)
            }
            Some(_) => (self.fallback(personality, caution), true),
            None => {
                let direction = self.fallback(personality, caution);
                (direction, direction != self.straight.direction)
            }
        }
    }

    /// Straight ahead while it is clear and roomy; otherwise the roomier turn
    /// if it is about as good as going straight.
    fn fallback(&self, personality: &Personality, caution: Caution) -> Direction {
        let tolerance = personality.tolerance;
        let clearance = match caution {
            Caution::Contested => personality.clearance,
            Caution::Bold => self.min_clearance * 2.0,
        };

        let straight = &self.straight;
        if self.is_acceptable(straight, caution, tolerance) && straight.ray >= clearance {
            return straight.direction;
        }

        let best_turn = self
            .turns
            .iter()
            .filter(|option| self.is_safe(option))
            .max_by(|a, b| {
                a.score(caution)
                    .total_cmp(&b.score(caution))
                    .then(a.ray.total_cmp(&b.ray))
            });

        match best_turn {
            Some(turn)
                if !self.is_safe(straight)
                    || turn.score(caution)
                        >= straight.score(caution) * (1.0 - tolerance) - SCORE_SLACK =>
            {
                turn.direction
            }
            _ if self.is_safe(straight) => straight.direction,
            _ => self.best(caution),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cycle(x: f64, y: f64, direction: Direction) -> Lightcycle {
        Lightcycle {
            position: ArenaPoint::new(x, y),
            direction,
            ..Default::default()
        }
    }

    fn p(x: f64, y: f64) -> ArenaPoint {
        ArenaPoint::new(x, y)
    }

    #[test]
    fn avoids_turning_into_a_pocket() {
        // The NPC has come up the middle of the arena to just below a wall
        // that hooks round to its right. Its own trail and the wall seal off
        // everything to its left, so only a right turn leads to open space.
        let mut arena = Arena::default();
        let npc_id = PlayerId::default();
        let lightcycle = cycle(300.0, 280.0, Direction::Up);
        arena.lightcycles.insert(npc_id, lightcycle);
        arena.lightribbons.insert(
            npc_id,
            Lightribbon {
                paths: vec![vec![p(300.0, 0.0), p(300.0, 280.0)]],
            },
        );
        arena.lightribbons.insert(
            PlayerId::default(),
            Lightribbon {
                paths: vec![vec![p(0.0, 290.0), p(304.0, 290.0), p(304.0, 284.0)]],
            },
        );

        let planner = Planner::new(&arena);
        let options = planner.options(npc_id, &lightcycle, 0.05);
        let left = options.get(Direction::Left).unwrap();
        let right = options.get(Direction::Right).unwrap();
        assert!(left.territory * 2.0 < right.territory);
        assert!(options.straight.territory * 2.0 < right.territory);
        for caution in [Caution::Contested, Caution::Bold] {
            assert_eq!(options.best(caution), Direction::Right);
            let personality = Personality::new(Style::Trickster);
            let vet = |request| options.vet(request, &personality, caution);
            assert_eq!(vet(Some(Direction::Left)), (Direction::Right, true));
            assert_eq!(vet(None), (Direction::Right, true));
        }
    }

    #[test]
    fn opponents_claim_the_cells_they_reach_first() {
        let mut arena = Arena::default();
        let npc_id = PlayerId::default();
        let npc = cycle(100.0, 280.0, Direction::Up);
        arena.lightcycles.insert(npc_id, npc);

        let alone = Planner::new(&arena).options(npc_id, &npc, 0.05);

        let other = Lightcycle {
            speed: LIGHTCYCLE_BOOST_SPEED,
            ..cycle(160.0, 280.0, Direction::Up)
        };
        arena.lightcycles.insert(PlayerId::default(), other);
        let contested = Planner::new(&arena).options(npc_id, &npc, 0.05);

        assert!(contested.straight.territory < alone.straight.territory * 0.75);
        let left = contested.get(Direction::Left).unwrap();
        let right = contested.get(Direction::Right).unwrap();
        assert!(right.territory < left.territory);
    }
}
