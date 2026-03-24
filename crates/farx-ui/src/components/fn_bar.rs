use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::Theme;

/// The function-key labels shown at the bottom of the screen, in classic FAR
/// Manager order.
const FN_ITEMS: &[(u8, &str)] = &[
    (1, "Help"),
    (2, "Menu"),
    (3, "View"),
    (4, "Edit"),
    (5, "Copy"),
    (6, "Move"),
    (7, "MkDir"),
    (8, "Del"),
    (9, "Menu"),
    (10, "Quit"),
];

/// Render the FAR Manager function key bar as a single line.
///
/// Each item renders the key number in the `fn_bar_key` style (black on cyan)
/// and the label in `fn_bar_label` style (cyan on black).
pub fn render_fn_bar(frame: &mut Frame, area: Rect, theme: &Theme) {
    let total_width = area.width as usize;
    let item_count = FN_ITEMS.len();
    // Each slot gets an equal share of the width
    let slot_width = if item_count > 0 {
        total_width / item_count
    } else {
        0
    };

    let mut spans: Vec<Span<'_>> = Vec::with_capacity(item_count * 2);

    for (i, &(num, label)) in FN_ITEMS.iter().enumerate() {
        let num_str = format!("{num}");
        let num_len = num_str.len();

        // Determine label width: fill the slot minus the number width
        let label_width = if i < item_count - 1 {
            slot_width.saturating_sub(num_len)
        } else {
            // Last item gets the remaining width
            total_width.saturating_sub(slot_width * (item_count - 1) + num_len)
        };

        // Pad or truncate the label to fill its slot
        let padded_label = if label.len() >= label_width {
            label[..label_width].to_string()
        } else {
            format!("{label}{}", " ".repeat(label_width - label.len()))
        };

        spans.push(Span::styled(num_str, theme.fn_bar_key));
        spans.push(Span::styled(padded_label, theme.fn_bar_label));
    }

    let line = Line::from(spans);
    let bar = Paragraph::new(line);
    frame.render_widget(bar, area);
}
