//! The sweep: every composer pop-up shot over the same pane through
//! `popupshot_tests::shot`, plus the model picker on a narrow tile and on a
//! light page. `#[ignore]`d: needs a GPU adapter.
use crate::chat::ChatPane;
use crate::chatmention::MentionEntry;
use crate::chatpalette::{after_edit, Kind, PaletteState};
use crate::popupshot_tests::{shot, W};

/// The model picker, open on the pane as `/model` opens it.
fn model(p: &mut ChatPane) {
    p.palette = Some(PaletteState {
        kind: Kind::Model,
        items: crate::menushot_tests::models(),
        sel: 1,
        entries: Vec::new(),
        touched: false,
    });
}

/// The palette `typed` opens, with `sel` selected.
fn palette(p: &mut ChatPane, typed: &str, sel: usize) {
    after_edit(&mut p.palette, typed, None, Vec::new);
    p.palette.as_mut().expect("the palette opens").sel = sel;
}

fn commands(p: &mut ChatPane) {
    palette(p, "/d", 1);
}

/// Every command, the selection deep in the list: the card scrolls and
/// its border says where you are.
fn commands_long(p: &mut ChatPane) {
    palette(p, "/", 12);
    let n = p.palette.as_ref().map_or(0, |x| x.items.len());
    assert!(n > 10, "{n} commands");
}

fn attach(p: &mut ChatPane) {
    let agent = |n: &str, r: &str| MentionEntry::Agent {
        name: n.into(),
        role: r.into(),
    };
    after_edit(&mut p.palette, "@", None, || {
        vec![
            agent("smith", "orchestrates the swarm"),
            agent("scout", "reads the tree"),
            MentionEntry::Skill {
                name: "verify".into(),
                desc: "drive the live GUI".into(),
            },
            MentionEntry::File("crates/crew-app/src/render.rs".into()),
        ]
    });
    assert!(p.palette.is_some(), "the attach picker opens on @");
}

fn find(p: &mut ChatPane) {
    let visible = p.visible_messages();
    let matches = crate::chatfind::filter(&visible, "plot");
    p.find = Some(crate::chatfind::ChatFind {
        query: "plot".into(),
        matches,
        sel: 0,
    });
}

fn history(p: &mut ChatPane) {
    let lines: Vec<String> = [
        "cargo test -p crew-app",
        "cargo clippy --workspace",
        "/doctor",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    p.histsearch = Some(crate::chathistsearch::HistSearch {
        query: "car".into(),
        saved: String::new(),
        matches: crate::chathistsearch::filter(&lines, "car"),
        sel: 0,
    });
}

fn key(p: &mut ChatPane) {
    let mut e = crate::keyentry::KeyEntry::new("OPENROUTER_API_KEY".into());
    e.set_waiting(true);
    e.paste("sk-or-v1-0123456789abcdef");
    p.keyentry = Some(e);
}

/// The shot's pane — the folded transcript, `/model claude` typed — with
/// the pop-up `open` sets open on it.
fn with(open: fn(&mut ChatPane)) -> ChatPane {
    let mut p = crate::composershot_tests::folded_pane();
    p.input = "/model claude".into();
    open(&mut p);
    p
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn popup_shot_every_composer_popup() {
    let _g = crate::app::theme_test_guard();
    let shots: [(&str, fn(&mut ChatPane)); 7] = [
        ("popup-model", model),
        ("popup-commands", commands),
        ("popup-commands-long", commands_long),
        ("popup-attach", attach),
        ("popup-find", find),
        ("popup-history", history),
        ("popup-key", key),
    ];
    for (name, open) in shots {
        if shot(name, &with(open), W).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
    // The same picker on a narrow tile and on a light page.
    shot("popup-model-narrow", &with(model), 420);
    crew_theme::set_theme(crew_theme::ThemeId::PaperLight);
    shot("popup-model-light", &with(model), W);
}

/// The pop-ups on the pages that are not the default dark one, and on a
/// quarter-width tile: the focused stroke, the accent legend and the
/// full-ink selected row are three colour roles that every theme has to
/// carry, and a 48-column tile is where a description column gives way.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn popup_shot_themes_and_tiles() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for (name, id) in [
        ("popup-model-crt-green", crew_theme::ThemeId::CrtGreen),
        ("popup-model-nebula", crew_theme::ThemeId::Nebula),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        if shot(name, &with(model), W).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
    shot("popup-commands-quarter", &with(commands_long), 380);
    // Mid-rise: the card still a quarter row low, its bottom edge over the
    // composer's top border — the frame the rise is made of.
    let mut p = with(model);
    p.popup_rise
        .tick_at(true, 0, crate::motion::MotionLevel::Full);
    crate::popupshot_tests::shot_at("popup-model-rising", &p, W, 30);
    shot("popup-attach-quarter", &with(attach), 380);
    shot("popup-key-quarter", &with(key), 380);
}
