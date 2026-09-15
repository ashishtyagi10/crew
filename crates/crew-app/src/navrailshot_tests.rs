//! Off-screen render of the COLLAPSED nav — the rail at its own width, drawn
//! into the column it docks in, so the thing a user stares at all day can be
//! looked at rather than only asserted on.
//!
//! The rail shipped with nothing in it but the pane list, which no unit test
//! could have called wrong: every row it drew was correct. What was wrong was
//! the forty rows it drew nothing on, and that is a thing you have to see.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `cargo test -p crew-app --bin crew navrail_shot -- --ignored --nocapture`
use crew_render::PaneScene;

use crate::git::GitInfo;
use crate::layout::Rect;
use crate::navrailfoot::Foot;
use crate::panelist::PaneRow;
use crate::stats::Stats;

const H: u32 = 1000;

fn pane(index: usize, focused: bool, busy: bool) -> PaneRow {
    PaneRow {
        index,
        title: format!("pane {index}"),
        focused,
        activity: false,
        minimized: false,
        attention: None,
        busy,
        unread: 0,
        hovered: false,
    }
}

/// A session's worth of state, at the rail's own width: a busy crew, a laptop
/// under load, a link doing work, a dirty tree.
fn rail_shot(name: &str, panes: &[PaneRow]) -> Option<Vec<u8>> {
    let w = 120;
    let px = crate::shotdraw_tests::draw(w, H, 13.0, |cw, ch| {
        let rect = Rect {
            x: 12.0,
            y: 12.0,
            w: crate::navrail::px(cw),
            h: H as f32 - 24.0,
        };
        let foot = Foot {
            time: "09:41".into(),
            sky: Some("\u{2601}12\u{00b0}".into()),
            stats: Stats {
                cpu: 0.34,
                mem: 0.71,
                disk: 0.92,
                net_rx: 1_260_000,
                net_tx: 4_096,
            },
            git: Some(GitInfo {
                branch: "main".into(),
                changed: 9,
                ahead: 1,
                behind: 0,
            }),
        };
        let mut scenes: Vec<PaneScene> = Vec::new();
        crate::panelcard::push_card_art(
            &mut scenes,
            rect,
            cw,
            ch,
            &crate::navrail::legend(true, ""),
            crew_theme::theme().legend_off,
            |cols, rows| {
                let mut cells = crate::navrail::rail_cells(panes, cols, rows, '\u{280b}');
                let used = panes.len().min(usize::from(rows)) as u16;
                let (foot, paint) = crate::navrailfoot::foot(&foot, cols, rows, used, ch / cw);
                cells.extend(foot);
                (cells, paint)
            },
        );
        scenes
    })?;
    crate::shotdraw_tests::write_png(name, &px, w, H);
    Some(px)
}

/// The rail as it docks, with a crew in it and with one pane only — the two
/// ends of how much of the column the pane list claims.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn navrail_shot_docked() {
    let _g = crate::app::theme_test_guard();
    let mut busy = vec![
        pane(1, true, true),
        pane(2, false, true),
        pane(3, false, false),
        pane(4, false, false),
    ];
    busy[2].attention = Some(('!', true));
    for (name, panes) in [
        ("navrail-crew", busy.as_slice()),
        ("navrail-one-pane", &busy[..1]),
    ] {
        let Some(px) = rail_shot(name, panes) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        // A rail that only drew its own frame would still put a few hundred
        // pixels of ink on the page; the readings are what push it past this.
        assert!(
            crate::shotgpu_tests::ink(&px) > 1200,
            "{name} drew {} lit pixels",
            crate::shotgpu_tests::ink(&px)
        );
    }
}

/// The same column on a light page and on a phosphor tube: the meters are the
/// finest thing the rail draws, and a capsule four columns long is exactly
/// what a light page's contrast floor and a tube's bloom can undo.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn navrail_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let panes = [pane(1, true, true), pane(2, false, false)];
    for (name, id) in [
        ("navrail-light", crew_theme::ThemeId::PaperLight),
        ("navrail-crt-green", crew_theme::ThemeId::CrtGreen),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        if rail_shot(name, &panes).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
