//! The other half of the hook contract: what the agent is shown after a call, what a file
//! that is broken or missing means, and the wiring that puts both around a real call.
use super::tests::{one, source, ARGS};
use super::*;

#[test]
fn a_post_hooks_output_lands_under_the_result() {
    let h = source(
        "post",
        &one("post_tool", "sys:edit", "echo formatted", None),
    )
    .load();
    let out = h.post("sys:edit", "{}", "edited a.rs".into());
    assert_eq!(out, "edited a.rs\n\nhook: formatted");
}

#[test]
fn a_post_hook_reads_the_result_too() {
    let h = source("postin", &one("post_tool", "*", "cat", None)).load();
    let out = h.post("sys:edit", "{}", "edited a.rs".into());
    let line = out.split_once("\n\nhook: ").unwrap().1;
    let line: serde_json::Value = serde_json::from_str(line).unwrap();
    assert_eq!(
        (line["event"].as_str(), line["result"].as_str()),
        (Some("post_tool"), Some("edited a.rs"))
    );
}

/// The tool already ran: a post hook that fails, or says nothing, leaves its result alone.
#[test]
fn a_post_hook_never_fails_the_call() {
    for (tag, cmd) in [
        ("pquiet", "exit 1"),
        ("pmiss", "crew-no-such-hook-anywhere"),
    ] {
        let h = source(tag, &one("post_tool", "*", cmd, Some("block"))).load();
        assert_eq!(h.post("sys:edit", "{}", "edited".into()), "edited");
    }
}

#[test]
fn a_chatty_post_hook_is_cut_to_the_cap() {
    let cmd = "head -c 5000 /dev/zero | tr '\\0' x";
    let h = source("pcap", &one("post_tool", "*", cmd, None)).load();
    let out = h.post("sys:edit", "{}", "edited".into());
    assert!(out.contains(&format!("{}\u{2026} (cut at {CAP} chars)", "x".repeat(CAP))));
    assert!(
        out.chars().count() < CAP + 60,
        "{} chars",
        out.chars().count()
    );
}

/// A guard file that does not parse is a guard that is not running: every call says so,
/// misspelt keys included, rather than going ahead as if nothing were configured.
#[test]
fn a_broken_file_blocks_every_call_and_says_how_out() {
    for (tag, json) in [("bad", "{not json"), ("typo", r#"{"pre_tools": []}"#)] {
        let h = source(tag, json).load();
        let err = h.pre("sys:list_dir", "{}").unwrap_err();
        assert!(
            err.contains("hooks.json") && err.contains("CREW_HOOKS=0"),
            "{err}"
        );
        assert_eq!(h.post("sys:list_dir", "{}", "a\nb".into()), "a\nb");
    }
}

#[test]
fn no_file_or_an_empty_one_means_no_hooks() {
    let mut s = source("none", "  \n");
    assert_eq!(s.load().pre("sys:run", ARGS), Ok(()));
    s.path = s.path.with_file_name("absent.json");
    assert_eq!(s.load().post("sys:run", ARGS, "ran".into()), "ran");
}

/// The wiring: a real session surface consults the hooks on a real call, before it runs and
/// after. Without the two lines in `SessionTools::call`, the listing simply succeeds.
#[test]
fn the_session_surface_runs_the_hooks_around_a_real_call() {
    use crate::broker::toolcall::ToolRunner;
    let mcp = std::sync::Arc::new(std::sync::Mutex::new(crate::mcp::McpHost::default()));
    let surface = |tag, json: &str| {
        super::super::SessionTools::for_test(std::sync::Arc::clone(&mcp), true)
            .with_hooks(source(tag, json))
    };
    let blocking = surface(
        "wpre",
        &one("pre_tool", "sys:list_dir", "echo no >&2; exit 2", None),
    );
    let err = blocking.call("sys", "list_dir", "{}").unwrap_err();
    assert_eq!(err, "blocked by hook: no");
    let after = surface("wpost", &one("post_tool", "sys:*", "echo seen", None));
    let out = after.call("sys", "list_dir", "{}").unwrap();
    assert!(out.ends_with("\n\nhook: seen"), "{out}");
}
