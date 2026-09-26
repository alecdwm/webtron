use anyhow::{Context, Error};
use log::warn;
use serde_derive::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::mem;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::server::PlayerColor;

const TOP_COUNT: usize = 10;
/// Longest name kept on the leaderboard, in characters.
const MAX_NAME_LENGTH: usize = 32;

/// The number of rounds each player has won, kept in a text file with one
/// JSON object per line:
///
/// ```text
/// {"name":"alec","wins":12,"color":"blue"}
/// {"name":"bob","wins":3,"color":"red"}
/// ```
///
/// JSON escapes line breaks and quotes inside names, so no name can break
/// the format. Players are identified by their name in the form the client
/// gives it (see `normalize_name`), which is how it is written to the file;
/// lines for the same player are added together. `color` is the colour the
/// player used in their last win, and can be left out.
///
/// Players are ranked by wins. Players with the same number of wins are
/// ranked in the order they appear in the file, which is the order they
/// reached that number: the file is kept sorted by wins, a player's first
/// win adds them at the end, and sorting never moves a player past others
/// with the same number of wins.
///
/// The file is the source of truth: it is read afresh for every win, so it
/// can be edited while the server runs. A win updates the player's first
/// line, or adds a line at the end for a player's first win, then sorts the
/// lines by the players' wins. Lines that can't be read stay where they are
/// and are otherwise ignored. Other lines are only rewritten when their
/// record changes. The file is replaced by writing a temporary file next to
/// it and renaming it over.
#[derive(Debug, Clone)]
pub struct Leaderboard {
    path: PathBuf,
    /// Held while reading and rewriting the file, so wins in different
    /// arenas can't overwrite each other.
    lock: Arc<Mutex<()>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Record {
    name: String,
    wins: u32,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_color"
    )]
    color: Option<PlayerColor>,
}

/// Reads a colour, treating one that isn't a player colour as no colour, so
/// a mistyped colour doesn't make the whole line unreadable.
fn deserialize_color<'de, D>(deserializer: D) -> Result<Option<PlayerColor>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = <serde_json::Value as serde::Deserialize>::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).ok())
}

/// One line of the leaderboard file.
enum Line {
    Record {
        key: String,
        record: Record,
        /// The line as it was read, until the record changes.
        text: Option<String>,
    },
    /// A line that isn't a record, kept as it was.
    Other(String),
}

/// Wins and colour per player, in the order the players first appear in the
/// leaderboard file.
#[derive(Debug, Default, PartialEq)]
pub struct Wins(Vec<(String, u32, Option<PlayerColor>)>);

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LeaderboardEntry {
    /// Position on the leaderboard, or `None` for a player with no wins.
    pub rank: Option<u32>,
    pub name: String,
    pub wins: u32,
    pub color: Option<PlayerColor>,
}

/// The leaderboard as one player sees it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LeaderboardStandings {
    pub top: Vec<LeaderboardEntry>,
    /// The player's own entry, whether or not they are in `top`.
    pub player: Option<LeaderboardEntry>,
}

impl Leaderboard {
    /// Checks that the file at `path` can be read. It doesn't need to exist.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let leaderboard = Self {
            path: path.to_path_buf(),
            lock: Default::default(),
        };
        leaderboard.read_lines()?;
        Ok(leaderboard)
    }

    fn read_lines(&self) -> Result<Vec<Line>, Error> {
        let contents = match fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("Failed to read leaderboard {}", self.path.display())
                });
            }
        };

        let lines: Vec<Line> = contents.lines().map(parse_line).collect();

        let unreadable = unreadable_line_numbers(&lines);
        if !unreadable.is_empty() {
            let numbers: Vec<String> = unreadable.iter().map(ToString::to_string).collect();
            warn!(
                "Ignoring unreadable lines in leaderboard {}: {}",
                self.path.display(),
                numbers.join(", ")
            );
        }

        Ok(lines)
    }

    /// Reads the current wins from the file.
    pub fn wins(&self) -> Result<Wins, Error> {
        let _lock = self
            .lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Ok(wins(&self.read_lines()?))
    }

    /// Adds a win for the named player and sets their colour, then returns
    /// the wins including it.
    pub fn record_win(&self, name: &str, color: PlayerColor) -> Result<Wins, Error> {
        let _lock = self
            .lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut lines = self.read_lines()?;

        let key = normalize_name(name);
        if !key.is_empty() {
            let existing = lines.iter_mut().find_map(|line| match line {
                Line::Record {
                    key: line_key,
                    record,
                    text,
                } if *line_key == key => Some((record, text)),
                _ => None,
            });
            match existing {
                Some((record, text)) => {
                    record.name = key;
                    record.wins += 1;
                    record.color = Some(color);
                    *text = None;
                }
                None => lines.push(Line::Record {
                    record: Record {
                        name: key.clone(),
                        wins: 1,
                        color: Some(color),
                    },
                    key,
                    text: None,
                }),
            }
            sort_lines(&mut lines);
            self.write_lines(&lines)?;
        }

        Ok(wins(&lines))
    }

    fn write_lines(&self, lines: &[Line]) -> Result<(), Error> {
        let mut contents = String::new();
        for line in lines {
            match line {
                Line::Record {
                    text: Some(text), ..
                } => contents.push_str(text),
                Line::Record { record, .. } => contents.push_str(&serde_json::to_string(record)?),
                Line::Other(other) => contents.push_str(other),
            }
            contents.push('\n');
        }

        let mut temporary = self.path.clone().into_os_string();
        temporary.push(".tmp");
        fs::write(&temporary, contents)
            .with_context(|| format!("Failed to write {}", temporary.to_string_lossy()))?;
        fs::rename(&temporary, &self.path)
            .with_context(|| format!("Failed to replace leaderboard {}", self.path.display()))?;

        Ok(())
    }
}

/// The 1-based numbers of lines that aren't records. Blank lines, including
/// any after the file's final newline, don't count.
fn unreadable_line_numbers(lines: &[Line]) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, line)| matches!(line, Line::Other(other) if !other.trim().is_empty()))
        .map(|(index, _)| index + 1)
        .collect()
}

fn parse_line(line: &str) -> Line {
    match serde_json::from_str::<Record>(line) {
        Ok(record) => {
            let key = normalize_name(&record.name);
            if key.is_empty() {
                Line::Other(line.to_string())
            } else {
                Line::Record {
                    key,
                    record,
                    text: Some(line.to_string()),
                }
            }
        }
        Err(_) => Line::Other(line.to_string()),
    }
}

fn wins(lines: &[Line]) -> Wins {
    let mut wins: Vec<(String, u32, Option<PlayerColor>)> = Vec::new();
    for line in lines {
        if let Line::Record { key, record, .. } = line {
            match wins.iter_mut().find(|(name, _, _)| name == key) {
                Some(entry) => {
                    entry.1 += record.wins;
                    // The first line with a colour gives the player's colour.
                    entry.2 = entry.2.or(record.color);
                }
                None => wins.push((key.clone(), record.wins, record.color)),
            }
        }
    }
    Wins(wins)
}

/// Sorts the records by their players' total wins, most first, keeping the
/// order of records with the same total. Lines that aren't records keep
/// their places, and the records are sorted around them.
fn sort_lines(lines: &mut [Line]) {
    let totals: HashMap<String, u32> = wins(lines)
        .0
        .into_iter()
        .map(|(name, wins, _)| (name, wins))
        .collect();

    let slots: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| matches!(line, Line::Record { .. }))
        .map(|(index, _)| index)
        .collect();

    let mut records: Vec<Line> = slots
        .iter()
        .map(|index| mem::replace(&mut lines[*index], Line::Other(String::new())))
        .collect();
    // `sort_by_key` is stable, so records with the same total keep their order.
    records.sort_by_key(|line| match line {
        Line::Record { key, .. } => Reverse(totals[key]),
        Line::Other(_) => Reverse(0),
    });

    for (index, record) in slots.into_iter().zip(records) {
        lines[index] = record;
    }
}

impl Wins {
    /// The top players, and the named player's own entry.
    pub fn standings(&self, name: Option<&str>) -> LeaderboardStandings {
        // Most wins first, then in file order, which `sort_by_key` keeps as
        // it is stable. Players with no wins aren't ranked.
        let mut ranked: Vec<(&String, u32, Option<PlayerColor>)> = self
            .0
            .iter()
            .filter(|(_, wins, _)| *wins > 0)
            .map(|(name, wins, color)| (name, *wins, *color))
            .collect();
        ranked.sort_by_key(|(_, wins, _)| Reverse(*wins));

        let entry = |index: usize| {
            let (name, wins, color) = ranked[index];
            LeaderboardEntry {
                rank: Some(index as u32 + 1),
                name: name.to_string(),
                wins,
                color,
            }
        };

        let top = (0..ranked.len().min(TOP_COUNT)).map(entry).collect();

        let player = name
            .map(normalize_name)
            .filter(|name| !name.is_empty())
            .map(
                |name| match ranked.iter().position(|(ranked, _, _)| **ranked == name) {
                    Some(index) => entry(index),
                    None => LeaderboardEntry {
                        rank: None,
                        name,
                        wins: 0,
                        color: None,
                    },
                },
            );

        LeaderboardStandings { top, player }
    }
}

/// Puts a name in the form the client's name input produces: lowercase,
/// with spaces as underscores and no other whitespace. Names are also cut
/// to `MAX_NAME_LENGTH` characters.
pub fn normalize_name(name: &str) -> String {
    name.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c == ' ' { '_' } else { c })
        .filter(|c| !c.is_whitespace())
        .take(MAX_NAME_LENGTH)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(rank: Option<u32>, name: &str, wins: u32) -> LeaderboardEntry {
        LeaderboardEntry {
            rank,
            name: name.to_string(),
            wins,
            color: Some(PlayerColor::White),
        }
    }

    impl Leaderboard {
        fn record_win_in_white(&self, name: &str) -> Result<Wins, Error> {
            self.record_win(name, PlayerColor::White)
        }
    }

    /// A leaderboard in a fresh file, removed when dropped.
    struct TestLeaderboard(Leaderboard);

    impl TestLeaderboard {
        fn new(name: &str, contents: Option<&str>) -> Self {
            let path =
                std::env::temp_dir().join(format!("webtron-{}-{}.jsonl", name, std::process::id()));
            let _ = fs::remove_file(&path);
            if let Some(contents) = contents {
                fs::write(&path, contents).unwrap();
            }
            Self(Leaderboard::open(&path).unwrap())
        }

        fn contents(&self) -> String {
            fs::read_to_string(&self.0.path).unwrap_or_default()
        }
    }

    impl Drop for TestLeaderboard {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0.path);
        }
    }

    #[test]
    fn counts_wins_by_name() {
        let leaderboard = TestLeaderboard::new("counts", None);
        leaderboard.0.record_win_in_white("alec").unwrap();
        leaderboard.0.record_win_in_white(" ALEC ").unwrap();
        leaderboard.0.record_win_in_white("AlEc").unwrap();
        let wins = leaderboard.0.record_win_in_white("bob").unwrap();

        let standings = wins.standings(Some("Bob"));
        assert_eq!(
            standings.top,
            vec![entry(Some(1), "alec", 3), entry(Some(2), "bob", 1)]
        );
        assert_eq!(standings.player, Some(entry(Some(2), "bob", 1)));
        assert_eq!(
            leaderboard.contents(),
            concat!(
                "{\"name\":\"alec\",\"wins\":3,\"color\":\"white\"}\n",
                "{\"name\":\"bob\",\"wins\":1,\"color\":\"white\"}\n",
            )
        );
    }

    #[test]
    fn only_winners_are_written() {
        let leaderboard = TestLeaderboard::new("winners", None);
        let wins = leaderboard.0.wins().unwrap();
        assert_eq!(
            wins.standings(Some("carol")).player,
            Some(LeaderboardEntry {
                color: None,
                ..entry(None, "carol", 0)
            })
        );
        assert_eq!(leaderboard.contents(), "");

        leaderboard.0.record_win_in_white("   ").unwrap();
        assert_eq!(leaderboard.contents(), "");
    }

    #[test]
    fn ranks_players_outside_the_top() {
        let leaderboard = TestLeaderboard::new("ranks", None);
        for index in 0..12 {
            for _ in 0..=index {
                leaderboard
                    .0
                    .record_win_in_white(&format!("player{:02}", index))
                    .unwrap();
            }
        }
        let wins = leaderboard.0.record_win_in_white("zed").unwrap();

        let standings = wins.standings(Some("zed"));
        assert_eq!(standings.top.len(), 10);
        assert_eq!(standings.top[0], entry(Some(1), "player11", 12));
        assert_eq!(standings.player, Some(entry(Some(13), "zed", 1)));
    }

    #[test]
    fn edits_to_the_file_are_kept() {
        let leaderboard = TestLeaderboard::new(
            "edits",
            Some(concat!(
                "# hand-edited\n",
                "{\"name\":\"bob\",\"wins\":7}\n",
                "{\"name\":\"ALEC\",\"wins\":2}\n",
                "{\"name\":\"alec\",\"wins\":1,\"note\":\"merged\"}\n",
                "{\"name\":\"broken\",\"wins\":\n",
                "\n",
                "{\"name\":\"ZERO\",\"wins\":0}\n",
            )),
        );

        let wins = leaderboard.0.record_win_in_white("Alec").unwrap();
        assert_eq!(
            leaderboard.contents(),
            concat!(
                "# hand-edited\n",
                "{\"name\":\"bob\",\"wins\":7}\n",
                "{\"name\":\"alec\",\"wins\":3,\"color\":\"white\"}\n",
                "{\"name\":\"alec\",\"wins\":1,\"note\":\"merged\"}\n",
                "{\"name\":\"broken\",\"wins\":\n",
                "\n",
                "{\"name\":\"ZERO\",\"wins\":0}\n",
            )
        );
        assert_eq!(
            wins.standings(None).top,
            vec![
                LeaderboardEntry {
                    color: None,
                    ..entry(Some(1), "bob", 7)
                },
                entry(Some(2), "alec", 4)
            ]
        );
    }

    #[test]
    fn any_name_survives_a_reload() {
        let names = [
            "new\nline",
            "carriage\rreturn",
            "\"quoted\", {\"wins\":999}",
            "back\\slash",
            "tab\tand\u{0}nul",
            "line\u{2028}separator",
            "ünïcødé 🏍",
        ];
        let leaderboard = TestLeaderboard::new("names", None);
        for name in names {
            leaderboard.0.record_win_in_white(name).unwrap();
        }

        assert_eq!(leaderboard.contents().lines().count(), names.len());
        let wins = leaderboard.0.wins().unwrap();
        for name in names {
            assert_eq!(
                wins.standings(Some(name)).player.map(|entry| entry.wins),
                Some(1),
                "{:?}",
                name
            );
        }
    }

    #[test]
    fn keeps_the_colour_of_the_last_win() {
        let leaderboard = TestLeaderboard::new(
            "colours",
            Some(concat!(
                "{\"name\":\"bob\",\"wins\":2,\"color\":\"green\"}\n",
                "{\"name\":\"carol\",\"wins\":1,\"color\":\"mauve\"}\n",
            )),
        );

        leaderboard.0.record_win("alec", PlayerColor::Blue).unwrap();
        let wins = leaderboard
            .0
            .record_win("ALEC", PlayerColor::Purple)
            .unwrap();

        assert_eq!(
            leaderboard.contents(),
            concat!(
                "{\"name\":\"bob\",\"wins\":2,\"color\":\"green\"}\n",
                "{\"name\":\"alec\",\"wins\":2,\"color\":\"purple\"}\n",
                "{\"name\":\"carol\",\"wins\":1,\"color\":\"mauve\"}\n",
            )
        );

        // A colour that isn't a player colour is treated as none.
        let colors: Vec<_> = wins
            .standings(None)
            .top
            .iter()
            .map(|entry| entry.color)
            .collect();
        assert_eq!(
            colors,
            vec![Some(PlayerColor::Green), Some(PlayerColor::Purple), None]
        );
    }

    #[test]
    fn ties_keep_the_order_players_reached_them() {
        let leaderboard = TestLeaderboard::new(
            "order",
            Some(concat!(
                "{\"name\":\"bob\",\"wins\":2}\n",
                "not json\n",
                "{\"name\":\"carol\",\"wins\":1}\n",
                "{\"name\":\"dave\",\"wins\":1}\n",
            )),
        );
        let names = |wins: Wins| -> Vec<String> {
            wins.standings(None)
                .top
                .into_iter()
                .map(|entry| entry.name)
                .collect()
        };

        // Dave reaches 2 wins after bob, and stays behind him.
        assert_eq!(
            names(leaderboard.0.record_win_in_white("dave").unwrap()),
            ["bob", "dave", "carol"]
        );
        // A new player goes to the end, behind carol who has 1 win too.
        assert_eq!(
            names(leaderboard.0.record_win_in_white("erin").unwrap()),
            ["bob", "dave", "carol", "erin"]
        );
        // Carol overtakes everyone, and the unreadable line stays put.
        leaderboard.0.record_win_in_white("carol").unwrap();
        assert_eq!(
            names(leaderboard.0.record_win_in_white("carol").unwrap()),
            ["carol", "bob", "dave", "erin"]
        );
        assert_eq!(
            leaderboard.contents(),
            concat!(
                "{\"name\":\"carol\",\"wins\":3,\"color\":\"white\"}\n",
                "not json\n",
                "{\"name\":\"bob\",\"wins\":2}\n",
                "{\"name\":\"dave\",\"wins\":2,\"color\":\"white\"}\n",
                "{\"name\":\"erin\",\"wins\":1,\"color\":\"white\"}\n",
            )
        );
    }

    #[test]
    fn only_non_blank_lines_that_are_not_records_are_unreadable() {
        let unreadable = |contents: &str| {
            let lines: Vec<Line> = contents.lines().map(parse_line).collect();
            unreadable_line_numbers(&lines)
        };
        let record = "{\"name\":\"alec\",\"wins\":1}";

        assert_eq!(unreadable(&format!("{record}\n")), Vec::<usize>::new());
        assert_eq!(unreadable(&format!("{record}\r\n")), Vec::<usize>::new());
        assert_eq!(
            unreadable(&format!("{record}\n\n  \n")),
            Vec::<usize>::new()
        );
        assert_eq!(unreadable(&format!("{record}\nnot json\n")), vec![2]);
    }

    #[test]
    fn names_match_the_client() {
        assert_eq!(normalize_name("  Big Al\t\n "), "big_al");
    }

    #[test]
    fn long_names_are_cut() {
        assert_eq!(
            normalize_name(&"A".repeat(100)),
            "a".repeat(MAX_NAME_LENGTH)
        );
    }
}
