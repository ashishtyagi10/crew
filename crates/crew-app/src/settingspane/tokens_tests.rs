use super::*;

#[test]
fn millions_read_the_way_a_person_would_say_them() {
    assert_eq!(label(5_000_000), "5");
    assert_eq!(label(25_000_000), "25");
    assert_eq!(label(7_500_000), "7.5");
    assert_eq!(label(5_123_456), "5.12");
    assert_eq!(label(FLOOR), "0.01");
    assert_eq!(label(0), "0");
}

#[test]
fn typing_millions_stores_tokens() {
    assert_eq!(parse("5"), Some(5_000_000));
    assert_eq!(parse("25"), Some(25_000_000));
    assert_eq!(parse(" 7.5 "), Some(7_500_000));
    assert_eq!(parse("0.01"), Some(10_000));
    // Under the floor is raised to it, never to zero: the footer divides by
    // the budget to draw its bar.
    assert_eq!(parse("0"), Some(FLOOR));
    assert_eq!(parse("0.000001"), Some(FLOOR));
    assert_eq!(parse("nope"), None);
    assert_eq!(parse(""), None);
    assert_eq!(parse("-3"), None);
}

/// The rule the module exists for: a buffer nobody edited must not rewrite
/// the stored value, even when it cannot represent it exactly.
#[test]
fn an_untouched_buffer_never_quantises_what_is_stored() {
    let odd = 5_123_456;
    assert_eq!(label(odd), "5.12");
    assert_eq!(commit("5.12", odd), odd, "an untouched display rewrote it");
    assert_eq!(commit(" 5.12 ", odd), odd, "whitespace defeated the guard");
    // An actual edit does commit, quantised as typed — that is the user's
    // number now, not a rounding of someone else's.
    assert_eq!(commit("6", odd), 6_000_000);
    assert_eq!(commit("5.13", odd), 5_130_000);
    // And a typo keeps what was there rather than landing on the floor.
    assert_eq!(commit("nope", odd), odd);
    assert_eq!(commit("", odd), odd);
}

/// Every value the form can produce survives the round trip it will take.
#[test]
fn everything_the_form_produces_reads_back_as_itself() {
    for typed in ["0", "0.01", "1", "5", "7.5", "25", "100", "999"] {
        let tokens = parse(typed).unwrap();
        assert_eq!(
            commit(&label(tokens), tokens),
            tokens,
            "`{typed}` did not survive a display/commit round trip"
        );
    }
}
