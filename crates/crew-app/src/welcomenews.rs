//! The release headline on the welcome screen, folded onto as many rows as
//! the page can spare.
//!
//! Split out of [`crate::welcometext`] for the 200-line cap. The headline
//! used to be one line clipped at a word: on a tall narrow tile (a quarter
//! split, a column beside an editor) it read `new in 0.29.62 · the inbox's
//! button…` over twenty empty rows. A headline is written to be read whole,
//! so where there are rows to hold it, it wraps instead of being cut.
use crate::welcometext::{fit_head, lowered, plain, whats_new, MARGIN};

/// The most rows the headline may take. Headlines are kept under ~70
/// characters, which fold onto three rows at 40 columns; a fourth is slack,
/// and past it the news would outweigh the welcome it sits under.
const MAX_ROWS: u16 = 4;

/// The newest changelog entry as `(lead, headline)`: `new in X · ` and its
/// first bold sentence, markdown dropped, whitespace folded, lowered to the
/// screen's voice.
pub(crate) fn headline() -> Option<(String, String)> {
    let body = crate::appregister::CHANGELOG;
    let heading = body.find("\n## ")? + 4;
    let rest = &body[heading..];
    let bold = rest.find("**")? + 2;
    let end = rest[bold..].find("**")?;
    let head: String = plain(&rest[bold..bold + end])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let version = rest[..rest.find('\n')?].trim();
    let lead = format!("new in {version} \u{b7} ");
    Some((lead, lowered(head.trim_end_matches('.'))))
}

/// The news rows for a `cols`-wide welcome whose rain is `h` rows tall, in a
/// page of `rows`. One row (the clipped line) unless the page has spare rows
/// below the stack, in which case the headline wraps between words onto up
/// to [`MAX_ROWS`] of them; only what still does not fit is cut, with `…`.
/// Empty when the line itself has no room (`whats_new`).
pub(crate) fn news_lines(cols: u16, rows: u16, h: u16, restore: bool) -> Vec<String> {
    let Some(one) = whats_new(usize::from(cols)) else {
        return Vec::new();
    };
    // Rows under the rain: blank, tagline, hint, the news, then (restoring)
    // a blank and the offer — and the stamp keeps the last row.
    let fixed = h + 4 + if restore { 2 } else { 0 };
    let spare = rows.saturating_sub(fixed).min(MAX_ROWS);
    let (Some((lead, head)), true) = (headline(), spare > 1) else {
        return vec![one];
    };
    let text = format!("{lead}{head}");
    let width = usize::from(cols) - 2 * (MARGIN + 2);
    let lines = wrap(&text, width, spare);
    balanced(&text, width, lines)
}

/// The same rows at the narrowest width that still needs no more of them:
/// greedy folding left `for them` alone under a full row, and a centred
/// stack reads as one block only when its rows are near one length. A cut
/// headline is left as it is — every column there is holding words.
fn balanced(text: &str, width: usize, lines: Vec<String>) -> Vec<String> {
    if lines.len() < 2 || lines.join(" ") != text {
        return lines;
    }
    (width / 2..width)
        .map(|w| wrap(text, w, u16::MAX))
        .find(|l| l.len() == lines.len() && l.iter().all(|r| r.chars().count() <= width))
        .unwrap_or(lines)
}

/// `text` folded greedily between words into lines of at most `width`,
/// no more than `max` of them; the last line holds whatever is left, cut
/// the way the one-line headline is.
fn wrap(text: &str, width: usize, max: u16) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut words = text.split(' ').peekable();
    while words.peek().is_some() {
        if lines.len() + 1 == usize::from(max) {
            let rest: Vec<&str> = words.collect();
            lines.push(fit_head(&rest.join(" "), width));
            break;
        }
        let mut line = String::new();
        while let Some(w) = words.peek() {
            let n = line.chars().count() + w.chars().count() + usize::from(!line.is_empty());
            if !line.is_empty() && n > width {
                break;
            }
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(w);
            words.next();
        }
        lines.push(line);
    }
    lines
}

#[cfg(test)]
#[path = "welcomenews_tests.rs"]
mod tests;
