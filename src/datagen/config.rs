//! The configuration file of `rip-datagen`.
//!
//! The file is flat TOML: one `key = value` per line, `#` comments, no tables. Values are strings
//! in double quotes, integers (with optional `_` separators), floats or booleans. Every key is
//! optional and falls back to its default, so a file written by an older `init` keeps working;
//! an unknown or repeated key is an error, so a typo never silently runs with a default.
//! Relative paths (`data_dir`, `book`) are resolved against the directory of the file.

use std::path::{Path, PathBuf};

use super::generator::GameSettings;

/// Name of the configuration file `rip-datagen` looks for in the working directory.
pub const CONFIG_FILE: &str = "rip-datagen.toml";

/// Where the base seed of a run comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedChoice {
    /// A new random seed for every run (the default).
    Random,
    /// A fixed seed, to reproduce a run.
    Fixed(u64),
}

/// Everything a run needs: where it writes, how many threads, and the settings of the games.
#[derive(Debug, Clone, PartialEq)]
pub struct RunConfig {
    /// Name of this data source; written to every log and manifest.
    pub source: String,
    /// Directory below which every run creates its own batch directory.
    pub data_dir: PathBuf,
    pub threads: usize,
    /// Recorded games of the whole run, spread over the threads; `None` runs until stopped.
    pub games: Option<u64>,
    pub seed: SeedChoice,
    /// Seconds between two progress lines in the log.
    pub log_interval: u64,
    /// Settings of the games; `seed` is replaced by each thread's own seed.
    pub settings: GameSettings,
}

/// The default thread count: three quarters of the hardware threads minus one, at least one
/// (the load rule of the match hosts, which keeps a quarter of the machine free).
pub fn default_threads() -> usize {
    let hardware = std::thread::available_parallelism().map_or(1, |n| n.get());
    (hardware * 3 / 4).saturating_sub(1).max(1)
}

/// Text of a new configuration file with every key at its default.
pub fn template(threads: usize) -> String {
    let g = GameSettings::default();
    format!(
        "\
# rip-datagen configuration (flat TOML: key = value, # starts a comment).
# Every key is optional; a missing key takes the default shown here.
# Relative paths are resolved against the directory of this file.

# --- run ----------------------------------------------------------------------------------------
# Name of this data source, written to every log and manifest (letters, digits, . _ -).
source = \"unnamed\"
# Every run creates its own batch directory <date>-<time>_<seed> below this directory.
data_dir = \"nnue_data\"
# Generator threads; the default leaves a quarter of the hardware threads free.
threads = {threads}
# Recorded games of the whole run, spread over the threads; 0 runs until Ctrl+C.
games = 1_000_000
# \"random\" draws a new base seed for every run; an integer reproduces a run (same settings and
# thread count give the same data).
seed = \"random\"
# Seconds between two progress lines in batch.log (the resolution of `rip-datagen stats`).
log_interval = 60

# --- games --------------------------------------------------------------------------------------
# Soft limit per move, in visited positions: the iteration that reaches it is the last one.
nodes = {nodes}
# Opening without a book: a random number of plies in this range, each random with random_prob
# percent and searched otherwise. Opening plies are never recorded.
random_plies_min = {rmin}
random_plies_max = {rmax}
random_prob = {rprob}
# Drop the game if |score| exceeds this (centipawns) after the opening.
opening_limit = {olimit}
# Adjudicate a win once |score| >= win_cp on win_plies consecutive searched plies.
win_cp = {win_cp}
win_plies = {win_plies}
# Adjudicate a draw once |score| <= draw_cp on draw_plies consecutive plies from move draw_from_move.
draw_cp = {draw_cp}
draw_plies = {draw_plies}
draw_from_move = {draw_from}
# The game is drawn at this full move number.
max_moves = {max_moves}
# Transposition table entries per thread.
hash_entries = {hash}
# Share of the recordable positions that is written, in (0, 1]: fewer positions per game.
sample = {sample:?}

# --- book opening (optional) --------------------------------------------------------------------
# Polyglot book; empty keeps the random opening above. Use a book that is not the source of the
# measurement openings.
book = \"\"
# Book plies per game (fewer when the book has no move), then random-phase plies.
book_plies_min = {bmin}
book_plies_max = {bmax}
book_random_plies_min = {brmin}
book_random_plies_max = {brmax}
",
        nodes = g.nodes,
        rmin = g.random_plies_min,
        rmax = g.random_plies_max,
        rprob = g.random_prob,
        olimit = g.opening_limit,
        win_cp = g.win_cp,
        win_plies = g.win_plies,
        draw_cp = g.draw_cp,
        draw_plies = g.draw_plies,
        draw_from = g.draw_from_move,
        max_moves = g.max_moves,
        hash = g.hash_entries,
        sample = g.sample,
        bmin = g.book_plies_min,
        bmax = g.book_plies_max,
        brmin = g.book_random_min,
        brmax = g.book_random_max,
    )
}

/// Writes a new configuration file with the defaults; refuses to overwrite an existing one.
pub fn init(path: &Path) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::AlreadyExists => {
                format!("{} exists already; it is not overwritten", path.display())
            }
            _ => format!("{}: {e}", path.display()),
        })?;
    std::io::Write::write_all(&mut file, template(default_threads()).as_bytes())
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// Reads and checks a configuration file.
pub fn load(path: &Path) -> Result<RunConfig, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let base = path.parent().unwrap_or(Path::new("."));
    parse(&text, base).map_err(|e| format!("{}: {e}", path.display()))
}

/// One value of the file.
#[derive(Debug, Clone, PartialEq)]
enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

impl Value {
    fn kind(&self) -> &'static str {
        match self {
            Value::Str(_) => "a string",
            Value::Int(_) => "an integer",
            Value::Float(_) => "a float",
            Value::Bool(_) => "a boolean",
        }
    }
}

/// Parses the file text; relative paths are resolved against `base`.
pub fn parse(text: &str, base: &Path) -> Result<RunConfig, String> {
    let mut entries: Vec<(String, Value, usize)> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = strip_comment(raw).trim().to_string();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("line {line_no}: expected key = value"))?;
        let key = key.trim();
        if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("line {line_no}: invalid key {key:?}"));
        }
        if entries.iter().any(|(k, _, _)| k == key) {
            return Err(format!("line {line_no}: {key} is set twice"));
        }
        let value = parse_value(value.trim()).map_err(|e| format!("line {line_no}: {e}"))?;
        entries.push((key.to_string(), value, line_no));
    }

    let mut cfg = RunConfig {
        source: "unnamed".to_string(),
        data_dir: base.join("nnue_data"),
        threads: default_threads(),
        games: Some(1_000_000),
        seed: SeedChoice::Random,
        log_interval: 60,
        settings: GameSettings::default(),
    };
    for (key, value, line_no) in entries {
        apply(&mut cfg, &key, value, base).map_err(|e| format!("line {line_no}: {key}: {e}"))?;
    }
    validate(&cfg)?;
    Ok(cfg)
}

/// The line without its comment; a `#` inside a string is kept.
fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    for (i, c) in line.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            '#' if !in_string => return &line[..i],
            _ => {}
        }
    }
    line
}

fn parse_value(text: &str) -> Result<Value, String> {
    if let Some(body) = text.strip_prefix('"') {
        let body = body
            .strip_suffix('"')
            .ok_or_else(|| format!("unterminated string {text}"))?;
        let mut out = String::new();
        let mut chars = body.chars();
        while let Some(c) = chars.next() {
            match c {
                '\\' => match chars.next() {
                    Some('\\') => out.push('\\'),
                    Some('"') => out.push('"'),
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    other => return Err(format!("unsupported escape \\{}", other.unwrap_or(' '))),
                },
                '"' => return Err(format!("unexpected quote in {text}")),
                _ => out.push(c),
            }
        }
        return Ok(Value::Str(out));
    }
    match text {
        "true" => return Ok(Value::Bool(true)),
        "false" => return Ok(Value::Bool(false)),
        _ => {}
    }
    let digits = text.replace('_', "");
    if let Ok(n) = digits.parse::<i64>() {
        return Ok(Value::Int(n));
    }
    if digits.contains(['.', 'e', 'E'])
        && let Ok(x) = digits.parse::<f64>()
    {
        return Ok(Value::Float(x));
    }
    Err(format!("invalid value {text} (strings need double quotes)"))
}

fn int<T: TryFrom<i64>>(value: Value) -> Result<T, String> {
    match value {
        Value::Int(n) => T::try_from(n).map_err(|_| format!("{n} is out of range")),
        other => Err(format!("expected an integer, got {}", other.kind())),
    }
}

fn string(value: Value) -> Result<String, String> {
    match value {
        Value::Str(s) => Ok(s),
        other => Err(format!("expected a string, got {}", other.kind())),
    }
}

fn float(value: Value) -> Result<f64, String> {
    match value {
        Value::Float(x) => Ok(x),
        Value::Int(n) => Ok(n as f64),
        other => Err(format!("expected a number, got {}", other.kind())),
    }
}

fn path(base: &Path, text: &str) -> PathBuf {
    let p = Path::new(text);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    }
}

fn apply(cfg: &mut RunConfig, key: &str, value: Value, base: &Path) -> Result<(), String> {
    let g = &mut cfg.settings;
    match key {
        "source" => cfg.source = string(value)?,
        "data_dir" => cfg.data_dir = path(base, &string(value)?),
        "threads" => cfg.threads = int(value)?,
        "games" => cfg.games = Some(int::<u64>(value)?).filter(|&n| n > 0),
        "seed" => {
            cfg.seed = match value {
                Value::Str(s) if s == "random" => SeedChoice::Random,
                Value::Int(n) if n >= 0 => SeedChoice::Fixed(n as u64),
                other => {
                    return Err(format!(
                        "expected \"random\" or a non-negative integer, got {other:?}"
                    ));
                }
            }
        }
        "log_interval" => cfg.log_interval = int(value)?,
        "nodes" => g.nodes = int(value)?,
        "random_plies_min" => g.random_plies_min = int(value)?,
        "random_plies_max" => g.random_plies_max = int(value)?,
        "random_prob" => g.random_prob = int(value)?,
        "opening_limit" => g.opening_limit = int(value)?,
        "win_cp" => g.win_cp = int(value)?,
        "win_plies" => g.win_plies = int(value)?,
        "draw_cp" => g.draw_cp = int(value)?,
        "draw_plies" => g.draw_plies = int(value)?,
        "draw_from_move" => g.draw_from_move = int(value)?,
        "max_moves" => g.max_moves = int(value)?,
        "hash_entries" => g.hash_entries = int(value)?,
        "sample" => g.sample = float(value)?,
        "book" => {
            let text = string(value)?;
            g.book = (!text.is_empty()).then(|| path(base, &text).to_string_lossy().into_owned());
        }
        "book_plies_min" => g.book_plies_min = int(value)?,
        "book_plies_max" => g.book_plies_max = int(value)?,
        "book_random_plies_min" => g.book_random_min = int(value)?,
        "book_random_plies_max" => g.book_random_max = int(value)?,
        _ => return Err("unknown key".to_string()),
    }
    Ok(())
}

/// Longest accepted source name.
const MAX_SOURCE_LEN: usize = 40;

fn validate(cfg: &RunConfig) -> Result<(), String> {
    let g = &cfg.settings;
    if cfg.source.is_empty()
        || cfg.source.len() > MAX_SOURCE_LEN
        || !cfg
            .source
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    {
        return Err(format!(
            "source must be 1-{MAX_SOURCE_LEN} letters, digits, '.', '_' or '-', got {:?}",
            cfg.source
        ));
    }
    if cfg.threads == 0 || cfg.log_interval == 0 {
        return Err("threads and log_interval must be at least 1".to_string());
    }
    if g.nodes == 0 || g.hash_entries == 0 || g.win_plies == 0 || g.draw_plies == 0 {
        return Err("nodes, hash_entries, win_plies and draw_plies must be at least 1".to_string());
    }
    if !(g.sample > 0.0 && g.sample <= 1.0) {
        return Err(format!("sample must lie in (0, 1], got {}", g.sample));
    }
    if g.random_plies_min > g.random_plies_max || g.random_prob > 100 {
        return Err(
            "random_plies_min must not exceed random_plies_max, random_prob at most 100"
                .to_string(),
        );
    }
    if g.book_plies_min > g.book_plies_max || g.book_random_min > g.book_random_max {
        return Err("book_plies and book_random_plies need min <= max".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_here(text: &str) -> Result<RunConfig, String> {
        parse(text, Path::new("/cfg"))
    }

    #[test]
    fn an_empty_file_gives_the_defaults() {
        let cfg = parse_here("").unwrap();
        assert_eq!(cfg.source, "unnamed");
        assert_eq!(cfg.data_dir, Path::new("/cfg/nnue_data"));
        assert_eq!(cfg.threads, default_threads());
        assert_eq!(cfg.games, Some(1_000_000));
        assert_eq!(cfg.seed, SeedChoice::Random);
        assert_eq!(cfg.log_interval, 60);
        assert_eq!(cfg.settings, GameSettings::default());
    }

    #[test]
    fn the_template_parses_back_to_the_defaults() {
        let cfg = parse_here(&template(7)).unwrap();
        assert_eq!(cfg.threads, 7);
        let defaults = parse_here("threads = 7").unwrap();
        assert_eq!(cfg, defaults);
    }

    #[test]
    fn every_key_is_read() {
        let cfg = parse_here(
            r#"
            source = "host-a"   # a comment
            data_dir = "/abs/data"
            threads = 3
            games = 0
            seed = 42
            log_interval = 5
            nodes = 800
            random_plies_min = 6
            random_plies_max = 10
            random_prob = 100
            opening_limit = 300
            win_cp = 1500
            win_plies = 3
            draw_cp = 5
            draw_plies = 6
            draw_from_move = 30
            max_moves = 300
            hash_entries = 4_096
            sample = 0.33
            book = "books/b.bin"
            book_plies_min = 6
            book_plies_max = 10
            book_random_plies_min = 1
            book_random_plies_max = 3
            "#,
        )
        .unwrap();
        assert_eq!(
            (cfg.source.as_str(), cfg.data_dir.as_path()),
            ("host-a", Path::new("/abs/data"))
        );
        assert_eq!(
            (cfg.threads, cfg.games, cfg.seed),
            (3, None, SeedChoice::Fixed(42))
        );
        assert_eq!(cfg.log_interval, 5);
        let g = &cfg.settings;
        assert_eq!(
            (
                g.nodes,
                g.random_plies_min,
                g.random_plies_max,
                g.random_prob
            ),
            (800, 6, 10, 100)
        );
        assert_eq!((g.opening_limit, g.win_cp, g.win_plies), (300, 1500, 3));
        assert_eq!((g.draw_cp, g.draw_plies, g.draw_from_move), (5, 6, 30));
        assert_eq!((g.max_moves, g.hash_entries, g.sample), (300, 4096, 0.33));
        assert_eq!(g.book.as_deref(), Some("/cfg/books/b.bin"));
        assert_eq!(
            (
                g.book_plies_min,
                g.book_plies_max,
                g.book_random_min,
                g.book_random_max
            ),
            (6, 10, 1, 3)
        );
    }

    #[test]
    fn an_empty_book_keeps_the_random_opening_and_a_hash_in_a_string_is_no_comment() {
        assert_eq!(parse_here("book = \"\"").unwrap().settings.book, None);
        let cfg = parse_here(r#"data_dir = "d#1" # comment"#).unwrap();
        assert_eq!(cfg.data_dir, Path::new("/cfg/d#1"));
        assert_eq!(parse_here("sample = 1").unwrap().settings.sample, 1.0);
    }

    #[test]
    fn nonsense_is_rejected_with_the_line_number() {
        for (text, needle) in [
            ("nodes = 0", "nodes"),
            ("threads = 0", "threads"),
            (
                "random_plies_min = 12\nrandom_plies_max = 8",
                "random_plies",
            ),
            ("random_prob = 101", "random_prob"),
            ("sample = 0", "sample"),
            ("sample = 1.5", "sample"),
            ("book_plies_min = 9\nbook_plies_max = 3", "book_plies"),
            ("bogus = 1", "line 1: bogus: unknown key"),
            ("nodes = 5\nnodes = 6", "line 2: nodes is set twice"),
            ("nodes = \"5\"", "expected an integer"),
            ("source = unquoted", "strings need double quotes"),
            ("source = \"two words\"", "source must be"),
            ("seed = -1", "seed"),
            ("seed = \"sometimes\"", "seed"),
            ("nodes = -5", "out of range"),
            ("just a line", "line 1: expected key = value"),
            ("data_dir = \"open", "unterminated"),
        ] {
            let err = parse_here(text).expect_err(text);
            assert!(err.contains(needle), "{text:?}: {err}");
        }
    }

    #[test]
    fn init_writes_the_template_once_and_never_overwrites() {
        let dir = std::env::temp_dir().join(format!("rip-datagen-init-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(CONFIG_FILE);
        let _ = std::fs::remove_file(&file);
        init(&file).unwrap();
        let cfg = load(&file).unwrap();
        assert_eq!(cfg.data_dir, dir.join("nnue_data"));
        std::fs::write(&file, "nodes = 77\n").unwrap();
        assert!(init(&file).unwrap_err().contains("not overwritten"));
        assert_eq!(load(&file).unwrap().settings.nodes, 77);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_missing_file_is_an_error() {
        assert!(load(Path::new("no/such/rip-datagen.toml")).is_err());
    }
}
