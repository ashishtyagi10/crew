//! A picker row's label, fitted to its card. A path (a row with no
//! description, a `/` and no space: the `@` attach and `/view` pickers) keeps its
//! tail, where the file is named, cut on a separator as the cwd legend is
//! (`cwdshow::fit_legend`): clipped at the end, two files in a narrow tile
//! both read `@crates/crew-app/src/viewpane/render_…`. Anything else keeps
//! its start. A fuzzy match's marks stay on the letters they matched.
use ratatui::style::{Color, Style};
use ratatui::text::Span;

use crate::suggest::MenuItem;

/// The label's spans in at most `avail` columns, one per character.
pub(crate) fn spans(item: &MenuItem, avail: usize, fg: Color) -> Vec<Span<'static>> {
    let hit = crate::cmdrow::hit_style(fg);
    shown(item, avail)
        .into_iter()
        .map(|(c, at)| {
            let marked = at.is_some_and(|i| item.hit.contains(&i));
            Span::styled(
                c.to_string(),
                if marked { hit } else { Style::new().fg(fg) },
            )
        })
        .collect()
}

/// Each character shown, with its index in the label (`None` for the `…`).
fn shown(item: &MenuItem, avail: usize) -> Vec<(char, Option<usize>)> {
    let label = item.label.as_str();
    let n = label.chars().count();
    // No space: a chat-find or history row ("user: @scout what changed in
    // plot/…") has a `/` too, and cut from its front it lost the words.
    let path = label.contains('/') && !label.contains(char::is_whitespace);
    if !(item.desc.is_empty() && path && n > avail) {
        let fit = crate::chatwidth::clip_w(label, avail);
        return fit.chars().enumerate().map(|(i, c)| (c, Some(i))).collect();
    }
    let lead = usize::from(label.starts_with('@'));
    let fit = crate::cwdshow::fit_legend(&label[lead..], avail.saturating_sub(lead));
    let tail = fit.chars().count() - 1;
    let head = label.chars().take(lead).map(|c| (c, Some(0)));
    let rest = (fit.chars().skip(1).enumerate()).map(|(k, c)| (c, Some(n - tail + k)));
    head.chain(std::iter::once(('\u{2026}', None)))
        .chain(rest)
        .collect()
}

#[cfg(test)]
#[path = "cmdlabel_tests.rs"]
mod tests;
