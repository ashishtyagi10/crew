use super::*;

fn notes() -> String {
    let dir = std::path::PathBuf::from(std::env::var("CREW_PROJECT_DIR").unwrap());
    std::fs::read_to_string(dir.join(".crew").join("memory.md")).unwrap_or_default()
}

#[test]
fn the_openings_catch_a_lasting_preference_and_not_a_task() {
    for t in [
        "you should be talking like Agent Smith ..",
        "From now on, answer in one paragraph",
        "always run the tests with --workspace",
        "never push without asking",
    ] {
        assert!(backstop(t).is_some(), "{t}");
    }
    for t in [
        "talk me through the router",
        "never mind, use the other file",
        "always wondered: why is the floor 0.1125?",
        "should the tests always run with --workspace",
        "fix the test that always fails",
    ] {
        assert_eq!(backstop(t), None, "{t}");
    }
}

#[test]
fn the_routers_note_is_written_once_and_said_once() {
    let _g = crate::broker::testenv::mock("unused");
    let said = note(
        Some(Some("talk like Agent Smith")),
        "you should talk like him",
    )
    .unwrap();
    assert!(said.contains("remembered for every task"), "{said}");
    assert!(said.contains("talk like Agent Smith"), "{said}");
    assert_eq!(notes(), "- talk like Agent Smith\n");
    assert_eq!(note(Some(Some("Talk like agent smith")), "again"), None);
    assert_eq!(notes(), "- talk like Agent Smith\n");
    let carried = crate::broker::memory::with_memory("hello", "hello");
    assert!(carried.contains("talk like Agent Smith"), "{carried}");
}

#[test]
fn a_router_that_read_the_message_and_said_nothing_is_trusted() {
    let _g = crate::broker::testenv::mock("unused");
    assert_eq!(note(Some(None), "always run the tests"), None);
    assert_eq!(notes(), "");
}

#[test]
fn with_no_router_the_openings_decide_and_keep_the_users_words() {
    let _g = crate::broker::testenv::mock("unused");
    assert_eq!(note(None, "add a docstring"), None);
    let said = note(None, "from now on answer in one paragraph").unwrap();
    assert!(
        said.contains("from now on answer in one paragraph"),
        "{said}"
    );
    assert_eq!(notes(), "- from now on answer in one paragraph\n");
}
