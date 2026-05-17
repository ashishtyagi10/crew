use crate::theme::Theme;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Build a breadcrumb-style title Line for the panel border.
pub(super) fn build_breadcrumb_title<'a>(
    root: &std::path::Path,
    max_width: u16,
    is_active: bool,
    theme: &Theme,
) -> Line<'a> {
    use std::path::Component;

    let sep_style = Style::default()
        .fg(Color::Rgb(100, 100, 110))
        .bg(theme.panel_bg);

    let segment_style = if is_active {
        Style::default()
            .fg(theme.panel_header_fg)
            .bg(theme.panel_bg)
    } else {
        Style::default()
            .fg(Color::Rgb(140, 140, 150))
            .bg(theme.panel_bg)
    };

    let last_style = if is_active {
        Style::default()
            .fg(theme.panel_header_fg)
            .bg(theme.panel_bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.panel_header_fg)
            .bg(theme.panel_bg)
    };

    let components: Vec<String> = root
        .components()
        .filter_map(|c| match c {
            Component::RootDir => Some("/".to_string()),
            Component::Normal(s) => Some(s.to_string_lossy().to_string()),
            Component::Prefix(p) => Some(p.as_os_str().to_string_lossy().to_string()),
            _ => None,
        })
        .collect();

    if components.is_empty() {
        return Line::from(Span::styled(" / ", last_style));
    }

    let mut spans: Vec<Span<'a>> = Vec::new();
    spans.push(Span::styled(" ", sep_style));

    let total_len: usize = components.iter().map(|s| s.len()).sum::<usize>()
        + components.len().saturating_sub(1) * 3 // " ▸ " separators
        + 2; // leading/trailing space

    let max = max_width as usize;
    if total_len <= max {
        for (i, comp) in components.iter().enumerate() {
            let style = if i == components.len() - 1 {
                last_style
            } else {
                segment_style
            };
            spans.push(Span::styled(comp.clone(), style));
            if i < components.len() - 1 && comp != "/" {
                spans.push(Span::styled(" ▸ ", sep_style));
            }
        }
    } else {
        if let Some(first) = components.first() {
            spans.push(Span::styled(first.clone(), segment_style));
            if first != "/" {
                spans.push(Span::styled(" ▸ ", sep_style));
            }
        }
        spans.push(Span::styled("… ▸ ", sep_style));
        let tail_count = 2.min(components.len().saturating_sub(1));
        let tail_start = components.len().saturating_sub(tail_count);
        for i in tail_start..components.len() {
            let style = if i == components.len() - 1 {
                last_style
            } else {
                segment_style
            };
            spans.push(Span::styled(components[i].clone(), style));
            if i < components.len() - 1 {
                spans.push(Span::styled(" ▸ ", sep_style));
            }
        }
    }

    spans.push(Span::styled(" ", sep_style));
    Line::from(spans)
}

/// Given a click at column `x` within a panel at `panel_rect`, determine which
/// breadcrumb path segment was clicked. Returns the full path up to that segment.
pub fn breadcrumb_path_at_click(
    root: &std::path::Path,
    panel_rect: Rect,
    click_x: u16,
) -> Option<std::path::PathBuf> {
    use std::path::Component;

    let title_start = panel_rect.x + 2;
    if click_x < title_start {
        return None;
    }
    let offset = (click_x - title_start) as usize;

    let components: Vec<String> = root
        .components()
        .filter_map(|c| match c {
            Component::RootDir => Some("/".to_string()),
            Component::Normal(s) => Some(s.to_string_lossy().to_string()),
            Component::Prefix(p) => Some(p.as_os_str().to_string_lossy().to_string()),
            _ => None,
        })
        .collect();

    let mut running_offset = 0usize;
    let mut accumulated_path = std::path::PathBuf::new();

    for (i, comp) in components.iter().enumerate() {
        if comp == "/" {
            accumulated_path.push("/");
            running_offset += 1;
        } else {
            accumulated_path.push(comp);
            let seg_end = running_offset + comp.len();
            if offset < seg_end {
                return Some(accumulated_path);
            }
            running_offset = seg_end;
            if i < components.len() - 1 {
                running_offset += 3;
            }
        }

        if offset < running_offset {
            return Some(accumulated_path);
        }
    }

    None
}
