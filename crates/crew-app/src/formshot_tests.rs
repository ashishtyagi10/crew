//! Off-screen render of the two form-shaped panes: **Settings** (a bento of
//! fieldset cards with boxed inputs, checkboxes, pickers and a pinned
//! Save/Cancel row) and **/todo** (items, projects, due dates, the done
//! history).
//!
//! Both are laid into a `ratatui` `Buffer` and converted to cells, and both
//! reflow with width — which is exactly the shape that passes every unit test
//! and still reads badly on a tile. Neither had ever been looked at.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew form_shot -- --ignored`
use crate::config::CrewConfig;
use crate::settingspane::SettingsPane;
use crate::shotgpu_tests::shot_at;
use crate::todopane::TodoPane;

const H: u32 = 760;

fn settings() -> SettingsPane {
    let cfg = CrewConfig {
        theme: Some("paper-dark".into()),
        font_family: Some("Lilex".into()),
        notify_patterns: vec!["error".into(), "panic".into(), "\u{2713} done".into()],
        ..Default::default()
    };
    SettingsPane::new(
        cfg,
        [
            "Lilex",
            "MonoLisa",
            "SF Mono",
            "JetBrains Mono",
            "Fira Code",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect(),
    )
}

fn settings_shot(name: &str, w: u32) -> Option<Vec<u8>> {
    let p = settings();
    shot_at(name, w, H, 13.0, "settings", |cols, rows, _| {
        (p.cells(cols, rows), Vec::new())
    })
}

/// The form at the widths a tile gets. It lays itself out as one column or
/// two depending on the room, and the fold between them is where a form
/// stops being readable.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn form_shot_settings_width_sweep() {
    let _g = crate::app::theme_test_guard();
    for (name, w) in [
        ("settings-narrow", 480u32),
        ("settings-half", 720),
        ("settings-wide", 1180),
    ] {
        let Some(px) = settings_shot(name, w) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 3000, "{name} drew");
    }
}

/// The same form on a light page and on a tube: a boxed input's own edge is
/// the finest thing it draws, and it is drawn in the border roles that this
/// project has twice caught reading under 1.3 against a light page.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn form_shot_settings_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    // Paper-light and the two presets where accent and `text_muted` measured
    // the SAME lightness (sepia-dark 1.04, crt-violet 1.06), which is where a
    // focused control had nothing to say for itself.
    for (name, id) in [
        ("settings-light", crew_theme::ThemeId::PaperLight),
        ("settings-sepia-dark", crew_theme::ThemeId::SepiaDark),
        ("settings-crt-violet", crew_theme::ThemeId::CrtViolet),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let Some(px) = settings_shot(name, 900) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 3000, "{name} drew");
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

/// A week of a real list: overdue, due today with a time, due later, a couple
/// of untagged notes, three projects, and some done history under it.
fn todos() -> TodoPane {
    use crate::todopane::item::TodoItem;
    let now = crate::chattime::unix_now_ms();
    let day = 86_400_000u64;
    let mut p = TodoPane::new();
    let item =
        |id: u64, title: &str, project: Option<&str>, due: Option<u64>, done: bool| TodoItem {
            id,
            title: title.into(),
            done,
            done_ms: done.then_some(now - day),
            project: project.map(str::to_string),
            due_ms: due,
            due_has_time: due.is_some(),
            created_ms: now - 3 * day,
            notified: false,
        };
    p.items = vec![
        item(
            1,
            "shoot the whole chat pane",
            Some("crew"),
            Some(now - day),
            false,
        ),
        item(
            2,
            "floor the code field on the tubes",
            Some("crew"),
            Some(now + 3_600_000),
            false,
        ),
        item(3, "read the blame gutter runs", None, None, false),
        item(
            4,
            "renumber the unified diff from the file",
            Some("viewer"),
            Some(now + 2 * day),
            false,
        ),
        item(
            5,
            "ask about the empty right column",
            Some("design"),
            None,
            false,
        ),
        item(
            6,
            "restore the tube's light trace",
            Some("crew"),
            None,
            true,
        ),
        item(7, "give fern a focused frame", Some("crew"), None, true),
    ];
    p.show_done = true;
    p.sel = Some(1);
    p.input = "@crew ".into();
    p.cursor = 6;
    p
}

/// The list at the widths a tile gets. Every row negotiates a checkbox, a
/// title, a project chip and a due label for the same columns, and the width
/// where one of them wins outright is the width the list stops reading.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn form_shot_todo_width_sweep() {
    let _g = crate::app::theme_test_guard();
    let p = todos();
    for (name, w) in [
        ("todo-narrow", 380u32),
        ("todo-half", 640),
        ("todo-wide", 1100),
    ] {
        let px = shot_at(name, w, 480, 13.0, "todo", |cols, rows, _| {
            (p.cells(cols, rows), Vec::new())
        });
        let Some(px) = px else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 2000, "{name} drew");
    }
}
