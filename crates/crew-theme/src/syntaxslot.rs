//! Which ANSI slot each syntax class draws from.
//!
//! crew's own palettes share one mapping: magenta keywords, yellow types,
//! blue calls, green strings, and numbers in the code colour itself. The
//! classics took their schemes' six hues in v0.29.15 ([`crate::ansihue`]),
//! but still handed them out by crew's mapping, so monokai drew its keywords
//! in its purple and its strings in its green, when a monokai keyword is the
//! hot pink and a monokai string the yellow. Hue was the scheme's; which hue
//! went where was still crew's.
//!
//! So each palette says which slot each class takes. The colours themselves
//! are still derived from the slot by `crew-app`'s `chatink` and `chathue`,
//! on the same floors, so a class that moves slot is exactly as readable as
//! the one it replaced. On a classic the five hued classes take five
//! different slots, which is how each scheme draws them; `code`, the plain
//! identifier colour of a chat fence, may share one (solarized draws its
//! identifiers and calls in one blue).

/// The ANSI slot (1–6) each syntax class takes on one palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntaxSlots {
    /// `fn`, `let`, `def`…
    pub keyword: usize,
    /// A capitalised identifier.
    pub ty: usize,
    /// An identifier followed by `(`.
    pub func: usize,
    /// A numeric literal.
    pub number: usize,
    /// A string literal.
    pub string: usize,
    /// Plain code in a chat fence or an inline span.
    pub code: usize,
}

impl SyntaxSlots {
    /// crew's own mapping, on every palette that is not a classic.
    pub const HOUSE: SyntaxSlots = SyntaxSlots {
        keyword: 5,
        ty: 3,
        func: 4,
        number: 6,
        string: 2,
        code: 6,
    };

    /// The five hued classes, in a fixed order, for the tests and the shots.
    pub fn classes(self) -> [(&'static str, usize); 5] {
        [
            ("keyword", self.keyword),
            ("type", self.ty),
            ("func", self.func),
            ("number", self.number),
            ("string", self.string),
        ]
    }
}

/// Tokyo Night: purple keywords, blue calls, green strings, cyan types and
/// its orange numbers (the yellow slot is its nearest).
pub const TOKYO_NIGHT: SyntaxSlots = SyntaxSlots {
    keyword: 5,
    ty: 6,
    func: 4,
    number: 3,
    string: 2,
    code: 6,
};

/// Solarized: green statements, yellow types, blue calls and identifiers,
/// cyan strings, magenta numbers.
pub const SOLARIZED: SyntaxSlots = SyntaxSlots {
    keyword: 2,
    ty: 3,
    func: 4,
    number: 5,
    string: 6,
    code: 4,
};

/// Everforest: red keywords, green calls, yellow types, aqua strings,
/// purple numbers, blue identifiers.
pub const EVERFOREST: SyntaxSlots = SyntaxSlots {
    keyword: 1,
    ty: 3,
    func: 2,
    number: 5,
    string: 6,
    code: 4,
};

/// Gruvbox: red keywords, yellow types, green strings, purple numbers, blue
/// identifiers, and aqua calls (gruvbox draws calls and strings in one
/// green; the aqua is the scheme's own next step, where its built-ins go).
pub const GRUVBOX: SyntaxSlots = SyntaxSlots {
    keyword: 1,
    ty: 3,
    func: 6,
    number: 5,
    string: 2,
    code: 4,
};

/// Monokai: pink keywords, green calls, cyan types, yellow strings, purple
/// numbers.
pub const MONOKAI: SyntaxSlots = SyntaxSlots {
    keyword: 1,
    ty: 4,
    func: 2,
    number: 5,
    string: 3,
    code: 6,
};

#[cfg(test)]
#[path = "syntaxslot_tests.rs"]
mod tests;
