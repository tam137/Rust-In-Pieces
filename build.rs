//! Embeds the commit the build comes from as `RIP_GIT_COMMIT`, so `rip-datagen` can record it in
//! every batch it writes. Outside a git checkout (a source archive) the variable stays unset and
//! the binary reports `unknown`.

use std::path::PathBuf;
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if let Some(git_dir) = git(&["rev-parse", "--git-dir"]) {
        let git_dir = PathBuf::from(git_dir);
        // HEAD names the branch; the branch refs move with every commit. Only existing paths are
        // watched: a missing one would make Cargo rerun this script on every build.
        for watched in [
            git_dir.join("HEAD"),
            git_dir.join("refs/heads"),
            git_dir.join("packed-refs"),
        ] {
            if watched.exists() {
                println!("cargo:rerun-if-changed={}", watched.display());
            }
        }
    }
    if let Some(commit) = git(&["rev-parse", "--short=12", "HEAD"]) {
        println!("cargo:rustc-env=RIP_GIT_COMMIT={commit}");
    }
}
