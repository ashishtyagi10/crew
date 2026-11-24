//! The block memo is only worth having if it is indistinguishable from a whole render: every
//! test here compares it with [`crate::md::render`], line for line, span for span.
use super::*;
use crate::md::memo;
use crate::md::render as full;
use std::path::{Path, PathBuf};

pub(super) const WIDTHS: [usize; 3] = [24, 61, 100];

/// Every block kind the renderer draws, and the shapes that sit on a seam: a setext heading,
/// a table under a paragraph, an indented fence, a list with a fence hoisted out of it, a lazy
/// quote, an ordered list that starts at 7, HTML, a picture, and an empty quote — a block that
/// draws no rows and is still separated from its neighbours (first, where that shows).
pub(super) const MIXED: &str = ">\n\nTitle\n=====\n\nSome *prose* with `code` and a [link](http://x.y).\nsoft\nbreak\n\n\
| a | b |\n|---|--:|\n| 1 | 2 |\n\n    indented code\n    more\n\n```rust\nfn x() {}\n```\n\n\
7. seven\n8. eight\n   - nested\n     ```\n     hoisted\n     ```\n9. nine\n\n> quote\nlazy\n>\n> - in a quote\n\n\
---\n\n<details>\nhtml\n</details>\n\n- [ ] task\n- [x] done\n\n![a cat](cat.png)\n\n>\n\n## Two\nend\n";

/// Each `.md` the repository keeps, outside build output and dot-folders.
pub(super) fn repo_md() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let (p, name) = (e.path(), e.file_name().to_string_lossy().into_owned());
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            if p.is_dir() {
                walk(&p, out);
            } else if name.ends_with(".md") {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        &mut out,
    );
    out.sort();
    out
}

pub(super) fn manual() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/CREW.md");
    std::fs::read_to_string(p).expect("docs/CREW.md")
}

/// A seeded generator: the same edits every run, so a failure is one a person can replay.
pub(super) struct Lcg(pub u64);

impl Lcg {
    pub(super) fn next(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as usize) % n.max(1)
    }

    /// One random single-character edit of `text`, at a character boundary: an insert drawn
    /// mostly from the characters that move block boundaries, or a delete.
    pub(super) fn edit(&mut self, text: &mut String) {
        let bounds: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
        const INK: &[char] = &[
            '\n', '\n', '`', '#', '>', '-', '|', '[', ']', ':', '^', '1', '.', '*', ' ', ' ', '=',
            'a', 'é', '<', '!', '(', ')', '\t', '~', '_',
        ];
        if bounds.is_empty() || self.next(3) > 0 {
            let at = if bounds.is_empty() {
                0
            } else {
                bounds[self.next(bounds.len() + 1).min(bounds.len() - 1)]
            };
            let at = if self.next(50) == 0 { text.len() } else { at };
            text.insert(at, INK[self.next(INK.len())]);
        } else {
            text.remove(bounds[self.next(bounds.len())]);
        }
    }
}

#[test]
fn the_mixed_document_renders_alike_cold_and_warm() {
    let _f = crate::glyphs::force(false);
    assert!(
        memo::walk(MIXED).tops.is_some(),
        "MIXED must take the block path"
    );
    for w in WIDTHS {
        assert_eq!(render(MIXED, w), full(MIXED, w), "cold, width {w}");
        assert_eq!(render(MIXED, w), full(MIXED, w), "warm, width {w}");
    }
}

/// Random single-character edits — newlines, fences, `#`, `>`, `|`, `[^`, `]:` among them — and
/// after each one, the kept render equals a fresh one.
#[test]
fn random_edits_render_alike_after_every_edit() {
    let _f = crate::glyphs::force(false);
    let mut text = MIXED.repeat(3);
    let mut rng = Lcg(0x5eed);
    let mut block_path = 0;
    for i in 0..300 {
        rng.edit(&mut text);
        let w = WIDTHS[i % WIDTHS.len()];
        block_path += usize::from(memo::walk(&text).tops.is_some());
        assert_eq!(
            render(&text, w),
            full(&text, w),
            "edit {i}, width {w}: {text:?}"
        );
    }
    assert!(
        block_path > 100,
        "most edits took the block path ({block_path})"
    );
}

/// Typing a character into the middle of the manual renders one piece; every other piece is
/// copied.
#[test]
fn one_insert_renders_one_block() {
    let _f = crate::glyphs::force(false);
    let mut text = manual();
    assert!(
        memo::walk(&text).tops.is_some(),
        "the manual takes the block path"
    );
    render(&text, 80);
    // Halfway, stepped on to a char boundary: the manual is full of em dashes, and where its
    // middle byte lands moves with every edit and with CRLF checkouts (Windows CI).
    let half = (text.len() / 2..)
        .find(|&i| text.is_char_boundary(i))
        .unwrap();
    let mid = text[half..].find(" the ").expect("a word") + half;
    text.insert(mid + 1, 'x');
    let before = rendered();
    let got = render(&text, 80);
    assert_eq!(rendered() - before, 1, "one block rendered");
    assert_eq!(got, full(&text, 80));
}

/// The editor's own path — a render with the caret's block revealed — is the one that keeps
/// pieces: typing into one paragraph renders that paragraph.
#[test]
fn typing_with_a_revealed_block_renders_one_block() {
    let _f = crate::glyphs::force(false);
    let reveal = |t: &str| crate::md::reveal::block_at(t, 3);
    let mut text = format!("one\n\n{MIXED}");
    crate::md::render_revealed(&text, 50, reveal(&text));
    text.insert(1, 'x');
    let before = rendered();
    crate::md::render_revealed(&text, 50, reveal(&text));
    assert_eq!(rendered() - before, 1);
}

/// The icon set draws quote bars and bullets, so switching it renders every piece again.
#[test]
fn the_icon_set_is_part_of_the_key() {
    let text = "> quoted\n\n- item\n";
    let _a = crate::glyphs::force(false);
    assert_eq!(render(text, 40), full(text, 40));
    let _b = crate::glyphs::force(true);
    assert_eq!(render(text, 40), full(text, 40));
}

/// A footnote or a link reference definition ties blocks together: those documents are
/// rendered whole.
#[test]
fn a_block_that_reaches_another_renders_the_whole() {
    for text in [
        "see [it][r]\n\n[r]: http://x.y\n",
        "a note[^1]\n\n[^1]: the note\n",
    ] {
        assert!(memo::walk(text).tops.is_none(), "{text:?}");
        assert_eq!(render(text, 40), full(text, 40));
    }
}

#[path = "blockmemobench_tests.rs"]
mod bench;
