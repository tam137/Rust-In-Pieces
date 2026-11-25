//! One run of `rip-datagen`: threads, the batch directory and its records.
//!
//! A run creates `<data_dir>/<YYYYMMDD-HHMMSS>_<base seed>/` and in it, per thread `i`,
//! `part_<i>.txt` (whole games only, flushed after each), plus one `batch.log` and one
//! `MANIFEST.json` for the batch. Each thread owns a `Generator` with the seed
//! `seeds::thread_seed(base, i)`, so equal settings, base seed and thread count give equal files.
//! The main thread writes the log and the manifest; the generator threads only play and append.
//!
//! The stop flag (Ctrl+C) is read between games: every thread finishes the game it is playing,
//! writes it, and the run ends with status `stopped`.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, FixedOffset, Local};

use super::config::{RunConfig, SeedChoice};
use super::generator::{Counters, GameSettings, Generator};
use super::log::{LOG_FILE, counter_fields, format_event};
use super::manifest::{Json, MANIFEST_FILE, counters_json, settings_json, write_atomic};
use super::seeds::{random_base_seed, thread_seed};

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Running,
    Done,
    Stopped,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Running => "running",
            Status::Done => "done",
            Status::Stopped => "stopped",
        }
    }
}

/// Result of a finished run.
#[derive(Debug, Clone)]
pub struct RunSummary {
    pub batch_dir: PathBuf,
    pub base_seed: u64,
    pub status: Status,
    pub totals: Counters,
}

/// What a generator thread shares with the main thread.
struct Shared {
    counters: Mutex<Counters>,
    finished: AtomicBool,
    error: Mutex<Option<String>>,
}

/// The fixed facts of a run, as the log and the manifest record them.
struct Batch {
    name: String,
    dir: PathBuf,
    base_seed: u64,
    seed_random: bool,
    started: DateTime<FixedOffset>,
    targets: Vec<Option<u64>>,
}

/// Recorded games of thread `thread` (1-based): the total spread evenly, the remainder going to
/// the first threads; `None` (no total) runs until stopped.
pub fn thread_target(games: Option<u64>, threads: usize, thread: usize) -> Option<u64> {
    games.map(|total| {
        let threads = threads as u64;
        total / threads + u64::from((thread as u64) <= total % threads)
    })
}

fn now() -> DateTime<FixedOffset> {
    Local::now().fixed_offset()
}

/// Runs the configuration until every thread reached its target or `stop` is set.
/// `progress` receives one line per log interval for the terminal.
pub fn run(
    cfg: &RunConfig,
    stop: Arc<AtomicBool>,
    progress: &mut dyn FnMut(&str),
) -> Result<RunSummary, String> {
    crate::magic::init();
    if let Some(book) = &cfg.settings.book {
        super::generator::load_book(book)?;
    }
    let (base_seed, seed_random) = match cfg.seed {
        SeedChoice::Random => (random_base_seed(), true),
        SeedChoice::Fixed(seed) => (seed, false),
    };
    let started = now();
    let name = format!("{}_{base_seed}", started.format("%Y%m%d-%H%M%S"));
    let dir = cfg.data_dir.join(&name);
    std::fs::create_dir_all(&cfg.data_dir)
        .map_err(|e| format!("{}: {e}", cfg.data_dir.display()))?;
    std::fs::create_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let batch = Batch {
        name,
        dir,
        base_seed,
        seed_random,
        started,
        targets: (1..=cfg.threads)
            .map(|t| thread_target(cfg.games, cfg.threads, t))
            .collect(),
    };

    let mut log = LogWriter(BufWriter::new(
        File::create(batch.dir.join(LOG_FILE)).map_err(|e| format!("{LOG_FILE}: {e}"))?,
    ));
    log.event("start", &start_fields(cfg, &batch))?;
    log.event("config", &config_fields(&cfg.settings))?;
    for (i, target) in batch.targets.iter().enumerate() {
        log.event(
            "thread",
            &[
                ("thread", (i + 1).to_string()),
                ("seed", thread_seed(base_seed, i + 1).to_string()),
                (
                    "games",
                    target.map_or("unlimited".to_string(), |n| n.to_string()),
                ),
            ],
        )?;
    }

    let shared: Vec<Arc<Shared>> = (0..cfg.threads)
        .map(|_| {
            Arc::new(Shared {
                counters: Mutex::new(Counters::default()),
                finished: AtomicBool::new(false),
                error: Mutex::new(None),
            })
        })
        .collect();
    let snapshot = |shared: &[Arc<Shared>]| -> Vec<Counters> {
        shared
            .iter()
            .map(|s| s.counters.lock().unwrap().clone())
            .collect()
    };
    write_manifest(cfg, &batch, &snapshot(&shared), Status::Running, None)?;

    let mut handles = Vec::new();
    for (i, target) in batch.targets.iter().enumerate() {
        let settings = GameSettings {
            seed: thread_seed(base_seed, i + 1),
            ..cfg.settings.clone()
        };
        let path = batch.dir.join(format!("part_{}.txt", i + 1));
        let (shared, stop, target) = (shared[i].clone(), stop.clone(), *target);
        handles.push(std::thread::spawn(move || {
            if let Err(e) = generate(settings, &path, target, &stop, &shared) {
                *shared.error.lock().unwrap() = Some(e);
            }
            shared.finished.store(true, Ordering::SeqCst);
        }));
    }

    let clock = Instant::now();
    let interval = Duration::from_secs(cfg.log_interval);
    let mut next_report = interval;
    let mut stop_logged = false;
    let mut last_total = Counters::default();
    let mut last_report = Duration::ZERO;
    while !shared.iter().all(|s| s.finished.load(Ordering::SeqCst)) {
        std::thread::sleep(Duration::from_millis(100));
        if stop.load(Ordering::SeqCst) && !stop_logged {
            log.event("stop", &[("reason", "signal".to_string())])?;
            stop_logged = true;
        }
        let elapsed = clock.elapsed();
        if elapsed >= next_report {
            next_report += interval;
            let counters = snapshot(&shared);
            for (i, c) in counters.iter().enumerate() {
                if !shared[i].finished.load(Ordering::SeqCst) {
                    let mut fields = vec![("thread", (i + 1).to_string())];
                    fields.extend(counter_fields(c));
                    log.event("progress", &fields)?;
                }
            }
            write_manifest(cfg, &batch, &counters, Status::Running, None)?;
            let total = sum(&counters);
            let seconds = (elapsed - last_report).as_secs_f64().max(1e-9);
            progress(&format!(
                "{} games, {} positions, {:.0} pos/s over the last {:.0} s",
                total.games,
                total.positions,
                (total.positions - last_total.positions) as f64 / seconds,
                seconds
            ));
            (last_total, last_report) = (total, elapsed);
        }
    }
    for handle in handles {
        handle
            .join()
            .map_err(|_| "a generator thread panicked".to_string())?;
    }

    let counters = snapshot(&shared);
    let mut errors = Vec::new();
    for (i, (c, s)) in counters.iter().zip(&shared).enumerate() {
        let mut fields = vec![("thread", (i + 1).to_string())];
        fields.extend(counter_fields(c));
        log.event("thread_done", &fields)?;
        if let Some(e) = s.error.lock().unwrap().clone() {
            log.event(
                "error",
                &[("thread", (i + 1).to_string()), ("message", e.clone())],
            )?;
            errors.push(format!("thread {}: {e}", i + 1));
        }
    }
    let reached = batch
        .targets
        .iter()
        .zip(&counters)
        .all(|(target, c)| target.is_some_and(|t| c.games >= t));
    let status = if reached && errors.is_empty() {
        Status::Done
    } else {
        Status::Stopped
    };
    let totals = sum(&counters);
    let mut fields = vec![("status", status.label().to_string())];
    fields.extend(counter_fields(&totals));
    fields.push(("seconds", clock.elapsed().as_secs().to_string()));
    log.event("end", &fields)?;
    write_manifest(cfg, &batch, &counters, status, Some(now()))?;
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    Ok(RunSummary {
        batch_dir: batch.dir,
        base_seed,
        status,
        totals,
    })
}

/// One generator thread: plays until its target or the stop flag, appending whole games.
fn generate(
    settings: GameSettings,
    path: &Path,
    target: Option<u64>,
    stop: &AtomicBool,
    shared: &Shared,
) -> Result<(), String> {
    let file = File::create_new(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = BufWriter::new(file);
    if target == Some(0) {
        return Ok(());
    }
    let mut generator = Generator::new(settings);
    let mut games = 0;
    while target.is_none_or(|t| games < t) && !stop.load(Ordering::SeqCst) {
        let game = generator.play_game();
        if let Some(lines) = &game.lines {
            for line in lines {
                writeln!(out, "{line}").map_err(|e| e.to_string())?;
            }
            // Whole games only: an interrupted run never leaves half a game behind.
            out.flush().map_err(|e| e.to_string())?;
            games += 1;
        }
        shared.counters.lock().unwrap().add(&game);
    }
    Ok(())
}

/// The counters of all threads together.
pub fn sum(counters: &[Counters]) -> Counters {
    counters.iter().fold(Counters::default(), |mut a, c| {
        a.games += c.games;
        a.dropped += c.dropped;
        a.positions += c.positions;
        a.filtered += c.filtered;
        a.skipped += c.skipped;
        a.book_plies += c.book_plies;
        a.plies += c.plies;
        a.white_wins += c.white_wins;
        a.draws += c.draws;
        a.black_wins += c.black_wins;
        a
    })
}

fn start_fields(cfg: &RunConfig, batch: &Batch) -> Vec<(&'static str, String)> {
    let (net, fnv) = super::net();
    vec![
        ("batch", batch.name.clone()),
        ("datagen", super::DATAGEN_VERSION.to_string()),
        ("engine", super::engine_version().to_string()),
        ("commit", super::engine_commit().to_string()),
        ("net", net.to_string()),
        ("fnv", format!("{fnv:08x}")),
        ("arch", std::env::consts::ARCH.to_string()),
        ("os", std::env::consts::OS.to_string()),
        ("source", cfg.source.clone()),
        ("threads", cfg.threads.to_string()),
        (
            "games",
            cfg.games.map_or("unlimited".to_string(), |n| n.to_string()),
        ),
        ("seed", batch.base_seed.to_string()),
        ("seed_random", batch.seed_random.to_string()),
        ("log_interval", cfg.log_interval.to_string()),
    ]
}

/// The game settings as `key=value` pairs, under the manifest's names.
fn config_fields(g: &GameSettings) -> Vec<(String, String)> {
    let Json::Obj(pairs) = settings_json(g) else {
        unreachable!("settings are an object")
    };
    pairs
        .into_iter()
        .map(|(key, value)| {
            let text = match value {
                Json::Str(s) => s,
                Json::Null => String::new(),
                Json::Float(x) => format!("{x:?}"),
                other => other.pretty().trim_end().to_string(),
            };
            (key, text)
        })
        .collect()
}

/// `batch.log`, flushed after every line so a killed run loses nothing that was logged.
struct LogWriter(BufWriter<File>);

impl LogWriter {
    fn event<K: AsRef<str>, V: AsRef<str>>(
        &mut self,
        name: &str,
        fields: &[(K, V)],
    ) -> Result<(), String> {
        writeln!(self.0, "{}", format_event(&now(), name, fields))
            .and_then(|_| self.0.flush())
            .map_err(|e| format!("{LOG_FILE}: {e}"))
    }
}

fn write_manifest(
    cfg: &RunConfig,
    batch: &Batch,
    counters: &[Counters],
    status: Status,
    finished: Option<DateTime<FixedOffset>>,
) -> Result<(), String> {
    let (net, fnv) = super::net();
    let totals = sum(counters);
    let done = status == Status::Done;
    let stamp = |t: &DateTime<FixedOffset>| {
        Json::str(t.to_rfc3339_opts(chrono::SecondsFormat::Secs, false))
    };
    let parts = counters
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let target = batch.targets[i];
            Json::obj(vec![
                ("file", Json::str(format!("part_{}.txt", i + 1))),
                ("thread", Json::UInt(i as u64 + 1)),
                ("seed", Json::UInt(thread_seed(batch.base_seed, i + 1))),
                ("games_target", target.map_or(Json::Null, Json::UInt)),
                ("lines", Json::UInt(c.positions)),
                (
                    "counters",
                    counters_json(
                        c,
                        status != Status::Running && target.is_some_and(|t| c.games >= t),
                    ),
                ),
            ])
        })
        .collect();
    let json = Json::obj(vec![
        ("schema", Json::UInt(1)),
        ("kind", Json::str("data")),
        ("name", Json::str(batch.name.clone())),
        ("generator", Json::str("rip-datagen")),
        ("datagen_version", Json::str(super::DATAGEN_VERSION)),
        ("engine_version", Json::str(super::engine_version())),
        ("engine_commit", Json::str(super::engine_commit())),
        ("arch", Json::str(std::env::consts::ARCH)),
        ("os", Json::str(std::env::consts::OS)),
        (
            "net",
            Json::obj(vec![
                ("name", Json::str(net)),
                ("fnv", Json::str(format!("{fnv:08x}"))),
            ]),
        ),
        ("source", Json::str(cfg.source.clone())),
        ("status", Json::str(status.label())),
        ("created", stamp(&batch.started)),
        ("started", stamp(&batch.started)),
        ("updated", stamp(&now())),
        ("finished", finished.as_ref().map_or(Json::Null, stamp)),
        ("base_seed", Json::UInt(batch.base_seed)),
        ("seed_random", Json::Bool(batch.seed_random)),
        ("threads", Json::UInt(cfg.threads as u64)),
        ("games_target", cfg.games.map_or(Json::Null, Json::UInt)),
        ("positions", Json::UInt(totals.positions)),
        ("games", Json::UInt(totals.games)),
        ("dropped_games", Json::UInt(totals.dropped)),
        ("complete", Json::Bool(done)),
        ("totals", counters_json(&totals, done)),
        ("settings", settings_json(&cfg.settings)),
        ("settings_differ", Json::Arr(vec![])),
        ("parts", Json::Arr(parts)),
    ]);
    write_atomic(&batch.dir.join(MANIFEST_FILE), &json).map_err(|e| format!("{MANIFEST_FILE}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datagen::log::parse_log;

    fn test_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rip-runner-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn small(dir: &Path, threads: usize, games: u64, seed: u64) -> RunConfig {
        RunConfig {
            source: "test".to_string(),
            data_dir: dir.to_path_buf(),
            threads,
            games: Some(games),
            seed: SeedChoice::Fixed(seed),
            log_interval: 1,
            nice: 0,
            settings: GameSettings {
                nodes: 300,
                hash_entries: 1 << 14,
                ..GameSettings::default()
            },
        }
    }

    fn parts(dir: &Path, threads: usize) -> Vec<String> {
        (1..=threads)
            .map(|i| std::fs::read_to_string(dir.join(format!("part_{i}.txt"))).unwrap())
            .collect()
    }

    #[test]
    fn games_are_spread_over_the_threads() {
        let targets: Vec<_> = (1..=4).map(|t| thread_target(Some(10), 4, t)).collect();
        assert_eq!(targets, [Some(3), Some(3), Some(2), Some(2)]);
        assert_eq!(thread_target(Some(2), 4, 4), Some(0));
        assert_eq!(thread_target(None, 4, 1), None);
    }

    #[test]
    fn a_run_writes_parts_log_and_manifest_and_is_reproducible() {
        let dir = test_dir("repro");
        let quiet = |_: &str| {};
        let run_once = |seed| {
            run(
                &small(&dir, 2, 3, seed),
                Arc::new(AtomicBool::new(false)),
                &mut { quiet },
            )
            .unwrap()
        };
        let first = run_once(9);
        assert_eq!(first.status, Status::Done);
        assert_eq!(first.totals.games, 3);
        assert!(
            first
                .batch_dir
                .file_name()
                .unwrap()
                .to_string_lossy()
                .ends_with("_9")
        );

        let texts = parts(&first.batch_dir, 2);
        let lines: u64 = texts.iter().map(|t| t.lines().count() as u64).sum();
        assert_eq!(lines, first.totals.positions);

        let events = parse_log(&std::fs::read_to_string(first.batch_dir.join(LOG_FILE)).unwrap());
        let names: Vec<&str> = events.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(&names[..4], ["start", "config", "thread", "thread"]);
        assert_eq!(*names.last().unwrap(), "end");
        let start = &events[0];
        assert_eq!(start.get("source"), Some("test"));
        assert_eq!(start.get("seed"), Some("9"));
        assert_eq!(
            start.get("fnv"),
            Some(format!("{:08x}", crate::datagen::net().1).as_str())
        );
        let end = events.last().unwrap();
        assert_eq!(end.get("status"), Some("done"));
        assert_eq!(end.counters(), first.totals);
        assert_eq!(
            events.iter().filter(|e| e.name == "thread_done").count(),
            2,
            "one final line per thread"
        );

        let manifest = std::fs::read_to_string(first.batch_dir.join(MANIFEST_FILE)).unwrap();
        for needle in [
            "\"kind\": \"data\"",
            "\"generator\": \"rip-datagen\"",
            "\"status\": \"done\"",
            "\"complete\": true",
            "\"base_seed\": 9",
            "\"source\": \"test\"",
            &format!("\"positions\": {}", first.totals.positions),
            "\"file\": \"part_2.txt\"",
        ] {
            assert!(manifest.contains(needle), "{needle} missing in\n{manifest}");
        }

        // The batch name has a one-second resolution; a second run in the same second would
        // collide, so wait for the next one.
        std::thread::sleep(Duration::from_millis(1100));
        let again = run_once(9);
        assert_eq!(parts(&again.batch_dir, 2), texts, "same seed, same data");
        std::thread::sleep(Duration::from_millis(1100));
        let other = run_once(10);
        assert_ne!(parts(&other.batch_dir, 2), texts, "other seed, other data");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_stopped_run_keeps_whole_games_and_says_so() {
        let dir = test_dir("stop");
        let stop = Arc::new(AtomicBool::new(true));
        let mut cfg = small(&dir, 2, 100, 4);
        cfg.games = None;
        let summary = run(&cfg, stop, &mut |_| {}).unwrap();
        assert_eq!(summary.status, Status::Stopped);
        assert_eq!(summary.totals.games, 0);
        let manifest = std::fs::read_to_string(summary.batch_dir.join(MANIFEST_FILE)).unwrap();
        assert!(manifest.contains("\"status\": \"stopped\""));
        assert!(manifest.contains("\"complete\": false"));
        let log = std::fs::read_to_string(summary.batch_dir.join(LOG_FILE)).unwrap();
        let events = parse_log(&log);
        assert!(events.iter().any(|e| e.name == "stop"));
        assert_eq!(events.last().unwrap().get("status"), Some("stopped"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_book_fails_before_anything_is_written() {
        let dir = test_dir("book");
        let mut cfg = small(&dir, 1, 1, 1);
        cfg.settings.book = Some("no/such/book.bin".to_string());
        assert!(run(&cfg, Arc::new(AtomicBool::new(false)), &mut |_| {}).is_err());
        assert!(!dir.exists());
    }
}
