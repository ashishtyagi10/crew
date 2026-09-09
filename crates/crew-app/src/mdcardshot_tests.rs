//! The markdown CARD, looked at.
//!
//! `mdshot_tests` shoots a table, lists and the wrapper's edge cases on a
//! chat pane; `chatshot_tests` a whole transcript. Neither fixture has the
//! h1 badge and its rule, an h3, a fence's language badge beside its token
//! hues, a task list inside an ordered one, a table that WRAPS, a strike, a
//! footnote, a bare `www.` link or a picture — every construct the smith
//! pane learned to draw, in one document (`mdcardcells_tests::DOC`, whose
//! cell-level guards are not ignored), at three widths and on every theme
//! preset.
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew md_card_shot -- --ignored`
use crate::mdcardcells_tests::DOC;

const H: u32 = 980;

/// Pixel widths that land the card near 40, 60 and 100 columns at 13px.
const WIDTHS: [(&str, u32); 3] = [("w40", 356), ("w60", 512), ("w100", 828)];

fn pane() -> crate::chat::ChatPane {
    let plugin =
        crew_plugin::Plugin::spawn("sh", &["-c".to_string(), "cat >/dev/null".to_string()])
            .unwrap();
    let mut p = crate::chat::ChatPane::new(plugin, "crew".into());
    p.connected = true;
    p.messages = vec![crate::chatlayout::Message {
        sender: "smith".into(),
        text: DOC.into(),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    }];
    p
}

fn shot(name: &str, w: u32) -> bool {
    let p = pane();
    let px = crate::shotgpu_tests::shot_at(name, w, H, 13.0, "crew", |cols, rows, aspect| {
        eprintln!("{name}: {cols} cols × {rows} rows");
        crate::chatview::art(&p, cols, rows, aspect)
    });
    px.is_some_and(|px| {
        let n = crate::shotgpu_tests::ink(&px);
        assert!(n > 3_000, "{name} is all but blank: {n} ink pixels");
        true
    })
}

/// The whole document at the three widths.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn md_card_shot_widths() {
    let _g = crate::app::theme_test_guard();
    let mut any = false;
    for (suffix, w) in WIDTHS {
        any |= shot(&format!("md-card-{suffix}"), w);
    }
    if !any {
        eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
    }
}

/// The whole document on every theme preset.
#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn md_card_shot_themes() {
    let _a = crate::palette::test_guard();
    let _g = crate::app::theme_test_guard();
    for id in crew_theme::ALL_THEMES {
        crew_theme::set_theme(id);
        crate::palette::set_accent(crew_theme::theme().accent_default);
        shot(
            &format!("md-card-{}", format!("{id:?}").to_lowercase()),
            512,
        );
    }
    crate::palette::set_accent(crate::palette::DEFAULT_ACCENT);
}
