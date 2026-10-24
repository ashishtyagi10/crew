use super::routed_last;

/// The notes the harness writes under an answer go above its routing line,
/// the only line routing reads: the `@done` comes off the answer, and an
/// `@next` still hands off.
#[test]
fn the_harness_notes_go_above_the_routing_line() {
    let budget = "[tool budget spent \u{2014} 4 rounds of tool calls]";
    let done = format!("both read\n@done\n\n{budget}");
    assert_eq!(routed_last(&done), format!("both read\n\n{budget}\n@done"));
    let routed = crate::broker::parse_routing(&routed_last(&done));
    assert!(
        matches!(&routed, crate::broker::Routing::Done(a) if !a.contains("@done") && a.ends_with(budget)),
        "{routed:?}"
    );
    let list = "checklist: 1 of 3 done \u{2014} not done: add a test";
    let next = format!("half done\n@next reviewer\n\n{list}");
    let routed = crate::broker::parse_routing(&routed_last(&next));
    assert!(
        matches!(&routed, crate::broker::Routing::Relay { to, .. } if to == "reviewer"),
        "{routed:?}"
    );
    assert_eq!(routed_last("plain answer"), "plain answer");
    assert_eq!(routed_last("ends\n@done"), "ends\n@done");
    assert_eq!(
        routed_last(&format!("no directive\n\n{budget}")),
        format!("no directive\n\n{budget}")
    );
}
