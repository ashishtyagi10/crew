use super::*;

/// One resolution with every signal spelled inline: `keys` are env vars that
/// resolve, `signed` are CLI bins reporting signed-in, `clis` are bins merely
/// installed. A signed-in CLI is always also installed, as on a real machine.
fn resolved(
    mock: bool,
    pin: Option<&str>,
    keys: &[&str],
    signed: &[&str],
    clis: &[&str],
) -> Resolved {
    resolve(&Signals {
        mock,
        pin: pin.map(str::to_string),
        has_key: &|v| keys.contains(&v),
        signed_in: &|c| signed.contains(&c.bin),
        cli_installed: &|b| clis.contains(&b) || signed.contains(&b),
    })
}

const CLAUDE_SUB: Resolved = Resolved::Delegated {
    name: "claude-code",
    agent: "claude",
};

/// The whole discovery order as one table: pin > subscription > key > CLI >
/// nothing, with the mock short-circuiting everything (the test harness must
/// stay deterministic on any machine).
#[test]
fn the_discovery_order_is_pin_subscription_key_cli() {
    let table: &[(&str, Resolved)] = &[
        // (case, expected) — the case is built in the matching call below.
        ("mock beats everything", Resolved::Mock),
        (
            "pin beats a signed-in subscription",
            Resolved::Keyed("dashscope"),
        ),
        ("a delegated pin routes delegated", CLAUDE_SUB),
        ("subscription beats key", CLAUDE_SUB),
        ("key beats an installed cli", Resolved::Keyed("openrouter")),
        ("installed cli alone relays", Resolved::Relay),
        ("nothing at all is nothing", Resolved::None),
    ];
    let got: Vec<(&str, Resolved)> = vec![
        (
            "mock beats everything",
            resolved(
                true,
                Some("dashscope"),
                &["DASHSCOPE_API_KEY"],
                &["claude"],
                &[],
            ),
        ),
        (
            "pin beats a signed-in subscription",
            resolved(
                false,
                Some("dashscope"),
                &["DASHSCOPE_API_KEY"],
                &["claude"],
                &[],
            ),
        ),
        (
            "a delegated pin routes delegated",
            resolved(false, Some("claude-code"), &["DASHSCOPE_API_KEY"], &[], &[]),
        ),
        (
            "subscription beats key",
            resolved(false, None, &["DASHSCOPE_API_KEY"], &["claude"], &[]),
        ),
        (
            "key beats an installed cli",
            resolved(false, None, &["OPENROUTER_API_KEY"], &[], &["claude"]),
        ),
        (
            "installed cli alone relays",
            resolved(false, None, &[], &[], &["codex"]),
        ),
        (
            "nothing at all is nothing",
            resolved(false, None, &[], &[], &[]),
        ),
    ];
    for ((case, want), (case2, have)) in table.iter().zip(got.iter()) {
        assert_eq!(case, case2, "table rows out of order");
        assert_eq!(want, have, "{case}");
    }
}

/// The pin accepts the CLI's own spelling too: `CREW_PROVIDER=claude` and
/// `CREW_PROVIDER=claude-code` are the same explicit choice.
#[test]
fn a_delegated_pin_matches_name_or_binary() {
    for pin in ["claude-code", "claude", "Claude"] {
        assert_eq!(
            resolved(false, Some(pin), &[], &[], &[]),
            CLAUDE_SUB,
            "{pin}"
        );
    }
    assert_eq!(
        resolved(false, Some("codex"), &[], &[], &[]),
        Resolved::Delegated {
            name: "codex",
            agent: "codex"
        }
    );
}

/// An unknown pin falls through to auto-discovery — exactly what
/// `pick_provider(Some("bogus"), …)` has always done.
#[test]
fn an_unknown_pin_falls_through_to_keys() {
    assert_eq!(
        resolved(false, Some("bogus"), &["ANTHROPIC_API_KEY"], &[], &[]),
        Resolved::Keyed("anthropic")
    );
}

/// The key rung keeps the historic order: dashscope > openrouter >
/// anthropic > the DIRECT rows — adding a provider can never change what an
/// existing install resolves to.
#[test]
fn keys_keep_the_historic_discovery_order() {
    let all = [
        "DASHSCOPE_API_KEY",
        "OPENROUTER_API_KEY",
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
    ];
    assert_eq!(
        resolved(false, None, &all, &[], &[]),
        Resolved::Keyed("dashscope")
    );
    assert_eq!(
        resolved(false, None, &all[1..], &[], &[]),
        Resolved::Keyed("openrouter")
    );
    assert_eq!(
        resolved(false, None, &all[2..], &[], &[]),
        Resolved::Keyed("anthropic")
    );
    assert_eq!(
        resolved(false, None, &all[3..], &[], &[]),
        Resolved::Keyed("openai")
    );
}

/// Between the two delegated providers, claude leads — matching the keyless
/// roster's own lead — and a signed-in codex still wins over any key.
#[test]
fn delegated_probe_order_is_claude_then_codex() {
    assert_eq!(
        resolved(false, None, &[], &["claude", "codex"], &[]),
        CLAUDE_SUB
    );
    assert_eq!(
        resolved(false, None, &["OPENROUTER_API_KEY"], &["codex"], &[]),
        Resolved::Delegated {
            name: "codex",
            agent: "codex"
        }
    );
}
