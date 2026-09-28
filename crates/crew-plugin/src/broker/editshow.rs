//! What an edit left in the file, shown under `sys:edit`'s one-line report.
//!
//! WHY: the report said where and how many lines, `edited main.rs at line 41
//! (1 line → 2, +1)`, and not what the file now says there. A doubled brace,
//! a line joined to its neighbour or an indent one level off are all
//! invisible in that line, so an agent that wanted to check its work spent a
//! `sys:read_file` round on it, and one that did not (most of them) never
//! found out. Claude Code's `Edit` answers with the edited region, and that
//! is what lets a model catch its own mistake on the spot.
//!
//! This half finds the lines each edit's new text now covers; the rows, the
//! context around them and the cap are [`super::editrows`].
use super::editrows::{layout, Edit};

/// Where each edit's new text sits in the file as it is NOW. A batch applies
/// its edits in order, so a later one can move an earlier one's text, or
/// rewrite part of it; the spans are kept up to date as each one lands.
#[derive(Default)]
pub(super) struct Landed(Vec<Span>);

/// Bytes `start..end` of the buffer. An empty span with `cut` lines is whole
/// lines taken out, which leaves no line of its own to show.
struct Span {
    start: usize,
    end: usize,
    cut: usize,
}

impl Landed {
    /// Note that the one `old` in `before` was replaced by `new`.
    pub(super) fn swap(&mut self, before: &str, old: &str, new: &str) {
        let at = before.find(old).unwrap_or(0);
        let (gone, grew) = (at + old.len(), new.len() as isize - old.len() as isize);
        let whole = new.is_empty() && old.ends_with('\n') && line_start(before, at);
        let cut = if whole { old.matches('\n').count() } else { 0 };
        let end = at + new.len();
        let mut here = Span {
            start: at,
            end,
            cut,
        };
        self.0.retain_mut(|s| {
            if s.end <= at {
                return true;
            }
            if s.start >= gone {
                s.start = s.start.saturating_add_signed(grew);
                s.end = s.end.saturating_add_signed(grew);
                return true;
            }
            // Text this edit rewrote part of: one region now, not two.
            here.start = here.start.min(s.start);
            if s.end > gone {
                here.end = here.end.max(s.end.saturating_add_signed(grew));
            }
            false
        });
        if here.start != here.end {
            here.cut = 0;
        }
        self.0.push(here);
    }

    /// The edited lines of `after`, numbered, with the lines around them.
    pub(super) fn show(&self, after: &str) -> String {
        layout(self.0.iter().map(|s| s.lines(after)).collect(), after)
    }
}

fn line_start(s: &str, at: usize) -> bool {
    at == 0 || s.as_bytes()[at - 1] == b'\n'
}

/// The line byte `x` is on, counting `\n`s before it the way grep and
/// `sys:read_file {"line": N}` count, so a CRLF file numbers the same.
fn line_of(s: &str, x: usize) -> usize {
    1 + s.as_bytes()[..x].iter().filter(|&&b| b == b'\n').count()
}

impl Span {
    fn lines(&self, after: &str) -> Edit {
        let first = line_of(after, self.start);
        let gap = self.start == self.end;
        // Whole lines out only while the gap is still at a line start.
        let cut = if gap && line_start(after, self.start) {
            self.cut
        } else {
            0
        };
        let last = match (gap, cut) {
            (true, 0) => first,
            (true, _) => first - 1,
            _ => line_of(after, self.end - 1),
        };
        Edit { first, last, cut }
    }
}

#[cfg(test)]
#[path = "editshow_tests.rs"]
mod tests;
