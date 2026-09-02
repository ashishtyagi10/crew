use super::*;
use crate::viewpane::detect::Extractor;
use std::time::{Duration, SystemTime};

fn text(l: &CardLine) -> String {
    l.iter().map(|c| c.c).collect()
}

fn card(why: Opaque, meta: Option<FileMeta>, cols: usize) -> Vec<CardLine> {
    opaque_card(why, meta.as_ref(), cols)
}

#[test]
fn fmt_size_stays_in_bytes_under_a_kib() {
    assert_eq!(fmt_size(427), "427 B");
}

#[test]
fn fmt_size_steps_up_a_unit_at_a_time() {
    assert_eq!(fmt_size(1536), "1.5K");
    assert_eq!(fmt_size(34 * 1024 * 1024), "34M");
}

#[test]
fn mtime_str_reports_unknown_without_a_stat() {
    assert_eq!(mtime_str(None), "unknown");
}

#[test]
fn mtime_str_reports_something_other_than_unknown_for_a_real_time() {
    // Not asserting the exact relative-time bucket (that's `chattime`'s own
    // test) — only that a real `SystemTime` produces something other than
    // the "no data" sentinel.
    let t = SystemTime::now() - Duration::from_secs(300);
    assert_ne!(mtime_str(Some(t)), "unknown");
}

#[test]
fn a_metadata_card_with_a_stat_shows_size_and_mtime() {
    let meta = FileMeta {
        size: 34 * 1024 * 1024,
        modified: Some(SystemTime::now() - Duration::from_secs(3600)),
    };
    let ls = card(Opaque::Binary, Some(meta), 60);
    let body: String = ls.iter().map(text).collect::<Vec<_>>().join("\n");
    assert!(body.contains("34M"), "shows the size: {body}");
    assert!(body.contains("modified"), "shows a modified line: {body}");
}

#[test]
fn a_metadata_card_without_a_stat_omits_the_metadata_line_but_keeps_the_offer() {
    // `Unreadable` is the one rung `load_now` reaches with no successful
    // `stat` behind it — there is nothing to report a size or mtime FROM.
    let ls = card(Opaque::Unreadable, None, 60);
    let body: String = ls.iter().map(text).collect::<Vec<_>>().join("\n");
    assert!(!body.contains("modified"), "nothing to report: {body}");
    assert!(body.contains("press"), "still offers o: {body}");
}

#[test]
fn an_unreadable_file_does_not_claim_to_be_binary() {
    // Fix 2's fold-in item: `load_now` used to tag every read failure
    // (not-found, permission-denied, ...) `Opaque::Binary`, which is a
    // false, specific claim about bytes the loader never read. Mutating
    // `opaque_card`'s `Unreadable` arm back to the `Binary` wording is
    // exactly the regression this guards against.
    let ls = card(Opaque::Unreadable, None, 60);
    let head = text(&ls[0]);
    assert!(
        !head.contains("binary"),
        "an unreadable file is not a binary file: {head}"
    );
}

#[test]
fn each_opaque_reason_gets_a_distinct_head_line() {
    // Broader than any single wording check: no two reasons should collapse
    // onto the same head text, which is what "doesn't discriminate" would
    // look like here.
    let heads: Vec<String> = [
        Opaque::Binary,
        Opaque::NotUtf8,
        Opaque::NoExtractor(Extractor::PdfToText),
        Opaque::Unreadable,
    ]
    .into_iter()
    .map(|why| text(&card(why, None, 60)[0]))
    .collect();
    for i in 0..heads.len() {
        for j in (i + 1)..heads.len() {
            assert_ne!(heads[i], heads[j], "reasons {i} and {j} read the same");
        }
    }
}

#[test]
fn the_card_still_offers_o_for_every_reason() {
    let ls = card(Opaque::NoExtractor(Extractor::PdfToText), None, 60);
    let body: String = ls.iter().map(text).collect::<Vec<_>>().join("\n");
    assert!(body.contains("press") && body.contains('o'), "got {body}");
}

/// A narrow viewer wraps the card's sentences rather than cutting them.
#[test]
fn a_narrow_card_wraps_every_line_and_loses_no_word() {
    let meta = FileMeta {
        size: 34 * 1024 * 1024,
        modified: Some(SystemTime::now() - Duration::from_secs(3600)),
    };
    let ls = card(Opaque::Binary, Some(meta), 30);
    for l in &ls {
        assert!(l.len() <= 30, "{:?}", text(l));
    }
    let body: String = ls.iter().map(text).collect::<Vec<_>>().join(" ");
    assert!(body.contains("default app"), "{body}");
    assert!(body.contains("ago"), "{body}");
    assert!(ls.len() > 4, "wrapped onto more rows: {}", ls.len());
}
