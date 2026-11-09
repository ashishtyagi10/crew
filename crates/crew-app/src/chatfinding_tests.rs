use super::*;
use crate::chatlayout::Message;

const REVIEW: &str = crate::reviewshot_tests::REVIEW;

fn text(row: &CardLine) -> String {
    row.iter().map(|c| c.c).collect()
}

fn review(text: &str) -> Message {
    Message {
        sender: "smith \u{2192} user".into(),
        text: text.into(),
        ts: String::new(),
        meta: String::new(),
        usage: None,
        expanded: false,
    }
}

/// The chip carries its severity as a block: the label's cells sit on the
/// severity's colour, a blocker's on the bell, a warn's on another ink.
#[test]
fn a_finding_leads_with_a_chip_in_its_severity_colour() {
    let _g = crate::app::theme_test_guard();
    let rows = crate::chatbody::body_lines(REVIEW, 90, (220, 220, 220), false);
    let t = crew_theme::theme();
    let bg_of = |row: &CardLine, word: &str| {
        let s = text(row);
        let at = s.chars().count() - s[s.find(word).unwrap()..].chars().count();
        row[at].bg
    };
    let blocker = rows.iter().find(|r| text(r).contains("blocker")).unwrap();
    assert_eq!(
        bg_of(blocker, "blocker"),
        Some(sev_color("blocker")),
        "{}",
        text(blocker)
    );
    let warn = rows.iter().find(|r| text(r).contains("warn")).unwrap();
    assert_eq!(bg_of(warn, "warn"), Some(t.ansi[11]));
    // The citation follows in path ink, the separators gone.
    let s = text(blocker);
    assert!(
        s.contains("crates/crew-theme/src/lib.rs:42 `luminance`")
            || s.contains("lib.rs:42 luminance"),
        "{s}"
    );
    let at = s.chars().count() - s[s.find("crates/").unwrap()..].chars().count();
    assert_eq!(blocker[at].fg, crate::chatink::link_color());
    assert!(
        !s.contains('\u{2014}'),
        "no em dash left between the fields: {s}"
    );
}

/// The verdict and the blank row above it leave the body: two rows fewer
/// than the markdown list, the count now in the header.
#[test]
fn the_verdict_becomes_the_header_tally() {
    let _g = crate::app::theme_test_guard();
    let rows = crate::chatbody::body_lines(REVIEW, 90, (220, 220, 220), false);
    assert!(rows.iter().all(|r| !text(r).contains("verdict")));
    let md = crate::chatmd::map_chat(crate::md::render_chat(REVIEW, 88), 88, (0, 0, 0), None);
    assert!(rows.len() + 2 <= md.len(), "{} vs {}", rows.len(), md.len());
    let head = crate::chatcard::header_line(&review(REVIEW), 0, None);
    let s = text(&head);
    assert!(
        s.ends_with("\u{b7} 1 blocker \u{b7} 1 warn \u{b7} 1 nit"),
        "{s}"
    );
    let one = head.iter().rposition(|c| c.c == '1').unwrap();
    assert_eq!(head[one].fg, crew_theme::theme().text_muted, "nit is quiet");
    let b = s.find("1 blocker").unwrap();
    assert_eq!(head[s[..b].chars().count()].fg, sev_color("blocker"));
}

/// A wrapped why hangs under the citation, not under the chip.
#[test]
fn a_long_why_hangs_under_the_citation() {
    let _g = crate::app::theme_test_guard();
    let rows = crate::chatbody::body_lines(REVIEW, 40, (220, 220, 220), false);
    let first = rows
        .iter()
        .position(|r| text(r).contains("blocker"))
        .unwrap();
    let head = text(&rows[first]);
    assert!(
        head.contains("\u{2026}") && head.contains("lib.rs:42"),
        "{head:?}"
    );
    let cite = head.chars().position(|c| c == '\u{2026}').unwrap();
    let next = text(&rows[first + 1]);
    assert_eq!(
        next.chars().take_while(|c| *c == ' ').count(),
        cite,
        "{next:?}"
    );
}

/// Bullets that only look like findings, with no verdict, stay prose; a
/// reply with prose around the findings keeps it.
#[test]
fn only_a_checked_review_is_drawn_as_findings() {
    assert!(is_review(REVIEW));
    assert!(
        !is_review("- warn — a.rs:3 — maybe"),
        "no verdict: an answer"
    );
    assert!(tally_cells("plain words").is_empty());
    let _g = crate::app::theme_test_guard();
    let both = format!("Two things stand out.\n\n{REVIEW}");
    let rows = card_lines(&both, 90, (220, 220, 220), None);
    assert!(text(&rows[0]).contains("Two things stand out."));
}

/// The blocker must not read as the warn on any page with hues to tell them
/// by: the bell was the first pick, and on prism it is the warn's amber.
#[test]
fn blocker_and_warn_read_apart_in_every_hued_palette() {
    let _g = crate::app::theme_test_guard();
    for id in crew_theme::ALL_THEMES.iter().filter(|id| !id.is_crt()) {
        crew_theme::set_theme(*id);
        let (b, w) = (sev_color("blocker"), sev_color("warn"));
        let d = |x: u8, y: u8| (x as i32 - y as i32).abs();
        let apart = d(b.0, w.0) + d(b.1, w.1) + d(b.2, w.2);
        assert!(apart > 60, "{id:?}: blocker {b:?} vs warn {w:?}");
        assert!(b.0 > b.1, "{id:?}: a blocker is red-leaning, {b:?}");
    }
}
