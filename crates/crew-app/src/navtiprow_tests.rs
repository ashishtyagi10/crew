//! The hover card asks about the hovered row alone, waits for a pointer that moved, and
//! stands under the input bar and the toasts.
use super::tests::card;
use super::*;
use crate::navfar::tests::{drawn_at, find_drawn, nav_app};

/// Folders `names` under one parent, noted in Far; the base to remove after.
fn siblings(tag: &str, names: &[&str]) -> (std::path::PathBuf, std::path::PathBuf) {
    let base = std::env::temp_dir().join(format!("navtip-{tag}-{}", std::process::id()));
    let parent = base.join("p");
    for n in names {
        std::fs::create_dir_all(parent.join(n)).unwrap();
        crate::farrecent::note(&parent.join(n));
    }
    (base, parent)
}

/// Rest on the row drawn at `word`, past the wait: whether a card stands.
fn rests_to_card(a: &mut CrewApp, word: &str) -> bool {
    a.cursor = drawn_at(a, word);
    a.build_frame();
    a.nav_tip.show_at = 0;
    card(&a.build_frame()).is_some()
}

/// Two folders under one parent: the long-named one's `~/…/p` is cut, the short one's is
/// whole. The same parent text cut on the other row is no reason for a card on this one.
#[test]
fn a_siblings_cut_parent_gives_no_card() {
    let _g = crate::app::theme_test_guard();
    crate::farrecent::clear();
    let long = "longsib-".to_string() + &"x".repeat(120);
    let (base, parent) = siblings("sib", &[&long, "zq"]);
    let at = crate::disktile::short_path(&parent);
    let mut a = nav_app("far");
    a.cursor_in = true;
    // Room for the short row's parent whole, not for the long one's.
    a.config.nav_width = (crate::chatwidth::str_w(&at) as f32 + 16.0) * 8.0;
    assert!(
        find_drawn(&mut a, &at).is_some(),
        "the short row's parent whole"
    );
    assert!(rests_to_card(&mut a, "longsib"), "the cut row has its card");
    assert!(!rests_to_card(&mut a, "zq"), "the whole row has none");
    crate::farrecent::clear();
    let _ = std::fs::remove_dir_all(&base);
}

/// The wheel brings another cut folder under a resting pointer: no card, however long it
/// rests, until the pointer moves — then the wait runs as usual.
#[test]
fn a_row_scrolled_under_a_still_pointer_waits_for_a_move() {
    let _g = crate::app::theme_test_guard();
    crate::farrecent::clear();
    let names: Vec<String> = (0..20)
        .map(|i| format!("w{i:02}-{}", "y".repeat(90)))
        .collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let (base, _) = siblings("wheel", &refs);
    let mut a = nav_app("far");
    a.geo_override = Some((8.0, 17.0, 1280.0, 260.0, 1.0));
    a.cursor_in = true;
    let first = crate::navfar::name_and_parent(&crate::farrecent::list()[0]).0;
    assert!(rests_to_card(&mut a, &first[..3]), "a cut row, rested on");
    let before = a.far_item_at_cursor();
    a.last_input_ms = crate::anim::now_ms();
    a.scroll_at_cursor(-1, true);
    assert_ne!(a.far_item_at_cursor(), before, "the wheel moved the list");
    a.build_frame();
    a.nav_tip.show_at = 0;
    assert!(card(&a.build_frame()).is_none(), "not for a list that slid");
    let now = crate::anim::now_ms();
    assert!(!a.nav_tip_due(now + 10 * DELAY_MS), "and nothing is owed");
    // A nudge within the row: the wait begins, and costs no frame till its end.
    a.cursor.0 += 1.0;
    assert!(!a.nav_tip_due(now), "the wait begins");
    assert!(a.nav_tip_due(now + DELAY_MS), "one frame at its end");
    a.nav_tip.show_at = 0;
    assert!(card(&a.build_frame()).is_some(), "then the card");
    crate::farrecent::clear();
    let _ = std::fs::remove_dir_all(&base);
}

/// On a narrow window the card may reach the input bar and the toasts: it draws under both.
#[test]
fn the_card_stands_under_the_bar_and_the_toasts() {
    let _g = crate::app::theme_test_guard();
    crate::farrecent::clear();
    let long = "deepsib-".to_string() + &"z".repeat(120);
    let (base, _) = siblings("order", &[&long]);
    let mut a = nav_app("far");
    a.cursor_in = true;
    a.cursor = drawn_at(&mut a, "deepsib");
    a.toasts
        .push("orderly".into(), "note", false, crate::anim::now_ms());
    a.build_frame();
    a.nav_tip.show_at = 0;
    let scenes = a.build_frame();
    let (tip, _) = card(&scenes).expect("the card");
    let at = |p: &dyn Fn(&crew_render::PaneScene) -> bool| scenes.iter().position(p);
    let tip = at(&|s| s.x == tip.x && s.y == tip.y && s.overlay).unwrap();
    let bottom = scenes.iter().map(|s| s.y).fold(0.0, f32::max);
    let bar = at(&|s| s.y == bottom && !s.overlay).expect("the input bar");
    let words = |s: &crew_render::PaneScene| s.cells.iter().map(|c| c.c).collect::<String>();
    let toast = at(&|s| words(s).contains("orderly")).expect("the toast");
    assert!(
        tip < bar && tip < toast,
        "card {tip}, bar {bar}, toast {toast}"
    );
    crate::farrecent::clear();
    let _ = std::fs::remove_dir_all(&base);
}
