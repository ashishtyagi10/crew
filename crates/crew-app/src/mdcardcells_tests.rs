//! The markdown CARD's cell-level guards: what `mdcardshot_tests`' PNGs are
//! meant to show, asserted on the cells so a regression fails in CI and not
//! only in somebody's eye. One fixture holds every construct the smith pane
//! learned to draw — the h1 badge and its rule, an h2 and h3, a fence's
//! language badge beside its token hues, a task list inside an ordered one,
//! a table that WRAPS, a quote, a strike, a footnote, a bare `www.` link and
//! a picture — and the shot tests render the same `DOC`.
use crate::chatbody::{body_lines, CardLine};

/// Every construct the card draws, once each.
pub(crate) const DOC: &str = "# Release notes

## What changed

### Pictures and tables

```rust
#[derive(Debug)]
fn main() -> Result<(), Error> {
    let n: u32 = 42;
    println!(\"{}\", n);
}
```

1. read the frame
   - [x] shoot the surface
   - [ ] look at the shot
2. lay it out

| renderer | glyph cache | notes |
|---|---:|---|
| ghostty | atlas | metal and opengl, the fastest of the three by a wide margin |
| wezterm | shaper | lua config, multiplexer built in |

> A pane that draws nothing at one size is blank on somebody's screen.

This was ~~struck~~ and footnoted[^1]; see www.example.com for more.

![the screenshot](shot.png)

[^1]: the note.
";

fn text(l: &CardLine) -> String {
    l.iter().map(|c| c.c).collect()
}

fn width(l: &CardLine) -> usize {
    l.iter().map(|c| crate::chatwidth::char_w(c.c)).sum()
}

/// The display columns the `│` separators of `row` stand on.
fn seps(l: &CardLine) -> Vec<usize> {
    let mut at = 0;
    let mut out = Vec::new();
    for c in l {
        if c.c == '\u{2502}' {
            out.push(at);
        }
        at += crate::chatwidth::char_w(c.c);
    }
    out
}

fn card(cols: usize) -> Vec<CardLine> {
    body_lines(DOC, cols, crew_theme::theme().ink, false)
}

fn row_with<'a>(lines: &'a [CardLine], needle: &str) -> (usize, &'a CardLine) {
    lines
        .iter()
        .enumerate()
        .find(|(_, l)| text(l).contains(needle))
        .unwrap_or_else(|| panic!("no row holds {needle:?}"))
}

#[test]
fn no_row_is_wider_than_the_card() {
    let _g = crate::app::theme_test_guard();
    for cols in [40, 60, 100] {
        for l in card(cols) {
            assert!(width(&l) <= cols, "{cols}: {:?} is {}", text(&l), width(&l));
        }
    }
}

/// A cap is the badge's rounded end: a half block in the badge's colour on
/// the ground the badge sits on. On a field (the fence badge) the ground
/// is the field's `bg`; on the page (the h1 badge) the ground is the page —
/// `bg: None` by design — and the cap's own ink must then be the block's
/// colour, or it is a stray glyph beside a block rather than its end.
#[test]
fn every_badge_cap_sits_on_its_ground() {
    let _g = crate::app::theme_test_guard();
    let caps = [
        crate::segment::LEFT_CAP,
        crate::segment::RIGHT_CAP,
        crate::segment::LEFT_CAP_NERD,
        crate::segment::RIGHT_CAP_NERD,
    ];
    let lines = card(60);
    let mut n = 0;
    for l in &lines {
        for (i, c) in l.iter().enumerate() {
            if !caps.contains(&c.c) {
                continue;
            }
            n += 1;
            let block =
                match c.c == crate::segment::LEFT_CAP || c.c == crate::segment::LEFT_CAP_NERD {
                    true => l.get(i + 1),
                    false => l.get(i.wrapping_sub(1)),
                }
                .and_then(|b| b.bg);
            assert!(
                c.bg.is_some() || block == Some(c.fg),
                "cap {:?} in {:?} has no ground: bg {:?}, ink {:?}, block {:?}",
                c.c,
                text(l),
                c.bg,
                c.fg,
                block
            );
        }
    }
    assert_eq!(n, 4, "the h1 badge and the fence badge: two caps each");
}

#[test]
fn every_fence_body_cell_sits_on_the_code_field() {
    let _g = crate::app::theme_test_guard();
    let lines = card(60);
    let field = Some(crate::chatink::code_bg());
    for needle in ["let n: u32", "println!", "#[derive"] {
        let (_, l) = row_with(&lines, needle);
        assert!(
            l[1..].iter().all(|c| c.bg == field || c.bg.is_some()),
            "{needle}: {:?}",
            l.iter().map(|c| c.bg).collect::<Vec<_>>()
        );
        assert!(l[1..].iter().filter(|c| c.bg == field).count() > 10);
    }
    let (_, badge) = row_with(&lines, " rust ");
    assert!(
        badge[1..].iter().all(|c| c.bg.is_some()),
        "the badge row is field"
    );
}

#[test]
fn the_h1_rule_is_exactly_as_wide_as_the_heading_row() {
    let _g = crate::app::theme_test_guard();
    let lines = card(60);
    let (i, h1) = row_with(&lines, "Release notes");
    let rule = &lines[i + 1];
    assert!(
        rule[1..].iter().all(|c| c.c == '\u{2500}'),
        "{:?}",
        text(rule)
    );
    assert_eq!(
        width(rule),
        width(h1),
        "{:?} under {:?}",
        text(rule),
        text(h1)
    );
}

#[test]
fn table_separators_align_across_header_body_and_continuation_rows() {
    let _g = crate::app::theme_test_guard();
    let lines = card(40);
    let (_, header) = row_with(&lines, "renderer");
    let ruled: Vec<&CardLine> = lines
        .iter()
        .filter(|l| text(l).contains('\u{2502}'))
        .collect();
    assert!(ruled.len() > 3, "the notes cell wraps: {}", ruled.len());
    for l in &ruled {
        assert_eq!(seps(l), seps(header), "{:?}", text(l));
    }
}

#[test]
fn the_image_row_carries_the_source_as_its_link() {
    let _g = crate::app::theme_test_guard();
    let lines = card(60);
    let (_, l) = row_with(&lines, "the screenshot");
    assert!(l[1..].iter().all(|c| c.link.as_deref() == Some("shot.png")));
}
