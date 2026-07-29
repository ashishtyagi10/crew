//! How `keys::apply` reinterprets input while `ViewPane::search` is live:
//! Esc cancels a search before it closes the pane, and while the needle is
//! still being typed every input is a needle edit rather than its
//! normal-mode meaning. Split out of `keys.rs` to keep that file to the
//! scroll/action dispatch it's named for.
use crate::viewpane::keys::ViewInput;
use crate::viewpane::search::Search;
use crate::viewpane::ViewPane;

/// True when `input` was consumed by a live search — either it cancelled
/// the search (Esc) or was swallowed as a needle edit while typing. `keys::
/// apply` returns early in that case, leaving the normal scroll/action match
/// untouched.
pub(crate) fn intercept(p: &mut ViewPane, input: ViewInput, cols: u16, rows: u16) -> bool {
    if p.search.is_none() {
        return false;
    }
    if matches!(input, ViewInput::Close) {
        p.search = None;
        return true;
    }
    if p.search.as_ref().is_some_and(|s| s.typing) {
        apply_typing(p, input, cols, rows);
        return true;
    }
    false
}

/// `n`/`N` outside a live search do nothing — there is nothing to jump to.
pub(crate) fn jump(
    p: &mut ViewPane,
    cols: u16,
    rows: u16,
    step: impl FnOnce(&mut Search) -> Option<usize>,
) {
    if let Some(line) = p.search.as_mut().and_then(step) {
        p.scroll = line;
        p.clamp_scroll(cols, rows);
    }
}

/// While a search is being typed, every input is a needle edit — including
/// `e`/`o`/`r`/`s`/`n`/`N`/`/`, which `view_key` has already folded into
/// their normal-mode `ViewInput`s. Reverse-mapping them back to the one
/// character each represents is what lets you type a needle containing any
/// of those letters without firing Edit/Reload/ToggleRaw or jumping hits.
fn apply_typing(p: &mut ViewPane, input: ViewInput, cols: u16, rows: u16) {
    let Some(search) = p.search.as_mut() else {
        return;
    };
    match input {
        ViewInput::Char(c) => search.needle.push(c),
        ViewInput::Backspace => {
            search.needle.pop();
        }
        ViewInput::Edit => search.needle.push('e'),
        ViewInput::OpenExternal => search.needle.push('o'),
        ViewInput::Reload => search.needle.push('r'),
        ViewInput::ToggleRaw => search.needle.push('s'),
        ViewInput::NextHit => search.needle.push('n'),
        ViewInput::PrevHit => search.needle.push('N'),
        ViewInput::Slash => search.needle.push('/'),
        ViewInput::Enter => {
            search.typing = false;
            // Every scroll action clamps the stored offset, this jump
            // included — otherwise a hit past the last full page strands
            // `scroll` beyond content length and deadens later Up ticks.
            if let Some(line) = search.next() {
                p.scroll = line;
                p.clamp_scroll(cols, rows);
            }
            return;
        }
        // Close is handled by `intercept` before reaching here; scroll/no-op
        // inputs have no meaning while typing a needle.
        _ => return,
    }
    recompute_hits(p, cols);
}

/// Recompute `p.search`'s hits against the pane's current rendered text.
/// Called on every needle edit so `n`/`N` always walk what's on screen, not
/// a search that's gone stale since the last keystroke.
fn recompute_hits(p: &mut ViewPane, cols: u16) {
    let Some(needle) = p.search.as_ref().map(|s| s.needle.clone()) else {
        return;
    };
    // Scoped so the immutable borrow of `p` behind this `Ref` ends before
    // `p.search.as_mut()` below needs a mutable one.
    let lines: Vec<String> = {
        let cache = p.lines_for(cols);
        cache.lines.iter().map(line_text).collect()
    };
    let hits = crate::viewpane::search::find_matches(&lines, &needle);
    if let Some(search) = p.search.as_mut() {
        search.hits = hits;
    }
}

fn line_text(line: &crate::chatbody::CardLine) -> String {
    line.iter().map(|c| c.c).collect()
}
