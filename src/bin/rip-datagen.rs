//! `rip-datagen`: self-play training data for the NNUE trainer, with the engine's embedded net.
//!
//! See `src/datagen/mod.rs` for the parts and `nnue_training/README.md` for the workflow.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use suprah::datagen::config::{self, CONFIG_FILE};
use suprah::datagen::{control, pager, runner, stats};

const USAGE: &str = "\
rip-datagen - self-play training data for the NNUE trainer

Commands:
  init     [--config FILE]   write a configuration with the defaults (never overwrites)
  start    [--config FILE]   generate one batch in the background; the terminal is free at once
  status   [--config FILE]   is a run active? pid, start, threads, games, positions, speed
  stop     [--config FILE] [--no-wait]
                             stop the active run after the games being played (waits for it
                             unless --no-wait)
  run      [--config FILE]   generate one batch in the foreground (Ctrl+C stops it the same way)
  stats    [--config FILE | --dir DIR] [--since YYYY-MM-DD] [--no-pager]
                             statistics over every batch of the data directory
  version                    generator version, engine version, commit and net
  help                       this overview

The configuration is rip-datagen.toml in the working directory unless --config names another;
its data_dir holds the batches. One run per data directory: start and run refuse while one is
active. A run creates <data_dir>/<date>-<time>_<seed>/ with part_<i>.txt per thread, batch.log
and MANIFEST.json; a background run writes its terminal output to <data_dir>/rip-datagen.out.

Typical use:
  rip-datagen init      # once, then edit rip-datagen.toml (source, threads, nodes, ...)
  rip-datagen start
  rip-datagen status
  rip-datagen stop
";

/// Command-line options after the command.
#[derive(Debug, Default, PartialEq)]
struct Options {
    config: Option<PathBuf>,
    dir: Option<PathBuf>,
    since: Option<chrono::NaiveDate>,
    no_pager: bool,
    no_wait: bool,
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
            ("stop", "--no-wait") => o.no_wait = true,
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
    let (path, cfg) = load_config(o)?;
    // First of all, so every thread of the run - generators, watcher, Ctrl+C - inherits it.
    if let Err(e) = control::lower_priority(cfg.nice) {
        eprintln!("rip-datagen: {e}");
    }
    let config_abs = std::fs::canonicalize(&path).unwrap_or(path.clone());
    let _lock = control::acquire(&cfg.data_dir, &config_abs)?;
    let stop = Arc::new(AtomicBool::new(false));
    let done = Arc::new(AtomicBool::new(false));
    control::watch_stop_file(&cfg.data_dir, stop.clone(), done.clone());
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
        "source {}, {} threads, nice {}, {}, data in {}",
        cfg.source,
        cfg.threads,
        cfg.nice,
        cfg.games
            .map_or("until Ctrl+C".to_string(), |n| format!("{n} games")),
        cfg.data_dir.display()
    );
    let summary = runner::run(&cfg, stop, &mut |line| {
        eprintln!("{} {line}", chrono::Local::now().format("%H:%M:%S"));
    });
    done.store(true, Ordering::SeqCst);
    let summary = summary?;
    eprintln!(
        "{}: {} games, {} positions in {}",
        summary.status.label(),
        summary.totals.games,
        summary.totals.positions,
        summary.batch_dir.display()
    );
    Ok(())
}

/// The configuration of `o`, which has to exist.
fn load_config(o: &Options) -> Result<(PathBuf, config::RunConfig), String> {
    let path = config_path(o);
    if !path.exists() {
        return Err(format!(
            "no configuration {}; create one with `rip-datagen init`",
            path.display()
        ));
    }
    let cfg = config::load(&path)?;
    Ok((path, cfg))
}

/// `start`: the same run as `run`, as a process of its own that outlives the terminal.
fn start(o: &Options) -> Result<(), String> {
    let (path, cfg) = load_config(o)?;
    if let Some(lock) = control::current(&cfg.data_dir) {
        return Err(format!(
            "a run is active in {} (pid {}, started {}); see `rip-datagen status`",
            cfg.data_dir.display(),
            lock.pid,
            lock.started
        ));
    }
    std::fs::create_dir_all(&cfg.data_dir)
        .map_err(|e| format!("{}: {e}", cfg.data_dir.display()))?;
    let config_abs = std::fs::canonicalize(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let out_path = cfg.data_dir.join(control::OUT_FILE);
    let out = std::fs::File::create(&out_path).map_err(|e| format!("{}: {e}", out_path.display()))?;
    let err = out.try_clone().map_err(|e| format!("{}: {e}", out_path.display()))?;
    let exe = std::env::current_exe().map_err(|e| format!("cannot find this binary: {e}"))?;
    let mut command = std::process::Command::new(exe);
    command
        .arg("run")
        .arg("--config")
        .arg(&config_abs)
        .stdin(std::process::Stdio::null())
        .stdout(out)
        .stderr(err);
    detach(&mut command);
    let mut child = command.spawn().map_err(|e| format!("cannot start the run: {e}"))?;
    // The run takes the lock within milliseconds; a run that fails at once (a bad book, a lock
    // taken meanwhile) has ended by then and left its reason in the output file.
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            let output = std::fs::read_to_string(&out_path).unwrap_or_default();
            return Err(format!("the run ended at once ({status}):\n{}", output.trim_end()));
        }
        if control::current(&cfg.data_dir).is_some_and(|l| l.pid == child.id()) {
            break;
        }
    }
    println!(
        "started: pid {}, {} threads, data in {}\noutput in {}\n`rip-datagen status` shows it, `rip-datagen stop` ends it",
        child.id(),
        cfg.threads,
        cfg.data_dir.display(),
        out_path.display()
    );
    Ok(())
}

/// Puts the background run in a process group of its own, so the terminal's Ctrl+C and hang-up
/// do not reach it.
#[cfg(unix)]
fn detach(command: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

/// Starts the background run without a console window, in a process group of its own.
#[cfg(windows)]
fn detach(command: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
}

/// `stop`: asks the active run to stop and, unless `--no-wait`, waits until it has.
fn stop(o: &Options) -> Result<(), String> {
    let (_, cfg) = load_config(o)?;
    let lock = control::request_stop(&cfg.data_dir)?;
    println!(
        "stop requested: pid {} finishes the games being played",
        lock.pid
    );
    if o.no_wait {
        return Ok(());
    }
    let started = std::time::Instant::now();
    while control::current(&cfg.data_dir).is_some_and(|l| l.pid == lock.pid) {
        std::thread::sleep(std::time::Duration::from_secs(1));
        if started.elapsed().as_secs() % 15 == 0 {
            println!("  waiting ({} s)", started.elapsed().as_secs());
        }
    }
    println!("stopped after {} s", started.elapsed().as_secs());
    Ok(())
}

/// `status`: the active run of the data directory, or that there is none.
fn status(o: &Options) -> Result<(), String> {
    let (_, cfg) = load_config(o)?;
    let Some(lock) = control::current(&cfg.data_dir) else {
        if control::stale_lock(&cfg.data_dir) {
            println!(
                "no run active in {} (a lock of an ended process is left; the next start replaces it)",
                cfg.data_dir.display()
            );
        } else {
            println!("no run active in {}", cfg.data_dir.display());
        }
        return Ok(());
    };
    println!("run active in {}", cfg.data_dir.display());
    println!("  pid       {}", lock.pid);
    println!("  started   {}", lock.started);
    println!("  config    {}", lock.config);
    let now = chrono::Local::now().fixed_offset();
    let running: Vec<_> = stats::load(&cfg.data_dir, now)?
        .into_iter()
        .filter(|b| b.status == "running")
        .collect();
    match running.last() {
        Some(b) => {
            let seconds = now.signed_duration_since(b.started).num_seconds().max(1);
            println!("  batch     {}", b.name);
            println!("  threads   {}", b.threads);
            println!("  games     {}", b.totals.games);
            println!("  positions {}", b.totals.positions);
            println!(
                "  speed     {:.0} positions/s, {:.1} M/h (since the start)",
                b.totals.positions as f64 / seconds as f64,
                b.totals.positions as f64 * 3600.0 / seconds as f64 / 1e6
            );
            println!(
                "  updated   {}",
                b.last.with_timezone(&chrono::Local).format("%H:%M:%S")
            );
        }
        None => println!("  (no progress written yet; the first line comes after log_interval)"),
    }
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
        "run" | "init" | "stats" | "start" | "stop" | "status" => parse_options(command, rest).and_then(|o| match command {
            "run" => run(&o),
            "start" => start(&o),
            "stop" => stop(&o),
            "status" => status(&o),
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
        assert!(parse_options("stop", &args(&["--no-wait"])).unwrap().no_wait);
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
            ("start", &["--no-wait"]),
            ("status", &["--dir", "d"]),
        ] {
            assert!(
                parse_options(command, &args(bad)).is_err(),
                "{command} accepted {bad:?}"
            );
        }
    }

    #[test]
    fn help_names_every_command() {
        for command in ["init", "start", "status", "stop", "run", "stats", "version", "help"] {
            assert!(USAGE.contains(&format!("\n  {command} ")), "help lacks {command}");
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
