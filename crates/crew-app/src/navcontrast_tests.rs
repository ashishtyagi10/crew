//! A contrast contract over the left nav's *drawn cells*, on every theme.
//!
//! The nav's colours are all derived — the accent, the load ramp, the ranked
//! history figures, the dimmed log stamps — and derived roles are exactly the
//! ones that drift off a page nobody looked at. This walks what
//! `StatsPane::cells` actually emits, so a colour that never appears on screen
//! cannot pass and one that appears in a section nobody thought about cannot
//! hide.
//!
//! The rules (`─`) are exempt: every theme's ramp puts its borders at 2.0 on
//! purpose, and they are furniture, not something you read.
use crate::applog::{LogEntry, LogLevel};
use crate::panelist::PaneRow;
use crate::statspane::StatsPane;

fn fixture() -> (StatsPane, Vec<LogEntry>, Vec<PaneRow>) {
    let mut sp = StatsPane::new();
    sp.refresh(std::path::Path::new("."));
    sp.seed_history(&[9, 12, 40, 18, 11]);
    let log = ["12:00 started", "12:01 connected", "12:01 build failed"]
        .iter()
        .enumerate()
        .map(|(i, t)| LogEntry {
            level: if i == 2 {
                LogLevel::Error
            } else {
                LogLevel::Info
            },
            text: (*t).to_string(),
        })
        .collect();
    let panes = (1..=3)
        .map(|i| PaneRow {
            index: i,
            title: format!("pane{i}"),
            focused: i == 1,
            activity: i == 3,
            minimized: false,
            attention: (i == 2).then_some(('!', true)),
            busy: i == 1,
            unread: if i == 3 { 4 } else { 0 },
            hovered: false,
        })
        .collect();
    (sp, log, panes)
}

/// Glyphs that are frame, not content: the ramp puts them at 2.0 deliberately.
const FURNITURE: &str = "─·";

/// Every glyph the nav draws clears the mark floor against the cell it is
/// drawn on, on every theme in the set.
#[test]
fn every_nav_glyph_clears_the_mark_floor_on_every_theme() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let floor = crew_theme::readable::MARK_FLOOR;
    let mut bad: Vec<String> = Vec::new();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let (sp, log, panes) = fixture();
        for c in sp
            .cells(26, 48, &panes, &log, 0)
            .iter()
            .filter(|c| c.c != ' ' && !FURNITURE.contains(c.c))
        {
            let r = crew_theme::contrast_ratio(c.fg, c.bg);
            if r < floor - 0.01 {
                bad.push(format!(
                    "{}: {:?} at r{} c{} reads {r:.2}, floor {floor}",
                    id.as_str(),
                    c.c,
                    c.row,
                    c.col
                ));
            }
        }
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
    assert!(bad.is_empty(), "{}", bad.join("\n  "));
}

/// …and the same holds when the user has set an accent the page cannot carry.
/// Crew's own brand green reads at 1.2 on every light page in the set; before
/// `palette::accent` grew a floor, that took the section legends, the clock,
/// the load and the PANES key down with it.
#[test]
fn a_hostile_user_accent_cannot_take_the_nav_below_the_floor() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    let floor = crew_theme::readable::MARK_FLOOR;
    let mut bad: Vec<String> = Vec::new();
    // Crew green, and two more a user could plausibly pick that no page in the
    // set can carry at both poles.
    for accent in [(0, 255, 160), (255, 240, 90), (30, 30, 40)] {
        for id in crew_theme::ALL_THEMES {
            crew_theme::set_theme(id);
            crate::palette::set_accent(accent);
            let (sp, log, panes) = fixture();
            let worst = sp
                .cells(26, 48, &panes, &log, 0)
                .iter()
                .filter(|c| c.c != ' ' && !FURNITURE.contains(c.c))
                .map(|c| (crew_theme::contrast_ratio(c.fg, c.bg) * 100.0) as i32)
                .min()
                .unwrap_or(i32::MAX);
            if (worst as f32 / 100.0) < floor - 0.01 {
                bad.push(format!(
                    "{} with accent {accent:?}: worst glyph reads {:.2}",
                    id.as_str(),
                    worst as f32 / 100.0
                ));
            }
        }
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
    assert!(bad.is_empty(), "{}", bad.join("\n  "));
}

/// The other OS accessibility switch. High contrast raises the text floor the
/// whole palette is derived against, and the accent's own floor is read from
/// the same place — so asking for more contrast must not leave the one colour
/// the user picked sitting at the old floor.
#[test]
fn high_contrast_raises_the_navs_floor_too() {
    let _a = crate::palette::test_guard();
    // `theme_test_guard` is this crate's one serializer for the derived-colour
    // globals, and the high-contrast flag changes what every one of them comes
    // out as — crew-theme's own `contrast::test_lock` is `cfg(test)`-private to
    // that crate, and would not serialize these callers anyway.
    let _g = crate::app::theme_test_guard();
    crew_theme::contrast::set_high_contrast(true);
    let floor = crew_theme::readable::MARK_FLOOR;
    let mut bad: Vec<String> = Vec::new();
    let mut lifted = 0;
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
        let hi = crew_theme::contrast_ratio(crate::palette::accent(), crew_theme::theme().page_bg);
        crew_theme::contrast::set_high_contrast(false);
        let normal =
            crew_theme::contrast_ratio(crate::palette::accent(), crew_theme::theme().page_bg);
        crew_theme::contrast::set_high_contrast(true);
        if hi < normal - 0.01 {
            bad.push(format!(
                "{}: high contrast LOWERED it ({hi:.2} < {normal:.2})",
                id.as_str()
            ));
        }
        if hi > normal + 0.5 {
            lifted += 1;
        }
        let (sp, log, panes) = fixture();
        for c in sp
            .cells(26, 48, &panes, &log, 0)
            .iter()
            .filter(|c| c.c != ' ' && !FURNITURE.contains(c.c))
        {
            let r = crew_theme::contrast_ratio(c.fg, c.bg);
            if r < floor - 0.01 {
                bad.push(format!("{}: {:?} reads {r:.2}", id.as_str(), c.c));
            }
        }
    }
    crew_theme::contrast::set_high_contrast(false);
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
    assert!(bad.is_empty(), "{}", bad.join("\n  "));
    // …and the switch actually moved something, on the pages where crew green
    // needed the help. Otherwise this test agrees with nothing.
    assert!(lifted > 0, "high contrast changed no theme's accent");
}
