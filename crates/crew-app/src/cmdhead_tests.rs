use super::*;

use crate::panecard::{pane_card, Bar};

/// Every column of the badge, as a string, for a card `cols` wide.
fn top_row(cells: &[CellView], cols: u16) -> String {
    (0..cols)
        .map(|c| {
            cells
                .iter()
                .find(|x| x.row == 0 && x.col == c)
                .map_or(' ', |x| x.c)
        })
        .collect()
}

#[test]
fn a_short_name_is_the_tick_and_the_name() {
    assert_eq!(
        label("cargo build", 20).as_deref(),
        Some("\u{2576} cargo build")
    );
}

/// A clipped badge is exactly its budget wide — not one column over (which
/// would overwrite the corner) and not one under (which would leave the
/// ellipsis floating).
#[test]
fn a_long_name_loses_its_tail_and_fits_exactly() {
    for avail in MIN_COLS..24 {
        let s = label("cargo test --workspace --all-targets", avail).expect("fits");
        assert_eq!(
            s.chars().count() as u16,
            avail,
            "badge for {avail} columns was {s:?}"
        );
        assert!(s.starts_with("\u{2576} c"), "the head survives: {s:?}");
        assert!(s.ends_with('\u{2026}'), "the tail is elided: {s:?}");
    }
}

#[test]
fn there_is_a_width_below_which_it_says_nothing() {
    for avail in 0..MIN_COLS {
        assert_eq!(label("cargo build", avail), None, "at {avail} columns");
    }
    assert!(label("cargo build", MIN_COLS).is_some());
}

#[test]
fn a_nameless_command_draws_nothing() {
    assert_eq!(label("   ", 40), None);
}

/// The badge is on the card, and only while the pane is scrolled back.
#[test]
fn the_card_names_the_command_only_when_scrolled_back() {
    let scrolled = Bar {
        title: "shell",
        scroll: 40,
        total: 400,
        at_cmd: Some("cargo build"),
        assemble_t: 1.0,
        ..Default::default()
    };
    let cells = pane_card(60, 10, &scrolled);
    let row = top_row(&cells, 62);
    assert!(row.contains("cargo build"), "top border was {row:?}");

    let at_bottom = Bar {
        title: "shell",
        total: 400,
        assemble_t: 1.0,
        ..Default::default()
    };
    let row = top_row(&pane_card(60, 10, &at_bottom), 62);
    assert!(
        !row.contains("cargo build"),
        "the prompt is on screen: {row:?}"
    );
}

/// The badge never reaches into the legend, and it never draws a fragment:
/// `put` overwrites, so a collision is a piece of a word, not two glyphs in
/// one column. Swept across every width a card can be.
#[test]
fn it_is_drawn_whole_or_not_at_all_at_every_width() {
    for gcols in 1u16..=80 {
        let b = Bar {
            index: Some(3),
            title: "crew",
            scroll: 12,
            total: 400,
            at_cmd: Some("cargo build"),
            assemble_t: 1.0,
            min_btn: true,
            ..Default::default()
        };
        let cells = pane_card(gcols, 8, &b);
        let row = top_row(&cells, gcols + 2);
        // The legend is what identifies the card; the badge must never eat
        // into it, so either the whole name is there or none of it is.
        let whole = row.contains("cargo build");
        let elided = row.contains('\u{2026}');
        let tick = row.contains('\u{2576}');
        assert!(
            !tick || whole || elided,
            "fragment at {gcols} columns: {row:?}"
        );
        if tick {
            assert!(
                row.contains("crew") || row.trim_start_matches(['\u{256d}', '\u{2500}']).len() < 8,
                "the badge ate the legend at {gcols} columns: {row:?}"
            );
        }
    }
}
