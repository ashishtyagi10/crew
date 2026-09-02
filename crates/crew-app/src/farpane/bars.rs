//! The Far pane's bottom rows: the command line, the function-key bar, and
//! the make-folder prompt that takes the latter over. Split from `render.rs`
//! (child module — shares its parent's private helpers).
use super::*;

/// Function-key labels shown along the bottom bar (classic Far layout).
const FKEYS: [(&str, &str); 8] = [
    ("1", "Help"),
    ("3", "View"),
    ("4", "Edit"),
    ("5", "Copy"),
    ("6", "RenMov"),
    ("7", "MkFold"),
    ("8", "Delete"),
    ("10", "Quit"),
];

/// The Far command line: `<cwd> $ <typed>▏`, the directory dimmed and the typed
/// command in the ink colour with a cursor bar. While a command runs, a dimmed
/// `⟳ <cmd>` note follows the prompt. Truncated from the left to fit.
#[allow(clippy::too_many_arguments)] // one bar, eight independent knobs
pub(super) fn command_bar(
    buf: &mut Buffer,
    area: Rect,
    folder: &str,
    cmdline: &str,
    ghost: Option<&str>,
    ask_hint: Option<&str>,
    suggested: bool,
    running: Option<&str>,
) {
    let t = crew_theme::theme();
    let bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let dim = Color::Rgb(t.text_muted.0, t.text_muted.1, t.text_muted.2);
    let ink = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    // A landed `!` suggestion REPLACES the bar's normal styling with the
    // same selected look the panel listing uses for its cursor row (ink on
    // an accent fill) — a highlighted, still-editable suggestion.
    let cmd_style = if suggested {
        Style::new().fg(bg).bg(accent_color())
    } else {
        Style::new().fg(ink).bg(bg)
    };
    let mut spans = vec![
        Span::styled(format!("{folder} "), Style::new().fg(dim).bg(bg)),
        Span::styled("$ ", Style::new().fg(accent_color()).bg(bg)),
        Span::styled(format!("{cmdline}▏"), cmd_style),
    ];
    if let Some(g) = ghost {
        spans.push(Span::styled(g.to_string(), Style::new().fg(dim).bg(bg)));
    }
    if let Some(hint) = ask_hint {
        spans.push(Span::styled(
            format!("  {hint}"),
            Style::new().fg(dim).bg(bg),
        ));
    }
    if let Some(cmd) = running {
        spans.push(Span::styled(
            format!("  \u{27f3} {cmd}"),
            Style::new().fg(dim).bg(bg),
        ));
    }
    Paragraph::new(Line::from(spans))
        .style(Style::new().bg(bg))
        .render(area, buf);
}

/// The panel's selected entry, full and untruncated — `"name/"` for folders,
/// `"name · size"` for files. `None` on an empty listing.
pub(super) fn selected_label(panel: &Panel) -> Option<String> {
    let e = panel.entries.get(panel.sel)?;
    Some(if e.is_dir {
        format!("{}/", e.name)
    } else {
        format!("{} \u{b7} {}", e.name, fmt_size(e.size))
    })
}

/// The mini-status row above the command line: the active panel's selected
/// entry in full (the listing truncates long names; this row is the readable
/// copy). Blank on an empty listing — the row is always present, so the
/// layout never jumps.
pub(super) fn status_bar(buf: &mut Buffer, area: Rect, label: Option<&str>) {
    let t = crew_theme::theme();
    let bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let ink = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let line = match label {
        Some(l) => Line::from(Span::styled(
            ellipsize_keeping_suffix(l, area.width as usize),
            Style::new().fg(ink).bg(bg),
        )),
        None => Line::default(),
    };
    Paragraph::new(line)
        .style(Style::new().bg(bg))
        .render(area, buf);
}

/// Fit `label` into `width` columns: the ` · size` suffix stays intact and
/// the name ellipsizes (the same rule the listing rows use). A label without
/// the separator (a directory) ellipsizes plainly from the end.
fn ellipsize_keeping_suffix(label: &str, width: usize) -> String {
    if label.chars().count() <= width || width == 0 {
        return label.to_string();
    }
    let (name, suffix) = match label.rfind(" \u{b7} ") {
        Some(i) => label.split_at(i),
        None => (label, ""),
    };
    let keep = width.saturating_sub(suffix.chars().count() + 1);
    let head: String = name.chars().take(keep).collect();
    format!("{head}\u{2026}{suffix}")
}

/// blank cells, so a bg-only space would never reach the GPU.
pub(super) fn function_bar(buf: &mut Buffer, area: Rect) {
    let t = crew_theme::theme();
    let bar_bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let cap = Style::new().fg(accent_color());
    let mut spans = Vec::new();
    for (k, label) in FKEYS {
        spans.push(Span::styled(format!("F{k} "), cap));
        spans.push(Span::styled("\u{2590}", cap)); // ▐ left pill edge
        spans.push(Span::styled(
            label,
            Style::new().fg(bar_bg).bg(accent_color()),
        ));
        spans.push(Span::styled("\u{258c}", cap)); // ▌ right pill edge
    }
    Paragraph::new(Line::from(spans))
        .style(Style::new().bg(bar_bg))
        .render(area, buf);
}

/// The bottom-row text prompt (F7 make-folder), replacing the function bar.
pub(super) fn prompt_bar(buf: &mut Buffer, area: Rect, prompt: &super::super::Prompt) {
    let t = crew_theme::theme();
    let bar_bg = Color::Rgb(t.page_bg.0, t.page_bg.1, t.page_bg.2);
    let bar_fg = Color::Rgb(t.ink.0, t.ink.1, t.ink.2);
    let label = match prompt.kind {
        super::super::PromptKind::MkDir => "Create folder: ",
    };
    let line = prompt_text(label, &prompt.input, area.width as usize);
    Paragraph::new(Line::from(Span::styled(
        line,
        Style::new().fg(bar_fg).bg(bar_bg),
    )))
    .style(Style::new().bg(bar_bg))
    .render(area, buf);
}

/// The prompt's row: the label, then the input with its caret. A name longer
/// than the row keeps its END — the part being typed — behind a `…`, so the
/// caret never leaves the screen. It did: on a 2×2 tile a folder name past
/// forty characters was typed blind.
pub(super) fn prompt_text(label: &str, input: &str, width: usize) -> String {
    use crate::chatwidth::{char_w, str_w};
    const CARET: char = '\u{258f}';
    let room = width.saturating_sub(str_w(label) + 1);
    if str_w(input) <= room {
        return format!("{label}{input}{CARET}");
    }
    // Everything after `…` that fits, taken from the end.
    let mut w = 0;
    let mut tail: Vec<char> = Vec::new();
    for c in input.chars().rev() {
        if w + char_w(c) > room.saturating_sub(1) {
            break;
        }
        w += char_w(c);
        tail.push(c);
    }
    let tail: String = tail.into_iter().rev().collect();
    format!("{label}\u{2026}{tail}{CARET}")
}

#[cfg(test)]
#[path = "bars_tests.rs"]
mod tests;
