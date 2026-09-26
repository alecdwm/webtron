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

/// Each human player's wins, kills and deaths, kept in a text file with one
/// JSON object per line:
///
/// ```text
/// {"name":"alec","wins":12,"color":"blue","rounds_played":40,"player_kills":4,"npc_kills":30,"player_deaths":3,"npc_deaths":20,"suicides":2,"arena_deaths":5,"headon_crashes":1}
/// ```
///
/// - `wins`: rounds won.
/// - `color`: the colour of their last win.
/// - `rounds_played`: rounds started, including ones cut short.
/// - `player_kills`: human players who crashed into this player's trail.
/// - `npc_kills`: NPCs that crashed into this player's trail.
/// - `player_deaths`: crashes into another human player's trail.
/// - `npc_deaths`: crashes into an NPC's trail.
/// - `suicides`: crashes into their own trail.
/// - `arena_deaths`: crashes into the arena boundary.
/// - `headon_crashes`: crashes into another lightcycle.
///
/// Fields are written in this order, and counts of zero are left out. Any
/// of these but `name` can be left out when editing. JSON escapes line breaks and
/// quotes inside names, so no name can break the format. Players are
/// identified by their name in the form the client gives it (see
/// `normalize_name`), which is how it is written to the file; lines for the
/// same player are added together. A player gets a line the first time one
/// of these happens to them.
///
/// Players are ranked by wins, and players with no wins aren't ranked.
/// Players with the same number of wins are
/// ranked in the order they appear in the file, which is the order they
/// reached that number: the file is kept sorted by wins, a new player is
/// added at the end, and sorting never moves a player past others with the
/// same number of wins.
///
/// The file is the source of truth: it is read afresh for every change, so
/// it can be edited while the server runs. A change updates the player's
/// first line, or adds a line at the end for a new player, then sorts the
/// lines by the players' wins. Lines that can't be read stay where they are
/// and are otherwise ignored. Other lines are only rewritten when their
/// record changes. The file is replaced by writing a temporary file next to
/// it and renaming it over.
#[derive(Debug, Clone)]
pub struct Leaderboard {
    path: PathBuf,
    /// Held while reading and rewriting the file, so changes from different
    /// arenas can't overwrite each other.
    lock: Arc<Mutex<()>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Record {
    name: String,
    #[serde(default, skip_serializing_if = "is_zero")]
    wins: u32,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_color"
    )]
    color: Option<PlayerColor>,
    #[serde(default, skip_serializing_if = "is_zero")]
    rounds_played: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    player_kills: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    npc_kills: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    player_deaths: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    npc_deaths: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    suicides: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    arena_deaths: u32,
    #[serde(default, skip_serializing_if = "is_zero")]
    headon_crashes: u32,
}

fn is_zero(count: &u32) -> bool {
    *count == 0
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

/// Something that happened to a player, to count on the leaderboard.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stat {
    /// Won a round, in this colour.
    Win(PlayerColor),
    RoundPlayed,
    PlayerKill,
    NpcKill,
    PlayerDeath,
    NpcDeath,
    Suicide,
    ArenaDeath,
    HeadonCrash,
}

/// One player's totals from the leaderboard file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlayerStats {
    pub name: String,
    pub wins: u32,
    pub player_kills: u32,
    pub npc_kills: u32,
    pub suicides: u32,
    pub arena_deaths: u32,
    pub color: Option<PlayerColor>,
    pub rounds_played: u32,
    pub player_deaths: u32,
    pub npc_deaths: u32,
    pub headon_crashes: u32,
}

impl PlayerStats {
    fn kills(&self) -> u32 {
        self.player_kills.saturating_add(self.npc_kills)
    }
}

/// Every player's totals, in the order the players first appear in the
/// leaderboard file.
#[derive(Debug, Default, PartialEq)]
pub struct Stats(Vec<PlayerStats>);

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LeaderboardEntry {
    /// Position on the leaderboard, or `None` for a player who isn't ranked.
    pub rank: Option<u32>,
    pub name: String,
    pub wins: u32,
    /// Human players and NPCs killed.
    pub kills: u32,
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

    /// Reads the file, adds one to the named players' counts for each event,
    /// writes it back if anything changed, and returns the new totals.
    pub fn record(&self, events: &[(String, Stat)]) -> Result<Stats, Error> {
        let _lock = self
            .lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut lines = self.read_lines()?;

        let mut changed = false;
        for (name, stat) in events {
            let key = normalize_name(name);
            if key.is_empty() {
                continue;
            }

            let index = match lines.iter().position(
                |line| matches!(line, Line::Record { key: line_key, .. } if *line_key == key),
            ) {
                Some(index) => index,
                None => {
                    lines.push(Line::Record {
                        record: Record::default(),
                        key: key.clone(),
                        text: None,
                    });
                    lines.len() - 1
                }
            };
            let Line::Record { record, text, .. } = &mut lines[index] else {
                unreachable!("the line was found or added as a record");
            };

            record.name = key;
            match stat {
                Stat::Win(color) => {
                    record.wins = record.wins.saturating_add(1);
                    record.color = Some(*color);
                }
                Stat::RoundPlayed => record.rounds_played = record.rounds_played.saturating_add(1),
                Stat::PlayerKill => record.player_kills = record.player_kills.saturating_add(1),
                Stat::NpcKill => record.npc_kills = record.npc_kills.saturating_add(1),
                Stat::PlayerDeath => record.player_deaths = record.player_deaths.saturating_add(1),
                Stat::NpcDeath => record.npc_deaths = record.npc_deaths.saturating_add(1),
                Stat::Suicide => record.suicides = record.suicides.saturating_add(1),
                Stat::ArenaDeath => record.arena_deaths = record.arena_deaths.saturating_add(1),
                Stat::HeadonCrash => {
                    record.headon_crashes = record.headon_crashes.saturating_add(1)
                }
            }
            *text = None;
            changed = true;
        }

        if changed {
            sort_lines(&mut lines);
            self.write_lines(&lines)?;
        }

        Ok(stats(&lines))
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

fn stats(lines: &[Line]) -> Stats {
    let mut stats: Vec<PlayerStats> = Vec::new();
    let mut indices: HashMap<&str, usize> = HashMap::new();
    for line in lines {
        let Line::Record { key, record, .. } = line else {
            continue;
        };
        let index = *indices.entry(key).or_insert_with(|| {
            stats.push(PlayerStats {
                name: key.clone(),
                ..Default::default()
            });
            stats.len() - 1
        });
        let player = &mut stats[index];
        // Counts saturate, so a huge count typed into the file can't overflow.
        player.wins = player.wins.saturating_add(record.wins);
        player.player_kills = player.player_kills.saturating_add(record.player_kills);
        player.npc_kills = player.npc_kills.saturating_add(record.npc_kills);
        player.suicides = player.suicides.saturating_add(record.suicides);
        player.arena_deaths = player.arena_deaths.saturating_add(record.arena_deaths);
        player.rounds_played = player.rounds_played.saturating_add(record.rounds_played);
        player.player_deaths = player.player_deaths.saturating_add(record.player_deaths);
        player.npc_deaths = player.npc_deaths.saturating_add(record.npc_deaths);
        player.headon_crashes = player.headon_crashes.saturating_add(record.headon_crashes);
        // The first line with a colour gives the player's colour.
        player.color = player.color.or(record.color);
    }
    Stats(stats)
}

/// Sorts the records by their players' total wins, most first, keeping the
/// order of records with the same total. Lines that aren't records keep
/// their places, and the records are sorted around them.
fn sort_lines(lines: &mut [Line]) {
    let totals: HashMap<String, u32> = stats(lines)
        .0
        .into_iter()
        .map(|player| (player.name, player.wins))
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

impl Stats {
    /// The top players, and the named player's own entry.
    pub fn standings(&self, name: Option<&str>) -> LeaderboardStandings {
        // Most wins first, then in file order, which `sort_by_key` keeps as
        // it is stable. Players with no wins aren't ranked.
        let mut ranked: Vec<&PlayerStats> =
            self.0.iter().filter(|player| player.wins > 0).collect();
        ranked.sort_by_key(|player| Reverse(player.wins));

        let entry = |rank: Option<u32>, player: &PlayerStats| LeaderboardEntry {
            rank,
            name: player.name.clone(),
            wins: player.wins,
            kills: player.kills(),
            color: player.color,
        };

        let top = ranked
            .iter()
            .take(TOP_COUNT)
            .enumerate()
            .map(|(index, player)| entry(Some(index as u32 + 1), player))
            .collect();

        let player = name
            .map(normalize_name)
            .filter(|name| !name.is_empty())
            .map(
                |name| match ranked.iter().position(|player| player.name == name) {
                    Some(index) => entry(Some(index as u32 + 1), ranked[index]),
                    None => match self.0.iter().find(|player| player.name == name) {
                        Some(player) => entry(None, player),
                        None => entry(
                            None,
                            &PlayerStats {
                                name,
                                ..Default::default()
                            },
                        ),
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
            kills: 0,
            color: Some(PlayerColor::White),
        }
    }

    impl Leaderboard {
        fn record_win_in_white(&self, name: &str) -> Result<Stats, Error> {
            self.record_win(name, PlayerColor::White)
        }

        fn record_win(&self, name: &str, color: PlayerColor) -> Result<Stats, Error> {
            self.record(&[(name.to_string(), Stat::Win(color))])
        }

        fn stats(&self) -> Result<Stats, Error> {
            self.record(&[])
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
        let wins = leaderboard.0.stats().unwrap();
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
        let wins = leaderboard.0.stats().unwrap();
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
        let names = |wins: Stats| -> Vec<String> {
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
    fn counts_kills_and_deaths() {
        let leaderboard = TestLeaderboard::new("kills", None);
        let event = |name: &str, stat| (name.to_string(), stat);
        leaderboard
            .0
            .record(&[
                event("alec", Stat::NpcKill),
                event("alec", Stat::NpcKill),
                event("alec", Stat::PlayerKill),
                event("alec", Stat::Suicide),
                event("bob", Stat::ArenaDeath),
                event("bob", Stat::HeadonCrash),
                event("bob", Stat::NpcDeath),
                event("bob", Stat::PlayerDeath),
                event("bob", Stat::RoundPlayed),
            ])
            .unwrap();
        let stats = leaderboard
            .0
            .record(&[event("alec", Stat::ArenaDeath)])
            .unwrap();

        assert_eq!(
            leaderboard.contents(),
            concat!(
                "{\"name\":\"alec\",\"player_kills\":1,\"npc_kills\":2,\"suicides\":1,\"arena_deaths\":1}\n",
                "{\"name\":\"bob\",\"rounds_played\":1,\"player_deaths\":1,\"npc_deaths\":1,\"arena_deaths\":1,\"headon_crashes\":1}\n",
            )
        );

        // Players without wins aren't ranked, but still see their own entry.
        let standings = stats.standings(Some("alec"));
        assert_eq!(standings.top, vec![]);
        assert_eq!(
            standings.player,
            Some(LeaderboardEntry {
                kills: 3,
                color: None,
                ..entry(None, "alec", 0)
            })
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
