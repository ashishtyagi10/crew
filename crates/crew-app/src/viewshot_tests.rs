//! Off-screen render of the file VIEWER — the pane `/view`, Cmd+click on a
//! `path:line` and every diff review open into.
//!
//! It is the largest surface in the app (a source rung, a markdown rung, a
//! data rung, a CSV table, a unified diff, a side-by-side diff, a blame
//! gutter, an outline, a live `/` search) and the only one of that size with
//! no picture of itself. Every rung has unit tests over its `CardLine`s;
//! none of them says what the pane LOOKS like with the gutter, the wrap and
//! the syntax ladder all in the same frame.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew view_shot -- --ignored`
use crate::shotgpu_tests::shot_at;
use crate::viewpane::detect::Format;
use crate::viewpane::load::Loaded;
use crate::viewpane::{LoadState, ViewPane};

const H: u32 = 620;

fn pane(name: &str, format: Format, text: &str) -> ViewPane {
    let mut p = ViewPane::open(std::env::temp_dir().join(name));
    p.state = LoadState::Ready {
        format,
        loaded: Loaded {
            text: text.into(),
            truncated: None,
            meta: None,
        },
    };
    p
}

const RUST: &str = "\
//! The signed distance field the dial's scale is drawn from.
use crate::plot::sdf;

/// Coverage for one sample: negative is inside, positive is out.
pub(crate) fn coverage(d: f32, scale: f32) -> f32 {
    (0.5 - d * scale).clamp(0.0, 1.0)
}

fn arc(p: (f32, f32), r: f32, half_w: f32, a0: f32, a1: f32) -> f32 {
    // A round-capped arc: the caps are geometry, not stamped dots, which is \
why an eleven-tick face costs the same as a three-tick one.
    let mid = (a0 + a1) * 0.5;
    sdf::capsule(p, r, half_w, mid)
}
";

const DIFF: &str = "\
diff --git a/crates/crew-app/src/plot/dial.rs b/crates/crew-app/src/plot/dial.rs
index 3f2a1b0..9c4d7e1 100644
--- a/crates/crew-app/src/plot/dial.rs
+++ b/crates/crew-app/src/plot/dial.rs
@@ -18,9 +18,11 @@ pub(crate) struct Dial {
     pub centre: (f32, f32),
     pub r: f32,
-    /// Ticks around the scale.
-    pub ticks: usize,
+    /// Ticks around the scale. A face four columns wide or more gets
+    /// twenty-one of them; anything narrower gets eleven.
+    pub ticks: usize,
+    /// The hand's own half-width at the hub, capped so a big instrument's
+    /// hand reads slimmer rather than merely larger.
+    pub hub_w: f32,
 }
";

const MD: &str = "\
# The dial

The scale runs 240 degrees, from eight o'clock to four.

- lit ticks, drawn as **cones off the pivot**
- the digits go in the open bottom third
- `fill_sdf` samples coverage, not a predicate

```rust
let cov = (0.5 - d * scale).clamp(0.0, 1.0);
```

| face | ticks | quads |
| ---- | ----- | ----- |
| nav  | 11    | 380   |
| dash | 21    | 1490  |
";

const CSV: &str = "\
theme,focused vs unfocused,page floor
paper-dark,8.33,2.01
crt-green,6.28,2.41
harbor,2.77,2.00
fern,1.60,2.00
";

fn blame_lines(n: usize) -> Vec<crate::viewpane::blame::Line> {
    // Runs, the way real blame comes back: the gutter labels BOUNDARIES, so
    // a fixture that changed commit every line would show a label on every
    // row and hide the one thing the column is for.
    let who = [
        ("3f2a1b0", "ashish"),
        ("9c4d7e1", "claude"),
        ("a10ff32", "ashish"),
    ];
    (0..n)
        .map(|i| {
            let (sha, author) = who[(i / 5) % who.len()];
            crate::viewpane::blame::Line {
                sha: sha.into(),
                author: author.into(),
            }
        })
        .collect()
}

fn view_shot(name: &str, p: &ViewPane, w: u32) -> Option<Vec<u8>> {
    shot_at(name, w, H, 13.0, "dial.rs", |cols, rows, _| {
        (p.cells(cols, rows), Vec::new())
    })
}

/// Each rung at the width a tile actually gets. A viewer that only reads on
/// a full window is a viewer nobody can put beside the pane they are editing.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn view_shot_every_rung() {
    let _g = crate::app::theme_test_guard();
    let rungs = [
        ("view-code", Format::Code { lang: "rust" }, RUST),
        ("view-diff", Format::Diff, DIFF),
        ("view-md", Format::Markdown, MD),
        ("view-csv", Format::Csv { delim: ',' }, CSV),
    ];
    for (name, format, text) in rungs {
        let p = pane("v.rs", format, text);
        for (suffix, w) in [("", 900u32), ("-narrow", 480)] {
            let Some(px) = view_shot(&format!("{name}{suffix}"), &p, w) else {
                eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
                return;
            };
            assert!(crate::shotgpu_tests::ink(&px) > 3000, "{name}{suffix} drew");
        }
    }
}

/// The two rungs that add a COLUMN to the layout rather than a colour: the
/// blame gutter (which the text is wrapped around, not decorated with) and
/// the side-by-side review.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn view_shot_gutter_and_split() {
    let _g = crate::app::theme_test_guard();
    let mut blamed = pane("v.rs", Format::Code { lang: "rust" }, RUST);
    blamed.blame = crate::viewpane::blamejob::Blame::On(blame_lines(64));
    let mut split = pane("v.diff", Format::Diff, DIFF);
    split.split = true;
    for (name, p, w) in [
        ("view-blame", &blamed, 900u32),
        ("view-blame-narrow", &blamed, 480),
        ("view-split", &split, 900),
        ("view-split-narrow", &split, 480),
    ] {
        let Some(px) = view_shot(name, p, w) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 3000, "{name} drew");
    }
}

/// The syntax ladder is the viewer's whole readability story, and it is the
/// one thing a single-phosphor tube has to tell with lightness alone.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn view_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let p = pane("v.rs", Format::Code { lang: "rust" }, RUST);
    let d = pane("v.diff", Format::Diff, DIFF);
    for (name, id) in [
        ("view-light", crew_theme::ThemeId::PaperLight),
        ("view-crt-green", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let Some(px) = view_shot(name, &p, 900) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 3000, "{name} drew");
        view_shot(&format!("{name}-diff"), &d, 900);
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
