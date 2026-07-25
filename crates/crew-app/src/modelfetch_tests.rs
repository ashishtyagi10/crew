use super::*;

fn model(id: &str, price: Option<(u64, u64)>, context: u32) -> LiveModel {
    LiveModel {
        id: id.to_string(),
        name: id.to_string(),
        price,
        free: price == Some((0, 0)),
        context,
    }
}

#[test]
fn cache_round_trips_priced_free_and_unknown_rows_exactly() {
    let models = vec![
        model(
            "anthropic/claude-sonnet-5",
            Some((3_000_000, 15_000_000)),
            1_000_000,
        ),
        model(
            "meta-llama/llama-3.3-70b-instruct:free",
            Some((0, 0)),
            131_072,
        ),
        model("weird/no-pricing", None, 0),
    ];
    let body = cache_body(&models);
    let back = crew_hive::catalog::parse_models(&body).expect("round-tripped cache parses");
    assert_eq!(back, models, "cache round trip must not drift a single µ$");
}

#[test]
fn oversized_cache_file_is_rejected_before_being_read_into_memory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("models-openrouter.json");
    // A planted file well past the generous ceiling: must be rejected by the
    // `metadata().len()` check alone, never handed to `read_to_string`.
    let oversized = "x".repeat((MAX_CACHE_BYTES + 1) as usize);
    std::fs::write(&path, oversized).unwrap();
    assert!(read_cache_at(&path).is_none());
}

#[test]
fn a_normal_sized_fresh_cache_still_reads() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("models-openrouter.json");
    let models = vec![model(
        "anthropic/claude-sonnet-5",
        Some((3_000_000, 15_000_000)),
        1_000_000,
    )];
    std::fs::write(&path, cache_body(&models)).unwrap();
    let back = read_cache_at(&path).expect("small fresh cache reads back");
    assert_eq!(back, models);
}

#[test]
fn per_token_formats_without_losing_precision_on_read_back() {
    // A price that would drift under naive f64 division (1/3 c/Mtok-ish
    // values are exactly the kind of decimal float division mangles).
    for microusd in [1, 3_000_000, 15_000_000, 999_999, 1_000_000_000_000] {
        let s = per_token(microusd);
        let parsed: f64 = s.parse().expect("valid decimal string");
        let back = (parsed * 1_000_000.0 * 1_000_000.0).round() as u64;
        assert_eq!(back, microusd, "{s} did not round-trip");
    }
}
