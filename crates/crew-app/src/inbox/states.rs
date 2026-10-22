//! The list with nothing in it, looking finished (goal
//! 2026-10-20-inbox-list-look, item 9): while a page loads, rows in outline
//! the shape of real ones; an empty inbox, a drawn mark and when Gmail was
//! last asked; a search that found nothing, the way back; and a list Gmail
//! would not give, what went wrong in words and the key that mends it.
use crew_render::Paint;

use super::draw::Canvas;
use super::InboxPane;
use crate::chatwidth::str_w;

/// What the pane shows in place of rows.
enum Empty<'a> {
    Loading,
    Failed(&'a str),
    NoMatch(&'a str),
    Zero,
}

impl InboxPane {
    fn emptiness(&self) -> Empty<'_> {
        match (&self.failed, self.busy, self.query.trim()) {
            (Some(e), _, _) => Empty::Failed(e),
            (None, true, _) => Empty::Loading,
            (None, false, "") => Empty::Zero,
            (None, false, q) => Empty::NoMatch(q),
        }
    }

    /// The words of an empty list, centred under where the mark is drawn.
    pub(crate) fn empty(&self, cv: &mut Canvas) {
        let t = crew_theme::theme();
        let (title, sub) = match self.emptiness() {
            Empty::Loading => return,
            Empty::Failed(e) => {
                let (what, key) = failure(e);
                (what.to_string(), key.to_string())
            }
            Empty::NoMatch(q) => (
                format!("no mail matches \u{201c}{q}\u{201d}"),
                "esc goes back to the inbox \u{b7} / searches again".into(),
            ),
            Empty::Zero => ("inbox zero".into(), self.checked()),
        };
        let row = mark_row(cv.rows) + MARK_ROWS;
        for (r, s, fg, bold) in [
            (row, &title, t.ink, true),
            (row + 1, &sub, t.text_muted, false),
        ] {
            let col = cv.cols.saturating_sub(str_w(s) as u16) / 2;
            cv.text(r, col.max(1), cv.cols, s, fg, bold);
        }
    }

    /// When Gmail was last asked: `checked just now`, `checked 4m ago`.
    fn checked(&self) -> String {
        let now = chrono::Utc::now().timestamp_millis();
        match self.checked_ms.map(|at| super::age::said(at, now)) {
            None => String::new(),
            Some(s) if s == "now" => "checked just now".into(),
            Some(s) if s.ends_with(['m', 'h']) => format!("checked {s} ago"),
            Some(s) => format!("checked {s}"),
        }
    }

    /// The paint of an empty list: the outline rows while loading, the mark
    /// over an empty inbox.
    pub(crate) fn empty_paint(&self, cols: u16, rows: u16, aspect: f32) -> Vec<Paint> {
        match self.emptiness() {
            Empty::Loading => skeleton(cols, rows, self.density.two(rows)),
            Empty::Zero => mark(cols, rows, aspect),
            _ => Vec::new(),
        }
    }
}

/// The rows the mark takes, the words going under it.
const MARK_ROWS: u16 = 4;

fn mark_row(rows: u16) -> u16 {
    rows.saturating_sub(MARK_ROWS + 2) / 2
}

/// A ring with a tick in it, in the accent: done, nothing waiting.
fn mark(cols: u16, rows: u16, aspect: f32) -> Vec<Paint> {
    use crate::plot::sdf;
    let mut c = crate::plot::Canvas::new(cols, rows, aspect);
    let a = c.row_units();
    let (cx, cy) = (f32::from(cols) / 2.0, (f32::from(mark_row(rows)) + 1.6) * a);
    let r = 1.3 * a;
    let accent = crate::palette::accent();
    let bbox = (cx - r * 1.2, cy - r * 1.2, r * 2.4, r * 2.4);
    c.fill_sdf(bbox, accent, 0.9, |x, y| {
        sdf::ring((x, y), (cx, cy), r, 0.07 * a)
    });
    let (p, q, s) = (
        (cx - 0.45 * r, cy + 0.02 * r),
        (cx - 0.12 * r, cy + 0.34 * r),
        (cx + 0.48 * r, cy - 0.3 * r),
    );
    c.fill_sdf(bbox, accent, 0.9, |x, y| {
        sdf::capsule((x, y), p, q, 0.08 * a).min(sdf::capsule((x, y), q, s, 0.08 * a))
    });
    c.paint()
}

/// Rows in outline while the first page comes, where real ones will sit: a
/// bar where the names go, one for the time, and one for the subject, of
/// varied lengths.
fn skeleton(cols: u16, rows: u16, two: bool) -> Vec<Paint> {
    const NAMES: [f32; 6] = [14.0, 9.0, 18.0, 11.0, 16.0, 8.0];
    const SUBJECT: [f32; 6] = [0.6, 0.45, 0.7, 0.5, 0.65, 0.4];
    let muted = crew_theme::theme().text_muted;
    let bar = |x: f32, y: f32, w: f32| Paint {
        alpha: 0.16,
        ..Paint::solid(x, y + 0.3, w.max(0.0), 0.4, muted)
    };
    let lines = if two { 2 } else { 1 };
    let (step, end) = (
        lines + super::layout::gap(lines),
        f32::from(cols.saturating_sub(2)),
    );
    let x0 = super::row::TEXT as f32;
    let room = end - x0 - 4.0;
    let top = super::layout::TOP;
    let mut out = Vec::new();
    for k in 0..usize::from((rows.saturating_sub(1 + top) + step - lines) / step) {
        let y = f32::from(top + k as u16 * step);
        let names = NAMES[k % 6].min(room / 3.0);
        out.push(bar(x0, y, names));
        out.push(bar(end - 3.0, y, 3.0));
        let (sx, sy) = match two {
            true => (x0, y + 1.0),
            false => (x0 + room / 3.0 + 2.0, y),
        };
        out.push(bar(sx, sy, (end - 4.0 - sx) * SUBJECT[k % 6]));
    }
    out
}

/// A failed list in words, and the key that mends it: never the raw error.
pub(crate) fn failure(e: &str) -> (&'static str, &'static str) {
    known(e).unwrap_or(("Gmail wouldn\u{2019}t list the inbox", "R tries again"))
}

/// A failure crew can name, in words, and the key that mends it. Matched
/// without case: Gmail writes "Rate Limit" and "User-rate limit" both.
pub(crate) fn known(e: &str) -> Option<(&'static str, &'static str)> {
    let again = "R tries again";
    let sign = "/inbox signout, then Enter, signs in again";
    let low = e.to_ascii_lowercase();
    let has = |s: &str| low.contains(s);
    Some(match () {
        // The token's renewal is the first call to fail when offline.
        _ if has("could not reach gmail") || has("could not reach google") => {
            ("Gmail can\u{2019}t be reached from here", again)
        }
        _ if has("insufficient authentication scopes") => {
            ("this sign-in can\u{2019}t read your mail", sign)
        }
        _ if has("has not been used") || has("is disabled") || has("accessnotconfigured") => (
            "the Gmail API is off in your Google project",
            "turn it on (setup step 2), then R",
        ),
        _ if has("invalid_client") => (
            "Google no longer knows your OAuth client",
            "/inbox signout, then set up a new one",
        ),
        _ if has("401") || has("refused the sign-in") => {
            ("Gmail turned down crew\u{2019}s sign-in", sign)
        }
        _ if has(" 429") || has("rate limit") || has("quota") => {
            ("Gmail asked crew to slow down", "R in a minute")
        }
        _ if has("gmail 5") => ("Gmail is having trouble", again),
        _ => return None,
    })
}

#[cfg(test)]
#[path = "states_tests.rs"]
mod tests;
