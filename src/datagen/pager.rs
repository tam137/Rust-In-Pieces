//! Shows long output scrollable: through the user's pager when standard output is a terminal,
//! printed as it is otherwise (a pipe, a file, or `--no-pager`).
//!
//! The pager is `$PAGER` if set, else `less -FRX` (quit at once when the text fits the screen,
//! pass the colours, keep the text on the screen afterwards) and `more` on Windows. If the pager
//! cannot be started, the text is printed instead.

use std::io::{IsTerminal, Write};
use std::process::{Command, Stdio};

/// The pager command: `$PAGER` split at whitespace, or the platform default.
pub fn pager_command(pager_env: Option<&str>) -> Vec<String> {
    if let Some(cmd) = pager_env.map(str::trim).filter(|c| !c.is_empty()) {
        return cmd.split_whitespace().map(str::to_string).collect();
    }
    if cfg!(windows) {
        vec!["more".to_string()]
    } else {
        vec!["less".to_string(), "-FRX".to_string()]
    }
}

/// Shows `text`, through the pager when `use_pager` and standard output is a terminal.
pub fn show(text: &str, use_pager: bool) {
    if use_pager && std::io::stdout().is_terminal() {
        let pager = pager_command(std::env::var("PAGER").ok().as_deref());
        if let Ok(mut child) = Command::new(&pager[0])
            .args(&pager[1..])
            .stdin(Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                // A pager quit early closes the pipe; that is not an error.
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
            return;
        }
    }
    print!("{text}");
    let _ = std::io::stdout().flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pager_comes_from_the_environment_or_the_platform() {
        assert_eq!(pager_command(Some("most -s")), ["most", "-s"]);
        let default = pager_command(None);
        assert_eq!(default, pager_command(Some("   ")));
        if cfg!(windows) {
            assert_eq!(default, ["more"]);
        } else {
            assert_eq!(default, ["less", "-FRX"]);
        }
    }
}
