use super::*;

#[test]
fn slugs_are_unique_and_non_empty() {
    let mut seen: Vec<&str> = Vec::new();
    for m in catalog() {
        assert!(!m.slug.is_empty(), "empty slug for {}", m.name);
        assert!(!m.name.is_empty(), "empty name for {}", m.slug);
        assert!(!seen.contains(&m.slug), "duplicate slug {}", m.slug);
        seen.push(m.slug);
    }
}

#[test]
fn free_rows_are_zero_priced_and_paid_rows_are_not() {
    for m in catalog() {
        if m.free {
            assert_eq!(m.price, Some((0, 0)), "free row {} must price at 0", m.slug);
        } else if let Some((inp, out)) = m.price {
            assert!(inp > 0 && out > 0, "paid row {} has a zero rate", m.slug);
        }
    }
}

#[test]
fn the_majors_are_all_represented() {
    for v in [
        Vendor::Anthropic,
        Vendor::OpenAI,
        Vendor::Alibaba,
        Vendor::DeepSeek,
        Vendor::Nvidia,
    ] {
        assert!(
            catalog().iter().any(|m| m.vendor == v),
            "no rows for {}",
            v.label()
        );
    }
}

#[test]
fn priced_rows_match_the_pricing_table() {
    // The catalog badge and the statusline `$` must agree: a 1M-in call on
    // the catalog's price equals `pricing::cost_microusd` for the same
    // slug, for *any* row — not just Anthropic's. `cost_microusd` returns
    // 0 for a slug that matches no `RATES` pattern (and legitimately for
    // free rows, which are `(0, 0)` in the catalog too), so those are
    // skipped rather than asserted on: an unmatched row proves nothing
    // about agreement.
    for m in catalog().iter().filter(|m| m.price.is_some()) {
        let (inp, _) = m.price.expect("filtered to priced rows");
        let got = crate::pricing::cost_microusd(m.slug, 1_000_000, 0);
        if got == 0 {
            continue;
        }
        assert_eq!(got, inp, "catalog and pricing disagree on {}", m.slug);
    }
}

/// The models Anthropic serves today (its model table, 2026-09) are rows at
/// their list price, with the alias OpenRouter's public list gives them, and
/// lead the Anthropic block so the picker offers the newest first.
#[test]
fn current_claude_models_are_listed_newest_first() {
    let row = |slug: &str| catalog().iter().position(|m| m.slug == slug);
    const M: u64 = 1_000_000;
    for (slug, alias, price) in [
        (
            "claude-opus-5-5",
            "anthropic/claude-opus-5.5",
            (4 * M, 20 * M),
        ),
        (
            "claude-sonnet-5-5",
            "anthropic/claude-sonnet-5.5",
            (2 * M, 10 * M),
        ),
        (
            "claude-fable-5-1",
            "anthropic/claude-fable-5.1",
            (10 * M, 50 * M),
        ),
    ] {
        let i = row(slug).unwrap_or_else(|| panic!("{slug} is not catalogued"));
        let m = &catalog()[i];
        assert_eq!(m.vendor, Vendor::Anthropic, "{slug}");
        assert_eq!(m.or_slug, Some(alias), "{slug}");
        assert_eq!(m.price, Some(price), "{slug}");
        assert_eq!(m.context, 1_000_000, "{slug}");
        assert!(i < row("claude-opus-5").unwrap(), "{slug} after Opus 5");
    }
    let sonnet5 = &catalog()[row("claude-sonnet-5").unwrap()];
    assert_eq!(sonnet5.price, Some((2 * M, 10 * M)));
}
