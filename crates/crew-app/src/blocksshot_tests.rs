//! `/blocks` fits the tile it is opened in, and the picture of it — on the
//! plain rung `/tools` draws on (`toolshot_tests` holds the harness).
//!
//! `#[ignore]`d (needs a GPU adapter, writes PNGs):
//! `CREW_SHOT_DIR=<dir> cargo test -p crew-app --bin crew blocks_shot -- --ignored --nocapture`
use super::*;
use crate::toolshot_tests::{intact, tools_shot};

/// Three blocks a session actually has: a short one, a long test command
/// that outgrows the row, and one still running.
fn spans() -> Spans {
    let mut s = Spans::default();
    s.started("cargo build".into(), 0, 0);
    s.finished(Some(0), 40, 12_000);
    s.started(
        "CREW_SHOT_DIR=/tmp/shots cargo test -p crew-app --bin crew blocks_shot -- --ignored --nocapture"
            .into(),
        40,
        20_000,
    );
    s.finished(Some(101), 90, 144_000);
    s.started("git status".into(), 90, 150_000);
    s
}

/// No row is wider than the narrowest tile the listing is opened in. The row
/// was `{n}  {took}  {outcome}  {command}` with the command uncut, so the one
/// column that matters wrapped wherever the viewer happened to be narrow.
#[test]
fn no_row_is_wider_than_a_tiled_viewer() {
    let text = listing(&spans(), "shell", 160_000);
    for line in text.lines().filter(|l| l.starts_with(' ')) {
        assert!(
            line.chars().count() <= ROW_W,
            "{} cols: {line:?}",
            line.chars().count()
        );
    }
}

/// A command longer than its column is cut in the middle — both ends tell
/// two commands apart — and repeated whole underneath, so nothing is lost.
#[test]
fn a_long_command_is_cut_on_its_row_and_whole_underneath() {
    let text = listing(&spans(), "shell", 160_000);
    let row = text
        .lines()
        .find(|l| l.contains("\u{2717} 101"))
        .expect("the failed row");
    assert!(row.contains('\u{2026}'), "cut: {row:?}");
    assert!(row.contains("CREW_SHOT"), "the head survives: {row:?}");
    assert!(row.contains("nocapture"), "the tail survives: {row:?}");
    let under: Vec<&str> = text
        .lines()
        .skip_while(|l| !l.contains("\u{2717} 101"))
        .skip(1)
        .take_while(|l| l.starts_with("      "))
        .collect();
    assert!(!under.is_empty(), "repeated underneath: {text}");
    assert_eq!(
        under.iter().map(|l| l.trim()).collect::<Vec<_>>().join(" "),
        spans().recent().nth(1).unwrap().name,
        "whole, on words"
    );
    // …and a short one is neither cut nor repeated.
    let short = text.lines().find(|l| l.contains("cargo build")).unwrap();
    assert!(!short.contains('\u{2026}'), "{short:?}");
    assert_eq!(text.matches("cargo build").count(), 1);
}

#[test]
#[ignore = "needs a GPU adapter; writes PNGs"]
fn blocks_shot_as_a_tile_and_wide() {
    let _g = crate::app::theme_test_guard();
    let text = listing(&spans(), "shell", 160_000);
    for (name, w) in [("blocks-tile", 420u32), ("blocks-wide", 1000)] {
        let Some(rows) = tools_shot(name, &text, w) else {
            eprintln!("no GPU adapter — skipping (this is a skip, not a pass)");
            return;
        };
        let all = rows.join("\n");
        assert!(all.contains("running"), "{name}:\n{all}");
        intact(&rows, &text, name);
        assert!(!all.contains("1 # blocks"), "{name} has a gutter:\n{all}");
    }
}
