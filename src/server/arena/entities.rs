use super::*;

#[derive(Debug, Copy, Clone, PartialEq, Serialize)]
pub struct Lightcycle {
    pub position: ArenaPoint,
    pub direction: Direction,
    pub speed: f64,
    pub dead: bool,
    pub death_cause: Option<DeathCause>,
}

impl Default for Lightcycle {
    fn default() -> Self {
        Self {
            position: ArenaPoint::origin(),
            direction: Direction::Up,
            speed: LIGHTCYCLE_SPEED,
            dead: Default::default(),
            death_cause: None,
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
            current.push(from.lerp(to, enter));
        }
        if current.len() >= 2 {
            pieces.push(mem::take(&mut current));
        } else {
            current.clear();
        }
        if exit < 1.0 {
            current.push(from.lerp(to, exit));
            current.push(to);
        }
    }

    if current.len() >= 2 {
        pieces.push(current);
    }

    pieces
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
