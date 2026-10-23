use crate::tools::split_tool_calls;

/// The tail of qwen-max's reply on the 2026-10-22 probe (a5w-run2), as it
/// came: four fenced edits, a line saying what comes next, and the test
/// command fenced as `sh`. Both of that run's relay turns ended on this shape
/// with nothing run; the edits are the calls they were written as.
const TURN: &str = "Let's make these remaining changes.

```sh
@tool sys:edit {\"path\": \"crates/crew-theme/src/readable_tests.rs\", \"old\": \"if r < floor {\", \"new\": \"if r < (floor).into() {\"}
@tool sys:edit {\"path\": \"crates/crew-theme/src/readable_tests.rs\", \"old\": \"x < floor {\", \"new\": \"x < (floor).into() {\"}
@tool sys:edit {\"path\": \"crates/crew-theme/src/signal_tests.rs\", \"old\": \"let bound: f32 = 2.0;\", \"new\": \"let bound: f64 = 2.0;\"}
@tool sys:edit {\"path\": \"crates/crew-theme/src/tagcolor_tests.rs\", \"old\": \"if r < raised - 0.02 {\", \"new\": \"if r < (raised - 0.02).into() {\"}
```

After making these changes, I will run the tests again to ensure that all the type mismatches are resolved and the tests pass.

```sh
cargo test -p crew-theme
```";

#[test]
fn a_shell_fence_under_the_calls_leaves_them_calls() {
    let (said, calls) = split_tool_calls(TURN).expect("the four edits are calls");
    assert_eq!(said, "Let's make these remaining changes.");
    assert_eq!(calls.len(), 4);
    assert!(calls.iter().all(|c| c.server == "sys" && c.tool == "edit"));
    assert!(
        calls[2].args.contains("let bound: f64"),
        "{}",
        calls[2].args
    );
}

#[test]
fn bash_and_a_tilde_fence_read_the_same() {
    let turn = TURN.replace("```sh\ncargo", "~~~bash\ncargo");
    let turn = turn.replace("crew-theme\n```", "crew-theme\n~~~");
    assert_eq!(split_tool_calls(&turn).map(|(_, c)| c.len()), Some(4));
}

/// The fence is wrapping, its lines are not: a block of commands is an
/// explanation, and the calls above it an example.
#[test]
fn a_shell_block_of_several_commands_still_makes_the_calls_an_example() {
    let turn = TURN.replace(
        "cargo test -p crew-theme",
        "cargo fmt --all\ncargo clippy\ncargo test -p crew-theme",
    );
    assert!(split_tool_calls(&turn).is_none());
}

#[test]
fn a_block_in_another_language_is_still_prose() {
    let turn = TURN.replace(
        "```sh\ncargo test -p crew-theme",
        "```rust\nlet bound: f64 = 2.0;",
    );
    assert!(split_tool_calls(&turn).is_none());
}
