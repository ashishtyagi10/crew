//! Message-body layout for the card view: message text renders through the
//! shared `md` engine (headings, bold/italic, links, lists, fenced code as a
//! bordered card — `╭─ lang` header, hard-wrapped verbatim lines on a subtly
//! dimmed background, `╰─` footer, ...). `chatmd` maps the engine's styled,
//! char-wrapped `MdLine`s to this module's display-width-wrapped `CardLine`s.

pub(crate) type Color = (u8, u8, u8);

/// One cell of a card line. `bg: None` means the pane's page background.
/// `link` carries the URL a markdown link span resolves to, so `clickopen`'s
/// click hit-test can recover it without re-parsing the message.
#[derive(Clone)]
pub(crate) struct CardCell {
    pub c: char,
    pub fg: Color,
    pub bold: bool,
    pub italic: bool,
    pub bg: Option<Color>,
    /// The URL a markdown link span resolves to; read by `clickopen`'s click
    /// hit-test (`chatview::link_at`) to recover it without re-parsing.
    pub link: Option<std::sync::Arc<str>>,
    /// The byte in the SOURCE this character came from, when it came from one
    /// verbatim (see [`crate::md::source`]). `None` for a character the
    /// renderer added — a bullet, a table rule, a code field's border, the
    /// space a soft break became — and those are exactly the places a cursor
    /// cannot go, because there is nothing there to type into.
    pub src: Option<u32>,
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
        src: None,
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
            for &c in &chars[start..end] {
                line.push(plain(c, fg, false));
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
#[path = "chatbody_tests.rs"]
mod tests;
