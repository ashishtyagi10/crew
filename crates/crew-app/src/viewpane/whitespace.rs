//! Tabs, and the rest of what a file says without printing anything.
//!
//! ## Tabs were invisible
//!
//! `place_row` — the guard every cell surface in crew places glyphs through —
//! skips characters of zero display width, because a zero-width glyph placed
//! at a column would be overprinted by the next one. `UnicodeWidthChar` gives
//! a tab a width of `None`, i.e. zero. So a tab-indented file opened in the
//! viewer was drawn **with its indentation missing**: not mangled, not
//! misaligned — simply gone, on every line of every Go file, Makefile and
//! kernel-style C source anyone pointed `/view` at.
//!
//! The fix is the one a terminal has always applied: expand a tab to the next
//! multiple of [`TAB_STOP`] columns. Eight, because that is what the PTY in
//! the pane beside the viewer does — `cat file` and `/view file` disagreeing
//! about how far in a line starts would be a worse bug than the one being
//! fixed, and every tool that prints a tab-indented file to a terminal (`git
//! diff` included) has settled on the same number.
//!
//! Expansion happens to the TEXT, before any rung sees it, so the syntax
//! paint, the wrap, the search and the diff pairing all agree about which
//! column a character is in — one of them working from unexpanded text would
//! put the colour somewhere the glyph is not.
//!
//! ## …and the rest, on request
//!
//! With `/invisibles` on, the characters that change what a file *means*
//! without printing anything say so: a tab shows its arrow, trailing spaces
//! show as middle dots, and a carriage return left by a CRLF file shows its
//! own mark. These are the three that cause real trouble — a tab where spaces
//! were meant, whitespace nobody sees at the end of a line, and a line ending
//! that makes a shell script fail with a message about a command that does
//! not exist.
//!
//! Marked, not merely substituted: [`prepare`] reports which characters it
//! made visible so they can be drawn in the muted ink. Recolouring by glyph
//! instead would dim a `·` that was genuinely in the file.

/// Columns a tab advances to the next multiple of — the terminal's own tab
/// stop, so the viewer and a `cat` in the pane beside it agree.
pub(crate) const TAB_STOP: usize = 8;

/// The arrow a revealed tab wears, in the first column it occupies.
const TAB_MARK: char = '\u{2192}';
/// A revealed trailing space.
const SPACE_MARK: char = '\u{b7}';
/// A revealed carriage return (the `CR` control picture).
const CR_MARK: char = '\u{240d}';

/// Text with tabs expanded, plus a flag per character saying whether it is
/// something [`prepare`] made visible.
pub(crate) struct Prepared {
    pub text: String,
    /// One `Vec<bool>` per line, one entry per character of `text`'s version
    /// of that line.
    pub marks: Vec<Vec<bool>>,
}

/// Expand every tab to the next [`TAB_STOP`] and, when `reveal`, mark the
/// invisible characters so they can be seen.
///
/// A carriage return is dropped either way when `reveal` is off: it has zero
/// width, so it was never drawn, and leaving it in the text only misleads
/// whatever measures the line.
pub(crate) fn prepare(text: &str, reveal: bool) -> Prepared {
    let mut out = String::with_capacity(text.len());
    let mut marks = Vec::new();
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let (rendered, line_marks) = prepare_line(line, reveal);
        out.push_str(&rendered);
        marks.push(line_marks);
    }
    Prepared { text: out, marks }
}

/// One line's expansion. Trailing whitespace is measured on the SOURCE line,
/// before expansion, so a run of tabs at the end of a line is marked as the
/// trailing whitespace it is rather than as ordinary indentation.
fn prepare_line(line: &str, reveal: bool) -> (String, Vec<bool>) {
    let chars: Vec<char> = line.chars().collect();
    // Where the line's own text ends: everything past here is trailing.
    let tail = chars
        .iter()
        .rposition(|c| !c.is_whitespace())
        .map_or(0, |i| i + 1);
    let mut out = String::new();
    let mut marks = Vec::new();
    let mut col = 0usize;
    for (i, &c) in chars.iter().enumerate() {
        let trailing = i >= tail;
        match c {
            '\t' => {
                let width = TAB_STOP - (col % TAB_STOP);
                for k in 0..width {
                    let head = k == 0 && reveal;
                    out.push(if head { TAB_MARK } else { ' ' });
                    marks.push(reveal);
                }
                col += width;
            }
            // A CRLF file's `\r` sits at the end of every line. It has no
            // width, so it has never been drawn; revealing it is the whole
            // point of showing it at all.
            '\r' => {
                if reveal {
                    out.push(CR_MARK);
                    marks.push(true);
                    col += 1;
                }
            }
            ' ' if trailing && reveal => {
                out.push(SPACE_MARK);
                marks.push(true);
                col += 1;
            }
            _ => {
                out.push(c);
                marks.push(false);
                col += crate::chatwidth::char_w(c);
            }
        }
    }
    (out, marks)
}

/// Recolour the characters `prepare` made visible to `muted`, in a paint
/// array indexed the same way its text is.
///
/// The paint arrives from the syntax tokenizer, which sees the expanded text
/// and has no idea a run of spaces used to be a tab — this is the only place
/// that knows.
pub(crate) fn dim(
    paints: &mut [Vec<super::codepaint::CharPaint>],
    marks: &[Vec<bool>],
    muted: (u8, u8, u8),
) {
    for (line, line_marks) in paints.iter_mut().zip(marks) {
        for (paint, marked) in line.iter_mut().zip(line_marks) {
            if *marked {
                *paint = (muted, false);
            }
        }
    }
}

#[cfg(test)]
#[path = "whitespace_tests.rs"]
mod tests;
