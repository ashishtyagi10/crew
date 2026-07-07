//! Message-body layout for the card view: message text renders through the
//! shared `md` engine (headings, bold/italic, links, lists, fenced code as a
//! bordered card — `╭─ lang` header, hard-wrapped verbatim lines on a subtly
//! dimmed background, `╰─` footer, ...). `chatmd` maps the engine's styled,
//! char-wrapped `MdLine`s to this module's display-width-wrapped `CardLine`s.

pub(crate) type Color = (u8, u8, u8);

/// One cell of a card line. `bg: None` means the pane's page background.
/// `link` carries the URL a markdown link span resolves to, so Task 6's
/// click hit-test can recover it without re-parsing the message.
#[derive(Clone)]
pub(crate) struct CardCell {
    pub c: char,
    pub fg: Color,
    pub bold: bool,
    pub italic: bool,
    pub bg: Option<Color>,
    /// The URL a markdown link span resolves to; read by Task 6's click
    /// hit-test (`chatview::link_at`) to recover it without re-parsing.
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

/// Lay out one message body through the shared markdown engine: prose,
/// headings, links and lists styled, fenced code blocks bordered + dimmed.
/// Lines are indented one column under the card's `▍sender` header.
/// When `source` is true, shows raw text without markdown rendering.
pub(crate) fn body_lines(text: &str, cols: usize, fg: Color, source: bool) -> Vec<CardLine> {
    let width = cols.saturating_sub(1).max(1);
    if source {
        // Source mode: show raw text, newline-split + word-wrapped, all cells plain.
        return source_lines(text, width, fg);
    }
    // Markdown mode: render through the markdown engine.
    let md_lines = crate::md::render_chat(text, width);
    crate::chatmd::map_lines(md_lines, width, fg)
}

/// Render text in source mode: newline-split, word-wrapped, all cells plain.
/// Each line is indented one column under the card's `▍sender` header.
fn source_lines(text: &str, width: usize, fg: Color) -> Vec<CardLine> {
    let mut out = Vec::new();
    for line_str in text.lines() {
        let chars: Vec<char> = line_str.chars().collect();
        let wrap_indices = crate::chatlayout::wrap_indices(&chars, width);
        for (start, end) in wrap_indices {
            let mut line = Vec::new();
            line.push(plain(' ', fg, false)); // indentation
            for i in start..end {
                line.push(plain(chars[i], fg, false));
            }
            out.push(line);
        }
    }
    if out.is_empty() {
        out.push(vec![plain(' ', fg, false)]); // ensure at least one empty line
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(line: &CardLine) -> String {
        line.iter().map(|c| c.c).collect()
    }

    #[test]
    fn newlines_split_prose_into_lines() {
        let lines = body_lines("one\ntwo", 40, (9, 9, 9), false);
        assert_eq!(lines.len(), 2);
        assert_eq!(text(&lines[0]), " one");
        assert_eq!(text(&lines[1]), " two");
    }

    #[test]
    fn code_block_gets_borders_language_tag_and_bg() {
        let lines = body_lines("see:\n```rust\nfn x() {}\n```", 40, (9, 9, 9), false);
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
        let lines = body_lines("```\nx\n```", 40, (9, 9, 9), false);
        assert_eq!(text(&lines[0]), " \u{256d}\u{2500} code");
    }

    #[test]
    fn long_code_lines_hard_wrap_verbatim() {
        let lines = body_lines("```\nlet a = 1;\n```", 6, (9, 9, 9), false);
        assert!(lines.iter().all(|l| l.len() <= 6));
        // Every character — including the spaces — survives the wrap.
        let joined: String = lines[1..lines.len() - 1]
            .iter()
            .map(|l| text(l)[1..].to_string())
            .collect();
        assert_eq!(joined, "let a = 1;");
    }

    // -- Task 4: full markdown, not just fenced code -----------------------

    #[test]
    fn bold_survives_to_cardcells() {
        let lines = body_lines("**hi**", 40, (9, 9, 9), false);
        assert!(
            lines[0][1..].iter().all(|c| c.bold),
            "not all bold: {}",
            text(&lines[0])
        );
    }

    #[test]
    fn heading_is_bold() {
        let lines = body_lines("# Title", 40, (9, 9, 9), false);
        assert_eq!(text(&lines[0]), " Title");
        assert!(lines[0][1..].iter().all(|c| c.bold));
    }

    #[test]
    fn link_cells_carry_url() {
        let lines = body_lines("go to [site](https://s.io) now", 60, (9, 9, 9), false);
        let joined = text(&lines[0]);
        let start = joined.find("site").expect("site text present");
        for cell in &lines[0][start..start + "site".len()] {
            assert_eq!(cell.link.as_deref(), Some("https://s.io"));
        }
    }

    #[test]
    fn bullet_list_renders() {
        let lines = body_lines("- one\n- two", 40, (9, 9, 9), false);
        assert_eq!(text(&lines[0]), " \u{2022} one");
        assert_eq!(text(&lines[1]), " \u{2022} two");
    }

    #[test]
    fn numbered_list_with_fenced_code_renders_chrome() {
        let lines = body_lines(
            "1. First do X:\n\n   ```bash\n   cmd --flag\n   ```",
            40,
            (9, 9, 9),
            false,
        );
        let all: Vec<String> = lines.iter().map(text).collect();
        assert!(
            all.iter().any(|l| l.contains("\u{256d}\u{2500} bash")),
            "missing code header chrome: {all:?}"
        );
        let cmd_row = all
            .iter()
            .position(|l| l.contains("cmd --flag"))
            .unwrap_or_else(|| panic!("missing verbatim code line: {all:?}"));
        assert!(
            lines[cmd_row][1].bg.is_some(),
            "code line should carry the code bg: {all:?}"
        );
    }

    #[test]
    fn cjk_prose_rechunks_to_display_width_budget() {
        let text_in = "\u{6f22}\u{5b57}".repeat(30);
        let lines = body_lines(&text_in, 20, (9, 9, 9), false);
        assert!(!lines.is_empty());
        for l in &lines {
            let w: usize = l.iter().map(|c| crate::chatwidth::char_w(c.c)).sum();
            assert!(
                w <= 20,
                "line exceeds width budget ({w} > 20): {:?}",
                text(l)
            );
        }
    }
}
