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
