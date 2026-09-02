//! The external text-extraction tools a rung can lean on, whether this
//! machine has them, and the reasons a file is opaque. Split from
//! [`super::detect`], which decides the rung; this is only about the tools.

/// An external text-extraction tool. `TextUtil` ships with macOS; `PdfToText`
/// comes from poppler and is frequently absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Extractor {
    TextUtil,
    PdfToText,
}

impl Extractor {
    /// The binary name, used to probe `PATH` and to name what to install.
    pub(crate) fn bin(self) -> &'static str {
        match self {
            Extractor::TextUtil => "textutil",
            Extractor::PdfToText => "pdftotext",
        }
    }

    /// What the user is told to install when this tool is missing.
    pub(crate) fn install_hint(self) -> &'static str {
        match self {
            Extractor::TextUtil => "textutil (ships with macOS)",
            Extractor::PdfToText => "pdftotext — brew install poppler",
        }
    }
}

/// Why a file gets the metadata card instead of a rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Opaque {
    Binary,
    NotUtf8,
    NoExtractor(Extractor),
    /// `read_capped` itself failed — not found, permission denied, or some
    /// other I/O error, none of which is "binary" (Fix 2's fold-in item:
    /// `load_now` used to tag every read failure `Binary`, a false, specific
    /// claim about bytes it never actually read).
    Unreadable,
}

/// Which tools are on `PATH`. Passed in rather than probed here so `detect`
/// stays pure and the "missing tool degrades a rung" rule is testable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Probe {
    pub textutil: bool,
    pub pdftotext: bool,
}

impl Probe {
    pub(crate) fn has(self, e: Extractor) -> bool {
        match e {
            Extractor::TextUtil => self.textutil,
            Extractor::PdfToText => self.pdftotext,
        }
    }
}
