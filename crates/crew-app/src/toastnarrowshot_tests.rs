//! The toast stack on a NARROW window. `toastshot_tests` shoots it at 900px,
//! where a 46-column card never wraps and a tile-wide window's `cols < 6`
//! branch never runs; this is the same stack at a quarter of that.
//!
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew toast_narrow -- --ignored`
use crate::layout::Rect;
use crate::toast::Toasts;

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn toast_narrow_shot_wraps_the_error() {
    let _g = crate::app::theme_test_guard();
    let (w, h) = (360u32, 260u32);
    let mut rows = Vec::new();
    let px = crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
        let mut toasts = Toasts::default();
        toasts.push(
            "error: failed to spawn shell: No such file or directory (os error 2)".into(),
            "error",
            true,
            1_000,
        );
        toasts.push("copied 3 lines".into(), "note", false, 1_000);
        let content = Rect {
            x: 0.0,
            y: 0.0,
            w: w as f32,
            h: h as f32,
        };
        let mut scenes = Vec::new();
        crate::toast::push_toasts(&mut scenes, &mut toasts, content, cw, ch, 1_400, None);
        for s in &scenes {
            let cols = (s.w / cw).round() as u16;
            let n = (s.h / ch).round() as u16;
            rows.extend(crate::goalshot_tests::dump(&s.cells, cols, n));
        }
        scenes
    });
    let Some(px) = px else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    crate::shotdraw_tests::write_png("toast-narrow", &px, w, h);
    for r in &rows {
        eprintln!("|{r}");
    }
    let all = rows.join("\n");
    assert!(
        all.contains("(os error 2)"),
        "the tail of the error is on the card:\n{all}"
    );
    assert!(all.contains("copied 3 lines"), "{all}");
}
