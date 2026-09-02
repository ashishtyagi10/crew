//! Off-screen render of the nav column's transients — the states the left
//! column passes through and nobody can hold still: the UPDATE card at every
//! stage of a `/update`, the parked-update legend, every marker a PANES row
//! can wear at once, and the ghost of a closing card mid-collapse.
//!
//! The nav is the narrowest surface in the app, so every one of these is a
//! width bug waiting to happen; `sidebarshot` draws one steady session and
//! none of them.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew transient_shot -- --ignored --nocapture`
use crate::goalshot_tests::dump;
use crate::panelist::PaneRow;
use crate::shotgpu_tests::shot_at;
use crate::update::{Stage, UpdateState};

/// The nav's default width in logical pixels (`Settings → Nav width`).
const NAV_W: u32 = 210;

fn card_shot(
    name: &str,
    legend: &str,
    w: u32,
    h: u32,
    cells: impl Fn(u16, u16) -> Vec<crew_render::CellView>,
) -> Option<Vec<String>> {
    let mut dumped = Vec::new();
    shot_at(
        &format!("transient-{name}"),
        w,
        h,
        13.0,
        legend,
        |cols, rows, _| {
            let c = cells(cols, rows);
            dumped = dump(&c, cols, rows);
            eprintln!("--- transient-{name} {cols}x{rows}");
            for l in &dumped {
                eprintln!("|{l}");
            }
            (c, Vec::new())
        },
    )?;
    Some(dumped)
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn transient_shot_update_card() {
    let _g = crate::app::theme_test_guard();
    let stages = [
        ("checking", Stage::Checking, NAV_W),
        ("downloading", Stage::Downloading("0.21.3".into()), NAV_W),
        ("done", Stage::Done("0.21.3".into()), NAV_W),
        ("uptodate", Stage::Note("already up to date".into()), NAV_W),
        (
            "failed",
            Stage::Note(
                "update failed: connection reset by peer while fetching the release \
                 assets from github.com"
                    .into(),
            ),
            NAV_W,
        ),
        (
            "failed-narrow",
            Stage::Note("update failed: connection reset by peer".into()),
            150,
        ),
    ];
    for (name, stage, w) in stages {
        let u = UpdateState::for_test(stage);
        let Some(rows) = card_shot(&format!("update-{name}"), "UPDATE", w, 96, |c, r| {
            crate::updatecard::update_cells(&u, c, r)
        }) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(rows.iter().any(|l| !l.is_empty()), "{name} drew nothing");
    }
}

fn row(index: usize, title: &str) -> PaneRow {
    PaneRow {
        index,
        title: title.into(),
        focused: false,
        activity: false,
        minimized: false,
        attention: None,
        busy: false,
        unread: 0,
        hovered: false,
    }
}

/// Every marker a row can wear, one row each, plus a title the column
/// cannot hold.
fn every_row() -> Vec<PaneRow> {
    let mut v = vec![
        row(1, "claude — crew"),
        row(2, "cargo watch -x check"),
        row(3, "zsh"),
        row(4, "claude — ask before rm"),
        row(5, "far ~/code/crew/crates/crew-app/src"),
        row(6, "dash"),
        row(7, "smith"),
    ];
    v[0].focused = true;
    v[0].busy = true;
    v[1].activity = true;
    v[1].unread = 12;
    v[2].attention = Some(('!', true));
    v[3].attention = Some(('?', true));
    v[3].busy = true;
    v[4].minimized = true;
    v[5].hovered = true;
    v[6].minimized = true;
    v[6].unread = 3;
    v
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn transient_shot_pane_rows_and_restart_legend() {
    let _g = crate::app::theme_test_guard();
    let panes = every_row();
    // The legend the stats card wears while a newer binary waits: the nav is
    // ~23 columns, so the compact form is the one that ships.
    let legend = crate::restartnote::legend("0.21.3", crate::boxdraw::title_budget(25));
    for (name, w, legend) in [
        ("panes", NAV_W, legend.as_str()),
        ("panes-narrow", 150, legend.as_str()),
        (
            "panes-plain",
            NAV_W,
            concat!("crew v", env!("CARGO_PKG_VERSION")),
        ),
    ] {
        let Some(rows) = card_shot(name, legend, w, 200, |c, _| {
            crate::panelist::pane_cells(&panes, c, 8, '\u{280b}')
        }) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(rows[0].contains("PANES"), "{rows:?}");
    }
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn transient_shot_ghost_card() {
    let _g = crate::app::theme_test_guard();
    for (name, t) in [
        ("ghost-early", 0.85),
        ("ghost-mid", 0.5),
        ("ghost-late", 0.15),
    ] {
        let (w, h) = (360u32, 220u32);
        let px = crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
            let mut scenes = Vec::new();
            let rect = crate::layout::Rect {
                x: 12.0,
                y: 12.0,
                w: w as f32 - 24.0,
                h: h as f32 - 24.0,
            };
            crate::panelcard::push_ghost(&mut scenes, rect, cw, ch, "cargo watch", t);
            scenes
        });
        let Some(px) = px else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        crate::shotdraw_tests::write_png(&format!("transient-{name}"), &px, w, h);
        eprintln!(
            "--- transient-{name}: {} ink pixels",
            crate::shotgpu_tests::ink(&px)
        );
    }
}
