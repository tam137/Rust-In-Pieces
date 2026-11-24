//! `rip-datagen stats`: statistics over every batch of a data directory, read from the
//! `batch.log` files (`log`), rendered as plain text with Unicode bars and a heatmap.
//!
//! Every `progress` and `thread_done` line carries a thread's cumulative counters; the difference
//! to the thread's previous line is the work done in that interval, and it is booked at the time
//! of the line in the viewer's time zone. With the default interval of 60 s this places the data
//! on the right hour to within a minute.
//!
//! `load` reads, `aggregate` sums, `render` draws: the two last ones are pure and are tested on
//! fixed data.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use chrono::{
    DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Timelike,
};

use super::generator::Counters;
use super::log::{Event, LOG_FILE, parse_log};
use super::runner::sum;

/// Work done in one log interval of one thread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Increment {
    pub time: DateTime<FixedOffset>,
    pub games: u64,
    pub positions: u64,
}

/// What `stats` knows about one batch.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchStats {
    pub name: String,
    pub source: String,
    pub net: String,
    pub fnv: String,
    pub datagen: String,
    pub threads: u64,
    /// `done`, `stopped`, `running`, or `aborted` (no `end` line and no line for a while).
    pub status: String,
    pub started: DateTime<FixedOffset>,
    pub last: DateTime<FixedOffset>,
    pub totals: Counters,
    pub increments: Vec<Increment>,
}

/// Reads one batch from its log events; `None` without a `start` line.
pub fn batch_from_events(events: &[Event], now: DateTime<FixedOffset>) -> Option<BatchStats> {
    let start = events.iter().find(|e| e.name == "start")?;
    let text = |key: &str| start.get(key).unwrap_or("?").to_string();
    let interval = start.num("log_interval").max(1) as i64;
    let mut latest: BTreeMap<u64, Counters> = BTreeMap::new();
    let mut increments = Vec::new();
    let mut status = None;
    for e in events {
        match e.name.as_str() {
            "progress" | "thread_done" => {
                let c = e.counters();
                let before = latest
                    .insert(e.num("thread"), c.clone())
                    .unwrap_or_default();
                let (games, positions) = (
                    c.games.saturating_sub(before.games),
                    c.positions.saturating_sub(before.positions),
                );
                if games > 0 || positions > 0 {
                    increments.push(Increment {
                        time: e.time,
                        games,
                        positions,
                    });
                }
            }
            "end" => status = e.get("status").map(str::to_string),
            _ => {}
        }
    }
    let last = events.iter().map(|e| e.time).max().unwrap_or(start.time);
    let status = status.unwrap_or_else(|| {
        let quiet = now.signed_duration_since(last) > Duration::seconds(3 * interval + 60);
        if quiet { "aborted" } else { "running" }.to_string()
    });
    Some(BatchStats {
        name: text("batch"),
        source: text("source"),
        net: text("net"),
        fnv: text("fnv"),
        datagen: text("datagen"),
        threads: start.num("threads"),
        status,
        started: start.time,
        last,
        totals: sum(&latest.into_values().collect::<Vec<_>>()),
        increments,
    })
}

/// Every batch below `data_dir` (each subdirectory with a `batch.log`), oldest first.
pub fn load(data_dir: &Path, now: DateTime<FixedOffset>) -> Result<Vec<BatchStats>, String> {
    let entries =
        std::fs::read_dir(data_dir).map_err(|e| format!("{}: {e}", data_dir.display()))?;
    let mut batches = Vec::new();
    for entry in entries.flatten() {
        let log = entry.path().join(LOG_FILE);
        if let Ok(text) = std::fs::read_to_string(&log)
            && let Some(batch) = batch_from_events(&parse_log(&text), now)
        {
            batches.push(batch);
        }
    }
    batches.sort_by_key(|b| b.started);
    Ok(batches)
}

/// Work booked per time slot.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Slot {
    pub games: u64,
    pub positions: u64,
}

impl Slot {
    fn add(&mut self, inc: &Increment) {
        self.games += inc.games;
        self.positions += inc.positions;
    }
}

/// Totals of a group of batches (a source, a net, a generator version).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Group {
    pub batches: u64,
    pub games: u64,
    pub positions: u64,
}

/// Everything `render` draws.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub now: NaiveDateTime,
    pub batches: Vec<BatchStats>,
    pub totals: Counters,
    /// Sum of the batch durations, and the same times the thread count.
    pub batch_seconds: i64,
    pub thread_seconds: i64,
    pub per_hour: BTreeMap<NaiveDateTime, Slot>,
    pub per_day: BTreeMap<NaiveDate, Slot>,
    /// Key: (year, month).
    pub per_month: BTreeMap<(i32, u32), Slot>,
    /// Key: (ISO year, ISO week).
    pub per_week: BTreeMap<(i32, u32), Slot>,
    pub hour_of_day: [Slot; 24],
    /// `[weekday from Monday][hour]`.
    pub heatmap: [[Slot; 24]; 7],
    pub by_source: BTreeMap<String, Group>,
    pub by_net: BTreeMap<String, Group>,
    pub by_version: BTreeMap<String, Group>,
}

/// Sums the batches in the time zone `tz`, relative to `now`.
pub fn aggregate<Tz: TimeZone>(
    batches: Vec<BatchStats>,
    now: DateTime<FixedOffset>,
    tz: &Tz,
) -> Summary {
    let local = |t: &DateTime<FixedOffset>| t.with_timezone(tz).naive_local();
    let mut s = Summary {
        now: local(&now),
        totals: sum(&batches.iter().map(|b| b.totals.clone()).collect::<Vec<_>>()),
        batch_seconds: 0,
        thread_seconds: 0,
        per_hour: BTreeMap::new(),
        per_day: BTreeMap::new(),
        per_month: BTreeMap::new(),
        per_week: BTreeMap::new(),
        hour_of_day: [Slot::default(); 24],
        heatmap: [[Slot::default(); 24]; 7],
        by_source: BTreeMap::new(),
        by_net: BTreeMap::new(),
        by_version: BTreeMap::new(),
        batches: Vec::new(),
    };
    for b in &batches {
        let seconds = b.last.signed_duration_since(b.started).num_seconds().max(0);
        s.batch_seconds += seconds;
        s.thread_seconds += seconds * b.threads as i64;
        for (map, key) in [
            (&mut s.by_source, b.source.clone()),
            (&mut s.by_net, format!("{} ({})", b.net, b.fnv)),
            (&mut s.by_version, b.datagen.clone()),
        ] {
            let g = map.entry(key).or_default();
            g.batches += 1;
            g.games += b.totals.games;
            g.positions += b.totals.positions;
        }
        for inc in &b.increments {
            let t = local(&inc.time);
            let hour = t.date().and_hms_opt(t.hour(), 0, 0).unwrap();
            s.per_hour.entry(hour).or_default().add(inc);
            s.per_day.entry(t.date()).or_default().add(inc);
            s.per_month
                .entry((t.year(), t.month()))
                .or_default()
                .add(inc);
            let week = t.iso_week();
            s.per_week
                .entry((week.year(), week.week()))
                .or_default()
                .add(inc);
            s.hour_of_day[t.hour() as usize].add(inc);
            s.heatmap[t.weekday().num_days_from_monday() as usize][t.hour() as usize].add(inc);
        }
    }
    s.batches = batches;
    s
}

/// `1234` -> `1.2 k`, `98765432` -> `98.8 M`.
pub fn human(n: f64) -> String {
    match n.abs() {
        x if x >= 1e9 => format!("{:.2} G", n / 1e9),
        x if x >= 1e6 => format!("{:.1} M", n / 1e6),
        x if x >= 1e4 => format!("{:.0} k", n / 1e3),
        x if x >= 1e3 => format!("{:.1} k", n / 1e3),
        _ => format!("{n:.0}"),
    }
}

fn percent(part: u64, whole: u64) -> String {
    if whole == 0 {
        "-".to_string()
    } else {
        format!("{:.1} %", 100.0 * part as f64 / whole as f64)
    }
}

fn duration(seconds: i64) -> String {
    let (h, m) = (seconds / 3600, seconds % 3600 / 60);
    if h >= 48 {
        format!("{:.1} d", seconds as f64 / 86400.0)
    } else {
        format!("{h} h {m:02} min")
    }
}

/// A horizontal bar of `width` cells for `value / max`, in eighths of a cell.
pub fn bar(value: u64, max: u64, width: usize) -> String {
    const EIGHTHS: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];
    if max == 0 || width == 0 {
        return " ".repeat(width);
    }
    let eighths = ((value as f64 / max as f64) * (width * 8) as f64).round() as usize;
    let eighths = eighths.min(width * 8).max(usize::from(value > 0));
    let mut out = "█".repeat(eighths / 8);
    if eighths % 8 > 0 {
        out.push(EIGHTHS[eighths % 8]);
    }
    let used = out.chars().count();
    out.push_str(&" ".repeat(width - used));
    out
}

/// Heatmap shade for `value / max`.
pub fn shade(value: u64, max: u64) -> char {
    if value == 0 || max == 0 {
        return '·';
    }
    match value as f64 / max as f64 {
        r if r <= 0.25 => '░',
        r if r <= 0.5 => '▒',
        r if r <= 0.75 => '▓',
        _ => '█',
    }
}

fn heading(out: &mut String, title: &str) {
    let _ = write!(out, "\n{title}\n{}\n", "─".repeat(title.chars().count()));
}

/// Rows of `label | bar | value`, the bars scaled to the largest value and to `width`.
fn bars(out: &mut String, rows: &[(String, Slot)], width: usize) {
    let label = rows
        .iter()
        .map(|(l, _)| l.chars().count())
        .max()
        .unwrap_or(0);
    let max = rows.iter().map(|(_, s)| s.positions).max().unwrap_or(0);
    // label, two gaps of 2, "{:>9} pos" (13) and "{:>7} games" with its gap (15).
    let bar_width = width.saturating_sub(label + 32).max(10);
    for (l, slot) in rows {
        let _ = writeln!(
            out,
            "{l:<label$}  {}  {:>9} pos  {:>7} games",
            bar(slot.positions, max, bar_width),
            human(slot.positions as f64),
            human(slot.games as f64)
        );
    }
}

fn groups(out: &mut String, title: &str, map: &BTreeMap<String, Group>, total: u64) {
    let width = map
        .keys()
        .map(|k| k.chars().count())
        .max()
        .unwrap_or(0)
        .max(title.len());
    let _ = writeln!(out, "{title:<width$}  batches     games  positions   share");
    for (key, g) in map {
        let _ = writeln!(
            out,
            "{key:<width$}  {:>7}  {:>8}  {:>9}  {:>6}",
            g.batches,
            human(g.games as f64),
            human(g.positions as f64),
            percent(g.positions, total)
        );
    }
}

/// The whole report; `width` is the terminal width the bars are fitted to.
pub fn render(s: &Summary, data_dir: &str, width: usize) -> String {
    let mut out = String::new();
    let t = &s.totals;
    let _ = writeln!(
        out,
        "rip-datagen stats - {data_dir} - {}",
        s.now.format("%Y-%m-%d %H:%M")
    );
    if s.batches.is_empty() {
        out.push_str(
            "\nNo batches yet: a run creates <data_dir>/<date>-<time>_<seed>/batch.log.\n",
        );
        return out;
    }

    heading(&mut out, "Overview");
    let count = |status: &str| s.batches.iter().filter(|b| b.status == status).count();
    let first = s.batches.iter().map(|b| b.started).min().unwrap();
    let last = s.batches.iter().map(|b| b.last).max().unwrap();
    let searched = (t.positions + t.filtered + t.skipped).max(1);
    let all_games = (t.games + t.dropped).max(1);
    let rows: Vec<(&str, String)> = vec![
        (
            "batches",
            format!(
                "{} (done {}, stopped {}, running {}, aborted {})",
                s.batches.len(),
                count("done"),
                count("stopped"),
                count("running"),
                count("aborted")
            ),
        ),
        (
            "sources / nets / versions",
            format!(
                "{} / {} / {}",
                s.by_source.len(),
                s.by_net.len(),
                s.by_version.len()
            ),
        ),
        (
            "positions",
            format!("{} ({})", human(t.positions as f64), t.positions),
        ),
        (
            "games",
            format!(
                "{} recorded, {} dropped ({})",
                human(t.games as f64),
                t.dropped,
                percent(t.dropped, all_games)
            ),
        ),
        (
            "positions per game",
            format!("{:.1}", t.positions as f64 / t.games.max(1) as f64),
        ),
        (
            "filtered / sampled out",
            format!(
                "{} / {}",
                percent(t.filtered, searched),
                percent(t.skipped, searched)
            ),
        ),
        (
            "results W / D / L",
            format!(
                "{} / {} / {}",
                percent(t.white_wins, t.games),
                percent(t.draws, t.games),
                percent(t.black_wins, t.games)
            ),
        ),
        (
            "plies per game",
            format!(
                "{:.0} (book plies {:.1})",
                t.plies as f64 / all_games as f64,
                t.book_plies as f64 / all_games as f64
            ),
        ),
        (
            "first start / last line",
            format!(
                "{}  ->  {}",
                first.format("%Y-%m-%d %H:%M"),
                last.format("%Y-%m-%d %H:%M")
            ),
        ),
        (
            "batch time / thread time",
            format!(
                "{} / {}",
                duration(s.batch_seconds),
                duration(s.thread_seconds)
            ),
        ),
        (
            "throughput",
            format!(
                "{} positions/h per batch, {:.1} positions/s per thread",
                human(t.positions as f64 * 3600.0 / s.batch_seconds.max(1) as f64),
                t.positions as f64 / s.thread_seconds.max(1) as f64
            ),
        ),
    ];
    for (key, value) in rows {
        let _ = writeln!(out, "{key:<26} {value}");
    }

    heading(&mut out, "Output per time unit");
    let window = |hours: i64| {
        let from = s.now - Duration::hours(hours);
        s.per_hour
            .iter()
            .filter(|(h, _)| **h > from - Duration::hours(1))
            .fold(Slot::default(), |mut a, (_, x)| {
                a.games += x.games;
                a.positions += x.positions;
                a
            })
    };
    let avg = |slots: Vec<Slot>| {
        let n = slots.len().max(1) as f64;
        let p: u64 = slots.iter().map(|x| x.positions).sum();
        let g: u64 = slots.iter().map(|x| x.games).sum();
        (p as f64 / n, g as f64 / n)
    };
    let _ = writeln!(out, "{:<28} {:>10} {:>10}", "", "positions", "games");
    for (label, hours) in [
        ("last hour", 1),
        ("last 24 hours", 24),
        ("last 7 days", 168),
        ("last 30 days", 720),
    ] {
        let w = window(hours);
        let _ = writeln!(
            out,
            "{label:<28} {:>10} {:>10}",
            human(w.positions as f64),
            human(w.games as f64)
        );
    }
    for (label, (p, g), n) in [
        (
            "per active hour",
            avg(s.per_hour.values().copied().collect()),
            s.per_hour.len(),
        ),
        (
            "per active day",
            avg(s.per_day.values().copied().collect()),
            s.per_day.len(),
        ),
        (
            "per active week",
            avg(s.per_week.values().copied().collect()),
            s.per_week.len(),
        ),
        (
            "per active month",
            avg(s.per_month.values().copied().collect()),
            s.per_month.len(),
        ),
    ] {
        let _ = writeln!(
            out,
            "{:<28} {:>10} {:>10}",
            format!("{label} ({n})"),
            human(p),
            human(g)
        );
    }
    if let Some((hour, slot)) = s.per_hour.iter().max_by_key(|(_, x)| x.positions) {
        let _ = writeln!(
            out,
            "{:<28} {:>10}   {}",
            "best hour",
            human(slot.positions as f64),
            hour.format("%Y-%m-%d %H:00")
        );
    }
    if let Some((day, slot)) = s.per_day.iter().max_by_key(|(_, x)| x.positions) {
        let _ = writeln!(
            out,
            "{:<28} {:>10}   {}",
            "best day",
            human(slot.positions as f64),
            day.format("%Y-%m-%d %a")
        );
    }

    heading(&mut out, "By source");
    groups(&mut out, "source", &s.by_source, t.positions);
    heading(&mut out, "By net");
    groups(&mut out, "net", &s.by_net, t.positions);
    heading(&mut out, "By generator version");
    groups(&mut out, "datagen", &s.by_version, t.positions);

    if let (Some(first), Some(last)) = (s.per_day.keys().next(), s.per_day.keys().next_back()) {
        /// Days shown at most; older days are summed up by the weeks and months below.
        const MAX_DAYS: i64 = 90;
        let from = (*last - Duration::days(MAX_DAYS - 1)).max(*first);
        heading(
            &mut out,
            &format!("Positions per day (since {})", from.format("%Y-%m-%d")),
        );
        let rows: Vec<(String, Slot)> = from
            .iter_days()
            .take_while(|d| d <= last)
            .map(|d| {
                (
                    d.format("%Y-%m-%d %a").to_string(),
                    s.per_day.get(&d).copied().unwrap_or_default(),
                )
            })
            .collect();
        bars(&mut out, &rows, width);
    }

    heading(&mut out, "Positions per week");
    let rows: Vec<(String, Slot)> = s
        .per_week
        .iter()
        .map(|((y, w), slot)| (format!("{y}-W{w:02}"), *slot))
        .collect();
    bars(&mut out, &rows, width);

    heading(&mut out, "Positions per month");
    let rows: Vec<(String, Slot)> = s
        .per_month
        .iter()
        .map(|((y, m), slot)| (format!("{y}-{m:02}"), *slot))
        .collect();
    bars(&mut out, &rows, width);

    heading(&mut out, "Positions by hour of day (all days)");
    let rows: Vec<(String, Slot)> = s
        .hour_of_day
        .iter()
        .enumerate()
        .map(|(h, slot)| (format!("{h:02}:00"), *slot))
        .collect();
    bars(&mut out, &rows, width);

    heading(&mut out, "Heatmap: weekday x hour of day (positions)");
    let max = s
        .heatmap
        .iter()
        .flatten()
        .map(|x| x.positions)
        .max()
        .unwrap_or(0);
    out.push_str("     ");
    for h in 0..24 {
        let _ = write!(
            out,
            "{}",
            if h % 3 == 0 {
                format!("{h:<3}")
            } else {
                String::new()
            }
        );
    }
    out.push('\n');
    for (day, name) in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
        .iter()
        .enumerate()
    {
        let cells: String = s.heatmap[day]
            .iter()
            .map(|x| shade(x.positions, max))
            .collect();
        let total: u64 = s.heatmap[day].iter().map(|x| x.positions).sum();
        let _ = writeln!(out, "{name}  {cells}  {:>9}", human(total as f64));
    }
    let _ = writeln!(
        out,
        "     · none  ░ <= 25 %  ▒ <= 50 %  ▓ <= 75 %  █ > 75 % of the busiest hour ({})",
        human(max as f64)
    );

    heading(&mut out, "Batches");
    let name_w = s
        .batches
        .iter()
        .map(|b| b.name.len())
        .max()
        .unwrap_or(0)
        .max(5);
    let source_w = s
        .batches
        .iter()
        .map(|b| b.source.len())
        .max()
        .unwrap_or(0)
        .max(6);
    let _ = writeln!(
        out,
        "{:<name_w$} {:<source_w$} {:<8} {:<17} {:>10} {:>3} {:>8} {:>9} {:>7}",
        "batch", "source", "status", "started", "duration", "thr", "games", "positions", "pos/s"
    );
    for b in &s.batches {
        let seconds = b.last.signed_duration_since(b.started).num_seconds().max(0);
        let _ = writeln!(
            out,
            "{:<name_w$} {:<source_w$} {:<8} {:<17} {:>10} {:>3} {:>8} {:>9} {:>7.0}",
            b.name,
            b.source,
            b.status,
            b.started.format("%Y-%m-%d %H:%M"),
            duration(seconds),
            b.threads,
            human(b.totals.games as f64),
            human(b.totals.positions as f64),
            b.totals.positions as f64 / seconds.max(1) as f64
        );
    }
    out
}

/// Terminal width from `COLUMNS`, else 100.
pub fn terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|c| c.parse().ok())
        .filter(|&w: &usize| w >= 60)
        .unwrap_or(100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datagen::log::{counter_fields, format_event, parse_log};

    fn at(text: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(text).unwrap()
    }

    fn counters(games: u64, positions: u64) -> Counters {
        Counters {
            games,
            positions,
            plies: games * 100,
            draws: games,
            ..Counters::default()
        }
    }

    /// A log of one batch: two threads, progress at the given times with cumulative counts.
    fn log(
        name: &str,
        source: &str,
        start: &str,
        lines: &[(&str, u64, u64, u64)],
        end: Option<&str>,
    ) -> String {
        let mut text = String::new();
        let start_fields = [
            ("batch", name),
            ("datagen", "1.0.0"),
            ("net", "netA"),
            ("fnv", "0000abcd"),
            ("source", source),
            ("threads", "2"),
            ("log_interval", "60"),
        ];
        text += &format_event(&at(start), "start", &start_fields);
        text.push('\n');
        for (time, thread, games, positions) in lines {
            let mut fields = vec![("thread", thread.to_string())];
            fields.extend(counter_fields(&counters(*games, *positions)));
            text += &format_event(&at(time), "progress", &fields);
            text.push('\n');
        }
        if let Some(end) = end {
            text += &format_event(&at(end), "end", &[("status", "done")]);
            text.push('\n');
        }
        text
    }

    fn sample() -> Vec<BatchStats> {
        let now = at("2026-11-24T12:00:00Z");
        let a = log(
            "a",
            "host-a",
            "2026-11-23T10:00:00Z",
            &[
                ("2026-11-23T10:01:00Z", 1, 2, 100),
                ("2026-11-23T10:01:00Z", 2, 1, 50),
                ("2026-11-23T11:01:00Z", 1, 6, 400),
                ("2026-11-23T11:01:00Z", 2, 3, 250),
            ],
            Some("2026-11-23T11:02:00Z"),
        );
        let b = log(
            "b",
            "host-b",
            "2026-11-24T11:50:00Z",
            &[("2026-11-24T11:58:00Z", 1, 1, 70)],
            None,
        );
        let c = log("c", "host-b", "2026-11-20T08:00:00Z", &[], None);
        [a, b, c]
            .iter()
            .map(|t| batch_from_events(&parse_log(t), now).unwrap())
            .collect()
    }

    #[test]
    fn increments_are_the_differences_of_the_cumulative_counters() {
        let batches = sample();
        let a = &batches[0];
        assert_eq!(a.status, "done");
        assert_eq!((a.totals.games, a.totals.positions), (9, 650));
        let positions: Vec<u64> = a.increments.iter().map(|i| i.positions).collect();
        assert_eq!(positions, [100, 50, 300, 200]);
        assert_eq!(batches[1].status, "running", "logged two minutes ago");
        assert_eq!(batches[2].status, "aborted", "no end and silent for days");
    }

    #[test]
    fn work_is_booked_per_hour_day_week_month_and_weekday() {
        let utc = FixedOffset::east_opt(0).unwrap();
        let s = aggregate(sample(), at("2026-11-24T12:00:00Z"), &utc);
        assert_eq!(s.totals.positions, 720);
        let day = |d| s.per_day[&NaiveDate::from_ymd_opt(2026, 11, d).unwrap()].positions;
        assert_eq!((day(23), day(24)), (650, 70));
        assert_eq!(s.hour_of_day[10].positions, 150);
        assert_eq!(s.hour_of_day[11].positions, 570);
        // 2026-11-23 is a Monday.
        assert_eq!(s.heatmap[0][11].positions, 500);
        assert_eq!(s.heatmap[1][11].positions, 70);
        assert_eq!(s.per_month[&(2026, 11)].positions, 720);
        assert_eq!(s.per_week.len(), 1);
        assert_eq!(
            s.by_source["host-b"],
            Group {
                batches: 2,
                games: 1,
                positions: 70
            }
        );
        assert_eq!(s.by_net.len(), 1);
    }

    #[test]
    fn the_view_follows_the_time_zone_of_the_viewer() {
        let plus_two = FixedOffset::east_opt(2 * 3600).unwrap();
        let s = aggregate(sample(), at("2026-11-24T12:00:00Z"), &plus_two);
        assert_eq!(s.hour_of_day[12].positions, 150);
        assert_eq!(s.hour_of_day[13].positions, 570);
    }

    #[test]
    fn the_report_has_every_section_and_the_numbers() {
        let utc = FixedOffset::east_opt(0).unwrap();
        let text = render(
            &aggregate(sample(), at("2026-11-24T12:00:00Z"), &utc),
            "data",
            100,
        );
        for needle in [
            "Overview",
            "batches                    3 (done 1, stopped 0, running 1, aborted 1)",
            "positions                  720 (720)",
            "Output per time unit",
            "last 24 hours",
            "best day",
            "2026-11-23 Mon",
            "By source",
            "host-a",
            "By net",
            "netA (0000abcd)",
            "Positions per day",
            "Positions per week",
            "2026-W48",
            "Positions per month",
            "2026-11",
            "Positions by hour of day",
            "Heatmap",
            "Mon  ",
            "Batches",
        ] {
            assert!(text.contains(needle), "{needle:?} missing in\n{text}");
        }
        assert!(
            text.lines().all(|l| l.chars().count() <= 100),
            "a line is wider than the terminal\n{text}"
        );
        assert!(
            render(
                &aggregate(vec![], at("2026-11-24T12:00:00Z"), &utc),
                "data",
                100
            )
            .contains("No batches yet")
        );
    }

    #[test]
    fn bars_and_shades_scale_to_the_maximum() {
        assert_eq!(bar(10, 10, 4), "████");
        assert_eq!(bar(5, 10, 4), "██  ");
        assert_eq!(bar(1, 1000, 4), "▏   ", "any work shows");
        assert_eq!(bar(0, 10, 3), "   ");
        assert_eq!(bar(3, 0, 2), "  ");
        assert_eq!(
            [
                shade(0, 8),
                shade(2, 8),
                shade(4, 8),
                shade(6, 8),
                shade(8, 8)
            ],
            ['·', '░', '▒', '▓', '█']
        );
        assert_eq!(
            (
                human(950.0),
                human(1234.0),
                human(56_789.0),
                human(98_765_432.0)
            ),
            ("950".into(), "1.2 k".into(), "57 k".into(), "98.8 M".into())
        );
    }

    #[test]
    fn load_reads_every_batch_directory() {
        let dir = std::env::temp_dir().join(format!("rip-stats-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (name, text) in [
            ("x", log("x", "s", "2026-11-23T10:00:00Z", &[], None)),
            ("y", "not a log".to_string()),
        ] {
            std::fs::create_dir_all(dir.join(name)).unwrap();
            std::fs::write(dir.join(name).join(LOG_FILE), text).unwrap();
        }
        std::fs::write(dir.join("stray.txt"), "x").unwrap();
        let batches = load(&dir, at("2026-11-24T12:00:00Z")).unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].name, "x");
        assert!(load(&dir.join("missing"), at("2026-11-24T12:00:00Z")).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
