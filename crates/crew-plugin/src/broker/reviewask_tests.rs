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
    ] {
        assert!(!asks_review(task), "{task}");
    }
}
