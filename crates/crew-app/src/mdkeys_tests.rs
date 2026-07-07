use super::*;
use crate::mdpane::{MdPane, Side};
use std::path::PathBuf;
use winit::keyboard::{Key, NamedKey};

fn pane(source: &str) -> MdPane {
    MdPane::new(PathBuf::from("/tmp/mdkeys-test-doc.md"), source.to_string())
}

#[test]
fn escape_classifies_as_close() {
    assert_eq!(md_key(&Key::Named(NamedKey::Escape), true), MdInput::Close);
}

#[test]
fn a_released_key_is_ignored() {
    assert_eq!(
        md_key(&Key::Named(NamedKey::Escape), false),
        MdInput::Ignore
    );
}

#[test]
fn r_key_requests_reload_case_insensitively() {
    assert_eq!(md_key(&Key::Character("r".into()), true), MdInput::Reload);
    assert_eq!(md_key(&Key::Character("R".into()), true), MdInput::Reload);
}

#[test]
fn tab_flips_active_side_back_and_forth() {
    let mut p = pane("x");
    assert!(reduce(&mut p, MdInput::Tab).is_none());
    assert_eq!(p.active, Side::Preview);
    reduce(&mut p, MdInput::Tab);
    assert_eq!(p.active, Side::Source);
}

#[test]
fn up_and_down_scroll_the_active_side_by_one_line() {
    let mut p = pane("x");
    p.scroll_src = 5;
    reduce(&mut p, MdInput::Down);
    assert_eq!(p.scroll_src, 6);
    reduce(&mut p, MdInput::Up);
    assert_eq!(p.scroll_src, 5);
}

#[test]
fn scroll_only_touches_whichever_side_is_active() {
    let mut p = pane("x");
    p.active = Side::Preview;
    p.scroll_prev = 5;
    reduce(&mut p, MdInput::Down);
    assert_eq!(p.scroll_prev, 6);
    assert_eq!(p.scroll_src, 0, "inactive side must not move");
}

#[test]
fn page_up_and_down_move_by_the_fixed_page_size() {
    let mut p = pane("x");
    p.scroll_src = 20;
    reduce(&mut p, MdInput::PageDown);
    assert_eq!(p.scroll_src, 20 + PAGE as usize);
    reduce(&mut p, MdInput::PageUp);
    assert_eq!(p.scroll_src, 20);
}

#[test]
fn scroll_floors_at_zero_for_lines_and_pages() {
    let mut p = pane("x");
    reduce(&mut p, MdInput::Up);
    assert_eq!(p.scroll_src, 0);
    reduce(&mut p, MdInput::PageUp);
    assert_eq!(p.scroll_src, 0);
}

#[test]
fn escape_action_closes_the_pane() {
    let mut p = pane("x");
    assert!(matches!(
        reduce(&mut p, MdInput::Close),
        Some(MdAction::Close)
    ));
}

#[test]
fn reload_picks_up_content_rewritten_after_open() {
    let path = std::env::temp_dir().join("crew_mdkeys_reload_test.md");
    std::fs::write(&path, "old").unwrap();
    let mut p = MdPane::new(path.clone(), "old".to_string());
    std::fs::write(&path, "new content").unwrap();
    assert!(reduce(&mut p, MdInput::Reload).is_none());
    assert_eq!(p.source, "new content");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn reload_failure_keeps_old_content_and_reports_a_status() {
    let path = std::env::temp_dir().join("crew_mdkeys_reload_missing_test.md");
    let _ = std::fs::remove_file(&path); // ensure it doesn't exist
    let mut p = MdPane::new(path, "kept".to_string());
    let action = reduce(&mut p, MdInput::Reload);
    assert_eq!(p.source, "kept", "old content stays on a failed reload");
    assert!(matches!(action, Some(MdAction::Status(_))));
}
