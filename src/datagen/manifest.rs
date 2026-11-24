//! `MANIFEST.json`: the provenance record of one batch, written by the run itself.
//!
//! The layout extends the data manifest of `scripts/nnue_manifest.py` (schema 1, `kind: data`), so
//! `set`, `register` and `nets` read it unchanged: `name`, `positions`, `games`, `complete`,
//! `settings` and `parts` mean what they meant there. New are the generator fields (versions,
//! commit, net, source, seeds, status and times). The sha256 of each part is not computed here; it
//! is added by `scripts/nnue_manifest.py data <batch>` once the batch is final.
//!
//! The file is written atomically (a temporary file renamed over it) at the start, after every
//! progress interval and at the end, so even a killed run leaves a manifest that describes the
//! data written up to its last interval, with status `running`.

use std::fmt::Write as _;
use std::path::Path;

use super::generator::{Counters, GameSettings};

/// File name of the manifest in a batch directory.
pub const MANIFEST_FILE: &str = "MANIFEST.json";

/// A JSON value; just enough to write a manifest without a serialisation crate.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    UInt(u64),
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// An object from `(key, value)` pairs, in the given order.
    pub fn obj<K: Into<String>>(pairs: Vec<(K, Json)>) -> Json {
        Json::Obj(pairs.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    pub fn str(text: impl Into<String>) -> Json {
        Json::Str(text.into())
    }

    /// Pretty-printed with two spaces per level, as `nnue_manifest.py` writes its files.
    pub fn pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out.push('\n');
        out
    }

    fn write(&self, out: &mut String, level: usize) {
        let pad = |out: &mut String, level: usize| out.push_str(&"  ".repeat(level));
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::UInt(n) => {
                let _ = write!(out, "{n}");
            }
            Json::Int(n) => {
                let _ = write!(out, "{n}");
            }
            Json::Float(x) if x.is_finite() => {
                let _ = write!(out, "{x:?}");
            }
            Json::Float(_) => out.push_str("null"),
            Json::Str(s) => write_string(out, s),
            Json::Arr(items) if items.is_empty() => out.push_str("[]"),
            Json::Obj(pairs) if pairs.is_empty() => out.push_str("{}"),
            Json::Arr(items) => {
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    pad(out, level + 1);
                    item.write(out, level + 1);
                    out.push_str(if i + 1 < items.len() { ",\n" } else { "\n" });
                }
                pad(out, level);
                out.push(']');
            }
            Json::Obj(pairs) => {
                out.push_str("{\n");
                for (i, (key, value)) in pairs.iter().enumerate() {
                    pad(out, level + 1);
                    write_string(out, key);
                    out.push_str(": ");
                    value.write(out, level + 1);
                    out.push_str(if i + 1 < pairs.len() { ",\n" } else { "\n" });
                }
                pad(out, level);
                out.push('}');
            }
        }
    }
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// The game settings, under the names `nnue_manifest.py` used for the old generator's settings.
pub fn settings_json(g: &GameSettings) -> Json {
    Json::obj(vec![
        ("nodes", Json::UInt(g.nodes)),
        ("random_plies_min", Json::UInt(g.random_plies_min as u64)),
        ("random_plies_max", Json::UInt(g.random_plies_max as u64)),
        ("random_prob", Json::UInt(g.random_prob as u64)),
        ("opening_limit", Json::Int(g.opening_limit as i64)),
        ("win_cp", Json::Int(g.win_cp as i64)),
        ("win_plies", Json::UInt(g.win_plies as u64)),
        ("draw_cp", Json::Int(g.draw_cp as i64)),
        ("draw_from_move", Json::Int(g.draw_from_move as i64)),
        ("draw_plies", Json::UInt(g.draw_plies as u64)),
        ("max_moves", Json::Int(g.max_moves as i64)),
        ("hash_entries", Json::UInt(g.hash_entries as u64)),
        ("sample", Json::Float(g.sample)),
        (
            "book",
            g.book.as_deref().map_or(Json::Null, |p| {
                // The file name only: a path names the machine it was generated on.
                Json::str(
                    Path::new(p)
                        .file_name()
                        .map_or(p.into(), |n| n.to_string_lossy()),
                )
            }),
        ),
        ("book_plies_min", Json::UInt(g.book_plies_min as u64)),
        ("book_plies_max", Json::UInt(g.book_plies_max as u64)),
        ("book_random_min", Json::UInt(g.book_random_min as u64)),
        ("book_random_max", Json::UInt(g.book_random_max as u64)),
    ])
}

/// The counters of one part or of the whole batch.
pub fn counters_json(c: &Counters, done: bool) -> Json {
    Json::obj(vec![
        ("done", Json::Bool(done)),
        ("games", Json::UInt(c.games)),
        ("dropped", Json::UInt(c.dropped)),
        ("positions", Json::UInt(c.positions)),
        ("filtered", Json::UInt(c.filtered)),
        ("skipped", Json::UInt(c.skipped)),
        ("plies", Json::UInt(c.plies)),
        ("book_plies", Json::UInt(c.book_plies)),
        ("white_wins", Json::UInt(c.white_wins)),
        ("draws", Json::UInt(c.draws)),
        ("black_wins", Json::UInt(c.black_wins)),
    ])
}

/// Writes `json` to `path` through a temporary file, so a reader never sees half a manifest.
pub fn write_atomic(path: &Path, json: &Json) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json.pretty())?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_is_pretty_printed_and_escaped() {
        let json = Json::obj(vec![
            ("name", Json::str("a \"b\"\\\n")),
            ("n", Json::UInt(3)),
            ("x", Json::Float(0.5)),
            ("one", Json::Float(1.0)),
            ("none", Json::Null),
            ("list", Json::Arr(vec![Json::Bool(true), Json::Int(-2)])),
            ("empty", Json::Arr(vec![])),
            ("nested", Json::obj(vec![("k", Json::Float(f64::NAN))])),
        ]);
        assert_eq!(
            json.pretty(),
            "{\n  \"name\": \"a \\\"b\\\"\\\\\\n\",\n  \"n\": 3,\n  \"x\": 0.5,\n  \"one\": 1.0,\n  \
             \"none\": null,\n  \"list\": [\n    true,\n    -2\n  ],\n  \"empty\": [],\n  \
             \"nested\": {\n    \"k\": null\n  }\n}\n"
        );
    }

    #[test]
    fn the_book_is_recorded_by_file_name_only() {
        let g = GameSettings {
            book: Some("/some/dir/books/wide.bin".to_string()),
            ..GameSettings::default()
        };
        let Json::Obj(pairs) = settings_json(&g) else {
            panic!("settings are an object")
        };
        let book = pairs.iter().find(|(k, _)| k == "book").unwrap();
        assert_eq!(book.1, Json::str("wide.bin"));
    }

    #[test]
    fn an_atomic_write_replaces_the_file() {
        let path = std::env::temp_dir().join(format!("rip-manifest-{}.json", std::process::id()));
        write_atomic(&path, &Json::UInt(1)).unwrap();
        write_atomic(&path, &Json::UInt(2)).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "2\n");
        assert!(!path.with_extension("json.tmp").exists());
        std::fs::remove_file(&path).unwrap();
    }
}
