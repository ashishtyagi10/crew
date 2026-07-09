use super::*;
use crate::broker::testenv;

#[test]
fn mock_provider_answers_the_ask() {
    let _env = testenv::mock("ls -la");
    let got = suggest_command("list files", Duration::from_secs(5)).unwrap();
    assert_eq!(got, "ls -la");
}

#[test]
fn ask_prompt_names_the_query_and_demands_one_bare_command() {
    let p = ask_prompt("kill whatever is on port 8080");
    assert!(p.contains("kill whatever is on port 8080"));
    let lower = p.to_lowercase();
    assert!(lower.contains("one") && lower.contains("command"));
    assert!(lower.contains(std::env::consts::OS));
}

#[test]
fn extract_command_takes_a_bare_reply_verbatim() {
    assert_eq!(extract_command("ls -la"), "ls -la");
    assert_eq!(extract_command("  du -sh *  \n"), "du -sh *");
}

#[test]
fn extract_command_prefers_the_fenced_block_over_prose() {
    assert_eq!(extract_command("```sh\nls -la\n```"), "ls -la");
    assert_eq!(
        extract_command("Use this:\n\n```bash\ngrep -r foo .\n```\nIt searches recursively."),
        "grep -r foo ."
    );
}

#[test]
fn extract_command_strips_inline_backticks_and_prompt_prefix() {
    assert_eq!(extract_command("`git status`"), "git status");
    assert_eq!(extract_command("$ du -sh *"), "du -sh *");
}

#[test]
fn extract_command_survives_an_empty_reply() {
    assert_eq!(extract_command(""), "");
    assert_eq!(extract_command("   \n  "), "");
}
