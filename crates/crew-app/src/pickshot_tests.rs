//! Off-screen render of the pickers `menushot` does not reach: the attach
//! picker (`@` — agents, skills, files, and the `@a+b` hint), the value
//! pickers that carry a colour swatch (`/theme `, `/gradient `), the path
//! picker (`/view <dir>/`), and the provider-key prompt.
//!
//! Each is `menu_card` or a card of its own with a row shape only that caller
//! produces — a swatch beside a description, a file path with no description
//! at all, a masked field under a legend that names an environment variable —
//! and none of them had been in a frame.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew pick_shot -- --ignored --nocapture`
use crate::chatmention::MentionEntry;
use crate::goalshot_tests::dump;
use crate::shotgpu_tests::shot_at;
use crate::suggest::MenuItem;

fn menu_shot(
    name: &str,
    legend: &str,
    items: &[MenuItem],
    sel: usize,
    w: u32,
) -> Option<Vec<String>> {
    let rows = crate::cmdmenu::menu_rows(items.len());
    let h = u32::from(rows) * 22 + 24;
    let mut dumped = Vec::new();
    shot_at(&format!("pick-{name}"), w, h, 13.0, legend, |cols, r, _| {
        let cells = crate::cmdmenu::menu_cells(items, sel, cols, r);
        dumped = dump(&cells, cols, r);
        eprintln!("--- pick-{name} {cols}x{r}");
        for l in &dumped {
            eprintln!("|{l}");
        }
        (cells, Vec::new())
    })?;
    Some(dumped)
}

fn roster() -> Vec<MentionEntry> {
    let agent = |n: &str, r: &str| MentionEntry::Agent {
        name: n.into(),
        role: r.into(),
    };
    let skill = |n: &str, d: &str| MentionEntry::Skill {
        name: n.into(),
        desc: d.into(),
    };
    vec![
        agent("planner", "breaks the goal into tasks"),
        agent("coder", "writes and edits code"),
        agent("reviewer", "reads diffs for bugs"),
        skill("release", "bump, tag and push a version"),
        skill("verify", "drive the live GUI and screenshot it"),
        MentionEntry::File("README.md".into()),
        MentionEntry::File("crates/crew-app/src/viewpane/render_tests.rs".into()),
        MentionEntry::File("docs/superpowers/goals/2026-09-01-close-the-open-goals.md".into()),
    ]
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn pick_shot_attach() {
    let _g = crate::app::theme_test_guard();
    let items = crate::chatpalette::chatpaletteitems::attach_items("", &roster(), false);
    for (name, w) in [
        ("attach-narrow", 380),
        ("attach-half", 640),
        ("attach-wide", 1100),
    ] {
        let Some(rows) = menu_shot(name, "attach", &items, 1, w) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(rows.iter().any(|r| r.contains("@planner")), "{rows:?}");
    }
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn pick_shot_values_with_swatches() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (name, text, w) in [
        ("theme", "/theme ", 640),
        ("theme-narrow", "/theme ", 380),
        ("gradient", "/gradient ", 640),
        ("theme-d", "/theme d", 640),
    ] {
        let items = crate::suggest::menu_items(text);
        let legend = text.trim().trim_start_matches('/');
        let Some(rows) = menu_shot(name, legend, &items, 1, w) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(!rows.is_empty());
    }
    crew_theme::set_theme(crew_theme::ThemeId::PaperLight);
    crate::palette::set_accent(crew_theme::theme().accent_default);
    let items = crate::suggest::menu_items("/theme ");
    menu_shot("theme-light", "theme", &items, 1, 640);
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn pick_shot_paths() {
    let _g = crate::app::theme_test_guard();
    let dir = tempfile::tempdir().expect("tempdir");
    for d in ["crates", "docs", "examples", ".git"] {
        std::fs::create_dir(dir.path().join(d)).unwrap();
    }
    for f in [
        "README.md",
        "CHANGELOG.md",
        "Cargo.toml",
        "a-very-long-file-name-that-somebody-generated-from-a-timestamp-2026-09-02T10-15-00.log",
    ] {
        std::fs::write(dir.path().join(f), b"").unwrap();
    }
    let items = crate::pathmenu::rows("/view ", dir.path()).expect("a path command");
    for (name, w) in [("paths-narrow", 380), ("paths-half", 640)] {
        let Some(rows) = menu_shot(name, "files", &items, 0, w) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(rows.iter().any(|r| r.contains("crates/")), "{rows:?}");
    }
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn pick_shot_key_prompt() {
    let _g = crate::app::theme_test_guard();
    for (name, w, waiting, typed) in [
        ("key-plain", 640, false, 24),
        ("key-waiting", 640, true, 0),
        ("key-narrow", 260, true, 40),
    ] {
        let mut entry = crate::keyentry::KeyEntry::new("OPENROUTER_API_KEY".into());
        entry.set_waiting(waiting);
        for _ in 0..typed {
            entry.key(&crate::chatkeys::ChatInput::Char('k'));
        }
        if waiting && typed > 0 {
            entry.forget_typing();
            entry.paste(&"k".repeat(typed));
        }
        let h = u32::from(entry.rows()) * 20 + 8;
        let px = crate::shotdraw_tests::draw(w, h, 13.0, |cw, ch| {
            let cols = (w as f32 / cw).floor() as u16;
            let cells = entry.card(cols);
            let rows = entry.rows();
            eprintln!("--- pick-{name} {cols}x{rows}");
            for l in dump(&cells, cols, rows) {
                eprintln!("|{l}");
            }
            vec![crew_render::PaneScene {
                cells,
                x: 0.0,
                y: 0.0,
                w: w as f32,
                h: f32::from(rows) * ch,
                focused: false,
                bordered: false,
                glass: false,
                scan: -1.0,
                overlay: true,
                paint: Vec::new(),
            }]
        });
        let Some(px) = px else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        crate::shotdraw_tests::write_png(&format!("pick-{name}"), &px, w, h);
    }
}
