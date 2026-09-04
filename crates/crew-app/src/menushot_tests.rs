//! Off-screen render of the menu card — the ONE widget the slash palette, the
//! attach picker, the model picker, `/todo`'s tag popup and every value
//! suggestion are all drawn as (`cmdmenu::menu_card`).
//!
//! One widget with five callers is exactly where drift hides: each caller has
//! its own tests, none of them looks at the card, and a row shape only one
//! caller produces (a section header, a greyed row, a colour swatch, a `needs
//! key` badge, a right-aligned chord) is a shape nobody ever sees rendered.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew menu_shot -- --ignored`
use crate::shotgpu_tests::shot_at;
use crate::suggest::MenuItem;

fn item(label: &str, desc: &str, key: Option<&'static str>) -> MenuItem {
    MenuItem {
        label: label.into(),
        desc: desc.into(),
        key,
        ..Default::default()
    }
}

/// The palette as it looks two characters into a query: matched characters
/// marked, chords on the right, a command with no chord among them.
fn commands() -> Vec<MenuItem> {
    let mut v = vec![
        item("/dash", "Open the dashboard pane", Some("Cmd+D")),
        item("/diff", "Review the working tree", None),
        item("/doctor", "Report what crew can reach", None),
        item("/dump", "Write the frame's cells to a file", None),
        item("/disk", "Where the disk went", None),
    ];
    for i in &mut v {
        i.hit = vec![1, 2]; // "/d" + the next letter, as a fuzzy hit marks it
    }
    v
}

/// The model picker: provider headers, a row the stack cannot serve, and a
/// row blocked on a key. Three shapes only this caller produces.
fn models() -> Vec<MenuItem> {
    let header = |l: &str| MenuItem {
        label: l.into(),
        header: true,
        ..Default::default()
    };
    vec![
        header("anthropic"),
        item("claude-opus-5", "the most capable", None),
        item("claude-sonnet-5", "balanced", None),
        header("openrouter"),
        MenuItem {
            needs: Some("OPENROUTER_API_KEY".into()),
            ..item("qwen3-max", "needs a key", None)
        },
        MenuItem {
            dim: true,
            ..item("llama-4-scout", "no provider on this stack", None)
        },
    ]
}

fn menu_shot(name: &str, items: &[MenuItem], sel: usize, w: u32) -> Option<Vec<u8>> {
    let rows = crate::cmdmenu::menu_rows(items.len());
    let h = u32::from(rows) * 22 + 24;
    // The shot's own card IS the popup's fieldset frame — `menu_card` is that
    // same `gradient_card` with `menu_cells` inset one column into it, so the
    // interior this hands back is the list at exactly the width it draws at.
    shot_at(name, w, h, 13.0, "commands", |cols, r, _| {
        (crate::cmdmenu::menu_cells(items, sel, cols, r), Vec::new())
    })
}

/// The palette at the widths a pane gives it: a quarter tile, a half, the
/// whole window. The description column, the chord column and the label
/// column all negotiate for the same row — a width where one of them wins
/// outright is a width where the list stops being readable.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn menu_shot_width_sweep() {
    let _g = crate::app::theme_test_guard();
    for (name, w) in [
        ("menu-narrow", 380),
        ("menu-half", 640),
        ("menu-wide", 1100),
    ] {
        let Some(px) = menu_shot(name, &commands(), 1, w) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 1500, "{name} drew");
    }
}

/// The palette when nothing matches: one dim note, no selection marker on
/// it, and a card that is still a card at a tile's width.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn menu_shot_no_match_note() {
    let _g = crate::app::theme_test_guard();
    let items = crate::cmdnote::rows("/xyzzy", std::path::Path::new(""));
    assert_eq!(items.len(), 1);
    let Some(px) = menu_shot("menu-nomatch", &items, 0, 380) else {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
        return;
    };
    assert!(crate::shotgpu_tests::ink(&px) > 300, "the note drew");
}

/// The same card on a light page and on a tube. Every description in every
/// picker is drawn in one colour, so a colour that only reads on a dark page
/// takes all five callers down together.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn menu_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (name, id) in [
        ("menu-light", crew_theme::ThemeId::PaperLight),
        ("menu-crt-green", crew_theme::ThemeId::CrtGreen),
        ("menu-sepia", crew_theme::ThemeId::SepiaLight),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        let Some(px) = menu_shot(name, &commands(), 1, 640) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 1500, "{name} drew");
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}

/// The model picker's own row shapes, which no other caller produces.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn menu_shot_model_picker() {
    let _g = crate::app::theme_test_guard();
    for (name, id) in [
        ("menu-models", crew_theme::ThemeId::PaperDark),
        ("menu-models-light", crew_theme::ThemeId::PaperLight),
    ] {
        crew_theme::set_theme(id);
        let Some(px) = menu_shot(name, &models(), 1, 640) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        assert!(crate::shotgpu_tests::ink(&px) > 1500, "{name} drew");
    }
}
