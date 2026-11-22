//! Off-screen shots of the live preview: the caret's block drawn as its markdown, one scene per
//! kind of block, everything around it rendered.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew doc_shot_reveal -- --ignored`
use crate::layout::Rect;

const MD: &str = "\
# Live preview

Some **bold** words, some *italic* ones, and a [link](https://example.invalid/page) to follow.

- a list item
- another, with `code` in it

> a quotation, which renders with a bar

```rust
fn main() {
    println!(\"hi\");
}
```

The last paragraph.
";

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn doc_shot_reveal() {
    let _g = crate::app::theme_test_guard();
    let (w, h) = (720u32, 460u32);
    let scenes = [
        ("reveal-none", None),
        ("reveal-heading", Some("preview")),
        ("reveal-bold-link", Some("words")),
        ("reveal-list-item", Some("another")),
        ("reveal-quote", Some("quotation")),
        ("reveal-fence", Some("println")),
    ];
    for (name, at) in scenes {
        let mut view = crate::docshot_tests::doc(MD);
        if let Some(needle) = at {
            view.start_editing(80);
            view.caret_at = Some(MD.find(needle).expect(needle) as u32);
            view.relayout_caret(80, 24);
        }
        let grid = crew_term::GridSize { cols: 80, rows: 24 };
        let legend = crate::docwin::legend::legend(&view, grid, None, None);
        let px = crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
            let m = 12.0;
            let rect = Rect {
                x: m,
                y: m,
                w: w as f32 - m * 2.0,
                h: h as f32 - m * 2.0,
            };
            crate::docwin::draw::scenes(rect, cw, ch, &legend, &view)
        });
        let Some(px) = px else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        crate::shotdraw_tests::write_png(name, &px, w, h);
    }
}
