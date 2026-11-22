//! The Explorer's git marks, as VS Code draws them: a changed file's name in its change's
//! colour with the letter at the row's end (M modified, A added, U untracked, D deleted, ! in
//! conflict), and a folder holding changes tinted with a dot. Read from the answer the nav's
//! git poll already has ([`crate::gitfiles`]); a tree outside a repository shows none.
use crate::explorer::Item;
use crate::gitfiles::{Files, Mark};

/// What a row says about git: a file's own mark, or a folder holding changed files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Said {
    File(Mark),
    Holds,
}

/// What `item`'s row says, if anything. The root never: it is the repository, all of it.
pub(crate) fn said(git: &Files, item: &Item) -> Option<Said> {
    if item.depth == 0 || item.more.is_some() {
        return None;
    }
    match item.dir {
        true => match git.mark(&item.path) {
            Some(Mark::Untracked) => Some(Said::File(Mark::Untracked)),
            _ => git.holds_changes(&item.path).then_some(Said::Holds),
        },
        false => git.mark(&item.path).map(Said::File),
    }
}

/// The ink a row in that state is written in: the diff viewer's added and removed inks, the
/// gauges' amber for a change.
pub(crate) fn ink(s: Option<Said>) -> (u8, u8, u8) {
    use crate::chatink::token_fg;
    use crate::md::syntax::Token;
    let t = crew_theme::theme();
    match s {
        Some(Said::File(Mark::Added | Mark::Untracked)) => token_fg(Token::Added),
        Some(Said::File(Mark::Deleted | Mark::Conflict)) => token_fg(Token::Removed),
        Some(_) => crew_theme::readable::warn(t),
        None => t.ink,
    }
}

/// The mark at the row's end.
pub(crate) fn letter(s: Said) -> &'static str {
    match s {
        Said::Holds => "\u{2022}",
        Said::File(m) => m.letter(),
    }
}

#[cfg(test)]
#[path = "explorergit_tests.rs"]
mod tests;
