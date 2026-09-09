use super::*;
use crate::motion::MotionLevel::{Full, Off, Subtle};

/// A scripted stream: `(arrival ms, chars in that delta)`.
fn stream(deltas: &[(u64, usize)]) -> (Reveal, usize) {
    let mut st = Reveal::default();
    let mut total = 0;
    for &(at, n) in deltas {
        st = st.arrived(at, total, total + n, Full);
        total += n;
    }
    (st, total)
}

#[test]
fn a_single_delta_types_out_instead_of_landing_whole() {
    let (st, total) = stream(&[(1_000, 200)]);
    assert_eq!(visible_len(&st, 1_000, total, Full), 0, "nothing at t=0");
    let mid = visible_len(&st, 1_100, total, Full);
    assert!(mid > 0 && mid < 200, "typing at +100ms: {mid}");
    assert_eq!(
        visible_len(&st, 4_000, total, Full),
        200,
        "all of it at +3s"
    );
}

#[test]
fn the_reveal_is_monotonic_and_never_exceeds_the_total() {
    let (st, total) = stream(&[(1_000, 30), (1_080, 40), (1_160, 10)]);
    let mut last = 0;
    for now in (1_160..2_500).step_by(7) {
        let v = visible_len(&st, now, total, Full);
        assert!(v >= last, "went backwards at {now}: {v} < {last}");
        assert!(v <= total, "exceeded the text at {now}: {v}");
        last = v;
    }
    assert_eq!(last, total);
}

#[test]
fn everything_pending_is_on_screen_within_catchup_of_the_last_delta() {
    for chunk in [1usize, 7, 40, 400, 3_000] {
        let (st, total) = stream(&[(1_000, 25), (1_080, chunk)]);
        assert!(
            visible_len(&st, 1_080, total, Full) < total,
            "chunk {chunk}: not yet"
        );
        let at = 1_080 + CATCHUP_MS;
        assert_eq!(
            visible_len(&st, at, total, Full),
            total,
            "chunk {chunk} at +400ms"
        );
    }
}

#[test]
fn a_steady_stream_never_trails_arrival_by_more_than_the_lag_bound() {
    // 30-char chunks every gate period (~375 cps), for two seconds.
    let deltas: Vec<(u64, usize)> = (0..25).map(|i| (1_000 + i * 80, 30)).collect();
    let mut st = Reveal::default();
    let mut total = 0;
    for &(at, n) in &deltas {
        st = st.arrived(at, total, total + n, Full);
        total += n;
        // Just before the NEXT chunk lands is the widest the gap gets.
        let lag = total - visible_len(&st, at + 79, total, Full);
        assert!(
            lag as f32 <= LAG_CHUNKS * 30.0 + 1.0,
            "lag {lag} chars at {at}"
        );
    }
}

#[test]
fn the_rate_never_drops_below_the_base_and_subtle_doubles_it() {
    let st = Reveal {
        shown: 0,
        at_ms: 0,
        chunk: 5,
        settled: false,
    };
    assert_eq!(rate_cps(&st, 5, Full), BASE_CPS);
    assert_eq!(rate_cps(&st, 5, Subtle), BASE_CPS * 2.0);
    assert!(
        rate_cps(&st, 500, Full) > BASE_CPS,
        "a big backlog types faster"
    );
}

#[test]
fn motion_off_reveals_everything_instantly() {
    let (st, total) = stream(&[(1_000, 200)]);
    assert_eq!(visible_len(&st, 1_000, total, Off), 200);
    assert_eq!(char_mix(0, Off), 1.0, "and no per-character fade");
    let mut lines = vec![vec![crate::chatbody::plain('x', (9, 9, 9), false)]];
    glow_tail(&mut lines, &st, 1_000, total, Off);
    assert_eq!(lines[0][0].fg, (9, 9, 9), "Off leaves the ink untouched");
}

#[test]
fn settling_continues_from_what_was_visible() {
    let (st, total) = stream(&[(1_000, 100)]);
    let seen = visible_len(&st, 1_200, total, Full);
    assert!(seen > 0 && seen < 100);
    // Longer settled text: no snap-back, and it finishes fast.
    let s = st.settle(1_200, total, 120, Full);
    assert_eq!(visible_len(&s, 1_200, 120, Full), seen);
    assert_eq!(visible_len(&s, 1_200 + CATCHUP_MS, 120, Full), 120);
    assert!(s.settled);
    // Shorter settled text than what was visible: all of it, at once.
    let short = st.settle(1_200, total, seen / 2, Full);
    assert_eq!(visible_len(&short, 1_200, seen / 2, Full), seen / 2);
}

#[test]
fn the_newest_characters_ramp_from_muted_to_ink() {
    assert_eq!(char_mix(0, Full), 0.0);
    assert!(char_mix(GLOW_MS / 2, Full) > 0.4 && char_mix(GLOW_MS / 2, Full) < 0.6);
    assert_eq!(char_mix(GLOW_MS, Full), 1.0);
    assert_eq!(
        char_mix(GLOW_MS, Subtle),
        1.0,
        "subtle ramps faster, never slower"
    );
    // A character revealed later is younger.
    let st = Reveal {
        shown: 0,
        at_ms: 1_000,
        chunk: 90,
        settled: false,
    };
    assert!(char_age_ms(&st, 2_000, 89, 90, Full) < char_age_ms(&st, 2_000, 0, 90, Full));
}

#[test]
fn clip_respects_char_boundaries() {
    assert_eq!(clip("héllo", 2), "hé");
    assert_eq!(clip("héllo", 99), "héllo");
    assert_eq!(clip("", 3), "");
    assert_eq!(clip("日本語", 1), "日");
}

#[test]
fn find_keys_provisional_cards_by_agent_and_settled_ones_by_stamp() {
    let m = crate::chatmsgs::tests::msg("coder \u{2192} user", "hi");
    let mut settled = m;
    settled.ts = "77".into();
    let reveals = vec![CardReveal {
        key: "coder".into(),
        ts: None,
        state: Reveal::default(),
    }];
    assert!(
        find(&reveals, &settled, true).is_some(),
        "streaming: by key"
    );
    assert!(
        find(&reveals, &settled, false).is_none(),
        "settled needs the stamp"
    );
    let keyed = vec![CardReveal {
        ts: Some("77".into()),
        ..reveals[0].clone()
    }];
    assert!(find(&keyed, &settled, false).is_some());
    assert!(find(&keyed, &settled, true).is_none());
}
