//! A palette row that is not a choice: a section title, or a note about the
//! rows around it (`@a+b  fans the task out to both, in parallel`).
//!
//! A note with a description is laid out in the rows' columns. The `@a+b`
//! hint was one string with two spaces in it, so its sentence started four
//! columns after the selector while every agent's role, right above it,
//! started at the shared description column: a ragged edge in the one row
//! that was explaining the others.
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::chatwidth::{clip_words, str_w};
use crate::suggest::MenuItem;

/// Columns of description worth a column of their own (`cmdrow`'s too).
const MIN_DESC: usize = 8;

/// `item` (a header) drawn in `avail` columns, dim. With a description, the
/// label is bold and the description starts at `desc_col`, where the
/// choices' descriptions start ([`crate::cmdrow::spans`]), cut on a word as
/// theirs are. A card too narrow for both columns says the whole note as one
/// line cut on a word.
pub(crate) fn line(item: &MenuItem, desc_col: usize, avail: usize, dim: Color) -> Line<'static> {
    let title = Style::new().fg(dim).add_modifier(Modifier::BOLD);
    let col = desc_col.max(str_w(&item.label) + 2);
    if item.desc.is_empty() || avail < col + MIN_DESC {
        // A card can be narrower than its section title, too — and a note
        // cut at a letter ended `· clear it …`, half an instruction; cut on
        // a word it drops the part it cannot say whole.
        let text = match item.desc.is_empty() {
            true => item.label.clone(),
            false => format!("{}  {}", item.label, item.desc),
        };
        return Line::from(Span::styled(clip_words(&text, avail), title));
    }
    Line::from(vec![
        Span::styled(item.label.clone(), title),
        Span::raw(" ".repeat(col - str_w(&item.label))),
        Span::styled(clip_words(&item.desc, avail - col), Style::new().fg(dim)),
    ])
}
