//! Off-screen terminal lines of programmer ligatures: on, off, and with the
//! cursor inside `=>`. `#[ignore]`d (GPU + PNG):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew lig_shot -- --ignored`
use std::sync::atomic::{AtomicBool, Ordering};

use crew_term::{GridSize, HeadlessTerm, TermModel};

use crate::session::to_cellviews;
use crate::shotgpu_tests::shot_at;

/// Read by `shotfont_tests::apply`: shoot with `font_ligatures` off.
pub static OFF: AtomicBool = AtomicBool::new(false);

/// The ruler row under it is what shows the grid held: `f` must sit over
/// the same column on both lines.
const LINE: &str = "a => b != c -> d === e <= f >= g |> h";
const RULER: &str = "0123456789012345678901234567890123456";

fn shoot(name: &str, cursor_col: Option<u16>) {
    // Cursor addressing is 1-based; park it where the shot wants it.
    let park = cursor_col.map_or(String::new(), |c| format!("\x1b[1;{c}H"));
    let body = format!("{LINE}\r\n{RULER}{park}");
    shot_at(name, 640, 120, 13.0, "zsh", |c, r, _| {
        let mut term = HeadlessTerm::new(GridSize { cols: c, rows: r });
        term.feed(body.as_bytes());
        (to_cellviews(&term.cells(true)), Vec::new())
    });
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn lig_shot_terminal() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    crew_theme::set_theme(crew_theme::ThemeId::PaperLight);
    OFF.store(false, Ordering::Relaxed);
    shoot("lig-on", None);
    // Col 4 is the `>` of `=>`: the caret sits inside the ligature.
    shoot("lig-cursor", Some(4));
    OFF.store(true, Ordering::Relaxed);
    shoot("lig-off", None);
    OFF.store(false, Ordering::Relaxed);
}
