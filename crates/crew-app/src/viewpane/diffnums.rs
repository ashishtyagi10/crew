//! What number a unified diff's gutter should show.
//!
//! Every other rung numbers rows by their position in the file, which for a
//! diff means numbering the PATCH — `diff --git` is line 1, `index` is 2, the
//! first hunk header is 5. Those are the line numbers of a file nobody has
//! open. The reader wants the number the hunk header already declares: where
//! this line is in the source.
//!
//! The side-by-side rung ([`super::diffsplit`]) has always shown exactly
//! that, from the same `@@ -old,+new @@` arithmetic, so the two views of one
//! review disagreed about what line you were looking at depending on which
//! way you had pressed `v`.
use super::diffpaint::{kind_of, Kind};

/// One entry per line of `text`: the source line to label it with, or `None`
/// for the rows that belong to no file — the `diff`/`index`/`---`/`+++`
/// headers and the `@@` hunk header itself.
///
/// A removed line is numbered in the OLD file and an added one in the NEW,
/// which is the only honest answer for each: they do not both exist.
pub(super) fn numbers(text: &str) -> Vec<Option<usize>> {
    let (mut old, mut new) = (0usize, 0usize);
    text.split('\n')
        .map(|line| match kind_of(line) {
            Kind::File => None,
            Kind::Hunk => {
                if let Some((o, n)) = super::diffsplit::hunk_start(line) {
                    (old, new) = (o, n);
                }
                None
            }
            Kind::Context => {
                let at = new;
                old += 1;
                new += 1;
                Some(at)
            }
            Kind::Removed => {
                let at = old;
                old += 1;
                Some(at)
            }
            Kind::Added => {
                let at = new;
                new += 1;
                Some(at)
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "diffnums_tests.rs"]
mod tests;
