use super::asks_pr;

#[test]
fn plain_requests_to_open_one_are_read() {
    for t in [
        "open a PR",
        "Open a pull request.",
        "make a pull request",
        "create a PR for this",
        "send this as a PR",
        "can you open a PR please?",
    ] {
        assert!(asks_pr(t), "{t}");
    }
}

#[test]
fn reviews_fixes_and_long_tasks_are_not_requests() {
    for t in [
        "review my PR",
        "review the pull request",
        "fix the PR comments",
        "merge the PR",
        "open PR #12",
        "why did the open a PR step fail",
        "add a retry to the uploader and then open a PR once the tests pass and the docs are updated",
        "commit this",
        "apply",
    ] {
        assert!(!asks_pr(t), "{t}");
    }
}
