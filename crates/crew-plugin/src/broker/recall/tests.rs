use super::*;

fn temp(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "crew-recall-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::remove_dir_all(&dir).ok();
    dir
}

#[test]
fn what_one_session_learned_is_recalled_by_the_next_one() {
    let dir = temp("across");
    {
        let mut first = Recall::open_at(&dir);
        first.record(
            "the linecap ratchet keeps failing on crates/crew-app/src/nav.rs",
            "it fails when a debt file grows",
        );
    } // the session ends; only the log survives
    let second = Recall::open_at(&dir);
    let block = second
        .context("why is the linecap ratchet red", &[])
        .expect("nothing was recalled across the session boundary");
    assert!(block.contains("it fails when a debt file grows"), "{block}");
    assert!(block.contains("crates/crew-app/src/nav.rs"), "{block}");
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn an_unrelated_request_recalls_nothing_even_with_a_full_graph() {
    let dir = temp("unrelated");
    let mut r = Recall::open_at(&dir);
    r.record("the linecap ratchet is red", "a debt file grew");
    assert!(r.context("what is the weather in berlin", &[]).is_none());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn crew_recall_zero_is_the_only_value_that_turns_the_graph_off() {
    assert!(enabled(None), "absent must mean on");
    assert!(!enabled(Some("0")));
    assert!(enabled(Some("1")) && enabled(Some("")));
}

#[test]
fn a_graph_that_is_off_remembers_nothing_and_recalls_nothing() {
    let mut r = Recall::disabled();
    r.record("the linecap ratchet is red", "a debt file grew");
    assert_eq!(r.nodes(), 0);
    assert!(r.context("the linecap ratchet", &[]).is_none());
}

#[test]
fn doctor_says_what_is_remembered_and_marks_an_empty_graph_absent() {
    assert_eq!(doctor_line(0, 0, 0, 0).0, '\u{2013}');
    let (mark, detail) = doctor_line(3, 9, 2, 0);
    assert_eq!(mark, '\u{2713}');
    assert!(
        detail.contains("3 turn(s)") && detail.contains("9 topic(s)"),
        "{detail}"
    );
    // A project that has never sent crew to the web says nothing about pages
    // rather than reading as one missing something.
    assert!(!detail.contains("page"), "{detail}");
    let (_, detail) = doctor_line(3, 9, 2, 5);
    assert!(detail.contains("5 page(s)"), "{detail}");
}

/// The wiring: what the arms (`swarm`, `stdio`, `fanout`) actually call.
mod ahead_of_a_task {
    use super::*;
    use crate::broker::session::Session;
    use crate::broker::thread;

    fn session_that_remembers() -> Session {
        let s = Session::default();
        lock(&s.recall).record(
            "why does the linecap ratchet keep failing",
            "because a file on the debt list grew",
        );
        s
    }

    #[test]
    fn a_task_carries_the_older_memory_first_and_itself_last() {
        let s = session_that_remembers();
        thread::lock(&s.thread).record("what is on the debt list", "nav.rs and session.rs");
        let out = ahead(&s, "the linecap ratchet is red again");
        let (graph, session) = (
            out.find("From earlier work").expect("no recalled block"),
            out.find("Earlier in this conversation")
                .expect("no thread block"),
        );
        assert!(graph < session, "the graph must come before this session");
        assert!(out.ends_with("the linecap ratchet is red again"), "{out}");
    }

    #[test]
    fn a_task_the_graph_knows_nothing_about_passes_through_byte_identical() {
        let s = session_that_remembers();
        assert_eq!(
            ahead(&s, "what is the weather in berlin"),
            "what is the weather in berlin"
        );
    }

    #[test]
    fn a_turn_still_in_the_thread_is_not_also_quoted_from_the_graph() {
        let s = Session::default();
        lock(&s.recall).record("the linecap ratchet is red", "a debt file grew");
        thread::lock(&s.thread).record("the linecap ratchet is red", "a debt file grew");
        let out = ahead(&s, "the linecap ratchet again");
        assert_eq!(
            out.matches("a debt file grew").count(),
            1,
            "the same turn was quoted from both memories: {out}"
        );
    }
}

#[test]
fn a_check_that_broke_this_way_before_is_recognised_in_the_next_session() {
    let dir = temp("repeat");
    {
        let mut first = Recall::open_at(&dir);
        let d = failure_digest("exit 1\nerror[E0308]: mismatched types\n --> src/a.rs:12:5\n");
        first.record("check: cargo test", &format!("failed: {d}"));
    } // the session ends; only the log survives
    let second = Recall::open_at(&dir);
    let p = second
        .seen_failing(
            "cargo test",
            "exit 1\nerror[E0308]: mismatched types\n --> src/a.rs:481:9\n",
        )
        .expect("the same failure was not recognised across the session boundary");
    assert_eq!(p.times, 1, "the earlier failure was not counted");
    assert!(
        second
            .seen_failing("cargo test", "exit 1\nerror[E0425]: cannot find `x`\n")
            .is_none(),
        "a different error was read as the same one"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_graph_that_is_off_never_claims_to_have_seen_a_failure_before() {
    let mut r = Recall::disabled();
    r.record("check: cargo test", "failed: error: boom");
    assert!(r
        .seen_failing("cargo test", "exit 1\nerror: boom\n")
        .is_none());
}
