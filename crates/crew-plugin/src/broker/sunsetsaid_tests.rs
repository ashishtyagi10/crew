use super::*;
use crate::broker::adapter::Usage;
use crew_hive::catalog::sunset::days_from_civil;

fn nov(d: u32) -> i64 {
    days_from_civil(2026, 11, d)
}

#[test]
fn only_a_model_near_its_date_has_a_notice() {
    assert_eq!(retiring("gpt-5", "", nov(1)), "", "40 days out is quiet");
    assert_eq!(
        retiring("gpt-5", "", nov(27)),
        "gpt-5 retires Dec 11 \u{2014} gpt-6.1-sol next"
    );
    assert_eq!(retiring("gpt-5.5", "gpt-5.5", nov(30)), "");
    assert_eq!(retiring("", "", nov(30)), "");
}

#[test]
fn the_pin_is_named_before_what_answered() {
    // The pinned chain's first model is retiring; another answered.
    let n = retiring(
        "claude-sonnet-4-5-20250929, claude-sonnet-5-5",
        "x",
        nov(24),
    );
    assert!(
        n.starts_with("claude-sonnet-4-5-20250929 retires Nov 30"),
        "{n}"
    );
    // A provider default (no pin) that turns out to be a dated snapshot.
    let n = retiring("", "gpt-5-2025-08-07", nov(27));
    assert!(n.starts_with("gpt-5-2025-08-07 retires Dec 11"), "{n}");
}

#[test]
fn a_notice_is_said_once() {
    let mut s = Said::default();
    assert!(!s.first(""));
    assert!(s.first("gpt-5 retires Dec 11 \u{2014} gpt-6.1-sol next"));
    assert!(!s.first("gpt-5 retires Dec 11 \u{2014} gpt-6.1-sol next"));
    assert!(s.first("o3 retires Dec 11 \u{2014} gpt-6.1-sol next"));
}

/// Through smith's row, across two turns (two `Told`s): the first reply
/// carries the line, the next turn's does not. The line names a model no
/// other test uses, since the session set is process-wide.
#[test]
fn smith_says_it_under_the_first_reply_of_the_session_only() {
    let line = "o3-pro-2025-06-10 retires Dec 11 \u{2014} gpt-6.1-sol next";
    let u = Usage {
        retiring: line.into(),
        ..Default::default()
    };
    let said = |ev: Option<crate::PluginEvent>| format!("{ev:?}");
    let first = said(crate::broker::fellback::Told::default().line("a", &u));
    assert!(first.contains(line), "{first}");
    let again = crate::broker::fellback::Told::default().line("b", &u);
    assert!(again.is_none(), "{again:?}");
}

#[test]
fn a_later_dial_keeps_the_notice() {
    let mut hop = Usage {
        retiring: "gpt-5 retires Dec 11 \u{2014} gpt-6.1-sol next".into(),
        ..Default::default()
    };
    hop.latest(Usage::default());
    assert!(hop.retiring.starts_with("gpt-5 retires"));
}

/// A real dial: an API agent pinned to a retiring snapshot carries the
/// notice on the usage its reply comes back with — the path a relay hop's
/// `Told::line` reads. (Prefix only, so it holds past Nov 30 too.)
#[test]
fn a_pinned_agents_reply_carries_the_notice() {
    use crate::broker::adapter::Adapter;
    let mock = std::sync::Arc::new(crew_hive::MockProvider {
        reply: "hi".to_string(),
    });
    let pin = "claude-sonnet-4-5-20250929";
    let a = crate::broker::apiadapter::ApiAdapter::new("planner", pin, "", None, mock).unwrap();
    let (_, u) = a
        .call_with_usage("task", std::time::Duration::from_secs(5))
        .unwrap();
    assert!(u.retiring.starts_with(&format!("{pin} retire")), "{u:?}");
    assert!(u.retiring.ends_with("claude-sonnet-5-5 next"), "{u:?}");
}
