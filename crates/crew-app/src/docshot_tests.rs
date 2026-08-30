//! Off-screen render of a DOCUMENT WINDOW — the surface that, by definition,
//! only exists on a real display.
//!
//! A window's picture cannot be photographed by the harness the panes use,
//! because the harness has no window. What it can photograph is everything
//! that decides the picture: the rect the frame is laid into, the legend on
//! its top border, and the document filling it. `docwin::draw` is a wgpu call
//! around exactly that, so a shot of these scenes at a window's proportions is
//! a shot of the window.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew doc_shot -- --ignored`
use crate::layout::Rect;
use crate::viewpane::detect::Format;
use crate::viewpane::load::Loaded;
use crate::viewpane::{LoadState, ViewPane};

const MD: &str = "\
# The document window

A document wants a window you can put on the other screen, size to a
comfortable measure, and leave open while the grid goes on being a grid.

## What it holds

- one file, framed, filling it
- no nav, no input bar, no tiles
- the legend names the file and how far through it you are

> It is a second *surface*, not a second app: one process, one broker, one
> theme, one font.

```rust
pub(crate) fn scenes(rect: Rect, cw: f32, ch: f32) -> Vec<PaneScene> {
    // the same card every pane is drawn in
}
```

| key | what it does          |
|-----|-----------------------|
| `w` | pop out of the grid   |
| `/` | search the document   |
| Esc | close the window      |
";

/// A document that NAMES a picture, beside the file it names — the shape a
/// README has, and the one path `imgcache::resolve` walks.
fn illustrated() -> (ViewPane, std::path::PathBuf) {
    let dir = std::env::temp_dir().join("docshot-illustrated");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let img = dir.join("chart.png");
    let px = image::RgbaImage::from_fn(320, 160, |x, y| {
        let (fx, fy) = (x as f32 / 320.0, y as f32 / 160.0);
        let bar = ((fx * 7.0) as u32).is_multiple_of(2);
        let h = 0.25 + ((fx * 7.0) as u32 as f32 % 5.0) * 0.14;
        match (bar, fy > 1.0 - h) {
            (true, true) => image::Rgba([90, 190, 200, 255]),
            _ => image::Rgba([28, 30, 34, 255]),
        }
    });
    image::DynamicImage::ImageRgba8(px)
        .save_with_format(&img, image::ImageFormat::Png)
        .expect("write");
    let doc = dir.join("README.md");
    // Written line by line rather than as one indented literal: leading
    // whitespace in a continued Rust string is CONTENT, and four spaces of it
    // in markdown is a code block — which is what the first take of this
    // fixture accidentally photographed.
    let text = [
        "# crew",
        "",
        "A native GPU terminal for AI workflows.",
        "",
        "![weekly usage](chart.png)",
        "",
        "The chart above is drawn on the paint layer, under the text, from a",
        "file the document names.",
        "",
    ]
    .join("\n");
    let text = text.as_str();
    let mut p = doc_at(doc, text);
    p.scroll = 0;
    (p, img)
}

fn doc(text: &str) -> ViewPane {
    doc_at(std::env::temp_dir().join("window.md"), text)
}

fn doc_at(path: std::path::PathBuf, text: &str) -> ViewPane {
    let mut p = ViewPane::open(path);
    p.state = LoadState::Ready {
        format: Format::Markdown,
        loaded: Loaded {
            text: text.into(),
            truncated: None,
            meta: None,
            image: None,
        },
    };
    p
}

/// A document that names a picture, with the picture in it.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn doc_shot_illustrated() {
    let _g = crate::app::theme_test_guard();
    let (view, img) = illustrated();
    // The read is on a worker, as it is in the app; a shot waits for it
    // rather than photographing a document mid-load.
    for _ in 0..400 {
        if crate::imgcache::get(&img).is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let (w, h) = (720u32, 620u32);
    let px = crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
        let m = 12.0;
        let rect = Rect {
            x: m,
            y: m,
            w: w as f32 - m * 2.0,
            h: h as f32 - m * 2.0,
        };
        crate::docwin::draw::scenes(rect, cw, ch, "README.md", &view)
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    crate::shotdraw_tests::write_png("doc-illustrated", &px, w, h);
    assert!(crate::shotgpu_tests::ink(&px) > 3_000);

    // Scrolled so the picture is half off the top: paint is free rectangles
    // and nothing else clips it, so this is the frame that says whether it is
    // cut to the pane or drawn over the frame above it.
    let mut half = view;
    half.scroll = 8;
    let px = crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
        let m = 12.0;
        let rect = Rect {
            x: m,
            y: m,
            w: w as f32 - m * 2.0,
            h: h as f32 - m * 2.0,
        };
        crate::docwin::draw::scenes(rect, cw, ch, "README.md \u{00b7} 60%", &half)
    });
    if let Some(px) = px {
        crate::shotdraw_tests::write_png("doc-illustrated-cut", &px, w, h);
    }
}

/// Scrolled into a section: the heading you are underneath, on the top row.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn doc_shot_sticky_heading() {
    let _g = crate::app::theme_test_guard();
    let mut body = String::from("# crew\n\nthe manual.\n\n## Themes\n\n");
    for i in 0..6 {
        body.push_str(&format!("A line of prose about themes, number {i}.\n\n"));
    }
    body.push_str("### CRT\n\n");
    for i in 0..14 {
        body.push_str(&format!("The tube does this, item {i}.\n\n"));
    }
    let mut view = doc(&body);
    view.scroll = 26;
    let (w, h) = (720u32, 460u32);
    let px = crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
        let m = 12.0;
        let rect = Rect {
            x: m,
            y: m,
            w: w as f32 - m * 2.0,
            h: h as f32 - m * 2.0,
        };
        crate::docwin::draw::scenes(rect, cw, ch, "CREW.md \u{00b7} 41%", &view)
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    crate::shotdraw_tests::write_png("doc-sticky", &px, w, h);
    assert!(crate::shotgpu_tests::ink(&px) > 3_000);
}

/// The document window as an EDITOR: the render, with a cursor in it. No
/// markers on screen, and the caret standing on a character of the document
/// rather than on the bullet beside it.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn doc_shot_caret() {
    let _g = crate::app::theme_test_guard();
    let (w, h) = (720u32, 460u32);
    for (name, steps) in [("doc-caret", 0usize), ("doc-caret-in-a-list", 120)] {
        let mut view = doc(MD);
        view.start_editing(80);
        for _ in 0..steps {
            view.move_caret(crate::viewpane::caret::Step::Right, 80, 24);
        }
        let px = crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
            let m = 12.0;
            let rect = Rect {
                x: m,
                y: m,
                w: w as f32 - m * 2.0,
                h: h as f32 - m * 2.0,
            };
            crate::docwin::draw::scenes(rect, cw, ch, "window.md", &view)
        });
        let Some(px) = px else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        crate::shotdraw_tests::write_png(name, &px, w, h);
        assert!(crate::shotgpu_tests::ink(&px) > 3_000, "{name} drew");
    }
}

/// The document window on a light page and through a green tube — the two
/// places a surface built and eyed on the dark theme goes wrong.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn doc_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let mut view = doc(MD);
    view.scroll = 6;
    for (suffix, id) in [
        ("light", crew_theme::ThemeId::PaperLight),
        ("crt-green", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let (w, h) = (720u32, 560u32);
        let px = crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
            let m = 12.0;
            let rect = Rect {
                x: m,
                y: m,
                w: w as f32 - m * 2.0,
                h: h as f32 - m * 2.0,
            };
            crate::docwin::draw::scenes(rect, cw, ch, "window.md \u{00b7} 22%", &view)
        });
        let Some(px) = px else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        crate::shotdraw_tests::write_png(&format!("doc-{suffix}"), &px, w, h);
        assert!(crate::shotgpu_tests::ink(&px) > 3_000, "doc-{suffix} drew");
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

/// A document window at the proportions it opens at (a reading measure, taller
/// than wide) and at a shape somebody has dragged it into.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn doc_shot_window() {
    let _g = crate::app::theme_test_guard();
    let view = doc(MD);
    for (name, w, h) in [
        ("doc-window", 720u32, 900u32),
        ("doc-window-wide", 1100, 620),
        ("doc-window-narrow", 460, 760),
    ] {
        let px = crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
            let m = 12.0;
            let rect = Rect {
                x: m,
                y: m,
                w: w as f32 - m * 2.0,
                h: h as f32 - m * 2.0,
            };
            crate::docwin::draw::scenes(rect, cw, ch, "window.md \u{00b7} 34%", &view)
        });
        let Some(px) = px else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        crate::shotdraw_tests::write_png(name, &px, w, h);
        assert!(
            crate::shotgpu_tests::ink(&px) > 3_000,
            "{name} drew a document"
        );
    }
}
