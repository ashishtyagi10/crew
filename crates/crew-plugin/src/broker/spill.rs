//! A long tool result saved whole, where the agent can read it back.
//!
//! Every result the agent sees is fitted under [`RUN_FIT`], and what the
//! fitting dropped was gone: the middle of a 3,000-line test log, the 400th
//! grep hit, the second half of a docs page. The only way back was to run the
//! tool again with narrower arguments, and a slow test suite costs minutes a
//! run. Cursor's "dynamic context discovery" answers this by writing a long
//! output to a file the agent can page and search; so does this. The fitted
//! result stays what it was — start, end, what was left out — and its last
//! line names the file, for `sys:read_file` to open at a line and `sys:grep`
//! to search.
//!
//! Files go in `.crew/out/` under the broker's working directory, where every
//! relative path the tools take resolves: crew's own bookkeeping, which
//! `changed::is_crew_artifact` and the checkpoints already leave out. A
//! `.gitignore` of `*` goes in first, so a repository that tracks `.crew/`
//! still never commits output, and only the newest thirty files stay.
//! Saving is best effort: on any IO error the result is exactly what it was
//! before this existed. `CREW_SPILL=0` turns it off.
use std::borrow::Cow;
use std::path::PathBuf;

use super::runfit::head_of;
use super::sysread::grouped;
use super::toolclip::RUN_FIT;

#[path = "spillfile.rs"]
mod file;
use file::write;

#[cfg(test)]
#[path = "spill_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "spillcall_tests.rs"]
mod call_tests;

#[cfg(test)]
#[path = "spilltest.rs"]
pub(super) mod spilltest;

/// Bytes one spill holds at most: `sys:grep` passes over files larger than
/// 1 MB, and a saved output the pointer says to search must be searchable.
pub(super) const CAP: usize = 1 << 20;

/// Ends a spill that [`CAP`] cut, so the file says it is not the whole.
const CUT: &str = "\n\u{2026} (saved output cut at 1 MB)\n";

/// A fitted result's standing with its saved whole: the bytes it may take so
/// that it and the pointer together stay within [`RUN_FIT`], and the pointer.
pub(super) struct Spill {
    pub(super) room: usize,
    line: Option<String>,
}

impl Spill {
    /// Nothing saved: all of [`RUN_FIT`] is the result's, and it ends as before.
    pub(super) fn none() -> Self {
        Self {
            room: RUN_FIT,
            line: None,
        }
    }

    pub(super) fn is_saved(&self) -> bool {
        self.line.is_some()
    }

    /// `text`, ended by the pointer on a line of its own when there is one.
    pub(super) fn close(&self, mut text: String) -> String {
        if let Some(line) = &self.line {
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(line);
        }
        text
    }
}

/// Save `whole`, what `tool` would have said with nothing left out, and say
/// where. A whole within [`RUN_FIT`] loses nothing to the fitting, so it is
/// not saved and the result is byte-identical to one that never spilled.
pub(super) fn saved(tool: &str, whole: &str) -> Spill {
    let on = std::env::var("CREW_SPILL").map_or(true, |v| v.trim() != "0");
    if whole.len() <= RUN_FIT || !on {
        return Spill::none();
    }
    let body = capped(whole);
    let Some(rel) = root().and_then(|r| write(&r, tool, &body)) else {
        return Spill::none();
    };
    let line = pointer(&body, &rel);
    Spill {
        room: RUN_FIT - line.len() - 1,
        line: Some(line),
    }
}

/// The one line that ends a spilled result: how much was saved, where, and
/// the two calls that reach it — the path in the JSON whole, so the agent
/// copies it rather than retyping it.
pub(super) fn pointer(body: &str, rel: &str) -> String {
    let n = body.lines().count();
    format!(
        "\u{2026} full output ({} line{}, {} KB) saved to {rel} \u{2014} read it with \
         sys:read_file {{\"path\": \"{rel}\", \"line\": N}} or search it with sys:grep",
        grouped(n),
        if n == 1 { "" } else { "s" },
        grouped(body.len().div_ceil(1024)),
    )
}

/// `whole` within [`CAP`], cut on a line end and marked when it was not.
fn capped(whole: &str) -> Cow<'_, str> {
    if whole.len() <= CAP {
        return Cow::Borrowed(whole);
    }
    Cow::Owned(format!("{}{CUT}", head_of(whole, CAP - CUT.len())))
}

/// The directory the spill goes under: the broker's working directory, so
/// the path in the pointer is one `sys:read_file` opens as written. Not
/// `CREW_PROJECT_DIR`: that moves crew's stores, not where a path resolves.
#[cfg(not(test))]
fn root() -> Option<PathBuf> {
    Some(PathBuf::from("."))
}

/// Lib tests share the crate's directory as their working directory, so a
/// test spills only into a directory it named (`spilltest::guard`); one that
/// did not gets today's result and leaves no file behind.
#[cfg(test)]
fn root() -> Option<PathBuf> {
    spilltest::root()
}
