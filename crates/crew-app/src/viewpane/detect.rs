//! Classifying a file into a rung of the viewer's format ladder. Pure: it
//! takes the head of the file's bytes, never a filesystem, so the worker
//! thread in `load` can call it and the tests can cover every rung without
//! fixtures. The one rule worth stating out loud: the binary sniff outranks
//! the extension, and only for the binary verdict — a `.md` full of strange
//! prose is still markdown, a `.md` that is really a JPEG is not.
use std::path::Path;

/// How many leading bytes `detect` is given. The caller reads at most this
/// much, so a multi-byte char can be cut in half at the boundary — see
/// `looks_utf8`.
pub(crate) const SNIFF_BYTES: usize = 8192;

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
    fn has(self, e: Extractor) -> bool {
        match e {
            Extractor::TextUtil => self.textutil,
            Extractor::PdfToText => self.pdftotext,
        }
    }
}

/// One rung of the ladder. `lang` is the `md/syntax.rs` language tag, `""`
/// for "text, no keywords".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Code { lang: &'static str },
    Markdown,
    Data { lang: &'static str },
    Csv { delim: char },
    Diff,
    Extract { via: Extractor },
    Opaque { why: Opaque },
}

/// Extension → rung. Kept as a flat table because it is read once per open
/// and a `match` here is easier to extend than a lazy map.
fn by_extension(ext: &str) -> Option<Format> {
    let f = match ext {
        "rs" => Format::Code { lang: "rust" },
        "ts" | "tsx" | "js" | "jsx" => Format::Code { lang: "js" },
        "py" => Format::Code { lang: "python" },
        "go" => Format::Code { lang: "go" },
        "c" | "h" | "cpp" | "hpp" | "cc" => Format::Code { lang: "c" },
        "sh" | "bash" | "zsh" => Format::Code { lang: "shell" },
        "md" | "markdown" | "mdx" => Format::Markdown,
        "json" => Format::Data { lang: "json" },
        "yaml" | "yml" => Format::Data { lang: "yaml" },
        "toml" => Format::Data { lang: "toml" },
        "ini" | "conf" | "cfg" => Format::Data { lang: "" },
        "csv" => Format::Csv { delim: ',' },
        "tsv" => Format::Csv { delim: '\t' },
        "diff" | "patch" => Format::Diff,
        "docx" | "doc" | "rtf" | "odt" => Format::Extract {
            via: Extractor::TextUtil,
        },
        "pdf" => Format::Extract {
            via: Extractor::PdfToText,
        },
        _ => return None,
    };
    Some(f)
}

/// A NUL byte in the head. The cheapest reliable "this is not text" signal,
/// and the only one allowed to overrule the extension.
fn looks_binary(head: &[u8]) -> bool {
    head.contains(&0)
}

/// Whether `head` is valid UTF-8, tolerating a multi-byte char sliced by the
/// sniff window: an error in the final 3 bytes of a full-size head is the
/// boundary, not corruption.
fn looks_utf8(head: &[u8]) -> bool {
    match std::str::from_utf8(head) {
        Ok(_) => true,
        Err(e) => head.len() >= SNIFF_BYTES && e.valid_up_to() + 4 > head.len(),
    }
}

/// Content sniff for files whose extension told us nothing.
fn by_content(head: &[u8]) -> Format {
    let text = String::from_utf8_lossy(head);
    if text.starts_with("#!") {
        return Format::Code { lang: "shell" };
    }
    if text.starts_with("diff --git") || text.starts_with("@@ ") || text.starts_with("--- ") {
        return Format::Diff;
    }
    Format::Code { lang: "" }
}

/// Classify `path` given the first [`SNIFF_BYTES`] of its bytes and which
/// extractors exist. See the module comment for the precedence rule.
pub(crate) fn detect(path: &Path, head: &[u8], probe: Probe) -> Format {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let named = by_extension(&ext);

    // An extract rung is binary by nature, so the binary sniff must not fire
    // on it — resolve it first, degrading when its tool is absent.
    if let Some(Format::Extract { via }) = named {
        return if probe.has(via) {
            Format::Extract { via }
        } else {
            Format::Opaque {
                why: Opaque::NoExtractor(via),
            }
        };
    }
    if looks_binary(head) {
        return Format::Opaque {
            why: Opaque::Binary,
        };
    }
    if !looks_utf8(head) {
        return Format::Opaque {
            why: Opaque::NotUtf8,
        };
    }
    named.unwrap_or_else(|| by_content(head))
}

#[cfg(test)]
#[path = "detect_tests.rs"]
mod tests;
