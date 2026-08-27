//! Text decorations a cell can carry: the underline family, strikethrough and
//! the separate underline colour (SGR 58).
//!
//! Lives here rather than in `crew-render` because both ends need it — the
//! terminal model reads the flags off the grid, the renderer turns them into
//! quads — and this is the crate they share. It is pure data: no GPU, no
//! terminal, no allocation.

/// Which underline a cell wears. `None` is the resting state and the one
/// almost every cell is in, so it is the `Default`.
#[derive(Hash, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum DecoLine {
    #[default]
    None,
    /// SGR 4 / 4:1 — one solid rule.
    Single,
    /// SGR 21 / 4:2 — two thin rules.
    Double,
    /// SGR 4:3 — the spell-check squiggle. Editors and language servers speak
    /// this one; a terminal that drops it shows a diagnostic as plain text.
    Curly,
    /// SGR 4:4.
    Dotted,
    /// SGR 4:5.
    Dashed,
}

/// Everything decorative about a cell that is drawn as a rule rather than a
/// glyph. Both members are independent: SGR 9 and SGR 4 can be on at once.
#[derive(Hash, Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct Deco {
    pub line: DecoLine,
    pub strike: bool,
    /// SGR 58's colour, when the program set one. `None` means "draw the rule
    /// in the cell's own foreground", which is what every terminal did before
    /// 58 existed and what most programs still expect.
    pub color: Option<(u8, u8, u8)>,
}

impl Deco {
    /// The resting decoration — usable in a `const` context, which
    /// `Default::default()` is not.
    pub const NONE: Deco = Deco {
        line: DecoLine::None,
        strike: false,
        color: None,
    };

    /// One underline of `line`, in the cell's own colour.
    pub fn underline(line: DecoLine) -> Self {
        Deco { line, ..Deco::NONE }
    }

    /// Whether this cell draws nothing at all — the fast path the renderer
    /// takes for the overwhelming majority of cells.
    pub fn is_blank(&self) -> bool {
        self.line == DecoLine::None && !self.strike
    }
}
