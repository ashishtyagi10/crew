//! Message-body layout for the card view: newline-aware prose wrapping plus
//! fenced ```code``` blocks rendered as a bordered card — `╭─ lang` header,
//! hard-wrapped verbatim lines on a subtly dimmed background, `╰─` footer.
use crate::chatcode::{parse_fences, Seg};
use crate::chatlayout::wrap_indices;

pub(crate) type Color = (u8, u8, u8);

/// One cell of a card line. `bg: None` means the pane's page background.
/// `link` carries the URL a markdown link span resolves to (Task 4+), so a
/// later click hit-test can recover it without re-parsing the message.
#[derive(Clone)]
pub(crate) struct CardCell {
    pub c: char,
    pub fg: Color,
    pub bold: bool,
    pub italic: bool,
    pub bg: Option<Color>,
    // Unread until Task 4 (markdown link spans) and Task 6 (click hit-test)
    // land; keep it here now since both consume this exact field.
    #[allow(dead_code)]
    pub link: Option<std::sync::Arc<str>>,
}

/// One rendered line of a message card.
pub(crate) type CardLine = Vec<CardCell>;

/// A cell on the page background.
pub(crate) fn plain(c: char, fg: Color, bold: bool) -> CardCell {
    CardCell {
        c,
        fg,
        bold,
        italic: false,
        bg: None,
        link: None,
    }
}

/// The code card's background: the page nudged toward the ink colour, so the
/// block reads as a card in every theme without a dedicated theme slot.
fn code_bg() -> Color {
    let t = crew_theme::theme();
    crate::anim::lerp_rgb(t.page_bg, t.ink, 0.08)
}

/// Lay out one message body: prose lines word-wrapped (respecting newlines),
/// code blocks bordered + dimmed. Lines are indented one column under the
/// card's `▍sender` header.
pub(crate) fn body_lines(text: &str, cols: usize, fg: Color) -> Vec<CardLine> {
    let muted = crew_theme::theme().text_muted;
    let width = cols.saturating_sub(1).max(1);
    let mut out: Vec<CardLine> = Vec::new();
    for seg in parse_fences(text) {
        match seg {
            Seg::Text(lines) => {
                for logical in lines {
                    let full: Vec<char> = logical.chars().collect();
                    for (s, e) in wrap_indices(&full, width) {
                        let mut line = vec![plain(' ', fg, false)];
                        line.extend(full[s..e].iter().map(|&c| plain(c, fg, false)));
                        out.push(line);
                    }
                }
            }
            Seg::Code { lang, lines } => {
                let label = if lang.is_empty() { "code" } else { &lang };
                out.push(rule(&format!("\u{256d}\u{2500} {label}"), width, muted)); // ╭─ lang
                let bg = Some(code_bg());
                for logical in lines {
                    // Hard chunking, not word wrap: code is copied verbatim,
                    // so no character (not even a break space) may be dropped.
                    let full: Vec<char> = logical.chars().collect();
                    let mut s = 0;
                    loop {
                        // Width-aware chunking: wide glyphs count two columns.
                        let e = crate::chatwidth::fit_end(&full, s, width);
                        let mut line = vec![plain(' ', fg, false)];
                        line.extend(full[s..e].iter().map(|&c| CardCell {
                            c,
                            fg,
                            bold: false,
                            italic: false,
                            bg,
                            link: None,
                        }));
                        out.push(line);
                        s = e;
                        if s >= full.len() {
                            break;
                        }
                    }
                }
                out.push(rule("\u{2570}\u{2500}", width, muted)); // ╰─
            }
        }
    }
    out
}

/// A one-column-indented muted border line (` ╭─ lang` / ` ╰─`), clipped to
/// the body width.
fn rule(s: &str, width: usize, fg: Color) -> CardLine {
    let mut line = vec![plain(' ', fg, false)];
    line.extend(s.chars().take(width).map(|c| plain(c, fg, false)));
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(line: &CardLine) -> String {
        line.iter().map(|c| c.c).collect()
    }

    #[test]
    fn newlines_split_prose_into_lines() {
        let lines = body_lines("one\ntwo", 40, (9, 9, 9));
        assert_eq!(lines.len(), 2);
        assert_eq!(text(&lines[0]), " one");
        assert_eq!(text(&lines[1]), " two");
    }

    #[test]
    fn code_block_gets_borders_language_tag_and_bg() {
        let lines = body_lines("see:\n```rust\nfn x() {}\n```", 40, (9, 9, 9));
        let all: Vec<String> = lines.iter().map(text).collect();
        assert_eq!(all[0], " see:");
        assert_eq!(all[1], " \u{256d}\u{2500} rust");
        assert_eq!(all[2], " fn x() {}");
        assert_eq!(all[3], " \u{2570}\u{2500}");
        // The code line sits on the dimmed card background; borders don't.
        assert!(lines[2][1].bg.is_some(), "code line should carry a bg");
        assert!(lines[1][1].bg.is_none(), "border stays on the page bg");
    }

    #[test]
    fn untagged_fence_is_labelled_code() {
        let lines = body_lines("```\nx\n```", 40, (9, 9, 9));
        assert_eq!(text(&lines[0]), " \u{256d}\u{2500} code");
    }

    #[test]
    fn long_code_lines_hard_wrap_verbatim() {
        let lines = body_lines("```\nlet a = 1;\n```", 6, (9, 9, 9));
        assert!(lines.iter().all(|l| l.len() <= 6));
        // Every character — including the spaces — survives the wrap.
        let joined: String = lines[1..lines.len() - 1]
            .iter()
            .map(|l| text(l)[1..].to_string())
            .collect();
        assert_eq!(joined, "let a = 1;");
    }
}
