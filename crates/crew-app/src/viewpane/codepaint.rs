//! Per-character colour and weight for the gutter rungs' syntax colouring
//! (Fix 1). Split out of `lines.rs` to keep that file under the file-length
//! budget; `lines::numbered` is the only caller.
use crate::md::syntax::{tokenize, Token};

/// One character's colour and weight (bold).
pub(crate) type CharPaint = ((u8, u8, u8), bool);

/// A token's colour and weight. `Plain` is left on the caller's `ink` — the
/// gutter rungs already colour identifiers and operators that way, and this
/// only owes the three classes the lexer actually claims a colour. Those
/// three follow the convention `chatink`/`chatmd` already establish for
/// fenced code in chat (`chatmd::span_style`'s `LineKind::Code` arm): colour
/// from the theme's derived syntax slots via `chatink::token_fg`, with
/// keywords set apart by WEIGHT rather than a fourth colour of their own —
/// `chatink`'s `keyword` slot is numerically the same as `code`, by design.
fn token_paint(tok: Token, ink: (u8, u8, u8)) -> CharPaint {
    match tok {
        Token::Plain => (ink, false),
        Token::Keyword => (crate::chatink::token_fg(tok), true),
        Token::Comment | Token::Str => (crate::chatink::token_fg(tok), false),
    }
}

/// One line's paint, one entry per `char`, in the same order as
/// `line.chars()` — `tokenize` covers every character of the line exactly
/// once, so the two sequences line up and a wrapped row can slice both by
/// the same indices. `lang` empty means "no lexer for this rung": every char
/// paints `ink`, unbold, which is the pre-Fix-1 behaviour for the rungs that
/// stay that way (`Extract`, and `Code`/`Data` files whose extension carries
/// no language).
pub(crate) fn line_paint(line: &str, lang: &str, ink: (u8, u8, u8)) -> Vec<CharPaint> {
    if lang.is_empty() {
        return line.chars().map(|_| (ink, false)).collect();
    }
    tokenize(line, lang)
        .into_iter()
        .flat_map(|(text, tok)| {
            let paint = token_paint(tok, ink);
            std::iter::repeat_n(paint, text.chars().count())
        })
        .collect()
}

#[cfg(test)]
#[path = "codepaint_tests.rs"]
mod tests;
