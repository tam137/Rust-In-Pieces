//! `rip-datagen`: self-play training data for the NNUE trainer, with the engine's embedded net.
//!
//! See `src/datagen/mod.rs` for the parts and `nnue_training/README.md` for the workflow.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use suprah::datagen::config::{self, CONFIG_FILE};
use suprah::datagen::{pager, runner, stats};

const USAGE: &str = "\
rip-datagen - self-play training data for the NNUE trainer

Usage:
  rip-datagen [run] [--config FILE]   generate one batch as the configuration says
  rip-datagen init [--config FILE]    write a configuration with the defaults (never overwrites)
  rip-datagen stats [--config FILE | --dir DIR] [--since YYYY-MM-DD] [--no-pager]
                                      statistics over every batch of the data directory
  rip-datagen version                 generator version, engine version, commit and net
  rip-datagen help                    this text

The configuration is rip-datagen.toml in the working directory unless --config names another.
A run creates <data_dir>/<date>-<time>_<seed>/ with part_<i>.txt per thread, batch.log and
MANIFEST.json. Ctrl+C stops after the games being played; a second Ctrl+C exits at once.
";

/// Command-line options after the command.
#[derive(Debug, Default, PartialEq)]
struct Options {
    config: Option<PathBuf>,
    dir: Option<PathBuf>,
    since: Option<chrono::NaiveDate>,
    no_pager: bool,
}

fn parse_options(command: &str, args: &[String]) -> Result<Options, String> {
    let mut o = Options::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || {
            iter.next()
                .cloned()
                .ok_or_else(|| format!("{arg} needs a value"))
        };
        match (command, arg.as_str()) {
            (_, "--config") => o.config = Some(PathBuf::from(value()?)),
            ("stats", "--dir") => o.dir = Some(PathBuf::from(value()?)),
            ("stats", "--since") => {
                let text = value()?;
                o.since = Some(
                    chrono::NaiveDate::parse_from_str(&text, "%Y-%m-%d")
                        .map_err(|_| format!("--since expects YYYY-MM-DD, got {text}"))?,
                );
            }
            ("stats", "--no-pager") => o.no_pager = true,
            _ => return Err(format!("unknown option {arg} for {command}")),
        }
    }
    Ok(o)
}

fn config_path(o: &Options) -> PathBuf {
    o.config
        .clone()
        .unwrap_or_else(|| PathBuf::from(CONFIG_FILE))
}

fn run(o: &Options) -> Result<(), String> {
    let path = config_path(o);
    if !path.exists() {
        return Err(format!(
            "no configuration {}; create one with `rip-datagen init`",
            path.display()
        ));
    }
    let cfg = config::load(&path)?;
    let stop = Arc::new(AtomicBool::new(false));
    let handler_stop = stop.clone();
    ctrlc::set_handler(move || {
        if handler_stop.swap(true, Ordering::SeqCst) {
            eprintln!("\nrip-datagen: second Ctrl+C, exiting now");
            std::process::exit(130);
        }
        eprintln!("\nrip-datagen: stopping after the games being played (Ctrl+C again exits now)");
    })
    .map_err(|e| format!("cannot install the Ctrl+C handler: {e}"))?;

    eprint!("{}", suprah::datagen::version_text());
    eprintln!(
        "source {}, {} threads, {}, data in {}",
        cfg.source,
        cfg.threads,
        cfg.games
            .map_or("until Ctrl+C".to_string(), |n| format!("{n} games")),
        cfg.data_dir.display()
    );
    let summary = runner::run(&cfg, stop, &mut |line| {
        eprintln!("{} {line}", chrono::Local::now().format("%H:%M:%S"));
    })?;
    eprintln!(
        "{}: {} games, {} positions in {}",
        summary.status.label(),
        summary.totals.games,
        summary.totals.positions,
        summary.batch_dir.display()
    );
    Ok(())
}

fn show_stats(o: &Options) -> Result<(), String> {
    let dir = match &o.dir {
        Some(dir) => dir.clone(),
        None => {
            let path = config_path(o);
            if !path.exists() {
                return Err(format!(
                    "no configuration {} to take the data directory from; pass --dir",
                    path.display()
                ));
            }
            config::load(&path)?.data_dir
        }
    };
    let now = chrono::Local::now().fixed_offset();
    let mut batches = stats::load(&dir, now)?;
    if let Some(since) = o.since {
        batches.retain(|b| b.started.with_timezone(&chrono::Local).date_naive() >= since);
    }
    let summary = stats::aggregate(batches, now, &chrono::Local);
    let text = stats::render(
        &summary,
        &dir.display().to_string(),
        stats::terminal_width(),
    );
    pager::show(&text, !o.no_pager);
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (command, rest) = match args.first().map(String::as_str) {
        None => ("run", &args[..]),
        Some(first) if first.starts_with("--") => ("run", &args[..]),
        Some(first) => (first, &args[1..]),
    };
    let result = match command {
        "help" | "-h" | "--help" => {
            print!("{USAGE}");
            Ok(())
        }
        "version" | "--version" => {
            print!("{}", suprah::datagen::version_text());
            Ok(())
        }
        "run" | "init" | "stats" => parse_options(command, rest).and_then(|o| match command {
            "run" => run(&o),
            "init" => {
                let path = config_path(&o);
                config::init(&path).map(|_| println!("wrote {}", path.display()))
            }
            _ => show_stats(&o),
        }),
        other => Err(format!("unknown command {other}\n\n{USAGE}")),
    };
    if let Err(message) = result {
        eprintln!("rip-datagen: {message}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn options_are_parsed_per_command() {
        let o = parse_options(
            "stats",
            &args(&["--dir", "d", "--since", "2026-11-01", "--no-pager"]),
        )
        .unwrap();
        assert_eq!(o.dir, Some(PathBuf::from("d")));
        assert_eq!(o.since, chrono::NaiveDate::from_ymd_opt(2026, 11, 1));
        assert!(o.no_pager);
        assert_eq!(
            parse_options("run", &args(&["--config", "c.toml"]))
                .unwrap()
                .config,
            Some(PathBuf::from("c.toml"))
        );
        for (command, bad) in [
            ("run", &["--dir", "d"][..]),
            ("init", &["--no-pager"]),
            ("stats", &["--since", "yesterday"]),
            ("stats", &["--config"]),
        ] {
            assert!(
                parse_options(command, &args(bad)).is_err(),
                "{command} accepted {bad:?}"
            );
        }
    }

    #[test]
    fn a_run_without_configuration_points_to_init() {
        let o = Options {
            config: Some(PathBuf::from("no/such/rip-datagen.toml")),
            ..Options::default()
        };
        let err = run(&o).unwrap_err();
        assert!(err.contains("rip-datagen init"), "{err}");
    }
}
