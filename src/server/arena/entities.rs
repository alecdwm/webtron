use super::*;

#[derive(Debug, Copy, Clone, PartialEq, Serialize)]
pub struct Lightcycle {
    pub position: ArenaPoint,
    pub direction: Direction,
    pub speed: f64,
    pub dead: bool,
    pub death_cause: Option<DeathCause>,
    /// How much braking the lightcycle has left, from 0 to 1. It drains
    /// while the brake is held and recharges once it is let go.
    pub brake: f64,
    /// Whether the player is holding the brake. It only slows the
    /// lightcycle while there is `brake` left.
    pub braking: bool,
}

impl Lightcycle {
    pub fn is_braking(&self) -> bool {
        self.braking && self.brake > 0.0
    }
}

impl Default for Lightcycle {
    fn default() -> Self {
        Self {
            position: ArenaPoint::origin(),
            direction: Direction::Up,
            speed: LIGHTCYCLE_SPEED,
            dead: Default::default(),
            death_cause: None,
            brake: 1.0,
            braking: false,
        }
    }
}

/// The trail left behind a lightcycle. Holes can cut a trail into several
/// paths; the last path is the one the lightcycle is still laying down, and its
/// last point follows the lightcycle.
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
pub struct Lightribbon {
    pub paths: Vec<Vec<ArenaPoint>>,
}

impl Lightribbon {
    pub fn new(start: ArenaPoint) -> Self {
        Self {
            paths: vec![vec![start, start]],
        }
    }

    /// Every segment of every path.
    pub fn segments(&self) -> impl Iterator<Item = (ArenaPoint, ArenaPoint)> + '_ {
        self.paths
            .iter()
            .flat_map(|path| path.windows(2).map(|segment| (segment[0], segment[1])))
    }

    pub fn last_point(&self) -> Option<ArenaPoint> {
        self.paths.last().and_then(|path| path.last()).copied()
    }

    pub fn append_point(&mut self, point: ArenaPoint) {
        match self.paths.last_mut() {
            Some(path) => path.push(point),
            None => self.paths.push(vec![point]),
        }
    }

    pub fn replace_latest_point(&mut self, point: ArenaPoint) {
        match self.paths.last_mut() {
            Some(path) => {
                path.pop();
                path.push(point);
            }
            None => self.paths.push(vec![point]),
        }
    }

    /// Starts a new path at `point`, leaving a gap in the trail from the end
    /// of the last one, as when a lightcycle wraps round the arena.
    pub fn start_path(&mut self, point: ArenaPoint) {
        self.paths.push(vec![point, point]);
    }

    /// The length of all of the trail's paths, not counting gaps between them.
    pub fn length(&self) -> f64 {
        self.segments().map(|(from, to)| (to - from).length()).sum()
    }

    /// Removes `length` from the oldest end of the trail. Paths left with no
    /// length are removed, except the last, which its lightcycle is laying.
    pub fn trim_front(&mut self, mut length: f64) {
        while length > 0.0 {
            let only_path = self.paths.len() == 1;
            let Some(path) = self.paths.first_mut() else {
                return;
            };
            if path.len() < 2 {
                return;
            }
            let (from, to) = (path[0], path[1]);
            let segment = (to - from).length();
            if segment > length {
                path[0] = point_along(from, to, length / segment);
                return;
            }
            length -= segment;
            path.remove(0);
            if path.len() < 2 {
                if only_path {
                    // Nothing left but the lightcycle's end of it.
                    path.insert(0, path[0]);
                    return;
                }
                self.paths.remove(0);
            }
        }
    }

    /// Removes every part of the trail inside the circle. `live_end` is the
    /// position of the lightcycle when it is still alive: if its end of the
    /// trail was removed, a new live path starts there.
    pub fn cut(&mut self, center: ArenaPoint, radius: f64, live_end: Option<ArenaPoint>) {
        let mut paths: Vec<Vec<ArenaPoint>> = self
            .paths
            .iter()
            .flat_map(|path| cut_path(path, center, radius))
            .collect();

        if let Some(end) = live_end
            && paths.last().and_then(|path| path.last()) != Some(&end)
        {
            paths.push(vec![end, end]);
        }

        self.paths = paths;
    }
}

/// A circle blown out of every trail by an exploding lightcycle. Trails laid
/// down after the hole was made cross it as normal.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Hole {
    pub player_id: PlayerId,
    pub position: ArenaPoint,
    pub radius: f64,
    /// When the lightcycle exploded.
    #[serde(skip)]
    pub created_at: DateTime<Utc>,
}

/// Splits a path into the pieces that lie outside the circle.
fn cut_path(path: &[ArenaPoint], center: ArenaPoint, radius: f64) -> Vec<Vec<ArenaPoint>> {
    let mut pieces = Vec::new();
    let mut current: Vec<ArenaPoint> = Vec::new();

    for segment in path.windows(2) {
        let (from, to) = (segment[0], segment[1]);

        let Some((enter, exit)) = circle_overlap(from, to, center, radius) else {
            if current.is_empty() {
                current.push(from);
            }
            current.push(to);
            continue;
        };

        if enter > 0.0 {
            if current.is_empty() {
                current.push(from);
            }
            current.push(point_along(from, to, enter));
        }
        if current.len() >= 2 {
            pieces.push(mem::take(&mut current));
        } else {
            current.clear();
        }
        if exit < 1.0 {
            current.push(point_along(from, to, exit));
            current.push(to);
        }
    }

    if current.len() >= 2 {
        pieces.push(current);
    }

    pieces
}

/// The point `t` of the way from `from` to `to`. Along a horizontal or
/// vertical segment, the coordinate the two ends share is kept exactly, so
/// the piece of trail left stays straight: a lightcycle turning off the end
/// of a trail that leans by a rounding error would be crashing into it.
fn point_along(from: ArenaPoint, to: ArenaPoint, t: f64) -> ArenaPoint {
    from + (to - from) * t
}

/// Returns the range of `t` in `0..=1` for which `from + (to - from) * t` lies
/// inside the circle, or `None` if the segment misses it.
fn circle_overlap(
    from: ArenaPoint,
    to: ArenaPoint,
    center: ArenaPoint,
    radius: f64,
) -> Option<(f64, f64)> {
    let d = to - from;
    let f = from - center;

    let a = d.dot(d);
    let c = f.dot(f) - radius * radius;
    if a <= f64::EPSILON {
        return (c <= 0.0).then_some((0.0, 1.0));
    }

    let b = 2.0 * f.dot(d);
    let discriminant = b * b - 4.0 * a * c;
    if discriminant <= 0.0 {
        return None;
    }

    let root = discriminant.sqrt();
    let enter = (-b - root) / (2.0 * a);
    let exit = (-b + root) / (2.0 * a);
    if exit <= 0.0 || enter >= 1.0 {
        return None;
    }

    Some((enter.max(0.0), exit.min(1.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> ArenaPoint {
        ArenaPoint::new(x, y)
    }

    #[test]
    fn trim_front_removes_the_oldest_part_of_the_trail() {
        let mut ribbon = Lightribbon {
            paths: vec![
                vec![p(0.0, 0.0), p(10.0, 0.0)],
                vec![p(20.0, 0.0), p(40.0, 0.0), p(40.0, 30.0)],
            ],
        };
        ribbon.trim_front(25.0);
        assert_eq!(
            ribbon.paths,
            vec![vec![p(35.0, 0.0), p(40.0, 0.0), p(40.0, 30.0)]]
        );
        assert_eq!(ribbon.length(), 35.0);
    }

    #[test]
    fn trimming_a_straight_trail_keeps_it_straight() {
        let x = 13.449999999999989;
        let mut ribbon = Lightribbon {
            paths: vec![vec![p(x, 100.0), p(x, 515.25)]],
        };
        for _ in 0..50 {
            ribbon.trim_front(2.75);
        }
        assert!(ribbon.paths[0].iter().all(|point| point.x == x));
    }

    #[test]
    fn cut_splits_a_path_crossing_the_circle() {
        let mut ribbon = Lightribbon {
            paths: vec![vec![p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0)]],
        };
        ribbon.cut(p(50.0, 0.0), 10.0, None);
        assert_eq!(
            ribbon.paths,
            vec![
                vec![p(0.0, 0.0), p(40.0, 0.0)],
                vec![p(60.0, 0.0), p(100.0, 0.0), p(100.0, 100.0)],
            ]
        );
    }

    #[test]
    fn cut_restarts_the_live_path_inside_the_circle() {
        let mut ribbon = Lightribbon {
            paths: vec![vec![p(0.0, 0.0), p(50.0, 0.0)]],
        };
        ribbon.cut(p(45.0, 0.0), 10.0, Some(p(50.0, 0.0)));
        assert_eq!(
            ribbon.paths,
            vec![
                vec![p(0.0, 0.0), p(35.0, 0.0)],
                vec![p(50.0, 0.0), p(50.0, 0.0)]
            ]
        );
    }

    #[test]
    fn cut_keeps_the_live_path_outside_the_circle() {
        let mut ribbon = Lightribbon {
            paths: vec![vec![p(0.0, 0.0), p(50.0, 0.0)]],
        };
        ribbon.cut(p(20.0, 0.0), 10.0, Some(p(50.0, 0.0)));
        assert_eq!(
            ribbon.paths,
            vec![
                vec![p(0.0, 0.0), p(10.0, 0.0)],
                vec![p(30.0, 0.0), p(50.0, 0.0)]
            ]
        );
    }
}
