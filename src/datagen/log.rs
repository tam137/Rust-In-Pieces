//! `batch.log`: the history of one data generation run, one event per line.
//!
//! A line is `<time> <event> key=value key=value ...`, the time in RFC 3339 with the offset of
//! the generating host, so the file reads as plain text and `rip-datagen stats` can parse it
//! back. Values that contain anything but `[A-Za-z0-9._:/+-]` are written in double quotes with
//! `\"` and `\\` escaped.
//!
//! Events, in the order a run writes them:
//! - `start`: batch, versions, net, source, threads, game target, base seed, log interval;
//! - `config`: every game setting;
//! - `thread`: per thread, its seed and game target;
//! - `progress`: per thread every `log_interval` seconds, its cumulative counters;
//! - `thread_done`: per thread when it ends, its final counters;
//! - `stop`: Ctrl+C was pressed, the threads finish their current game;
//! - `error`: a thread failed;
//! - `end`: status `done` or `stopped`, totals and the run time.

use std::fmt::Write as _;

use chrono::{DateTime, FixedOffset};

use super::generator::Counters;

/// File name of the log in a batch directory.
pub const LOG_FILE: &str = "batch.log";

/// One parsed line.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub time: DateTime<FixedOffset>,
    pub name: String,
    pub fields: Vec<(String, String)>,
}

impl Event {
    /// The value of `key`, if present.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// The value of `key` as a number, 0 when it is missing or not a number.
    pub fn num(&self, key: &str) -> u64 {
        self.get(key).and_then(|v| v.parse().ok()).unwrap_or(0)
    }

    /// The counters of a `progress` or `thread_done` line.
    pub fn counters(&self) -> Counters {
        Counters {
            games: self.num("games"),
            dropped: self.num("dropped"),
            positions: self.num("positions"),
            filtered: self.num("filtered"),
            skipped: self.num("skipped"),
            book_plies: self.num("book_plies"),
            plies: self.num("plies"),
            white_wins: self.num("white"),
            draws: self.num("draws"),
            black_wins: self.num("black"),
        }
    }
}

/// The key/value pairs of a set of counters, as `progress` and `thread_done` write them.
pub fn counter_fields(c: &Counters) -> Vec<(&'static str, String)> {
    vec![
        ("games", c.games.to_string()),
        ("dropped", c.dropped.to_string()),
        ("positions", c.positions.to_string()),
        ("filtered", c.filtered.to_string()),
        ("skipped", c.skipped.to_string()),
        ("plies", c.plies.to_string()),
        ("book_plies", c.book_plies.to_string()),
        ("white", c.white_wins.to_string()),
        ("draws", c.draws.to_string()),
        ("black", c.black_wins.to_string()),
    ]
}

fn is_plain(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '/' | '+' | '-'))
}

/// Formats one line (without the newline).
pub fn format_event<K: AsRef<str>, V: AsRef<str>>(
    time: &DateTime<FixedOffset>,
    name: &str,
    fields: &[(K, V)],
) -> String {
    let mut line = format!(
        "{} {name}",
        time.to_rfc3339_opts(chrono::SecondsFormat::Secs, false)
    );
    for (key, value) in fields {
        let value = value.as_ref();
        if is_plain(value) {
            let _ = write!(line, " {}={value}", key.as_ref());
        } else {
            let escaped = value
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', " ");
            let _ = write!(line, " {}=\"{escaped}\"", key.as_ref());
        }
    }
    line
}

/// Parses one line; `None` for a line that is not an event (empty, truncated or foreign).
pub fn parse_event(line: &str) -> Option<Event> {
    let mut rest = line.trim_end();
    let (time, after) = rest.split_once(' ')?;
    let time = DateTime::parse_from_rfc3339(time).ok()?;
    rest = after;
    let (name, mut rest) = rest.split_once(' ').unwrap_or((rest, ""));
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
        return None;
    }
    let mut fields = Vec::new();
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        let (key, after) = rest.split_once('=')?;
        if let Some(quoted) = after.strip_prefix('"') {
            let mut value = String::new();
            let mut chars = quoted.char_indices();
            let mut end = None;
            while let Some((i, c)) = chars.next() {
                match c {
                    '\\' => {
                        if let Some((_, next)) = chars.next() {
                            value.push(next);
                        }
                    }
                    '"' => {
                        end = Some(i + 1);
                        break;
                    }
                    _ => value.push(c),
                }
            }
            fields.push((key.to_string(), value));
            rest = &quoted[end?..];
        } else {
            let (value, after) = after.split_once(' ').unwrap_or((after, ""));
            fields.push((key.to_string(), value.to_string()));
            rest = after;
        }
    }
    Some(Event {
        time,
        name: name.to_string(),
        fields,
    })
}

/// Every event of a log text; lines that do not parse are skipped.
pub fn parse_log(text: &str) -> Vec<Event> {
    text.lines().filter_map(parse_event).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(text).unwrap()
    }

    #[test]
    fn a_line_reads_back_as_it_was_written() {
        let time = at("2026-11-24T14:03:00+01:00");
        let line = format_event(
            &time,
            "start",
            &[
                ("batch", "20261124-140300_42"),
                ("net", "r5 rnd \"quoted\" \\ name"),
                ("source", "host-a"),
                ("empty", ""),
            ],
        );
        assert!(
            line.starts_with("2026-11-24T14:03:00+01:00 start batch=20261124-140300_42 net=\"")
        );
        let event = parse_event(&line).unwrap();
        assert_eq!(event.time, time);
        assert_eq!(event.name, "start");
        assert_eq!(event.get("net"), Some("r5 rnd \"quoted\" \\ name"));
        assert_eq!(event.get("source"), Some("host-a"));
        assert_eq!(event.get("empty"), Some(""));
        assert_eq!(event.get("missing"), None);
    }

    #[test]
    fn counters_survive_a_round_trip() {
        let c = Counters {
            games: 12,
            dropped: 1,
            positions: 900,
            filtered: 80,
            skipped: 3,
            book_plies: 0,
            plies: 1500,
            white_wins: 5,
            draws: 4,
            black_wins: 3,
        };
        let line = format_event(&at("2026-11-24T14:04:00Z"), "progress", &counter_fields(&c));
        assert_eq!(parse_event(&line).unwrap().counters(), c);
    }

    #[test]
    fn foreign_and_truncated_lines_are_skipped() {
        let text = "garbage\n\n2026-11-24T14:04:00+00:00 end status=done\n2026-11-24T14:0";
        let events = parse_log(text);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].get("status"), Some("done"));
        assert!(parse_event("2026-11-24T14:04:00+00:00 progress key=\"open").is_none());
    }
}
