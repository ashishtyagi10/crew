use super::Glyph::*;
use super::*;
use std::cell::Cell;
use unicode_width::UnicodeWidthStr;

thread_local! {
    /// A test's override of [`on`], per thread so parallel tests cannot see
    /// each other's setting. Cleared when the [`Forced`] guard drops.
    static FORCE: Cell<Option<bool>> = const { Cell::new(None) };
}

/// The calling thread's override of [`on`], if a [`force`] guard is live.
pub(crate) fn forced() -> Option<bool> {
    FORCE.with(|f| f.get())
}

/// Holds a test's [`force`] until dropped.
pub(crate) struct Forced;

impl Drop for Forced {
    fn drop(&mut self) {
        FORCE.with(|f| f.set(None));
    }
}

/// Pin [`on`] for the calling test thread until the guard drops.
pub(crate) fn force(on: bool) -> Forced {
    FORCE.with(|f| f.set(Some(on)));
    Forced
}

/// Every mark: a glyph added without a row in both tables fails here.
fn all() -> Vec<Glyph<'static>> {
    let mut v = vec![
        Bullet1, Bullet2, Bullet3, Checked, Unchecked, Quote, DotOn, DotOff, Prompt, Image,
        Footnote, Dir, File, Hash, Pass, Fail, Tool, ToolOpen, Play,
    ];
    v.extend((0..8).map(Spinner));
    v.extend((0..2).map(Hourglass));
    let langs = [
        "rust", "python", "js", "ts", "go", "sh", "toml", "yaml", "json", "md", "sql",
    ];
    v.extend(langs.into_iter().chain(["diff", "brainfuck", ""]).map(Lang));
    v
}

/// Unicode Private Use: the BMP block and both supplementary planes.
fn is_pua(c: char) -> bool {
    matches!(u32::from(c), 0xE000..=0xF8FF | 0xF0000..=0xFFFFD | 0x100000..=0x10FFFD)
}

#[test]
fn every_nerd_glyph_is_one_pua_codepoint_one_cell_wide() {
    for g in all() {
        let s = nerd(g);
        let c = s.chars().next().expect("a glyph");
        assert_eq!(s.chars().count(), 1, "{g:?}: more than one char: {s:?}");
        assert!(
            is_pua(c),
            "{g:?}: U+{:04X} is not Private Use",
            u32::from(c)
        );
        assert_eq!(s.width(), 1, "{g:?}: not one cell wide");
    }
}

#[test]
fn every_fallback_is_the_plain_glyph_the_pane_always_drew() {
    for g in all() {
        let s = fallback(g);
        assert!(
            !s.chars().any(is_pua),
            "{g:?}: fallback {s:?} needs a Nerd Font"
        );
        match g {
            Lang(_) => assert_eq!(s, "", "a fence header falls back to its bare label"),
            Image => assert_eq!(s, "[image]"),
            _ => assert_eq!(s.width(), 1, "{g:?}: fallback {s:?} is not one cell"),
        }
    }
    let was = [
        Bullet1, Bullet2, Checked, Unchecked, Quote, Prompt, DotOn, DotOff,
    ]
    .map(fallback);
    assert_eq!(was, ["•", "◦", "✓", "☐", "▎", "❯", "●", "○"]);
}

#[test]
fn the_set_is_off_for_the_embedded_face_and_unknown_families() {
    // No override: the real switch, driven by coverage.
    set_family(None);
    assert!(!on(), "Lilex ships no icons");
    set_family(Some("No Such Family 77a0"));
    assert!(!on());
    assert_eq!(pick(Prompt), "❯");
    assert_eq!(pick_char(Quote), '▎');
}

#[test]
fn pick_follows_the_switch() {
    let on = force(true);
    assert_eq!(pick(Bullet1), "\u{f111}");
    assert_eq!(pick_char(Prompt), '\u{f054}');
    drop(on);
    let _off = force(false);
    assert_eq!(pick(Bullet1), "•");
}

#[test]
fn the_spinner_has_eight_frames_on_a_nerd_font_and_four_ascii_ones_off() {
    let on = force(true);
    let nf: Vec<&str> = (0..8).map(|i| spinner(i * 120)).collect();
    assert!(nf.iter().all(|f| f.chars().all(is_pua)));
    assert_eq!(nf.iter().collect::<std::collections::HashSet<_>>().len(), 8);
    assert_eq!(spinner(8 * 120), nf[0], "wraps after eight frames");
    drop(on);
    let _off = force(false);
    let ascii: Vec<&str> = (0..5).map(|i| spinner(i * 120)).collect();
    assert_eq!(ascii, ["|", "/", "-", "\\", "|"]);
}

#[test]
fn fence_header_is_icon_space_label_on_and_bare_label_off() {
    let on = force(true);
    assert_eq!(fence_header("rust", 40), "\u{e7a8} rust");
    assert_eq!(fence_header("", 40), "\u{f121} code");
    assert_eq!(fence_header("RUST,ignore", 40), "\u{e7a8} RUST,ignore");
    assert_eq!(
        fence_header("rust", 3 + 4),
        "\u{e7a8} \u{2026}",
        "clipped to width less the badge's four chrome cells, cut marked"
    );
    drop(on);
    let _off = force(false);
    assert_eq!(fence_header("rust", 40), "rust");
    assert_eq!(fence_header("", 40), "code");
}

#[test]
fn footnote_mark_is_bracketed_off_and_iconed_on() {
    let off = force(false);
    assert_eq!(footnote_mark("1"), "[1]");
    drop(off);
    let _on = force(true);
    assert_eq!(footnote_mark("1"), "\u{f24a}1");
}

/// The chat card, end to end: the fence header badge holds the rust icon when
/// the set is on (powerline caps), the bare label when off (half-block caps).
#[test]
fn a_chat_fence_header_row_leads_with_the_language_icon_when_on() {
    let header = || -> String {
        let md = crate::md::render_chat("```rust\nfn x() {}\n```", 40);
        crate::chatmd::map_lines(md, 40, (9, 9, 9))[0]
            .iter()
            .map(|c| c.c)
            .collect()
    };
    let on = force(true);
    assert!(
        header().starts_with("  \u{e0b6} \u{e7a8} rust \u{e0b4}"),
        "on: {:?}",
        header()
    );
    drop(on);
    let _off = force(false);
    assert!(header().starts_with("  ▐ rust ▌"), "off: {:?}", header());
}

/// The one door: every family the renderer is given passes through
/// `apply_family`, so the switch can never describe the previous font.
#[test]
fn no_set_font_family_call_outside_glyphs() {
    let mut stack = vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
    let mut hits = Vec::new();
    while let Some(dir) = stack.pop() {
        for path in std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()) {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if path.is_dir() {
                stack.push(path);
            } else if name.ends_with(".rs")
                && name != "glyphs.rs"
                && !name.ends_with("_tests.rs")
                && std::fs::read_to_string(&path)
                    .unwrap()
                    .contains(".set_font_family(")
            {
                hits.push(path);
            }
        }
    }
    assert!(
        hits.is_empty(),
        "set the family via glyphs::apply_family: {hits:?}"
    );
}
