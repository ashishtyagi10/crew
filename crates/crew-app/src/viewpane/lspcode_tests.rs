//! A server's quoted names are drawn as code, not as ticks.
use super::*;

fn text(l: &CardLine) -> String {
    l.iter().map(|c| c.c).collect()
}

/// rustc's ``cannot find function `helper` ``: the ticks go, and the name
/// stands upright in fuller ink inside the italic note.
#[test]
fn quoted_names_lose_their_ticks_and_stand_upright() {
    let _g = crate::app::theme_test_guard();
    let fg = (200, 80, 80);
    let l = row("cannot find function `helper` in this scope", fg, 2, 60);
    assert_eq!(
        text(&l),
        "  \u{2191} cannot find function helper in this scope"
    );
    let t = text(&l);
    let at = t[..t.find("helper").unwrap()].chars().count();
    let name = &l[at..at + "helper".len()];
    assert!(
        name.iter().all(|c| !c.italic && c.fg != fg),
        "the name is code"
    );
    assert!(
        l[at - 2].italic && l[at - 2].fg == fg,
        "the prose around it is not"
    );
    assert!(
        l[at + "helper".len() + 1].italic,
        "and the prose resumes after"
    );
}

/// A lone tick is punctuation, not the start of a span that runs to the end.
#[test]
fn an_unbalanced_tick_is_left_as_written() {
    let _g = crate::app::theme_test_guard();
    let l = row("expected one of `,` or `)", (200, 80, 80), 0, 60);
    assert_eq!(text(&l), "\u{2191} expected one of `,` or `)");
    assert!(l.iter().all(|c| c.italic));
}

/// Cut to the column still ends in `…`, measured without the ticks.
#[test]
fn a_clipped_note_measures_the_text_it_draws() {
    let _g = crate::app::theme_test_guard();
    let l = row(
        "unused variable: `a_rather_long_binding_name`",
        (200, 80, 80),
        0,
        24,
    );
    assert_eq!(l.len(), 24);
    assert!(text(&l).ends_with('\u{2026}'), "{:?}", text(&l));
    assert!(!text(&l).contains('`'));
}
