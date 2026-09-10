//! The sweep: every composer pop-up shot over the same pane through
//! `popupshot_tests::shot`, plus the model picker on a narrow tile and on a
//! light page. `#[ignore]`d: needs a GPU adapter.
use crate::chatlayout::Message;
use crate::chatmention::MentionEntry;
use crate::popupplace::Popup;
use crate::popupshot_tests::{shot, W};

const MODELS: &str = "models \u{00b7} sign in";

fn commands(c: u16) -> Popup {
    let mut pal = None;
    crate::chatpalette::after_edit(&mut pal, "/d", None, Vec::new);
    let pal = pal.expect("the palette opens on /d");
    crate::cmdmenu::popup("commands", &pal.items, 1, c)
}

/// Every command, the selection deep in the list: the card scrolls and
/// its border says where you are.
fn commands_long(c: u16) -> Popup {
    let mut pal = None;
    crate::chatpalette::after_edit(&mut pal, "/", None, Vec::new);
    let pal = pal.expect("the palette opens on /");
    assert!(pal.items.len() > 10, "{} commands", pal.items.len());
    crate::cmdmenu::popup("commands", &pal.items, 12, c)
}

fn attach(c: u16) -> Popup {
    let agent = |n: &str, r: &str| MentionEntry::Agent {
        name: n.into(),
        role: r.into(),
    };
    let mut pal = None;
    crate::chatpalette::after_edit(&mut pal, "@", None, || {
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
    let pal = pal.expect("the attach picker opens on @");
    crate::cmdmenu::popup("attach", &pal.items, 0, c)
}

fn find(visible: &[&Message], c: u16) -> Popup {
    let f = crate::chatfind::ChatFind {
        query: "plot".into(),
        matches: crate::chatfind::filter(visible, "plot"),
        sel: 0,
    };
    crate::chatfind::card(&f, visible, c)
}

fn history(c: u16) -> Popup {
    let lines: Vec<String> = [
        "cargo test -p crew-app",
        "cargo clippy --workspace",
        "/doctor",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    let h = crate::chathistsearch::HistSearch {
        query: "car".into(),
        saved: String::new(),
        matches: crate::chathistsearch::filter(&lines, "car"),
        sel: 0,
    };
    crate::chathistsearch::card(&h, c)
}

fn key(c: u16) -> Popup {
    let mut e = crate::keyentry::KeyEntry::new("OPENROUTER_API_KEY".into());
    e.set_waiting(true);
    e.paste("sk-or-v1-0123456789abcdef");
    e.card(c)
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn popup_shot_every_composer_popup() {
    let _g = crate::app::theme_test_guard();
    let mut p = crate::composershot_tests::folded_pane();
    p.input = "/model claude".into();
    let visible = p.visible_messages();
    let models = crate::menushot_tests::models();
    let model = |c| crate::cmdmenu::popup(MODELS, &models, 1, c);
    let found = |c| find(&visible, c);
    let shots: Vec<(&str, &dyn Fn(u16) -> Popup)> = vec![
        ("popup-model", &model),
        ("popup-commands", &commands),
        ("popup-commands-long", &commands_long),
        ("popup-attach", &attach),
        ("popup-find", &found),
        ("popup-history", &history),
        ("popup-key", &key),
    ];
    for (name, popup) in &shots {
        if shot(name, &p, W, popup).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
    // The same picker on a narrow tile and on a light page.
    shot("popup-model-narrow", &p, 420, &model);
    crew_theme::set_theme(crew_theme::ThemeId::PaperLight);
    shot("popup-model-light", &p, W, &model);
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
    let mut p = crate::composershot_tests::folded_pane();
    p.input = "/model claude".into();
    let models = crate::menushot_tests::models();
    let model = |c| crate::cmdmenu::popup(MODELS, &models, 1, c);
    for (name, id) in [
        ("popup-model-crt-green", crew_theme::ThemeId::CrtGreen),
        ("popup-model-sepia", crew_theme::ThemeId::SepiaLight),
        ("popup-model-nebula", crew_theme::ThemeId::Nebula),
    ] {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        if shot(name, &p, W, &model).is_none() {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        }
    }
    crew_theme::set_theme(crew_theme::ThemeId::PaperDark);
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
    shot("popup-commands-quarter", &p, 380, &commands_long);
    // Mid-rise: the card still a quarter row low, its bottom edge over the
    // composer's top border — the frame the rise is made of.
    p.popup_rise
        .tick_at(true, 0, crate::motion::MotionLevel::Full);
    crate::popupshot_tests::shot_at("popup-model-rising", &p, W, &model, 30);
    p.popup_rise
        .tick_at(false, 0, crate::motion::MotionLevel::Full);
    shot("popup-attach-quarter", &p, 380, &attach);
    shot("popup-key-quarter", &p, 380, &key);
}
