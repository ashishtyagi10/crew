//! The classification's job is to have no gaps. These tests are mostly about the gaps.
use super::*;
use crate::broker::systools;

/// The guard that matters: every tool crew ships on the `sys` surface has an explicit tier. Add
/// a fifth sys tool without classifying it and this fails — which is the only way the gate can
/// promise it saw every action before it fired.
#[test]
fn every_built_in_tool_is_classified() {
    let unclassified: Vec<String> = systools::tools()
        .iter()
        .filter(|t| sys_tier(&t.name).is_none())
        .map(|t| format!("{}:{}", t.server, t.name))
        .collect();
    assert!(
        unclassified.is_empty(),
        "these tools have no tier, so the gate cannot know whether they can be undone: \
         {unclassified:?} — add them to `sys_tier`"
    );
    assert_eq!(
        systools::tools().len(),
        5,
        "the sys surface is five tools: four that act, and `find_tools` to reach the rest"
    );
}

/// The default is the whole safety argument: a server nobody has classified might send mail.
#[test]
fn an_unknown_tool_is_treated_as_irreversible() {
    assert_eq!(
        tier_of("some-new-mcp-server", "do_thing"),
        Tier::Irreversible
    );
    assert_eq!(
        tier_of("sys", "a_tool_that_does_not_exist"),
        Tier::Irreversible
    );
    assert!(tier_of("gmail", "send").needs_approval());
}

#[test]
fn reading_never_needs_approval_and_shell_always_does() {
    assert_eq!(tier_of("sys", "read_file"), Tier::Read);
    assert_eq!(tier_of("sys", "list_dir"), Tier::Read);
    assert!(!tier_of("sys", "read_file").needs_approval());
    assert_eq!(tier_of("sys", "run"), Tier::Irreversible);
    assert!(
        tier_of("sys", "run").needs_approval(),
        "a shell command is a blank cheque — it is the one built-in that must ask"
    );
}

/// Writing a file on your own disk is recoverable in a way that telling another human something
/// is not, so it sits below the approval line — but it is still not a read.
#[test]
fn writing_a_file_is_reversible_not_read() {
    assert_eq!(tier_of("sys", "write_file"), Tier::Reversible);
    assert!(!tier_of("sys", "write_file").needs_approval());
    assert!(Tier::Read < Tier::Reversible && Tier::Reversible < Tier::Irreversible);
}

/// Read-only mode and the tier table must not be two separate opinions about which tools mutate.
#[test]
fn read_only_mode_blocks_exactly_the_non_read_tools() {
    for t in systools::tools() {
        let is_read = sys_tier(&t.name) == Some(Tier::Read);
        assert_eq!(
            blocked_by_read_only(&t.name),
            !is_read,
            "{} should {} be blocked in read-only mode",
            t.name,
            if is_read { "not" } else { "" }
        );
    }
}

#[test]
fn labels_are_stable_for_the_ledger() {
    assert_eq!(Tier::Read.label(), "read");
    assert_eq!(Tier::Reversible.label(), "reversible");
    assert_eq!(Tier::Irreversible.label(), "irreversible");
}
