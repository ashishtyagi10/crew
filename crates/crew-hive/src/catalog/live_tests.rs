use super::*;

const FIXTURE: &str = r#"{"data":[
  {"id":"anthropic/claude-sonnet-5","name":"Anthropic: Claude Sonnet 5",
   "context_length":1000000,
   "pricing":{"prompt":"0.000003","completion":"0.000015"}},
  {"id":"meta-llama/llama-3.3-70b-instruct:free","name":"Meta: Llama 3.3 70B (free)",
   "context_length":131072,
   "pricing":{"prompt":"0","completion":"0"}},
  {"id":"weird/no-pricing","name":"No Pricing","context_length":0,
   "pricing":{"prompt":"","completion":""}}
]}"#;

#[test]
fn parses_per_token_strings_into_microusd_per_mtok() {
    let got = parse_models(FIXTURE).expect("fixture parses");
    let sonnet = got
        .iter()
        .find(|m| m.id == "anthropic/claude-sonnet-5")
        .unwrap();
    // $0.000003/token * 1M tokens = $3 = 3_000_000 µ$.
    assert_eq!(sonnet.price, Some((3_000_000, 15_000_000)));
    assert!(!sonnet.free);
    assert_eq!(sonnet.context, 1_000_000);
}

#[test]
fn zero_price_is_free_and_unparseable_price_is_unknown() {
    let got = parse_models(FIXTURE).unwrap();
    let llama = got.iter().find(|m| m.id.ends_with(":free")).unwrap();
    assert!(llama.free);
    assert_eq!(llama.price, Some((0, 0)));
    let weird = got.iter().find(|m| m.id == "weird/no-pricing").unwrap();
    assert_eq!(weird.price, None); // never invent a number
    assert!(!weird.free);
}

#[test]
fn malformed_json_is_an_error_not_a_panic() {
    assert!(parse_models("not json").is_err());
    assert!(parse_models("{}").is_err());
}

#[test]
fn a_context_length_past_u32_max_falls_back_to_unknown_not_a_wrapped_value() {
    // 4294967297 = u32::MAX + 2. The old `as u32` cast would silently
    // wrap this to `1` — a small, wrong, but plausible-looking context
    // window. `try_from(..).unwrap_or(0)` must land on the honest
    // "unknown" (0) instead.
    let body = r#"{"data":[
      {"id":"garbage/context","name":"Garbage",
       "context_length":4294967297,
       "pricing":{"prompt":"0.000003","completion":"0.000015"}}
    ]}"#;
    let got = parse_models(body).unwrap();
    assert_eq!(got[0].context, 0);
}
