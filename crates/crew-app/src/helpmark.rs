//! Where a `/keys` filter matched, marked the way the palette marks it.
//!
//! Typing `pane` into `/keys` narrowed the list to the rows that say it and
//! then left you to find the word in each of them. The palette and the find
//! pop-up wash every matched character ([`crate::cmdrow::hit_style`]); this
//! is that wash on the panel's keys and descriptions.
use ratatui::style::Style;
use ratatui::text::Span;

/// `text` as spans in `base`, every case-insensitive occurrence of `needle`
/// washed. An empty needle marks nothing.
pub(crate) fn marked(text: &str, needle: &str, base: Style) -> Vec<Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    let hits = hits(&chars, needle);
    let hit = crate::cmdrow::hit_style(base.fg.unwrap_or(ratatui::style::Color::Reset));
    let mut out: Vec<Span<'static>> = Vec::new();
    let mut run = String::new();
    let mut in_hit = false;
    for (i, &c) in chars.iter().enumerate() {
        if hits[i] != in_hit && !run.is_empty() {
            let style = if in_hit { hit } else { base };
            out.push(Span::styled(std::mem::take(&mut run), style));
        }
        in_hit = hits[i];
        run.push(c);
    }
    if !run.is_empty() {
        out.push(Span::styled(run, if in_hit { hit } else { base }));
    }
    out
}

/// Which of `chars` fall inside a match of `needle`, compared char by char
/// in lower case (so a non-ASCII letter never shifts a byte offset).
fn hits(chars: &[char], needle: &str) -> Vec<bool> {
    let lower = |c: char| c.to_lowercase().next().unwrap_or(c);
    let n: Vec<char> = needle.chars().map(lower).collect();
    let mut out = vec![false; chars.len()];
    if n.is_empty() || n.len() > chars.len() {
        return out;
    }
    for at in 0..=chars.len() - n.len() {
        if chars[at..at + n.len()]
            .iter()
            .zip(&n)
            .all(|(&c, &m)| lower(c) == m)
        {
            out[at..at + n.len()].iter_mut().for_each(|h| *h = true);
        }
    }
    out
}

#[cfg(test)]
#[path = "helpmark_tests.rs"]
mod tests;
