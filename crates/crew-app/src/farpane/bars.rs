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
    let line = format!("{label}{}▏", prompt.input);
    Paragraph::new(Line::from(Span::styled(
        line,
        Style::new().fg(bar_fg).bg(bar_bg),
    )))
    .style(Style::new().bg(bar_bg))
    .render(area, buf);
}
