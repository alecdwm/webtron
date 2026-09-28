use super::*;
use std::cell::RefCell;

/// Side length of a planning grid cell, in arena units.
const CELL_SIZE: f64 = 4.0;
/// How far ahead, in cells, to plan. Space further away than this is left
/// out of every measurement.
const PLANNING_DEPTH: u16 = 45;
/// Differences in space smaller than this many cells are treated as noise.
const SCORE_SLACK: f64 = 10.0;

/// A coarse occupancy grid of the arena, used to measure how much space each
/// move leaves a lightcycle. It has a border of blocked cells one cell wide,
/// so neighbouring cells can be found without bounds checks. In an arena that
/// wraps round, the cells along each edge neighbour those along the opposite
/// edge instead. The cells either side of a portal neighbour those on the
/// opposite side of its twin.
struct Grid {
    /// Width including the border.
    stride: usize,
    columns: usize,
    rows: usize,
    /// For each cell, how many seconds from now it can be ridden through: 0
    /// for open cells, infinity for walls and lasting trails, and for trails
    /// that `Mutator::Snake` keeps short, when that part of the trail is
    /// trimmed away.
    free_at: Vec<f32>,
    wrap: bool,
    /// For each cell, whether it is beside a portal, going into which from
    /// it comes out at another cell: that cell, keyed by the cell and the
    /// side of it the portal is on, as `neighbours` orders them.
    linked: Vec<bool>,
    links: HashMap<(usize, usize), usize>,
    /// Working space for `distances`, kept between searches.
    search: RefCell<Search>,
}

/// A block of cells, from the columns and rows at `min` up to but not
/// including those at `max`, counted without the border.
#[derive(Debug, Clone, Copy)]
struct CellRect {
    min: (usize, usize),
    max: (usize, usize),
}

/// The cells a breadth-first search has entered. A cell has been entered by
/// the current search when its entry in `entered` is `generation`, so the
/// cells don't need clearing between searches.
#[derive(Default)]
struct Search {
    entered: Vec<u32>,
    generation: u32,
    queue: Vec<(usize, u16)>,
}

impl Grid {
    /// The grid of the arena with the part outside `bounds` blocked, as
    /// `Arena::bounds` gives them.
    #[cfg(test)]
    fn new(arena: &Arena, bounds: (ArenaPoint, ArenaPoint)) -> Self {
        let mut grid = Self::open(arena);
        grid.block_walls(arena);
        grid.block_outside(arena, bounds);
        for (id, lightribbon) in arena.lightribbons.iter() {
            grid.block_trail(arena, *id, lightribbon);
            grid.block_tip(arena, *id, lightribbon);
        }
        grid
    }

    /// The grid of the arena with only its border blocked, and its portals
    /// linked.
    fn open(arena: &Arena) -> Self {
        let columns = (arena.width / CELL_SIZE).ceil().max(1.0) as usize;
        let rows = (arena.height / CELL_SIZE).ceil().max(1.0) as usize;
        let stride = columns + 2;
        let mut free_at = vec![0.0; stride * (rows + 2)];
        for x in 0..stride {
            free_at[x] = f32::INFINITY;
            free_at[(rows + 1) * stride + x] = f32::INFINITY;
        }
        for y in 0..rows + 2 {
            free_at[y * stride] = f32::INFINITY;
            free_at[y * stride + columns + 1] = f32::INFINITY;
        }

        let mut grid = Self {
            stride,
            columns,
            rows,
            search: RefCell::new(Search {
                entered: vec![0; free_at.len()],
                ..Default::default()
            }),
            linked: vec![false; free_at.len()],
            links: HashMap::new(),
            free_at,
            wrap: arena.round.has(Mutator::Wrap),
        };

        let cells = |(min, max): (ArenaPoint, ArenaPoint)| CellRect {
            min: (
                (min.x / CELL_SIZE).round() as usize,
                (min.y / CELL_SIZE).round() as usize,
            ),
            max: (
                (max.x / CELL_SIZE).round() as usize,
                (max.y / CELL_SIZE).round() as usize,
            ),
        };
        for &[a, b] in &arena.portals {
            for (entry, exit) in [(a, b), (b, a)] {
                grid.add_portal(cells(entry), cells(exit));
            }
        }

        grid
    }

    /// Blocks the round's walled-off cells.
    fn block_walls(&mut self, arena: &Arena) {
        for (min, max) in arena.round.blocked_rects(arena.width, arena.height) {
            self.block_rect(min, max);
        }
    }

    /// Blocks the part of the arena outside `bounds`. The cells blocked for
    /// bounds inside others include all of those blocked for the others.
    fn block_outside(&mut self, arena: &Arena, (min, max): (ArenaPoint, ArenaPoint)) {
        let outside = [
            (ArenaPoint::origin(), ArenaPoint::new(arena.width, min.y)),
            (
                ArenaPoint::new(0.0, max.y),
                ArenaPoint::new(arena.width, arena.height),
            ),
            (ArenaPoint::origin(), ArenaPoint::new(min.x, arena.height)),
            (
                ArenaPoint::new(max.x, 0.0),
                ArenaPoint::new(arena.width, arena.height),
            ),
        ];
        for (min, max) in outside {
            self.block_rect(min, max);
        }
    }

    /// Blocks the cells along every segment of the trail of lightcycle `id`.
    fn block_trail(&mut self, arena: &Arena, id: PlayerId, lightribbon: &Lightribbon) {
        let trim = snake_trim(arena, id, lightribbon);
        let mut along = 0.0;
        for (from, to) in lightribbon.segments() {
            let length = (to - from).length();
            self.block_segment(from, to, |distance| match trim {
                // Free once the whole cell the point is in has gone.
                Some(trim) => trim.gone_at(along + distance + CELL_SIZE) as f32,
                None => f32::INFINITY,
            });
            along += length;
        }
    }

    /// Blocks the cells from the end of the trail of lightcycle `id` to the
    /// lightcycle, if it is alive.
    fn block_tip(&mut self, arena: &Arena, id: PlayerId, lightribbon: &Lightribbon) {
        if let (Some(last_point), Some(lightcycle)) =
            (lightribbon.last_point(), arena.lightcycles.get(&id))
            && !lightcycle.dead
        {
            self.block_segment(last_point, lightcycle.position, |_| f32::INFINITY);
        }
    }

    /// Panics unless the grid is the same as `other`.
    #[cfg(all(test, debug_assertions))]
    fn assert_matches(&self, other: &Grid) {
        assert_eq!(
            (self.stride, self.columns, self.rows, self.wrap),
            (other.stride, other.columns, other.rows, other.wrap)
        );
        assert_eq!(self.linked, other.linked);
        assert_eq!(self.links, other.links);
        if let Some(cell) = (0..self.free_at.len()).find(|&i| self.free_at[i] != other.free_at[i]) {
            panic!(
                "cell {cell} is free at {} rather than {}",
                self.free_at[cell], other.free_at[cell]
            );
        }
    }

    /// Blocks the cells along a segment until the time `free_at` gives for
    /// each point, from its distance along the segment.
    fn block_segment(&mut self, from: ArenaPoint, to: ArenaPoint, free_at: impl Fn(f64) -> f32) {
        let length = (to - from).length();
        let steps = (length / (CELL_SIZE * 0.5)).ceil().max(1.0) as usize;
        for step in 0..=steps {
            let t = step as f64 / steps as f64;
            if let Some(cell) = self.cell(from.lerp(to, t)) {
                self.free_at[cell] = self.free_at[cell].max(free_at(length * t));
            }
        }
    }

    /// Blocks every cell that overlaps the rectangle with corners `min` and
    /// `max`, edges included.
    fn block_rect(&mut self, min: ArenaPoint, max: ArenaPoint) {
        if max.x <= min.x || max.y <= min.y {
            return;
        }
        let x_steps = ((max.x - min.x) / CELL_SIZE).ceil() as usize;
        let y_steps = ((max.y - min.y) / CELL_SIZE).ceil() as usize;
        for i in 0..=x_steps {
            for j in 0..=y_steps {
                let x = (min.x + i as f64 * CELL_SIZE).min(max.x);
                let y = (min.y + j as f64 * CELL_SIZE).min(max.y);
                if let Some(cell) = self.cell(ArenaPoint::new(x, y)) {
                    self.free_at[cell] = f32::INFINITY;
                }
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

    /// Links the cells either side of portal `entry` to those on the
    /// opposite side of `exit`, level with them. Each portal is a line along
    /// the edges of cells, given as the columns and rows of cells it runs
    /// between.
    fn add_portal(&mut self, entry: CellRect, exit: CellRect) {
        let index = |x: usize, y: usize| (y + 1) * self.stride + x + 1;

        let mut link = |from: (usize, usize), side: usize, to: (usize, usize)| {
            let fits = |(x, y): (usize, usize)| x < self.columns && y < self.rows;
            if fits(from) && fits(to) {
                let cell = index(from.0, from.1);
                self.linked[cell] = true;
                self.links.insert((cell, side), index(to.0, to.1));
            }
        };
        // Portals are kept off the edges, so none of these go below 0.
        for y in entry.min.1..entry.max.1 {
            let up = y - entry.min.1 + exit.min.1;
            link((entry.min.0 - 1, y), 1, (exit.max.0, up));
            link((entry.max.0, y), 0, (exit.min.0 - 1, up));
        }
        for x in entry.min.0..entry.max.0 {
            let across = x - entry.min.0 + exit.min.0;
            link((x, entry.min.1 - 1), 3, (across, exit.max.1));
            link((x, entry.max.1), 2, (across, exit.min.1 - 1));
        }
    }

    /// The cell a lightcycle at `point` going in `direction` is in. One on a
    /// portal has just come out of it, and is taken to be a little further
    /// on, in the cell on the side it is going.
    fn cell_of(&self, point: ArenaPoint, direction: Direction) -> Option<usize> {
        if self.links.is_empty() {
            return self.cell(point);
        }
        self.cell(point + direction.as_velocity() * 0.01)
            .or_else(|| self.cell(point))
    }

    fn neighbours(&self, cell: usize) -> [usize; 4] {
        let mut neighbours = [cell - 1, cell + 1, cell - self.stride, cell + self.stride];
        if self.linked[cell] {
            for (side, neighbour) in neighbours.iter_mut().enumerate() {
                if let Some(&through) = self.links.get(&(cell, side)) {
                    *neighbour = through;
                }
            }
        }
        if self.wrap {
            let (x, y) = (cell % self.stride, cell / self.stride);
            let last_row = (self.rows - 1) * self.stride;
            if x == 1 {
                neighbours[0] = cell + self.columns - 1;
            }
            if x == self.columns {
                neighbours[1] = cell - (self.columns - 1);
            }
            if y == 1 {
                neighbours[2] = cell + last_row;
            }
            if y == self.rows {
                neighbours[3] = cell - last_row;
            }
        }
        neighbours
    }

    /// Spreads breadth-first from `start`, entering each cell that is free by
    /// the time a lightcycle going at `speed` gets there and that `enter`
    /// accepts at its distance from `start`. The start cell itself is always
    /// entered.
    fn distances(&self, start: usize, speed: f64, mut enter: impl FnMut(usize, u16) -> bool) {
        let seconds_per_cell = (CELL_SIZE / speed.max(1.0)) as f32;
        let mut search = self.search.borrow_mut();
        let Search {
            entered,
            generation,
            queue,
        } = &mut *search;
        *generation = generation.wrapping_add(1);
        if *generation == 0 {
            entered.fill(0);
            *generation = 1;
        }
        let generation = *generation;

        queue.clear();
        entered[start] = generation;
        queue.push((start, 0));

        let mut next = 0;
        while let Some(&(cell, distance)) = queue.get(next) {
            next += 1;
            let distance = distance + 1;
            for neighbour in self.neighbours(cell) {
                if self.free_at[neighbour] <= distance as f32 * seconds_per_cell
                    && entered[neighbour] != generation
                    && enter(neighbour, distance)
                {
                    entered[neighbour] = generation;
                    queue.push((neighbour, distance));
                }
            }
        }
    }
}

/// The segments of a trail that are laid for good: all but the last segment
/// of its last path, which the lightcycle is still stretching.
fn laid_paths(lightribbon: &Lightribbon) -> impl Iterator<Item = &[ArenaPoint]> {
    let last = lightribbon.paths.len().saturating_sub(1);
    lightribbon.paths.iter().enumerate().map(move |(i, path)| {
        if i == last {
            &path[..path.len().saturating_sub(1)]
        } else {
            &path[..]
        }
    })
}

/// A grid kept from one update to the next. `base` has the cells blocked by
/// walls, by the edges, and by the laid segments of trails that aren't being
/// trimmed. Each update adds the segments laid since the last one to it, and
/// blocks the rest of each trail on a copy of it.
///
/// Anything else that changes what `base` would hold starts it again: a
/// different round, arena size or portals, bounds that aren't inside the
/// last ones, and any trail blocked in it that has been cut, trimmed or
/// removed.
struct CachedGrid {
    width: f64,
    height: f64,
    round: Round,
    portals: Vec<[(ArenaPoint, ArenaPoint); 2]>,
    bounds: (ArenaPoint, ArenaPoint),
    base: Vec<f32>,
    /// For each trail with segments blocked in `base`, the paths they make.
    laid: HashMap<PlayerId, Vec<Vec<ArenaPoint>>>,
    /// `base` with the rest of each trail blocked.
    grid: Grid,
}

impl CachedGrid {
    fn new(arena: &Arena, bounds: (ArenaPoint, ArenaPoint)) -> Self {
        let mut grid = Grid::open(arena);
        grid.block_walls(arena);
        grid.block_outside(arena, bounds);
        Self {
            width: arena.width,
            height: arena.height,
            round: arena.round.clone(),
            portals: arena.portals.clone(),
            bounds,
            base: grid.free_at.clone(),
            laid: HashMap::new(),
            grid,
        }
    }

    /// Whether `base` can be brought up to date for the arena by blocking
    /// more cells.
    fn fits(&self, arena: &Arena, (min, max): (ArenaPoint, ArenaPoint)) -> bool {
        let (last_min, last_max) = self.bounds;
        let inside = min.x >= last_min.x
            && min.y >= last_min.y
            && max.x <= last_max.x
            && max.y <= last_max.y;
        inside
            && self.width == arena.width
            && self.height == arena.height
            && self.round == arena.round
            && self.portals == arena.portals
            && self.laid.iter().all(|(id, laid)| {
                arena.lightribbons.get(id).is_some_and(|lightribbon| {
                    snake_trim(arena, *id, lightribbon).is_none()
                        && laid.len() <= lightribbon.paths.len()
                        && laid_paths(lightribbon)
                            .zip(laid)
                            .all(|(path, laid)| path.starts_with(laid))
                })
            })
    }

    /// Brings the grid up to date for the arena, with the part outside
    /// `bounds` blocked.
    fn update(&mut self, arena: &Arena, bounds: (ArenaPoint, ArenaPoint)) {
        if !self.fits(arena, bounds) {
            *self = Self::new(arena, bounds);
        }

        // The blocking methods work on the grid's `free_at`, so `base` takes
        // its place while the newly laid segments are blocked.
        std::mem::swap(&mut self.base, &mut self.grid.free_at);
        if bounds != self.bounds {
            self.grid.block_outside(arena, bounds);
            self.bounds = bounds;
        }
        for (id, lightribbon) in arena.lightribbons.iter() {
            if snake_trim(arena, *id, lightribbon).is_some() {
                continue;
            }
            let laid = self.laid.entry(*id).or_default();
            for (i, path) in laid_paths(lightribbon).enumerate() {
                if i == laid.len() {
                    laid.push(Vec::new());
                }
                let known = laid[i].len();
                for segment in path[known.saturating_sub(1)..].windows(2) {
                    self.grid
                        .block_segment(segment[0], segment[1], |_| f32::INFINITY);
                }
                laid[i].extend_from_slice(&path[known..]);
            }
        }
        std::mem::swap(&mut self.base, &mut self.grid.free_at);

        self.grid.free_at.copy_from_slice(&self.base);
        for (id, lightribbon) in arena.lightribbons.iter() {
            if snake_trim(arena, *id, lightribbon).is_some() {
                self.grid.block_trail(arena, *id, lightribbon);
            } else if let Some(path) = lightribbon.paths.last()
                && let [.., from, to] = path[..]
            {
                self.grid.block_segment(from, to, |_| f32::INFINITY);
            }
            self.grid.block_tip(arena, *id, lightribbon);
        }
    }
}

/// Planning data kept from one update to the next, so it doesn't have to be
/// worked out again from scratch. It holds nothing an update doesn't bring
/// up to date, so a copy of it starts empty.
#[derive(Default)]
pub struct PlannerCache {
    grid: Option<CachedGrid>,
    first: Vec<f32>,
    first_owner: Vec<u8>,
    second: Vec<f32>,
}

impl Clone for PlannerCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

impl std::fmt::Debug for PlannerCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlannerCache").finish_non_exhaustive()
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
    grid: &'a Grid,
    /// The live lightcycles, indexed by `first_owner`.
    lightcycles: Vec<PlayerId>,
    /// For each cell, the earliest time any live lightcycle can reach it, in
    /// cells per unit of speed, and the index of that lightcycle.
    first: &'a [f32],
    first_owner: &'a [u8],
    /// For each cell, the earliest time any other lightcycle can reach it.
    second: &'a [f32],
}

impl<'a> Planner<'a> {
    /// Plans for the arena, keeping what can be kept for the next update in
    /// `cache`.
    pub fn new(arena: &'a Arena, cache: &'a mut PlannerCache) -> Self {
        let bounds = arena.bounds(NPC_SHRINK_LOOK_AHEAD_SECONDS);
        let PlannerCache {
            grid,
            first,
            first_owner,
            second,
        } = cache;
        let cached = grid.get_or_insert_with(|| CachedGrid::new(arena, bounds));
        cached.update(arena, bounds);
        let grid: &'a Grid = &cached.grid;
        // Tests check every kept grid against one built from scratch.
        #[cfg(all(test, debug_assertions))]
        grid.assert_matches(&Grid::new(arena, bounds));

        let cells = grid.free_at.len();
        first.clear();
        first.resize(cells, f32::INFINITY);
        second.clear();
        second.resize(cells, f32::INFINITY);
        first_owner.clear();
        first_owner.resize(cells, u8::MAX);

        let mut lightcycles = Vec::new();
        for (id, lightcycle) in arena.lightcycles.iter() {
            if lightcycle.dead {
                continue;
            }
            let Some(start) = grid.cell_of(lightcycle.position, lightcycle.direction) else {
                continue;
            };

            let index = lightcycles.len() as u8;
            lightcycles.push(*id);
            let speed = lightcycle.speed as f32;
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
            grid.distances(start, lightcycle.speed, |cell, distance| {
                let within = distance <= PLANNING_DEPTH;
                if within {
                    arrive(cell, distance);
                }
                within
            });
        }

        Self {
            arena,
            grid,
            lightcycles,
            first,
            first_owner,
            second,
        }
    }

    /// Evaluates going straight and turning either way. The path ahead of
    /// the lightcycle `ignoring` isn't treated as an obstacle.
    pub fn options(
        &self,
        npc_id: PlayerId,
        lightcycle: &Lightcycle,
        delta_time: f64,
        ignoring: Option<PlayerId>,
    ) -> Options {
        let [left, right] = lightcycle.direction.perpendicular_directions();
        let evaluate = |direction| self.evaluate(npc_id, lightcycle, direction, ignoring);
        Options {
            straight: evaluate(lightcycle.direction),
            turns: [evaluate(left), evaluate(right)],
            // Far enough to survive the next update at the current speed.
            min_clearance: lightcycle.speed * delta_time * 1.5 + 1.0,
        }
    }

    fn evaluate(
        &self,
        npc_id: PlayerId,
        lightcycle: &Lightcycle,
        direction: Direction,
        ignoring: Option<PlayerId>,
    ) -> Evaluation {
        let ray = look_ahead_distance(
            self.arena,
            lightcycle.position,
            direction,
            ignoring,
            lightcycle.speed,
        );

        // Start from the first free cell ahead. Where a trail runs through
        // the cells right ahead, start in the nearest one anyway.
        let ahead = |steps: f64| {
            let point = self
                .arena
                .advance(lightcycle.position, direction, CELL_SIZE * steps);
            self.grid.cell_of(point, direction)
        };
        let seconds_per_cell = CELL_SIZE / lightcycle.speed.max(1.0);
        let start = (1..=3)
            .map(|steps| steps as f64)
            .take_while(|steps| steps * CELL_SIZE < ray)
            .filter_map(|steps| Some((steps, ahead(steps)?)))
            .find(|(steps, cell)| self.grid.free_at[*cell] <= (steps * seconds_per_cell) as f32)
            .map(|(_, cell)| cell)
            .or_else(|| ahead(1.0));

        // Count the nearby cells this lightcycle can reach, and which of them
        // it reaches before any other.
        let own_index = self.lightcycles.iter().position(|id| *id == npc_id);
        let speed = lightcycle.speed as f32;
        let mut space = 0.0;
        let mut territory = 0.0;
        if let Some(start) = start {
            self.grid
                .distances(start, lightcycle.speed, |cell, distance| {
                    let others = match own_index {
                        Some(index) if self.first_owner[cell] as usize == index => {
                            self.second[cell]
                        }
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
    /// against the space it leaves. `clearance` is the distance to an obstacle
    /// ahead at which to turn off, and `tolerance` how much less than the best
    /// move's score a move may have, as a fraction of it. Returns the
    /// direction to take, and whether the request was overruled.
    pub fn vet(
        &self,
        request: Option<Direction>,
        clearance: f64,
        tolerance: f64,
        caution: Caution,
    ) -> (Direction, bool) {
        let turn = request.and_then(|direction| {
            self.turns
                .iter()
                .find(|option| option.direction == direction)
        });

        // A turn asked for has to leave room to turn again after it, not
        // just to survive the next update.
        let roomy = |turn: &Evaluation| turn.ray >= self.min_clearance * 2.0;
        match turn {
            Some(turn) if roomy(turn) && self.is_acceptable(turn, caution, tolerance) => {
                (turn.direction, false)
            }
            Some(_) => (self.fallback(clearance, tolerance, caution), true),
            None => {
                let direction = self.fallback(clearance, tolerance, caution);
                (direction, direction != self.straight.direction)
            }
        }
    }

    /// Straight ahead while it is clear and roomy; otherwise the roomier turn
    /// if it is about as good as going straight.
    fn fallback(&self, clearance: f64, tolerance: f64, caution: Caution) -> Direction {
        let clearance = match caution {
            Caution::Contested => clearance,
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

        let mut cache = PlannerCache::default();
        let planner = Planner::new(&arena, &mut cache);
        let options = planner.options(npc_id, &lightcycle, 0.05, None);
        let left = options.get(Direction::Left).unwrap();
        let right = options.get(Direction::Right).unwrap();
        assert!(left.territory * 2.0 < right.territory);
        assert!(options.straight.territory * 2.0 < right.territory);
        for caution in [Caution::Contested, Caution::Bold] {
            assert_eq!(options.best(caution), Direction::Right);
            let personality = Personality::new(Style::Trickster);
            let vet = |request| {
                options.vet(
                    request,
                    personality.clearance,
                    personality.tolerance,
                    caution,
                )
            };
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

        let alone =
            Planner::new(&arena, &mut PlannerCache::default()).options(npc_id, &npc, 0.05, None);

        let other = Lightcycle {
            speed: LIGHTCYCLE_BOOST_SPEED,
            ..cycle(160.0, 280.0, Direction::Up)
        };
        arena.lightcycles.insert(PlayerId::default(), other);
        let contested =
            Planner::new(&arena, &mut PlannerCache::default()).options(npc_id, &npc, 0.05, None);

        assert!(contested.straight.territory < alone.straight.territory * 0.75);
        let left = contested.get(Direction::Left).unwrap();
        let right = contested.get(Direction::Right).unwrap();
        assert!(right.territory < left.territory);
    }

    #[test]
    fn kept_grids_match_fresh_ones() {
        // `Planner::new` checks the grid it keeps against a fresh one every
        // update, in rounds where lightcycles crash and cut holes, trails are
        // trimmed and derezzed, the walls close in, and a new round starts
        // partway through.
        for mutators in [
            vec![Mutator::Obstacles],
            vec![Mutator::Snake],
            vec![Mutator::Wrap, Mutator::Portals],
            vec![Mutator::Shrink, Mutator::Obstacles],
            vec![Mutator::Snake, Mutator::Wrap, Mutator::Overdrive],
        ] {
            let mut arena = Arena::default();
            let start = |arena: &mut Arena| {
                arena.process_input(
                    PlayerId::default(),
                    ArenaInput::Start(Some(Scenario {
                        shape: None,
                        mutators: Some(mutators.clone()),
                        npcs: Some(Style::for_round(7)),
                    })),
                );
                arena.apply_updates();
                // Far enough in for the walls to be closing.
                arena.started = Some(Utc::now() - chrono::Duration::seconds(12));
            };
            start(&mut arena);
            for tick in 0..160 {
                if arena.started.is_none() || tick == 100 {
                    start(&mut arena);
                }
                if tick % 40 == 10
                    && let Some(id) = arena
                        .lightcycles
                        .iter()
                        .find(|(_, lightcycle)| !lightcycle.dead)
                        .map(|(id, _)| *id)
                {
                    ArenaUpdate::UpdateLightcycleApplyDeath(id, DeathCause::Boundary)
                        .apply(&mut arena);
                }
                if tick % 40 == 30 {
                    let dead: Vec<PlayerId> = arena
                        .lightcycles
                        .iter()
                        .filter(|(_, lightcycle)| lightcycle.dead)
                        .map(|(id, _)| *id)
                        .collect();
                    for id in dead {
                        ArenaUpdate::DerezLightribbon(id).apply(&mut arena);
                    }
                }
                arena.update(0.05);
                arena.clear_updates();
            }
        }
    }
}
