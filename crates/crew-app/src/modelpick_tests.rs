use super::*;

fn labels(q: &str) -> Vec<String> {
    rows(q, None).into_iter().map(|i| i.label).collect()
}

#[test]
fn default_row_leads_and_sections_are_headed() {
    let r = rows("", None);
    assert_eq!(r[0].label, "default");
    assert!(!r[0].header);
    assert_eq!(r[0].fill, "default");
    // The first section header is Anthropic, and every header is inert.
    let first_header = r.iter().find(|i| i.header).expect("a section header");
    assert_eq!(first_header.label, "anthropic");
    assert!(r.iter().filter(|i| i.header).all(|i| i.fill.is_empty()));
}

#[test]
fn every_header_has_at_least_one_row_under_it() {
    for q in ["", "claude", "free", "qwen"] {
        let r = rows(q, None);
        for (i, item) in r.iter().enumerate() {
            if item.header {
                assert!(
                    r.get(i + 1).is_some_and(|next| !next.header),
                    "empty section {:?} for query {q:?}",
                    item.label
                );
            }
        }
    }
}

#[test]
fn query_matches_name_slug_vendor_and_free_badge() {
    assert!(labels("sonnet").iter().any(|l| l.contains("Sonnet")));
    assert!(labels("claude-opus-5").iter().any(|l| l.contains("Opus 5")));
    assert!(labels("anthropic").iter().any(|l| l.contains("Claude")));
    // "free" is a first-class filter term.
    let free = rows("free", None);
    assert!(!free.is_empty());
    assert!(free
        .iter()
        .filter(|i| !i.header && i.fill != "default")
        .all(|i| i.desc.contains("free")));
}

#[test]
fn the_current_model_is_marked_once() {
    let r = rows("", Some("claude-sonnet-5"));
    let marked: Vec<&MenuItem> = r.iter().filter(|i| i.desc.contains('\u{25cf}')).collect();
    assert_eq!(marked.len(), 1);
    assert!(marked[0].label.contains("Sonnet 5"));
    // No current model → no mark anywhere.
    assert!(rows("", None).iter().all(|i| !i.desc.contains('\u{25cf}')));
}

#[test]
fn priced_rows_badge_dollars_and_unpriced_rows_badge_a_dash() {
    let r = rows("claude-sonnet-5", Some("x"));
    let row = r.iter().find(|i| !i.header && i.fill != "default").unwrap();
    assert!(row.desc.contains("$3/$15"), "{}", row.desc);
    let g = rows("gemini-2.5-pro", None);
    let row = g.iter().find(|i| !i.header && i.fill != "default").unwrap();
    assert!(row.desc.contains('\u{2014}'), "{}", row.desc);
}

#[test]
fn rows_submit_and_carry_a_slug() {
    for item in rows("", None).iter().filter(|i| !i.header) {
        assert!(item.submit, "{} should run on Enter", item.label);
        assert!(!item.fill.is_empty());
    }
}

#[test]
fn recents_lead_the_list_and_dont_duplicate_a_section_row() {
    let r = rows_with_recents("", None, &["qwen-max".to_string()]);
    let header = r.iter().position(|i| i.header && i.label == "recent");
    let anthropic = r.iter().position(|i| i.header && i.label == "anthropic");
    assert!(header < anthropic, "recent must lead the sections");
    // The recent row still appears in its own vendor section (it's a shortcut,
    // not a move) — exactly two rows carry the slug.
    assert_eq!(r.iter().filter(|i| i.fill == "qwen-max").count(), 2);
    // An unknown slug in recents is skipped rather than rendered blank.
    let r = rows_with_recents("", None, &["ghost-model".to_string()]);
    assert!(!r.iter().any(|i| i.header && i.label == "recent"));
}

#[test]
fn a_duplicated_recent_slug_renders_only_one_recent_row() {
    // A hand-edited config could carry the same slug twice; the recent
    // section must still show it once, not twice.
    let r = rows_with_recents("", None, &["qwen-max".to_string(), "qwen-max".to_string()]);
    let recent_header = r
        .iter()
        .position(|i| i.header && i.label == "recent")
        .unwrap();
    let next_header = r
        .iter()
        .skip(recent_header + 1)
        .position(|i| i.header)
        .unwrap();
    // Exactly one non-header row between "recent" and the next section.
    assert_eq!(next_header, 1);
}

#[test]
fn a_recent_stored_as_an_openrouter_alias_resolves_and_renders() {
    // `Route::fill_slug` persists `m.or_slug` (not `m.slug`) when OpenRouter
    // is the active provider, so an OpenRouter-primary user's recents list
    // carries the alias verbatim — e.g. "qwen-max" is stored as
    // "qwen/qwen3-max". That must still resolve to a row (and lead the
    // list), not silently vanish.
    let r = rows_with_recents("", None, &["qwen/qwen3-max".to_string()]);
    let header = r
        .iter()
        .position(|i| i.header && i.label == "recent")
        .expect("recent section renders for an OpenRouter-alias entry");
    let recent_row = &r[header + 1];
    assert_eq!(recent_row.label, "Qwen Max");
    // The current-model mark uses the same either/or match, so a current
    // pick stored as the alias is marked too.
    let r2 = rows_with_recents("", Some("qwen/qwen3-max"), &[]);
    let marked: Vec<&MenuItem> = r2.iter().filter(|i| i.desc.contains('\u{25cf}')).collect();
    assert_eq!(marked.len(), 1);
    assert_eq!(marked[0].label, "Qwen Max");
}

#[test]
fn model_row_dims_exactly_the_unserveable_routes() {
    // `rows()` itself can't be driven to a known route in a unit test — the
    // active provider comes from a live, once-per-process probe
    // (`shellprobe::provider_now`) that's never initialized under `cargo
    // test`, so every row it builds sees `Route::Unknown`. `model_row` is
    // the row-building step factored out so the dim wiring is testable on
    // its own, with an explicit route standing in for the probe result.
    use crew_hive::catalog::{ModelInfo, Vendor};
    let m = ModelInfo {
        name: "n",
        slug: "s",
        or_slug: None,
        vendor: Vendor::Anthropic,
        price: None,
        free: false,
        context: 0,
    };
    assert!(model_row(&m, Route::Missing("X"), false).dim);
    assert!(!model_row(&m, Route::Direct("anthropic"), false).dim);
    assert!(!model_row(&m, Route::Unknown, false).dim);
}
