use super::*;

fn info(name: &str, role: &str) -> crate::AgentInfo {
    crate::AgentInfo {
        name: name.into(),
        role: role.into(),
        model: String::new(),
    }
}

/// `/review`'s author election (`pick_by_role(&reg.infos(), is_critic)`) must
/// pick the agent whose OWN role advertises review capability, not the
/// literal name "reviewer" (no invented specialist is ever named that — see
/// `d49a6e1`) and not just the first-registered agent (LRU order, arbitrary).
/// `quality-auditor` is deliberately NOT first in the roster and carries no
/// name hint at all — only its role says "review, critique" — so a fixture
/// where the fallback (`travel-advisor`, first-in-roster) coincided with the
/// correct answer would prove nothing.
#[test]
fn review_author_is_elected_by_role_not_by_roster_order() {
    let agents = vec![
        info("travel-advisor", ""),
        info("quality-auditor", "review, critique"),
    ];
    assert_eq!(pick_by_role(&agents, is_critic), "quality-auditor");
}

#[test]
fn review_prompt_carries_the_diff_and_the_reviewer_contract() {
    let p = review_prompt("+ let x = unwrap();");
    assert!(p.contains("+ let x = unwrap();"), "diff included");
    let lower = p.to_lowercase();
    for word in ["blocker", "warn", "nit"] {
        assert!(
            lower.contains(word),
            "severity vocabulary names {word}: {p}"
        );
    }
    assert!(lower.contains("file"), "asks findings to name the file");
    assert!(lower.contains("verdict"), "asks for a closing verdict");
    assert!(
        lower.contains("clean") || lower.contains("no findings"),
        "tells the reviewer what to say for a clean diff"
    );
}

#[test]
fn review_prompt_orders_findings_by_severity() {
    let lower = review_prompt("x").to_lowercase();
    let (b, w, n) = (
        lower.find("blocker").unwrap(),
        lower.find("warn").unwrap(),
        lower.find("nit").unwrap(),
    );
    assert!(b < w && w < n, "severities are introduced worst-first");
}
