use super::*;

#[test]
fn sentinel_close_yields_answered() {
    let mut a = PendingAsk::new("q7".into(), 0, 0);
    assert!(matches!(
        a.observe(Obs {
            new_output: "working\u{2026}",
            idle_transition: false,
            now_ms: 100
        }),
        Step::Wait
    ));
    let s = a.observe(Obs {
        new_output: "\nCREW-ANS-q7: v2\n",
        idle_transition: false,
        now_ms: 200,
    });
    assert!(matches!(s, Step::Answered(t) if t == "v2"));
}

#[test]
fn idle_with_no_output_is_idle_no_engage() {
    let mut a = PendingAsk::new("q7".into(), 0, 0);
    assert!(matches!(
        a.observe(Obs {
            new_output: "",
            idle_transition: true,
            now_ms: 50
        }),
        Step::IdleNoEngage
    ));
}

#[test]
fn idle_after_output_without_close_is_stalled_with_partial() {
    let mut a = PendingAsk::new("q7".into(), 0, 0);
    a.observe(Obs {
        new_output: "thinking about it",
        idle_transition: false,
        now_ms: 100,
    });
    let s = a.observe(Obs {
        new_output: "",
        idle_transition: true,
        now_ms: 200,
    });
    assert!(matches!(s, Step::Stalled(Some(p)) if p.contains("thinking")));
}

#[test]
fn active_but_silent_past_adaptive_budget_is_stalled() {
    let mut a = PendingAsk::new("q7".into(), 0, 0);
    a.observe(Obs {
        new_output: "x",
        idle_transition: false,
        now_ms: 0,
    });
    // Silent from ms 0; base budget 4000 → waiting at 3999, stalled at 4001.
    assert!(matches!(
        a.observe(Obs {
            new_output: "",
            idle_transition: false,
            now_ms: 3_999
        }),
        Step::Wait
    ));
    assert!(matches!(
        a.observe(Obs {
            new_output: "",
            idle_transition: false,
            now_ms: 4_001
        }),
        Step::Stalled(_)
    ));
}

#[test]
fn long_stream_earns_more_patience() {
    let mut a = PendingAsk::new("q7".into(), 0, 0);
    for t in (0..=10_000).step_by(1_000) {
        a.observe(Obs {
            new_output: "chunk ",
            idle_transition: false,
            now_ms: t,
        });
    }
    // Streamed 0..10000 (span 10000) → budget ≈ 14000; still waiting at 12000.
    assert!(matches!(
        a.observe(Obs {
            new_output: "",
            idle_transition: false,
            now_ms: 12_000
        }),
        Step::Wait
    ));
}
