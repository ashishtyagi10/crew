//! Per-character colour and weight for the gutter rungs' syntax colouring
//! (Fix 1). Split out of `lines.rs` to keep that file under the file-length
//! budget; `lines::numbered` is the only caller.
use crate::md::syntax::{tokenize, Token};

/// One character's colour and weight (bold).
pub(crate) type CharPaint = ((u8, u8, u8), bool);

/// A token's colour and weight. `Plain` is left on the caller's `ink` — the
/// gutter rungs already colour identifiers and operators that way, and this
/// only owes the classes the lexer actually claims a colour. Those follow
/// the convention `chatink`/`chatspan` establish for fenced code in chat
/// (`chatspan::style`'s `LineKind::Code` arm): colour from the theme's
/// derived syntax slots via `chatink::token_fg` (the hued classes through
/// `chathue`), with keywords carrying WEIGHT on top of their hue.
///
/// On see-through glass each class is floored over `grounds` ([`grounds`]):
/// the slots were floored for chat's code FIELD, a second sheet the viewer
/// does not draw, so on the pane's own frost over a light desktop a file's
/// strings read 3.1:1 and its comments 3.3 (glass survey F7).
fn token_paint(tok: Token, ink: (u8, u8, u8), grounds: &[(u8, u8, u8)]) -> CharPaint {
    let t = crew_theme::theme();
    let floor = crate::chatink::COMMENT_PAGE_FLOOR;
    let on = |c| crew_theme::glasslegend::legible_on(t, c, floor, grounds);
    match tok {
        Token::Plain => (ink, false),
        Token::Keyword => (on(crate::chatink::token_fg(tok)), true),
        Token::Comment | Token::Str | Token::Type | Token::Func | Token::Number | Token::Attr => {
            (on(crate::chatink::token_fg(tok)), false)
        }
        // Diff line classes never reach this path today — `detect` routes
        // .diff/.patch files to their own rung (`lines::diff_lines`), not to
        // a `Code { lang }` — but the mapping is the honest one if they ever
        // do: the same colour the chat transcript gives them, regular weight.
        Token::Added | Token::Removed | Token::Hunk => (crate::chatink::token_fg(tok), false),
    }
}

/// What a viewed file's code stands on: the pane's frost over any desktop
/// while the window is sheer. None off glass, on an opaque window, or on a
/// tube, whose syntax ladder is lightness alone (`codefield::glassed`).
fn grounds() -> Vec<(u8, u8, u8)> {
    let t = crew_theme::theme();
    match crew_theme::glassborder::sheer() && !t.is_tube() {
        true => crew_theme::glasslegend::pane_grounds(t),
        false => Vec::new(),
    }
}

/// Above this many bytes, a line is painted uniformly rather than tokenized,
/// belt-and-braces against Part 1's fix rather than a limit this branch
/// expects to bite: `tokenize` is now linear, but linear-with-a-per-character
/// allocation is still not something to run inside a frame for a
/// multi-megabyte single-line file (the minified-bundle case Part 1 targets
/// is hundreds of KB to several MB on ONE line). 4 KiB comfortably covers any
/// line worth calling "long source" — it is many times the width of a
/// terminal — while sitting far below where a linear cost would start to be
/// felt in a 16ms frame, so this is cheap insurance rather than a cap anyone
/// should expect to hit on real code.
const MAX_COLOURED_LINE_BYTES: usize = 4096;

/// One line's paint, one entry per `char`, in the same order as
/// `line.chars()` — `tokenize` covers every character of the line exactly
/// once, so the two sequences line up and a wrapped row can slice both by
/// the same indices. `lang` empty means "no lexer for this rung": every char
/// paints `ink`, unbold, which is the pre-Fix-1 behaviour for the rungs that
/// stay that way (`Extract`, and `Code`/`Data` files whose extension carries
/// no language). A line over `MAX_COLOURED_LINE_BYTES` gets the same uniform
/// treatment regardless of `lang` — see that constant's doc.
pub(crate) fn line_paint(line: &str, lang: &str, ink: (u8, u8, u8)) -> Vec<CharPaint> {
    if lang.is_empty() || line.len() > MAX_COLOURED_LINE_BYTES {
        return line.chars().map(|_| (ink, false)).collect();
    }
    let grounds = grounds();
    tokenize(line, lang)
        .into_iter()
        .flat_map(|(text, tok)| {
            let paint = token_paint(tok, ink, &grounds);
            std::iter::repeat_n(paint, text.chars().count())
        })
        .collect()
}

#[cfg(test)]
#[path = "codepaint_tests.rs"]
mod tests;
