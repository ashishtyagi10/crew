//! A fenced code block's rows: the language label, the verbatim lines wrapped
//! to the field's width, and the blank row that closes it. Split from
//! `layout.rs` (which was two lines from its ceiling) when the wrap learned to
//! measure and to say that it happened.
//!
//! **A wrapped code line now says so.** It was cut with `chars.chunks(cw)` and
//! laid out flush, so the tail of a long line read as the next statement —
//! `Cov((0.5 - d * scale).clamp(0.0, 1.0))` under `let d = sdf::arc(…);` looks
//! exactly like a line of code that is there, and is not. The file viewer has
//! answered this since it was written, with a `↪` on the continuation row, so
//! that is the glyph here too.
//!
//! **And the cut is by display width now.** `chunks(cw)` counts CHARACTERS: a
//! line with CJK or an emoji in it took two columns per char and ran past the
//! tinted field it is laid into, which is the one edge a code block has.
use super::syntax::Token;
use super::{LineKind, MdLine, MdSpan, MdStyle};

/// Columns the continuation marker and its space take from a wrapped row.
const CONT_W: usize = 2;

/// The mark a continued row wears, in the comment colour rather than the
/// marker one: `chatfield::field_start` reads LEADING marker ink as "outside
/// the field" — which is how a blockquote's bar stays out of the tint — and a
/// mark that belongs to the code must stay inside it.
fn cont_span() -> MdSpan {
    MdSpan {
        text: "\u{21aa} ".to_string(),
        style: MdStyle {
            token: Token::Comment,
            ..MdStyle::default()
        },
        link: None,
        src: None,
    }
}

fn plain_span(text: String) -> MdSpan {
    MdSpan {
        text,
        style: MdStyle::default(),
        link: None,
        src: None,
    }
}

/// How many of `chars` fit in `room` display columns — at least one, so a
/// character wider than the room it is given still makes progress instead of
/// wrapping forever.
fn fit(chars: &[char], room: usize) -> usize {
    let mut w = 0;
    for (i, &c) in chars.iter().enumerate() {
        w += crate::chatwidth::char_w(c);
        if w > room {
            return i.max(1);
        }
    }
    chars.len()
}

/// One wrapped row's worth of spans, taken from `runs` starting at `cursor`
/// (a character offset into the whole line) and advancing it by `width`
/// characters.
///
/// The cursor lives across rows because a token may straddle a wrap: the two
/// halves become two spans carrying the same token, which is what keeps a long
/// string one colour all the way down.
fn code_spans(runs: &[(String, Token)], cursor: &mut usize, width: usize) -> Vec<MdSpan> {
    let (start, end) = (*cursor, *cursor + width);
    *cursor = end;
    let mut out: Vec<MdSpan> = Vec::new();
    let mut at = 0usize;
    for (text, token) in runs {
        let len = text.chars().count();
        let (from, to) = (at.max(start), (at + len).min(end));
        at += len;
        if from >= to {
            continue;
        }
        let slice: String = text
            .chars()
            .skip(from - (at - len))
            .take(to - from)
            .collect();
        out.push(MdSpan {
            text: slice,
            style: MdStyle {
                token: *token,
                ..MdStyle::default()
            },
            link: None,
            src: None,
        });
    }
    if out.is_empty() {
        out.push(plain_span(String::new()));
    }
    out
}

/// The rows of one fenced block at `cols` columns.
pub(super) fn lines(lang: String, src_lines: Vec<String>, cols: usize) -> Vec<MdLine> {
    // An untagged fence whose body reads as a diff is treated as one — the
    // same sniff family as `viewpane::detect::by_content` — so ```diff and a
    // bare paste of `git diff` output colour alike.
    let lang = if lang.is_empty() && super::syntaxdiff::looks_like_diff(&src_lines) {
        "diff".to_string()
    } else {
        lang
    };
    // Two columns narrower than the card: the chat renderer lays these lines
    // into a padded field (`chatfield::PAD` each side), and a code line that
    // used the full width would push its own right-hand pad off the card.
    let cw = cols.saturating_sub(crate::chatfield::PAD * 2).max(1);
    // The label (with its dev-icon on a Nerd Font) alone: the block's edges
    // are drawn by the tinted FIELD the chat card lays these lines into
    // (`chatfield`), not by corner glyphs.
    let header_text = crate::glyphs::fence_header(&lang, cw);
    let mut out = vec![MdLine {
        spans: vec![plain_span(header_text)],
        kind: LineKind::CodeHeader,
    }];
    for line in src_lines {
        out.extend(rows(&line, &lang, cw));
    }
    // Closed by a blank row of the field rather than a corner — see the
    // header above.
    out.push(MdLine {
        spans: vec![plain_span(String::new())],
        kind: LineKind::CodeFooter,
    });
    out
}

/// One source line as one or more rows: the first flush, the rest behind a
/// `↪` and that much narrower.
fn rows(line: &str, lang: &str, cw: usize) -> Vec<MdLine> {
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return vec![MdLine {
            spans: vec![plain_span(String::new())],
            kind: LineKind::Code,
        }];
    }
    // Tokenize the WHOLE source line, then cut it into rows. Doing it the
    // other way round would lex each wrapped chunk independently, and a string
    // or comment that crossed a wrap boundary would change colour mid-token.
    let runs = super::syntax::tokenize(line, lang);
    let mut cursor = 0usize;
    let mut out = Vec::new();
    while cursor < chars.len() {
        let first = out.is_empty();
        // A field only `CONT_W` wide has no room for the mark AND a character,
        // so a very narrow card keeps the code and drops the mark.
        let mark = !first && cw > CONT_W;
        let room = if mark { cw - CONT_W } else { cw };
        let take = fit(&chars[cursor..], room);
        let take = crate::chatwidth::soft_end(&chars[cursor..], 0, take);
        let mut spans = mark.then(cont_span).into_iter().collect::<Vec<_>>();
        spans.extend(code_spans(&runs, &mut cursor, take));
        out.push(MdLine {
            spans,
            kind: LineKind::Code,
        });
    }
    out
}

#[cfg(test)]
#[path = "codeblock_tests.rs"]
mod tests;
