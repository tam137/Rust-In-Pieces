use std::io::{self, Write};

pub struct StdoutWrapper;

impl StdoutWrapper {
    pub fn write(&self, msg: &str) {
        if let Err(e) = writeln!(io::stdout(), "{}", msg) {
            eprintln!("RIP Failed to write to stdout: {}", e);
        }
    }

    pub fn write_get_result(&self, msg: &str) -> Result<(), io::Error> {
        writeln!(io::stdout(), "{}", msg).map_err(|e| {
            e
        })
    }
}
