use super::asks_review;

#[test]
fn a_review_of_the_working_changes_is_asked_for() {
    for task in [
        "review my diff",
        "Review my changes.",
        "can you look over my changes?",
        "go over the diff",
        "review the staged change",
        "critique my uncommitted edits",
        // The quick look's words (`reviewlevel`).
        "give my changes a quick look",
        "skim my diff",
        "glance over the staged changes",
    ] {
        assert!(asks_review(task), "{task}");
    }
}

#[test]
fn anything_else_is_a_task_for_an_agent() {
    for task in [
        // Asks to act on what a review finds.
        "review my diff and fix what you find",
        "fix the review comments",
        "address the review on my changes",
        // A review of something the review path does not read.
        "review PR 12's changes",
        "review the diff on this branch",
        // No working changes named, or the word is a file name.
        "review the README",
        "what does the review command do?",
        "why does review.rs sort the diff best-first?",
        "show me the diff",
        // A light word with everyday "change", "patch" or "staged".
        "skim this article on climate change",
        "glance at the changes in the new iOS release",
        "take a quick look at the patch notes for 1.90",
        "skim the staged rollout plan",
    ] {
        assert!(!asks_review(task), "{task}");
    }
}
