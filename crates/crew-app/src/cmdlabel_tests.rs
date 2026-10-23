use super::spans;
use crate::suggest::MenuItem;
use ratatui::style::Color;

const LONG: &str = "@crates/crew-app/src/viewpane/render_tests.rs";

fn item(label: &str, hit: Vec<usize>) -> MenuItem {
    MenuItem {
        label: label.into(),
        hit,
        ..Default::default()
    }
}

/// A row its picker marks as a path, as `/view` and `@` files do.
fn path(label: &str, hit: Vec<usize>) -> MenuItem {
    MenuItem {
        path: true,
        ..item(label, hit)
    }
}

fn text(item: &MenuItem, avail: usize) -> String {
    spans(item, avail, Color::Reset)
        .iter()
        .map(|s| s.content.as_ref())
        .collect()
}

/// A path keeps the file it names, cut on a separator after its `@`.
#[test]
fn a_long_path_keeps_its_file_name() {
    let got = text(&path(LONG, vec![]), 30);
    assert_eq!(got, "@\u{2026}/viewpane/render_tests.rs");
    assert!(got.chars().count() <= 30);
    assert_eq!(
        text(&path("@a/b.rs", vec![]), 30),
        "@a/b.rs",
        "whole when it fits"
    );
}

/// The fuzzy match's mark stays on the letter it matched.
#[test]
fn a_match_mark_follows_its_letter() {
    let r = LONG.find("render").unwrap();
    let marked: String = spans(&path(LONG, vec![0, r]), 30, Color::Reset)
        .iter()
        .filter(|s| s.style.bg.is_some())
        .map(|s| s.content.to_string())
        .collect();
    assert_eq!(marked, "@r");
}

/// A row with a description, or no `/`, is cut at its end as before.
#[test]
fn anything_else_keeps_its_start() {
    let mut cmd = item("/attach-something-long", vec![]);
    cmd.desc = "attaches".into();
    assert!(text(&cmd, 10).starts_with("/attach"));
}

/// A row of words that holds a `/` (a chat-find or history hit) is not a
/// path: it keeps its start, where the sender and the match are.
#[test]
fn a_sentence_with_a_slash_keeps_its_start() {
    let hit = item(
        "user: @scout what changed in plot/heatmap.rs since v0.19.60?",
        vec![],
    );
    assert!(
        text(&hit, 30).starts_with("user: @scout what"),
        "{}",
        text(&hit, 30)
    );
}

/// A path with a space in it is still a path: six files under
/// `Application Support` all read `@Library/Application Su…`, the same row
/// six times, when a space made it read as words.
#[test]
fn a_path_with_a_space_keeps_its_file_name() {
    let row = path(
        "@Library/Application Support/Code/User/settings.json",
        vec![],
    );
    let got = text(&row, 24);
    assert!(got.ends_with("/settings.json"), "{got}");
}
