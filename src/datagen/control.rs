//! Run control of `rip-datagen` (Matt-Magie step 109): one run per data directory, a run in the
//! background, a gentle stop and the state of the current run.
//!
//! Two files next to the batches of a data directory carry it, so it works the same on every
//! platform and needs no signals:
//! - `rip-datagen.lock`: written by the run that owns the directory (its process id, start time and
//!   configuration), removed when it ends. A lock whose process is gone is stale and is taken over.
//! - `rip-datagen.stop`: written by `rip-datagen stop`. The run reads it like Ctrl+C - every thread
//!   finishes the game it is playing - and removes it when it ends.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// The lock of the run that owns a data directory.
pub const LOCK_FILE: &str = "rip-datagen.lock";
/// Asks the run of a data directory to stop after the games being played.
pub const STOP_FILE: &str = "rip-datagen.stop";
/// Standard output and error of a run started in the background.
pub const OUT_FILE: &str = "rip-datagen.out";

/// The contents of a lock file.
#[derive(Debug, Clone, PartialEq)]
pub struct Lock {
    pub pid: u32,
    pub started: String,
    pub config: String,
}

impl Lock {
    fn text(&self) -> String {
        format!("pid = {}\nstarted = {}\nconfig = {}\n", self.pid, self.started, self.config)
    }

    fn parse(text: &str) -> Option<Lock> {
        let field = |key: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(key)?.trim_start().strip_prefix('='))
                .map(|v| v.trim().to_string())
        };
        Some(Lock {
            pid: field("pid")?.parse().ok()?,
            started: field("started")?,
            config: field("config")?,
        })
    }
}

/// Whether the process `pid` still exists: `/proc` on Linux; elsewhere it cannot be told, so the
/// process counts as alive and `status` falls back to the batch log's heartbeat.
pub fn process_alive(pid: u32) -> bool {
    if cfg!(target_os = "linux") {
        Path::new("/proc").join(pid.to_string()).exists()
    } else {
        true
    }
}

/// The lock of `data_dir`, if one exists and its process is alive; a stale lock is reported as
/// `None` (and left for [`acquire`] to replace).
pub fn current(data_dir: &Path) -> Option<Lock> {
    let lock = Lock::parse(&std::fs::read_to_string(data_dir.join(LOCK_FILE)).ok()?)?;
    process_alive(lock.pid).then_some(lock)
}

/// Whether `data_dir` holds a lock file whose process is gone.
pub fn stale_lock(data_dir: &Path) -> bool {
    data_dir.join(LOCK_FILE).exists() && current(data_dir).is_none()
}

/// The lock of the running process on `data_dir`, held until it is dropped.
#[derive(Debug)]
pub struct Guard {
    dir: PathBuf,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.dir.join(LOCK_FILE));
        let _ = std::fs::remove_file(self.dir.join(STOP_FILE));
    }
}

/// Takes the lock of `data_dir` for this process; refuses while another live run holds it. A stop
/// request left by an earlier run is cleared so it cannot end this one.
pub fn acquire(data_dir: &Path, config: &Path) -> Result<Guard, String> {
    std::fs::create_dir_all(data_dir).map_err(|e| format!("{}: {e}", data_dir.display()))?;
    if let Some(lock) = current(data_dir) {
        if lock.pid != std::process::id() {
            return Err(format!(
                "a run is active in {} (pid {}, started {}); stop it with `rip-datagen stop`",
                data_dir.display(),
                lock.pid,
                lock.started
            ));
        }
    }
    let _ = std::fs::remove_file(data_dir.join(STOP_FILE));
    let lock = Lock {
        pid: std::process::id(),
        started: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        config: config.display().to_string(),
    };
    std::fs::write(data_dir.join(LOCK_FILE), lock.text())
        .map_err(|e| format!("{LOCK_FILE}: {e}"))?;
    Ok(Guard { dir: data_dir.to_path_buf() })
}

/// Asks the run of `data_dir` to stop; an error when no run is active.
pub fn request_stop(data_dir: &Path) -> Result<Lock, String> {
    let lock = current(data_dir).ok_or_else(|| format!("no run is active in {}", data_dir.display()))?;
    std::fs::write(data_dir.join(STOP_FILE), "stop\n").map_err(|e| format!("{STOP_FILE}: {e}"))?;
    Ok(lock)
}

/// Sets `stop` once a stop request appears in `data_dir`; the watcher ends with the run (when `done`
/// is set) or after setting the flag.
pub fn watch_stop_file(data_dir: &Path, stop: Arc<AtomicBool>, done: Arc<AtomicBool>) {
    let path = data_dir.join(STOP_FILE);
    std::thread::spawn(move || {
        while !done.load(Ordering::SeqCst) {
            if path.exists() {
                eprintln!("rip-datagen: stop requested, stopping after the games being played");
                stop.store(true, Ordering::SeqCst);
                return;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });
}

/// Lowers the scheduling priority of the calling thread to `nice` (0..=19); every thread it starts
/// afterwards inherits it, so a run calls it before it starts its generator threads.
#[cfg(unix)]
pub fn lower_priority(nice: i32) -> Result<(), String> {
    // SAFETY: setpriority only reads its integer arguments.
    let result = unsafe { libc::setpriority(libc::PRIO_PROCESS, 0, nice) };
    if result == 0 {
        Ok(())
    } else {
        Err(format!("cannot set the priority to nice {nice}: {}", std::io::Error::last_os_error()))
    }
}

/// Windows has no nice levels; the setting is ignored there.
#[cfg(not(unix))]
pub fn lower_priority(_nice: i32) -> Result<(), String> {
    Err("nice is not supported on this platform; the run keeps its normal priority".to_string())
}

/// The scheduling priority of the calling thread.
#[cfg(all(unix, test))]
fn current_priority() -> i32 {
    // SAFETY: getpriority only reads its integer arguments.
    unsafe { libc::getpriority(libc::PRIO_PROCESS, 0) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rip_control_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// The priority set by `lower_priority` holds for the thread and every thread it starts later.
    #[cfg(unix)]
    #[test]
    fn a_lowered_priority_is_inherited_by_new_threads() {
        std::thread::spawn(|| {
            lower_priority(7).unwrap();
            assert_eq!(current_priority(), 7);
            let child = std::thread::spawn(current_priority).join().unwrap();
            assert_eq!(child, 7, "a thread started afterwards inherits it");
        })
        .join()
        .unwrap();
    }

    #[test]
    fn the_lock_file_round_trips() {
        let lock = Lock { pid: 42, started: "2026-11-24 18:00:00".into(), config: "/x/rip-datagen.toml".into() };
        assert_eq!(Lock::parse(&lock.text()), Some(lock));
        assert_eq!(Lock::parse("pid = x\n"), None);
    }

    #[test]
    fn a_live_lock_refuses_a_second_run_and_is_released_on_drop() {
        let dir = test_dir("live");
        let guard = acquire(&dir, Path::new("c.toml")).unwrap();
        assert_eq!(current(&dir).map(|l| l.pid), Some(std::process::id()));
        // Another process's live lock: this process (pid 1 always exists on Linux) refuses.
        if cfg!(target_os = "linux") {
            std::fs::write(dir.join(LOCK_FILE), "pid = 1\nstarted = s\nconfig = c\n").unwrap();
            let err = acquire(&dir, Path::new("c.toml")).unwrap_err();
            assert!(err.contains("pid 1") && err.contains("rip-datagen stop"), "{err}");
        }
        drop(guard);
        assert!(!dir.join(LOCK_FILE).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_stale_lock_is_taken_over_and_an_old_stop_request_cleared() {
        if !cfg!(target_os = "linux") {
            return;
        }
        let dir = test_dir("stale");
        std::fs::create_dir_all(&dir).unwrap();
        // The largest pid Linux hands out is 2^22; this one does not exist.
        std::fs::write(dir.join(LOCK_FILE), "pid = 4194999\nstarted = s\nconfig = c\n").unwrap();
        std::fs::write(dir.join(STOP_FILE), "stop\n").unwrap();
        assert!(stale_lock(&dir) && current(&dir).is_none());
        assert!(request_stop(&dir).is_err(), "nothing runs, nothing to stop");
        let guard = acquire(&dir, Path::new("c.toml")).unwrap();
        assert!(!dir.join(STOP_FILE).exists(), "an old stop request must not end the new run");
        drop(guard);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_stop_request_sets_the_flag_of_the_run() {
        let dir = test_dir("stop");
        let guard = acquire(&dir, Path::new("c.toml")).unwrap();
        let (stop, done) = (Arc::new(AtomicBool::new(false)), Arc::new(AtomicBool::new(false)));
        watch_stop_file(&dir, stop.clone(), done.clone());
        assert_eq!(request_stop(&dir).unwrap().pid, std::process::id());
        let mut waited = 0;
        while !stop.load(Ordering::SeqCst) && waited < 50 {
            std::thread::sleep(Duration::from_millis(100));
            waited += 1;
        }
        assert!(stop.load(Ordering::SeqCst), "the watcher must see the stop file");
        done.store(true, Ordering::SeqCst);
        drop(guard);
        assert!(!dir.join(STOP_FILE).exists() && !dir.join(LOCK_FILE).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
