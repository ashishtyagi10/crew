//! What a document window's frame says on its top border: the file, whether
//! it is on disk, where the caret is in it, and how far through it you are.
//! A free function over the view, so it can be read without a window.
use crate::viewpane::ViewPane;
use crew_term::GridSize;

/// The legend, in order of what matters most while you are typing.
pub(crate) fn legend(
    view: &ViewPane,
    grid: GridSize,
    hint: Option<&str>,
    field: Option<String>,
) -> String {
    let name = view
        .path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| view.path.to_string_lossy().into_owned());
    // An editor owes you a standing answer to "is what I typed on disk".
    let name = match view.dirty {
        true => format!("{name} \u{25cf}"),
        false => name,
    };
    if let Some(h) = hint {
        return format!("{name} \u{00b7} {h}");
    }
    // Typing a URL takes the line the URL was already shown on.
    if let Some(field) = field {
        return format!("{name} \u{00b7} {field}");
    }
    let mut parts = vec![name];
    // The file's own line and column — what you would tell someone else, or
    // type into another editor's go-to-line.
    if let Some((line, col)) = view.caret_line_col() {
        parts.push(format!("{line}:{col}"));
    }
    // A link's target is invisible in a render; while the cursor is inside
    // one, the frame is where it says so.
    if let Some(url) = view.caret_link(grid.cols) {
        parts.push(format!("\u{2192} {url}"));
        return parts.join(" \u{00b7} ");
    }
    let (back, total) = view.position(grid.cols, grid.rows);
    if total > 0 && total > usize::from(grid.rows) {
        // The same reading the pane card's thumb is written from, spelled
        // out here because a window has no card border to draw a thumb on.
        let seen = total.saturating_sub(back);
        let pct = (seen * 100 / total.max(1)).min(100);
        parts.push(format!("{pct}%"));
    }
    parts.join(" \u{00b7} ")
}

#[cfg(test)]
#[path = "legend_tests.rs"]
mod tests;
