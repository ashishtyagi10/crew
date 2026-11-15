//! What crew draws on a SEE-THROUGH window, held to the mark floor on every
//! glass palette: the nav's cells, then whole windows.
//!
//! [`crate::navcontrast_tests`] holds every nav glyph to the floor against
//! the page. A sheer window does not show the page: everything stands on
//! the frost over whatever desktop is behind, and a colour derived against
//! `page_bg` by the widget that draws it (not a palette role, which
//! `glassink` floors) can clear the page and vanish on the glass. The load
//! history did, and a reply's list bullets (theme loop, 2026-11-14). These
//! walk the drawn cells over the frost on a black desktop and a white one.
use crate::navcontrast_tests::{fixture, FURNITURE};

#[test]
fn every_nav_glyph_clears_the_mark_floor_on_sheer_glass() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let floor = crew_theme::readable::MARK_FLOOR;
    crew_theme::glassborder::set_sheer(true);
    let mut bad: Vec<String> = Vec::new();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let grounds = crew_theme::glasslegend::pane_grounds(crew_theme::theme());
        if grounds.is_empty() {
            continue;
        }
        let (sp, log, panes) = fixture();
        let glance = crate::navglance::sample::glance();
        let list = crate::tabnav::NavList::from(panes.clone());
        for g in [None, Some(&glance)] {
            let strip = g.is_none().then_some("\u{2600} 24\u{00b0}");
            let cells = sp.cells(26, 64, &list, &log, 0, g, strip);
            let page = crew_theme::theme().page_bg;
            let glyphs = cells
                .iter()
                .filter(|c| c.c != ' ' && !FURNITURE.contains(c.c) && c.bg == page);
            for c in glyphs {
                let r = crew_theme::glasslegend::worst(c.fg, &grounds);
                if r < floor - 0.05 {
                    bad.push(format!("{} {:?} r{} {r:.2}", id.as_str(), c.c, c.row));
                }
            }
        }
    }
    crew_theme::glassborder::set_sheer(false);
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    assert!(
        bad.is_empty(),
        "{} under:\n  {}",
        bad.len(),
        bad.join("\n  ")
    );
}

/// What a window's scenes draw under the mark floor on `id`'s sheer glass:
/// one line per colour, with the first glyphs wearing it. A glyph on the
/// page is measured over the frost on a black desktop and a white one; a
/// glyph on a fill of its own (a chip, a code block) against that fill.
/// The settings form's `■` swatches are exempt: they show a colour, they
/// are not written in one.
fn under_floor(id: crew_theme::ThemeId, prep: &dyn Fn(&mut crate::app::CrewApp)) -> Vec<String> {
    crew_theme::set_theme(id);
    crew_theme::glassborder::set_sheer(true);
    crate::palette::set_accent(crew_theme::theme().accent_default);
    let grounds = crew_theme::glasslegend::pane_grounds(crew_theme::theme());
    if grounds.is_empty() {
        return Vec::new();
    }
    let mut app = crate::app::CrewApp {
        geo_override: Some((8.0, 16.0, 1600.0, 900.0, 1.0)),
        ..Default::default()
    };
    prep(&mut app);
    for p in &mut app.panes {
        p.born_ms = 0;
        p.zoom = Some(false);
    }
    let (page, floor) = (
        crew_theme::theme().page_bg,
        crew_theme::readable::MARK_FLOOR,
    );
    let mut seen = std::collections::BTreeMap::new();
    for s in app.build_frame() {
        let mark = |c: char| ('\u{2500}'..='\u{25ff}').contains(&c) || FURNITURE.contains(c);
        for c in s
            .cells
            .iter()
            .filter(|c| !c.c.is_whitespace() && !mark(c.c))
        {
            let r = match c.bg == page {
                true => crew_theme::glasslegend::worst(c.fg, &grounds),
                false => crew_theme::contrast_ratio(c.fg, c.bg),
            };
            if r < floor - 0.05 {
                let e = seen
                    .entry((c.fg, c.bg == page))
                    .or_insert((r, String::new()));
                if e.1.chars().count() < 28 {
                    e.1.push(c.c);
                }
            }
        }
    }
    seen.into_iter()
        .map(|((fg, on_page), (r, text))| {
            format!("{} {fg:?} page={on_page} {r:.2} {text:?}", id.as_str())
        })
        .collect()
}

fn audit(prep: &dyn Fn(&mut crate::app::CrewApp)) {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let bad: Vec<String> = crew_theme::ALL_THEMES
        .into_iter()
        .flat_map(|id| under_floor(id, prep))
        .collect();
    crew_theme::glassborder::set_sheer(false);
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    assert!(
        bad.is_empty(),
        "{} under:\n  {}",
        bad.len(),
        bad.join("\n  ")
    );
}

/// The dashboard, the to-do list and the file panels beside the open nav.
#[test]
fn a_window_of_tools_reads_on_sheer_glass() {
    audit(&|app| {
        for cmd in ["/dash", "/todo", "/far"] {
            app.submit_input(cmd.to_string());
        }
        app.config.show_nav = true;
        app.config.nav_collapsed = false;
    });
}

/// Agent smith mid-turn beside a shell: the work the window is for.
#[test]
fn smith_and_a_shell_read_on_sheer_glass() {
    use crate::glassworkshot_tests::{pane, plan, shell};
    audit(&|app| {
        let mut smith = crate::chatshot_tests::live_pane();
        smith.checklist = plan();
        app.panes
            .push(pane(crate::pane::PaneContent::Chat(smith), None));
        app.panes.push(shell());
        app.focused = 0;
    });
}
