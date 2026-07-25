use super::*;

fn info(price: Option<(u64, u64)>, free: bool, context: u32) -> ModelInfo {
    ModelInfo {
        name: "n",
        slug: "s",
        or_slug: Some("vendor/alias"),
        vendor: crew_hive::catalog::Vendor::Other,
        price,
        free,
        context,
    }
}

fn live(id: &str, price: Option<(u64, u64)>, free: bool, context: u32) -> LiveModel {
    LiveModel {
        id: id.to_string(),
        name: id.to_string(),
        price,
        free,
        context,
    }
}

#[test]
fn fills_a_curated_none_price_from_a_matching_live_row() {
    let m = info(None, false, 0);
    let live = [live("vendor/alias", Some((5, 10)), false, 0)];
    let (price, _, free) = enrich_with(&m, "vendor/alias", &live);
    assert_eq!(price, Some((5, 10)));
    assert!(!free);
}

#[test]
fn never_overwrites_a_curated_price_that_disagrees_with_live() {
    let m = info(Some((3_000_000, 15_000_000)), false, 0);
    // A live row that (implausibly) disagrees must not win.
    let live = [live("vendor/alias", Some((1, 1)), true, 0)];
    let (price, _, free) = enrich_with(&m, "vendor/alias", &live);
    assert_eq!(
        price,
        Some((3_000_000, 15_000_000)),
        "curated price must stand"
    );
    assert!(
        !free,
        "curated paid status must stand even if live disagrees"
    );
}

#[test]
fn fills_a_zero_curated_context_but_not_a_known_one() {
    let unknown = info(None, false, 0);
    let rows = [live("vendor/alias", None, false, 1_000_000)];
    let (_, context, _) = enrich_with(&unknown, "vendor/alias", &rows);
    assert_eq!(context, 1_000_000);

    let known = info(None, false, 200_000);
    let (_, context, _) = enrich_with(&known, "vendor/alias", &rows);
    assert_eq!(
        context, 200_000,
        "a known context window is never overwritten"
    );
}

#[test]
fn no_matching_live_row_leaves_the_curated_row_untouched() {
    let m = info(None, false, 0);
    let live = [live("some/other-model", Some((5, 10)), false, 42)];
    let (price, context, free) = enrich_with(&m, "vendor/alias", &live);
    assert_eq!(price, None);
    assert_eq!(context, 0);
    assert!(!free);
}

#[test]
fn a_zero_price_live_match_marks_free_without_inventing_a_number() {
    let m = info(None, false, 0);
    let live = [live("vendor/alias", Some((0, 0)), true, 0)];
    let (price, _, free) = enrich_with(&m, "vendor/alias", &live);
    assert_eq!(price, Some((0, 0)));
    assert!(free);
}

#[test]
fn no_or_slug_never_touches_the_live_overlay() {
    let m = ModelInfo {
        or_slug: None,
        ..info(None, false, 0)
    };
    // enrich() (not enrich_with) is the one that reads the process-global;
    // a row with no OpenRouter alias must short-circuit before ever locking
    // it, so this can't be flaky against whatever another test published.
    let (price, context, free) = enrich(&m);
    assert_eq!((price, context, free), (None, 0, false));
}

#[test]
fn set_live_publishes_through_the_global_for_enrich() {
    // A smoke test of the real plumbing (set_live → the shared Mutex →
    // enrich), using an id no other test or catalog row can collide with,
    // so it stays safe to run alongside every other test in the binary.
    set_live(vec![live(
        "test-only/does-not-exist-elsewhere",
        Some((7, 9)),
        false,
        4096,
    )]);
    let m = ModelInfo {
        or_slug: Some("test-only/does-not-exist-elsewhere"),
        ..info(None, false, 0)
    };
    let (price, context, free) = enrich(&m);
    assert_eq!(price, Some((7, 9)));
    assert_eq!(context, 4096);
    assert!(!free);
}
