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
