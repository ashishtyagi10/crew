//! Keyboard-shortcuts overlay (`/keys`): a centered rounded popup listing the
//! bindings, rendered with ratatui and dismissed by any key press.
use crew_render::CellView;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, List, ListItem, Widget};

use crate::palette::accent_color;

/// `(keys, description)` rows shown in the overlay.
const BINDINGS: &[(&str, &str)] = &[
    ("Ctrl+Tab / Ctrl+Shift+Tab", "Next / previous pane"),
    ("Cmd+1 … 9", "Jump to pane N"),
    ("Cmd+A", "Jump to next active pane"),
    ("Cmd+{ / Cmd+}", "Move pane left / right"),
    ("Cmd+I", "Focus the input bar"),
    ("Cmd+T", "New shell pane"),
    ("Cmd+, / Cmd+J", "Settings / chat pane"),
    ("Cmd+G", "Toggle sidebar"),
    ("Cmd+Z", "Zoom focused pane"),
    ("Cmd+S", "Broadcast to all panes"),
    ("Cmd+= / Cmd+- / Cmd+0", "Font size + / - / reset"),
    ("Cmd+C / Cmd+V", "Copy screen / paste"),
    (
        "Cmd+Click",
        "open URL/file/dir · copy a code block in an agent pane",
    ),
    ("Cmd+W / Cmd+M", "Close pane / maximize"),
    ("Cmd+K", "Clear focused pane scrollback"),
    (
        "Ctrl+Shift+L",
        "Cycle themes (dark \u{2192} light \u{2192} crt)",
    ),
    ("Ctrl+Shift+M", "Chat: markdown preview \u{2194} raw source"),
    ("Ctrl+O", "Chat: compact transcript view"),
    ("Shift+PageUp / Shift+PageDown", "Scroll focused pane"),
    ("Shift+Home / Shift+End", "Scroll to top / bottom"),
    ("/ (in input)", "Command palette"),
    (
        "! · * · ? · ?? (in input)",
        "New pane / broadcast / ask ai a command / explain this pane",
    ),
    ("Cmd+Q", "Quit"),
];

/// Keys that mean something specific inside an agent pane. The overlay
/// claimed to list "the bindings" and had none of these — including the two
/// that answer a drafted plan, added in v0.6.46.
const CHAT_BINDINGS: &[(&str, &str)] = &[
    (
        "Enter",
        "Send · answers a pending plan when the composer is empty",
    ),
    (
        "Esc",
        "Discard a pending plan · interrupt a running turn · close",
    ),
    ("Shift+Enter", "Newline instead of sending"),
    ("Tab", "Complete the leading @agent or /construct"),
    (
        "Up / Down",
        "Recall a prompt you already sent · navigate an open popup",
    ),
    (
        "Tab / Right",
        "Take the suggested rest of a prompt you sent before",
    ),
    (
        "@ · # (in composer)",
        "Attach an agent, skill or file · remember a note",
    ),
    (
        "@file:120-180 (in composer)",
        "Attach just those lines, instead of the whole file",
    ),
    (
        "@a+b (in composer)",
        "Fan one task out to both agents, in parallel",
    ),
];

/// Width of the key column. Every description starts here.
const KEY_COL: usize = 26;

/// Preferred overlay size in cells: both binding tables, a spacer and a
/// heading between them, plus borders/title/hint.
///
/// It used to include every slash command as well, which made 58 rows —
/// taller than a default window, so the commands were silently cut off at
/// the bottom. They are not here any more: the composer's palette has listed
/// them since v0.6.52, grouped and filterable, which is strictly better than
/// an unscrollable overflowing column. `/keys` is for keys.
///
/// The WIDTH is measured the same way, and was not: it was the constant 58,
/// which left 30 columns for a description, and eight rows had outgrown that.
/// ratatui clips a `Line` without a word of complaint, so `Esc` read "Discard
/// a pending plan · inter" — losing both the interrupt and the close — and
/// `@a+b` lost "in parallel", which is the entire point of the binding. The
/// same lesson as the footer in v0.6.57: a truncated instruction teaches the
/// half that fits, and nobody can see that the rest existed.
pub fn size() -> (u16, u16) {
    let rows = BINDINGS.len() + CHAT_BINDINGS.len() + 2 + 4;
    let widest = BINDINGS
        .iter()
        .chain(CHAT_BINDINGS)
        .map(|(k, d)| KEY_COL.max(k.chars().count() + 1) + d.chars().count())
        .max()
        .unwrap_or(KEY_COL);
    ((widest + 2) as u16, rows as u16)
}

/// Render the help overlay into a `cols × rows` grid.
pub fn help_cells(cols: u16, rows: u16) -> Vec<CellView> {
    if cols < 12 || rows < 4 {
        return Vec::new();
    }
    let t = crew_theme::theme();
    let text_col = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let dim_col = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let panel_col = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let mut buf = Buffer::empty(Rect::new(0, 0, cols, rows));
    let item = |left: &str, right: &str| {
        ListItem::new(Line::from(vec![
            Span::styled(
                format!("{left:<width$}", width = KEY_COL),
                Style::new().fg(accent_color()),
            ),
            Span::styled(right.to_string(), Style::new().fg(text_col)),
        ]))
    };
    let mut items: Vec<ListItem> = BINDINGS.iter().map(|(k, d)| item(k, d)).collect();
    items.push(ListItem::new(Line::from(""))); // spacer
    items.push(ListItem::new(Line::from(Span::styled(
        "in an agent pane",
        Style::new().fg(dim_col),
    ))));
    for (k, d) in CHAT_BINDINGS {
        items.push(item(k, d));
    }
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(accent_color()))
        .style(Style::new().bg(panel_col))
        .title(Span::styled(
            format!(" keys · crew v{} ", env!("CARGO_PKG_VERSION")),
            Style::new().fg(accent_color()),
        ));
    let inner = block.inner(buf.area);
    block.render(buf.area, &mut buf);
    List::new(items).render(inner, &mut buf);
    // dismissal hint on the bottom border
    let hint = " any key to close ";
    let hint_col = cols.saturating_sub(hint.len() as u16 + 2);
    for (i, ch) in hint.chars().enumerate() {
        let col = hint_col + i as u16;
        if let Some(cell) = buf.cell_mut((col, rows - 1)) {
            cell.set_char(ch).set_fg(dim_col);
        }
    }
    crate::tui::to_cells_opaque(&buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The overlay rebuilt row by row, which is how it is actually read.
    /// `to_cells_opaque` fills every cell — blanks included — so scanning a
    /// flat character stream matches text that never appears on one line.
    fn rows_of(cells: &[CellView]) -> Vec<String> {
        let mut rows: std::collections::BTreeMap<u16, String> = Default::default();
        for c in cells {
            rows.entry(c.row).or_default().push(c.c);
        }
        rows.into_values().collect()
    }

    /// Letters and digits only, on both sides. The buffer fills blank cells
    /// with a glyph that is neither a space nor whitespace, so neither a raw
    /// nor a whitespace-squeezed comparison finds text that spans words.
    fn letters(s: &str) -> String {
        s.chars().filter(char::is_ascii_alphanumeric).collect()
    }

    fn shows(cells: &[CellView], needle: &str) -> bool {
        let want = letters(needle);
        rows_of(cells).iter().any(|r| letters(r).contains(&want))
    }

    #[test]
    fn renders_bindings_with_border() {
        let (w, h) = size();
        let cells = help_cells(w, h);
        assert!(cells.iter().any(|c| c.c == '╭'));
        assert!(shows(&cells, "Ctrl+Tab"), "app bindings listed");
        assert!(shows(&cells, "in an agent pane"), "chat section listed");
    }

    /// The keys added with the plan card must be findable. A binding nobody
    /// can discover is a binding nobody has.
    #[test]
    fn the_chat_pane_keys_are_documented() {
        let (w, h) = size();
        let cells = help_cells(w, h);
        for needle in [
            "Enter",
            "Esc",
            "pending plan",
            "Shift+Enter",
            "Tab",
            "Recall a prompt",
            "Attach just those lines",
        ] {
            assert!(shows(&cells, needle), "missing {needle}");
        }
    }

    /// The overlay must FIT a normal window. At 58 rows — every slash command
    /// included — it did not, and ratatui cut the bottom off in silence.
    #[test]
    fn the_overlay_fits_a_default_window() {
        let (w, h) = size();
        // 800 logical px over a ~20px cell is about 40 rows; the default
        // window is 1200x800 (`handler::resumed`), and a cell is ~9px wide.
        assert!(h <= 40, "overlay is {h} rows and will be truncated");
        assert!(w <= 130, "overlay is {w} cols and will be truncated");
    }

    /// EVERY description in full, not just the ones that happened to fit.
    /// `the_chat_pane_keys_are_documented` passed throughout the eight rows
    /// that were being clipped, because each of its needles sat inside the
    /// first 30 columns — an assertion on a prefix cannot see a missing tail.
    #[test]
    fn no_description_is_clipped() {
        let (w, h) = size();
        let cells = help_cells(w, h);
        for (k, d) in BINDINGS.iter().chain(CHAT_BINDINGS) {
            assert!(shows(&cells, d), "clipped description for {k}: {d}");
        }
    }

    #[test]
    fn tiny_renders_nothing() {
        assert!(help_cells(8, 3).is_empty());
    }

    /// Chords the manual carries that the overlay deliberately does not: pane
    /// cycling has a documented second spelling, and the overlay lists the
    /// primary one. Declared, so a real omission stays visible.
    const OVERLAY_OMITS: &[&str] = &["Cmd+]", "Cmd+["];

    /// The manual says `/keys` shows "this list in-app". It did not:
    /// **Ctrl+Shift+L** and **Ctrl+Shift+M** were documented and missing from
    /// the overlay, and **Ctrl+O** was implemented, tested, and in neither list
    /// — a working binding no user could discover from anywhere. Two lists,
    /// nothing comparing them, exactly as before.
    #[test]
    fn the_overlay_and_the_manual_list_the_same_chords() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/CREW.md");
        let Ok(docs) = std::fs::read_to_string(&path) else {
            return; // docs not shipped in this build context
        };
        for (keys, _) in BINDINGS.iter().chain(CHAT_BINDINGS) {
            for chord in keys.split(" / ") {
                let chord = chord.trim();
                // Only modifier chords. Bare keys (`Enter`, `Right`, `Tab`) are
                // the composer's own editing verbs, and composer syntax
                // (`@a+b`) merely contains a `+` — a chord is where drift
                // hides, and a chord starts with a modifier.
                if !is_chord(chord) {
                    continue;
                }
                assert!(
                    docs.contains(chord),
                    "the overlay shows `{chord}`, which docs/CREW.md never mentions"
                );
            }
        }
        for chord in documented_chords(&docs) {
            let listed = BINDINGS
                .iter()
                .chain(CHAT_BINDINGS)
                .any(|(k, _)| k.contains(&chord));
            assert!(
                listed || OVERLAY_OMITS.contains(&chord.as_str()),
                "docs/CREW.md documents `{chord}`, which /keys never shows"
            );
        }
    }

    /// A modifier chord, as opposed to a bare key or composer syntax.
    fn is_chord(s: &str) -> bool {
        ["Cmd+", "Ctrl+", "Alt+", "Shift+"]
            .iter()
            .any(|m| s.starts_with(m))
    }

    /// The bolded chords in the manual's shortcuts table.
    fn documented_chords(docs: &str) -> Vec<String> {
        let Some(start) = docs.find("## Keyboard shortcuts") else {
            return Vec::new();
        };
        let table = &docs[start..];
        let end = table[3..]
            .find("\n## ")
            .map(|i| i + 3)
            .unwrap_or(table.len());
        let mut out = Vec::new();
        let mut rest = &table[..end];
        while let Some(i) = rest.find("**") {
            rest = &rest[i + 2..];
            let Some(j) = rest.find("**") else { break };
            let span = rest[..j].trim().to_string();
            rest = &rest[j + 2..];
            if is_chord(&span) {
                out.push(span);
            }
        }
        out.sort();
        out.dedup();
        out
    }
}
