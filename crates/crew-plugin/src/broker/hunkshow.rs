//! A file change put into words a person can judge it by: numbered hunks, clipped.
//!
//! WHY clipped hard: the question rides a chat pane, and an approval for a 900-line rewrite that
//! scrolls the whole transcript away helps nobody. Forty lines is enough to see WHAT is being
//! done; the marker says how much more there is.
use super::hunkdiff::Hunk;

/// Diff rows shown before `… N more lines`.
const MAX_ROWS: usize = 40;
/// Characters of a changed line shown.
const MAX_WIDTH: usize = 120;

/// The hunk's heading, numbered as the person will type it. `at` is its first OLD line, 1-based.
pub(crate) fn header(k: usize, h: &Hunk) -> String {
    format!("[{}] @@ {} @@", k + 1, h.at + 1)
}

fn row(sign: char, line: &str) -> String {
    let text = line.trim_end_matches(['\n', '\r']);
    let text = match text.chars().count() > MAX_WIDTH {
        true => format!(
            "{}\u{2026}",
            text.chars().take(MAX_WIDTH).collect::<String>()
        ),
        false => text.to_string(),
    };
    format!("{sign} {text}")
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// What goes into the question: the words after the dash (`2 changes · `) and the rows below it.
/// `old` is `None` for a file that does not exist yet.
pub(crate) fn show(old: Option<&str>, new: &str, hunks: &[Hunk]) -> (String, String) {
    let mut rows = Vec::new();
    let lead = match old {
        None => {
            let lines: Vec<&str> = new.split_inclusive('\n').collect();
            rows.extend(lines.iter().map(|l| row('+', l)));
            format!("new file, {} \u{b7} ", plural(lines.len(), "line", "lines"))
        }
        Some(_) if hunks.is_empty() => "no changes \u{b7} ".to_string(),
        Some(_) => {
            for (k, h) in hunks.iter().enumerate() {
                rows.push(header(k, h));
                rows.extend(h.old.iter().map(|l| row('-', l)));
                rows.extend(h.new.iter().map(|l| row('+', l)));
            }
            format!("{} \u{b7} ", plural(hunks.len(), "change", "changes"))
        }
    };
    let more = rows.len().saturating_sub(MAX_ROWS);
    rows.truncate(MAX_ROWS);
    if more > 0 {
        rows.push(format!(
            "\u{2026} {}",
            plural(more, "more line", "more lines")
        ));
    }
    // Picks only when every hunk is on screen: choosing a change you were never shown is
    // exactly what this question exists to prevent.
    rows.push(match hunks.len() > 1 && old.is_some() && more == 0 {
        true => super::hunkpicks::PICK_KEYS.to_string(),
        false => "Enter allows \u{b7} Esc refuses".to_string(),
    });
    (lead, format!("\n{}", rows.join("\n")))
}

#[cfg(test)]
#[path = "hunkshow_tests.rs"]
mod tests;
