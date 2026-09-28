use super::super::{Exchange, Exchanges};
use crate::tools::seen::Seen;
use crate::tools::{ToolCall, Tools};

/// `n` lines of 50 chars, then a last line naming `tag`.
fn body(tag: &str, n: usize) -> String {
    let mut lines: Vec<String> = (0..n)
        .map(|i| format!("{tag} line {i:03}: {}", "x".repeat(40)))
        .collect();
    lines.push(format!("END OF {tag}"));
    lines.join("\n")
}

fn read(tag: &str) -> Exchange {
    Exchange::new(
        &format!("reading {tag}"),
        "fs:read",
        &format!("{{\"path\":\"{tag}\"}}"),
        body(tag, 40),
    )
}

#[test]
fn the_last_round_stays_whole_and_the_rest_are_their_calls() {
    let mut log = Exchanges::default();
    for tag in ["a", "b"] {
        log.push(read(tag));
    }
    log.push_round(vec![read("c"), read("d")]);
    assert!(log.tighten());
    let shown = log.render();
    assert_eq!(
        shown,
        format!(
            "CALLED fs:read {{\"path\":\"a\"}}\nCALLED fs:read {{\"path\":\"b\"}}\n{}\n\n{}\n\n{}",
            super::LEFT_OUT,
            read("c"),
            read("d")
        )
    );
    assert!(!shown.contains("reading a") && !shown.contains("a line 000"));
    // Cut once: a second cut has nothing left to give.
    assert!(!log.tighten());
    // And the next round pushes the one before it down to its call.
    log.push(read("e"));
    let next = log.render();
    assert!(next.contains("CALLED fs:read {\"path\":\"d\"}\n"), "{next}");
    assert!(!next.contains("END OF d") && next.contains("END OF e"));
}

/// One round is already only its last round: cutting changes nothing.
#[test]
fn a_log_of_one_round_cannot_be_cut() {
    let mut log = Exchanges::default();
    assert!(!log.tighten(), "empty");
    let mut log = Exchanges::default();
    log.push_round(vec![read("a"), read("b")]);
    assert!(!log.tighten());
}

struct Reads;

impl Tools for Reads {
    fn hint(&self) -> String {
        String::new()
    }
    fn call(&self, _: &str, _: &str, _: &str) -> Result<String, String> {
        Ok(String::new())
    }
    fn repeatable(&self, _: &str, _: &str) -> bool {
        true
    }
}

/// Once cut, a read of an older round is not shown anywhere, short or not,
/// so a repeat of it runs rather than pointing at nothing.
#[test]
fn once_cut_a_repeat_of_an_older_round_runs_again() {
    let call = |tag: &str| ToolCall {
        server: "fs".into(),
        tool: "read".into(),
        args: format!("{{\"path\":\"{tag}\"}}"),
    };
    let mut log = Exchanges::default();
    let mut seen = Seen::default();
    let short = Exchange::new("", "fs:read", "{\"path\":\"s\"}", "tiny".into());
    for (tag, e) in [("s", short), ("b", read("b"))] {
        seen.ran(&Reads, &call(tag), log.next_entry(), true);
        log.push(e);
    }
    // Uncut, the short one is shown whole and the last round is too.
    assert_eq!(log.repeat(&seen, &call("s")), Some(1));
    assert_eq!(log.repeat(&seen, &call("b")), Some(2));
    assert!(log.tighten());
    assert_eq!(log.repeat(&seen, &call("s")), None);
    assert_eq!(log.repeat(&seen, &call("b")), None);
}
