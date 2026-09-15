use super::*;

use crate::broker::recall;

fn capture() -> (
    std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    impl FnMut(PluginEvent) -> anyhow::Result<()>,
) {
    let said = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = said.clone();
    let emit = move |ev: PluginEvent| {
        if let PluginEvent::Message { text, .. } = &ev {
            sink.lock().unwrap().push(text.clone());
        }
        Ok(())
    };
    (said, emit)
}

#[test]
fn the_question_is_recognised_with_and_without_a_subject() {
    assert_eq!(asks("what do you remember?"), Some(String::new()));
    assert_eq!(
        asks("What do you remember about the router?"),
        Some("the router".into())
    );
    assert_eq!(
        asks("what do you know about linecap"),
        Some("linecap".into())
    );
    assert_eq!(
        asks("do you remember the nav crash"),
        Some("the nav crash".into())
    );
}

#[test]
fn a_question_about_something_else_is_an_ordinary_task() {
    for task in [
        "remember to run the tests",
        "tell me what you remember",
        "the router does not remember its last turn",
    ] {
        assert_eq!(
            asks(task),
            None,
            "{task:?} was taken as the memory question"
        );
    }
}

#[test]
fn the_answer_reads_the_graph_out_without_a_model_call() {
    let session = Session::default();
    recall::lock(&session.recall).record(
        "why does the linecap ratchet fail in crates/crew-app/src/nav.rs",
        "because a file on the debt list grew",
    );
    let (said, mut emit) = capture();
    gate("what do you remember about linecap?", &session, &mut emit)
        .expect("the question was not recognised")
        .unwrap();
    let text = said.lock().unwrap().join("\n");
    assert!(text.contains("what I remember about"), "{text}");
    assert!(text.contains("a file on the debt list grew"), "{text}");
    assert!(text.contains("crates/crew-app/src/nav.rs"), "{text}");
    assert!(
        text.contains("1 turn(s)"),
        "the graph's size is part of it: {text}"
    );
    assert_eq!(turns_about(&session, "linecap"), 1);
}

#[test]
fn a_subject_the_graph_has_never_seen_says_so_rather_than_guessing() {
    let session = Session::default();
    recall::lock(&session.recall).record("the linecap ratchet", "a debt file grew");
    let (said, mut emit) = capture();
    gate("what do you know about kubernetes", &session, &mut emit)
        .unwrap()
        .unwrap();
    let text = said.lock().unwrap().join("\n");
    assert!(
        text.contains("nothing in the recall graph reaches it"),
        "{text}"
    );
}

#[test]
fn asking_about_everything_reports_the_store_rather_than_a_subject() {
    let session = Session::default();
    let (said, mut emit) = capture();
    gate("what do you remember?", &session, &mut emit)
        .unwrap()
        .unwrap();
    let text = said.lock().unwrap().join("\n");
    assert!(
        text.starts_with("what I remember in this project:"),
        "{text}"
    );
    assert!(text.contains("graph:"), "{text}");
    assert!(
        text.contains("standing notes") || text.contains("no standing notes"),
        "{text}"
    );
}
