//! Offscreen shot of a composer pop-up in a crew pane — the model picker
//! open over `/model claude` — placed by `popupplace::above_composer`, and
//! the same frame placed the old way (composer rows subtracted, footer
//! rows not) for the eye to compare. `#[ignore]`d: needs a GPU adapter.
//! `cargo test -p crew-app --bin crew popup_shot -- --ignored --nocapture`;
//! PNGs land in `$CREW_SHOT_DIR` (default `target/screenshots`).
use crate::chat::ChatPane;
use crate::layout::Rect;
use crew_plugin::Plugin;
use crew_render::PaneScene;

const W: u32 = 900;
const H: u32 = 520;

fn pane() -> ChatPane {
    let plugin = Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.input = "/model claude".into();
    p
}

/// The frame: the pane, then the picker card as an overlay at `y`.
fn frame(
    p: &ChatPane,
    cw: f32,
    ch: f32,
    place: impl Fn(&ChatPane, Rect, f32) -> f32,
) -> (Vec<PaneScene>, f32) {
    let r = Rect {
        x: 0.0,
        y: 0.0,
        w: W as f32,
        h: H as f32,
    };
    let cols = (r.w / cw).floor() as u16;
    let rows = (r.h / ch).floor() as u16;
    let items = crate::menushot_tests::models();
    let mr = crate::cmdmenu::menu_rows(items.len());
    let mh = f32::from(mr) * ch;
    let y = place(p, r, mh);
    let scenes = vec![
        PaneScene {
            cells: p.cells(cols, rows),
            x: r.x,
            y: r.y,
            w: r.w,
            h: r.h,
            focused: true,
            bordered: false,
            glass: false,
            scan: -1.0,
            overlay: false,
            paint: Vec::new(),
        },
        PaneScene {
            cells: crate::cmdmenu::menu_card("model", &items, 1, cols, mr),
            x: r.x,
            y,
            w: r.w,
            h: mh,
            focused: false,
            bordered: false,
            glass: false,
            scan: -1.0,
            overlay: true,
            paint: Vec::new(),
        },
    ];
    (scenes, y + mh)
}

/// Whether any pixel in rows `y0..y1` of the left `x1` px differs from the
/// page: the composer's `❯` prompt lives there.
fn inked(px: &[u8], y0: u32, y1: u32, x1: u32) -> bool {
    let page = &px[..4];
    (y0..y1.min(H)).any(|y| {
        (0..x1).any(|x| {
            let i = ((y * W + x) * 4) as usize;
            let d = |k: usize| (px[i + k] as i32 - page[k] as i32).abs();
            d(0) + d(1) + d(2) > 60
        })
    })
}

/// New placement: the pop-up's bottom edge meets the composer's top, so
/// the prompt row under it still shows its `❯`. Old placement: the same
/// card sits `summary` rows lower, on the composer.
#[test]
#[ignore]
fn popup_shot_stands_on_the_composer() {
    let p = pane();
    let mut bottom = 0.0;
    let mut ch_px = 0.0;
    let px = crate::shotdraw_tests::draw(W, H, 13.0, |cw, ch| {
        ch_px = ch;
        let (scenes, b) = frame(&p, cw, ch, |p, r, mh| {
            crate::popupplace::above_composer(p, r, cw, ch, mh)
        });
        bottom = b;
        scenes
    })
    .expect("gpu adapter");
    crate::shotdraw_tests::write_png("popup-on-composer", &px, W, H);
    // The composer's bordered card: top border row, then the `❯` row.
    let prompt_row = bottom + ch_px;
    assert!(
        inked(
            &px,
            prompt_row as u32,
            (prompt_row + ch_px) as u32,
            (ch_px * 3.0) as u32
        ),
        "the composer prompt is hidden under the pop-up"
    );

    let old = crate::shotdraw_tests::draw(W, H, 13.0, |cw, ch| {
        frame(&p, cw, ch, |p, r, mh| {
            let cols = (r.w / cw).floor() as u16;
            let comp = f32::from(crate::chatinput::composer_rows(
                &p.input,
                cols,
                (r.h / ch).floor() as u16,
            )) * ch;
            (r.y + r.h - comp - mh).max(0.0)
        })
        .0
    })
    .expect("gpu adapter");
    crate::shotdraw_tests::write_png("popup-old-placement", &old, W, H);
}
