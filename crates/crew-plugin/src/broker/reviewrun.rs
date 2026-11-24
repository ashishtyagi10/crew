//! One review at its level, from prompt to the text the pane shows.
//!
//! Pulled out of `review_cmd` so what each level costs is one function's
//! doing: the elected reviewer is called once for a quick look or a standard
//! review, and once more for a deep review's second look (`reviewverify`).
//! Every level's findings are checked against the tree (`reviewfindings`),
//! which costs no call.
use std::path::Path;
use std::time::Duration;

use super::adapter::Adapter;
use super::reviewfindings::Tidied;
use super::reviewlevel::{self, Level};

/// What a review came back with: the text to show (`None` for an empty
/// reply) or the call's error, and the smith rows to show before it.
pub(crate) struct Ran {
    pub reply: Result<Option<Tidied>, String>,
    pub notes: Vec<String>,
}

/// Review `diff` in `dir` at `level` with the reviewer `a`.
pub(crate) fn run(level: Level, dir: &Path, diff: &str, a: &dyn Adapter, timeout: Duration) -> Ran {
    // Over the route's `high` for as long as this review runs: a quick
    // look thinks `low`.
    let _effort = level.effort().map(|e| super::routeeffort::enter(Some(e)));
    let callers = if level.reads_callers() {
        super::reviewcallers::section(dir, diff)
    } else {
        String::new()
    };
    let body = reviewlevel::prompt(level, diff, &callers);
    let (reply, edited) = super::reviewguard::call(dir, a, &body, timeout);
    let mut notes: Vec<String> = edited.into_iter().collect();
    let r = match reply {
        Err(e) => {
            return Ran {
                reply: Err(e),
                notes,
            }
        }
        Ok(r) if r.trim().is_empty() => {
            return Ran {
                reply: Ok(None),
                notes,
            }
        }
        Ok(r) => r,
    };
    // Checked before shown: a finding citing a file or line that is not
    // there is dropped and said so, the verdict recounted. The quick look's
    // cut comes after, so a false finding never takes a true one's place.
    let mut t = super::reviewfindings::tidy_in(dir, diff, &r);
    if level == Level::Quick {
        t = reviewlevel::worst(t, reviewlevel::QUICK_MAX);
    }
    if level == Level::Deep {
        let (seen, more) = super::reviewverify::second_look(dir, diff, &callers, a, timeout, t);
        t = seen;
        notes.extend(more);
    }
    if let Some(label) = level.label(reviewlevel::count(&t.text)) {
        t.text = format!("{label}\n\n{}", t.text);
    }
    Ran {
        reply: Ok(Some(t)),
        notes,
    }
}

#[cfg(test)]
#[path = "reviewrun_tests.rs"]
mod tests;
