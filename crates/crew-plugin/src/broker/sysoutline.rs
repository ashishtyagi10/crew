//! `sys:outline {"path"}`: what is in a file, and at which line.
//!
//! To find where `impl Broker` or `fn frame` lives in a 700-line file an
//! agent paged through it from the top, five `sys:read_file` pages and five of
//! its eight tool rounds, or grepped for the names one at a time, and it had
//! to know the names first. Aider's repo map and an IDE's outline answer
//! "what is in this file, and where" in one look; this is that look: each
//! definition on a row with its line, indented by what it is inside, for the
//! agent to open with `sys:read_file {"line": N}`.
//!
//! Recognised line by line, per language family ([`super::sysoutlinelang`]
//! and its siblings), not parsed: good enough to navigate, and no parser or
//! language server to carry or to wait for.
use std::path::Path;

use super::sysoutlinejs::{go, js};
use super::sysoutlinelang::rust;
use super::sysoutlinepy::{markdown, python};
use super::sysread::grouped;
use super::toolclip::RUN_FIT;

#[cfg(test)]
#[path = "sysoutline_tests.rs"]
mod tests;

/// Bytes of file outlined at most. Source past it is generated or vendored,
/// and its outline would not fit the answer anyway.
const TOO_BIG: u64 = 8 * 1024 * 1024;

/// Bytes kept back from [`RUN_FIT`] for the closing line that says what was
/// left out: two 20-digit counts and its words come to well under this.
const TAIL: usize = 200;

/// One definition: its 1-based line, how many listed blocks it is inside,
/// and its row's text.
pub(super) struct Item {
    pub(super) line: usize,
    pub(super) depth: usize,
    pub(super) text: String,
}

impl Item {
    /// The definition on the line at 0-based `index`, numbered the way
    /// `sys:grep` and `sys:read_file {"line"}` number it, from 1.
    pub(super) fn at(index: usize, depth: usize, text: String) -> Self {
        Item {
            line: index + 1,
            depth,
            text,
        }
    }
}

/// `sys:outline`: the outline of the file at `path`.
pub(crate) fn outline(path: &str) -> Result<String, String> {
    let p = Path::new(path);
    if p.is_dir() {
        return Err(format!(
            "{path} is a directory \u{2014} sys:outline reads one file; list what is in it with sys:glob {{\"pattern\": \"*\", \"path\": \"{path}\"}}"
        ));
    }
    let meta = std::fs::metadata(p).map_err(|e| super::syspath::with_hint("outline", path, e))?;
    if meta.len() > TOO_BIG {
        return Err(format!(
            "{path} is {} MB, too big to outline \u{2014} sys:grep it for the name you want",
            meta.len() >> 20
        ));
    }
    let bytes = std::fs::read(p).map_err(|e| super::syspath::with_hint("outline", path, e))?;
    let name = p
        .file_name()
        .map_or(path.into(), |n| n.to_string_lossy().into_owned());
    render(&name, &String::from_utf8_lossy(&bytes))
}

/// The outline of `src`, a file called `name`: a line naming it and its
/// length, then a row per definition, fitted to [`RUN_FIT`].
pub(super) fn render(name: &str, src: &str) -> Result<String, String> {
    let ext = Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase());
    let items = match ext.as_deref() {
        Some("rs") => rust(src),
        Some("py" | "pyi") => python(src),
        Some("js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts") => js(src),
        Some("go") => go(src),
        Some("md" | "markdown") => markdown(src),
        Some(other) => {
            return Err(format!(
                "no outline for .{other} files \u{2014} use sys:grep to find a name in it"
            ))
        }
        None => {
            return Err(format!(
                "no outline for {name}, a file with no extension to tell its language \u{2014} use sys:grep to find a name in it"
            ))
        }
    };
    let lines = src.lines().count();
    let head = format!(
        "{name} \u{2014} {} line{}",
        grouped(lines),
        if lines == 1 { "" } else { "s" }
    );
    if items.is_empty() {
        return Ok(format!(
            "{head}\n  no definitions found \u{2014} read it with sys:read_file"
        ));
    }
    Ok(fit(head, &items, lines.to_string().len().max(4)))
}

/// The rows under `head`, every one when they fit in [`RUN_FIT`]. When they
/// do not, the top-level ones go in first, then each deeper level in turn, in
/// file order, until the next would not fit; what is shown stays in file
/// order, and a last line counts what was not.
fn fit(head: String, items: &[Item], width: usize) -> String {
    let rows: Vec<String> = items
        .iter()
        .map(|it| format!("{:>width$}  {}{}", it.line, "  ".repeat(it.depth), it.text))
        .collect();
    let whole = head.len() + rows.iter().map(|r| r.len() + 1).sum::<usize>();
    let mut kept = vec![whole <= RUN_FIT; rows.len()];
    if whole > RUN_FIT {
        let mut order: Vec<usize> = (0..items.len()).collect();
        order.sort_by_key(|&i| items[i].depth);
        let mut bytes = head.len() + 1 + TAIL;
        for i in order {
            if bytes + rows[i].len() + 1 > RUN_FIT {
                break;
            }
            bytes += rows[i].len() + 1;
            kept[i] = true;
        }
    }
    let mut out = head;
    for (row, _) in rows.iter().zip(&kept).filter(|(_, k)| **k) {
        out.push('\n');
        out.push_str(row);
    }
    let left = |top: bool| {
        items
            .iter()
            .zip(&kept)
            .filter(|(it, k)| !**k && (it.depth == 0) == top)
            .count()
    };
    let (top, nested) = (left(true), left(false));
    if top + nested > 0 {
        out.push('\n');
        out.push_str(&tail(top, nested));
    }
    out
}

/// The closing line of an outline that did not fit whole.
fn tail(top: usize, nested: usize) -> String {
    let n =
        |k: usize, what: &str| format!("{} {what}{}", grouped(k), if k == 1 { "" } else { "s" });
    if top == 0 {
        return format!(
            "\u{2026} {} left out, every top-level one kept \u{2014} sys:read_file with \"line\" at a block to see inside it",
            n(nested, "nested definition")
        );
    }
    format!(
        "\u{2026} {} left out ({} top-level, {} nested) \u{2014} too many to list; sys:grep for the name you want",
        n(top + nested, "definition"),
        grouped(top),
        grouped(nested)
    )
}
