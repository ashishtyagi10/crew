//! The chat pane's icon set: a Nerd Font glyph for each mark when the active
//! family really carries one, and the Unicode character the pane always drew
//! otherwise. Nothing here regresses a plain font — every fallback is the
//! glyph that was there before there was a choice.
//!
//! "On" is decided by coverage, not by name: `crew_render::has_glyph` reads
//! [`PROBE`] off the family's own character map each time the family
//! changes, and the answer sits in one atomic (a bool load per pick). The
//! family reaches the renderer ONLY through [`apply_family`] — a source scan
//! keeps every other `set_font_family` call out of crew-app, so a new setter
//! site cannot leave the icon set describing the previous font.
use std::sync::atomic::{AtomicBool, Ordering};

/// nf-dev-rust. Any Nerd Font maps it; no ordinary text face does.
pub(crate) const PROBE: char = '\u{e7a8}';

/// Whether the active family covers the icon set ([`set_family`] writes).
static ON: AtomicBool = AtomicBool::new(false);

/// Push `family` to the renderer AND re-read icon coverage for it. The one
/// door: see the module doc.
pub(crate) fn apply_family(r: &mut crew_render::Renderer, family: Option<String>) {
    set_family(family.as_deref());
    r.set_font_family(family);
}

/// Re-read icon coverage for `family` (`None` = the embedded face).
pub(crate) fn set_family(family: Option<&str>) {
    ON.store(crew_render::has_glyph(family, PROBE), Ordering::Relaxed);
}

/// Whether picks draw Nerd Font glyphs right now.
pub(crate) fn on() -> bool {
    #[cfg(test)]
    if let Some(forced) = tests::forced() {
        return forced;
    }
    ON.load(Ordering::Relaxed)
}

/// One mark the pane draws. `Lang` is a fence header's language; `Spinner`
/// is a frame index (see [`spinner`] for the count).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Glyph<'a> {
    Bullet1,
    Bullet2,
    Bullet3,
    Checked,
    Unchecked,
    Quote,
    Spinner(u8),
    DotOn,
    DotOff,
    Prompt,
    Image,
    Footnote,
    Dir,
    File,
    Lang(&'a str),
}

/// nf-md-circle_slice_1..8: a pie filling clockwise.
const NF_SPINNER: [&str; 8] = [
    "\u{f0a9e}",
    "\u{f0a9f}",
    "\u{f0aa0}",
    "\u{f0aa1}",
    "\u{f0aa2}",
    "\u{f0aa3}",
    "\u{f0aa4}",
    "\u{f0aa5}",
];
/// The ASCII spinner every font has.
const ASCII_SPINNER: [&str; 4] = ["|", "/", "-", "\\"];

/// The Nerd Font glyph for `g`.
pub(crate) fn nerd(g: Glyph) -> &'static str {
    match g {
        Glyph::Bullet1 | Glyph::DotOn => "\u{f111}", // nf-fa-circle
        Glyph::Bullet2 | Glyph::DotOff => "\u{f10c}", // nf-fa-circle_o
        Glyph::Bullet3 => "\u{f0c8}",                // nf-fa-square
        Glyph::Checked => "\u{f046}",                // nf-fa-check_square_o
        Glyph::Unchecked => "\u{f096}",              // nf-fa-square_o
        Glyph::Quote => "\u{f10d}",                  // nf-fa-quote_left
        Glyph::Spinner(i) => NF_SPINNER[usize::from(i) % NF_SPINNER.len()],
        Glyph::Prompt => "\u{f054}",   // nf-fa-chevron_right
        Glyph::Image => "\u{f03e}",    // nf-fa-picture_o
        Glyph::Footnote => "\u{f24a}", // nf-fa-sticky_note_o
        Glyph::Dir => "\u{f07b}",      // nf-fa-folder
        Glyph::File => "\u{f15b}",     // nf-fa-file
        Glyph::Lang(l) => lang_nerd(l),
    }
}

/// The glyph `g` drew before there was an icon set — the Unicode mark every
/// font has. `Lang` has none (the header is the bare label), `Image` is the
/// `[image]` tag and `Footnote` opens `[label]` (see [`footnote_mark`]).
pub(crate) fn fallback(g: Glyph) -> &'static str {
    match g {
        Glyph::Bullet1 => "\u{2022}",   // •
        Glyph::Bullet2 => "\u{25e6}",   // ◦
        Glyph::Bullet3 => "\u{25aa}",   // ▪
        Glyph::Checked => "\u{2713}",   // ✓
        Glyph::Unchecked => "\u{2610}", // ☐
        Glyph::Quote => "\u{258e}",     // ▎
        Glyph::Spinner(i) => ASCII_SPINNER[usize::from(i) % ASCII_SPINNER.len()],
        Glyph::DotOn => "\u{25cf}",  // ●
        Glyph::DotOff => "\u{25cb}", // ○
        Glyph::Prompt => "\u{276f}", // ❯
        Glyph::Image => "[image]",
        Glyph::Footnote => "[",
        Glyph::Dir => "\u{25b8}",  // ▸
        Glyph::File => "\u{00b7}", // ·
        Glyph::Lang(_) => "",
    }
}

/// What `g` draws as right now: the Nerd Font glyph when the set is on,
/// its fallback otherwise.
pub(crate) fn pick(g: Glyph) -> &'static str {
    if on() {
        nerd(g)
    } else {
        fallback(g)
    }
}

/// [`pick`] as one char, for the cell-at-a-time callers.
pub(crate) fn pick_char(g: Glyph) -> char {
    pick(g).chars().next().unwrap_or(' ')
}

/// The composer's prompt mark (`❯`, or a chevron icon).
pub(crate) fn prompt() -> char {
    pick_char(Glyph::Prompt)
}

/// The spinner frame for `now_ms`: eight pie slices on a Nerd Font, the
/// four ASCII strokes otherwise, either way stepping every 120ms.
pub(crate) fn spinner(now_ms: u64) -> &'static str {
    let frames = if on() {
        NF_SPINNER.len()
    } else {
        ASCII_SPINNER.len()
    };
    pick(Glyph::Spinner(((now_ms / 120) % frames as u64) as u8))
}

/// A fence header's text, clipped to `width`: `<icon> <label>` on a Nerd
/// Font, the bare label otherwise (`code` for an untagged fence).
pub(crate) fn fence_header(lang: &str, width: usize) -> String {
    let label = if lang.is_empty() { "code" } else { lang };
    let text = if on() {
        format!("{} {label}", nerd(Glyph::Lang(lang)))
    } else {
        label.to_string()
    };
    crate::chatwidth::clip_w(&text, width)
}

/// A `[^label]` reference's mark: `<icon>label` on a Nerd Font, `[label]`
/// otherwise.
pub(crate) fn footnote_mark(label: &str) -> String {
    if on() {
        format!("{}{label}", nerd(Glyph::Footnote))
    } else {
        format!("[{label}]")
    }
}

/// The dev-icon for a fence language, keyed on the info string's first word
/// (```rust,ignore and ```sh title=… still find theirs), case-insensitive.
fn lang_nerd(lang: &str) -> &'static str {
    let key = lang
        .split(|c: char| c == ',' || c.is_whitespace())
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match key.as_str() {
        "rust" | "rs" => "\u{e7a8}",                               // nf-dev-rust
        "python" | "py" => "\u{e606}",                             // nf-seti-python
        "js" | "javascript" | "jsx" => "\u{e74e}",                 // nf-dev-javascript
        "ts" | "typescript" | "tsx" => "\u{e628}",                 // nf-seti-typescript
        "go" | "golang" => "\u{e626}",                             // nf-seti-go
        "sh" | "bash" | "zsh" | "shell" | "console" => "\u{e795}", // nf-dev-terminal
        "toml" => "\u{e6b2}",                                      // nf-seti-toml
        "yaml" | "yml" => "\u{e6a8}",                              // nf-seti-yml
        "json" => "\u{e60b}",                                      // nf-seti-json
        "md" | "markdown" => "\u{f48a}",                           // nf-oct-markdown
        "sql" => "\u{e706}",                                       // nf-dev-database
        "diff" | "patch" => "\u{f440}",                            // nf-oct-diff
        _ => "\u{f121}",                                           // nf-fa-code
    }
}

#[cfg(test)]
#[path = "glyphs_tests.rs"]
pub(crate) mod tests;
#[cfg(test)]
pub(crate) use tests::force;
