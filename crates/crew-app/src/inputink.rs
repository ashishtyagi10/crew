//! Colour for the text being typed in the command bar.
//!
//! Every character used to be `ink`, so the bar said nothing about what you
//! had written until you pressed Enter and found out. Shells have painted
//! their input for years — fish's red-until-it-resolves is the canonical
//! example — and the reason is the same here: a slash command with a typo in
//! it, a flag, a quoted path, all look identical in one colour.
//!
//! Three things are marked, and nothing else. The bar is one row and the text
//! in it is short; a syntax highlighter's worth of colour on twelve
//! characters is decoration, not information.
use crate::cmddefs::COMMANDS;

/// How the leading `/token` resolves.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Cmd {
    /// A command, exactly.
    Known,
    /// Not yet a command, but the start of one — mid-typing.
    Partial,
    /// Nothing begins with this. `/them` is on its way to `/theme`; `/zzz`
    /// is on its way to nothing, and saying so before Enter is the point.
    Unknown,
}

/// Classify `word` (with its leading slash) against the command table.
pub(crate) fn classify(word: &str) -> Cmd {
    let w = word.to_lowercase();
    if COMMANDS.iter().any(|c| c.name == w) {
        return Cmd::Known;
    }
    match COMMANDS.iter().any(|c| c.name.starts_with(&w)) {
        true => Cmd::Partial,
        false => Cmd::Unknown,
    }
}

/// One colour per character of `text`.
pub(crate) fn paint(text: &str) -> Vec<(u8, u8, u8)> {
    let t = crew_theme::theme();
    let string = crate::chatink::token_fg(crate::md::syntax::Token::Str);
    let chars: Vec<char> = text.chars().collect();
    let mut out = vec![t.ink; chars.len()];
    // The leading word, when it is a slash command.
    let head = chars
        .iter()
        .position(|c| c.is_whitespace())
        .unwrap_or(chars.len());
    if chars.first() == Some(&'/') {
        let word: String = chars[..head].iter().collect();
        let fg = match classify(&word) {
            Cmd::Known => crate::palette::accent(),
            Cmd::Partial => t.text_muted,
            Cmd::Unknown => t.bell,
        };
        out[..head].fill(fg);
    }
    // Flags and quoted runs in the rest of the line.
    let mut i = head;
    let mut quote: Option<char> = None;
    while i < chars.len() {
        let c = chars[i];
        if let Some(q) = quote {
            out[i] = string;
            if c == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        if c == '"' || c == '\'' {
            quote = Some(c);
            out[i] = string;
            i += 1;
            continue;
        }
        // A flag is a `-` at the start of a word, and runs to the next space.
        let starts_word = i == 0 || chars[i - 1].is_whitespace();
        if c == '-' && starts_word {
            while i < chars.len() && !chars[i].is_whitespace() {
                out[i] = t.dim;
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    out
}

#[cfg(test)]
#[path = "inputink_tests.rs"]
mod tests;
