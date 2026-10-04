use std::sync::{Arc, Mutex};

use super::*;

#[test]
fn only_a_bang_with_a_command_is_a_command() {
    assert_eq!(command("!cargo test"), Some("cargo test"));
    assert_eq!(command("!  ls -la "), Some("ls -la"));
    assert_eq!(command("!"), None);
    assert_eq!(command("! "), None);
    assert_eq!(command("run !this"), None);
}

/// Output is fenced as code — with tildes when it carries backticks of its
/// own — and a command that could not run says why.
#[test]
fn the_card_fences_the_output_and_says_why_it_did_not_run() {
    let (shown, kept) = card("echo hi", Ok("hi\nexit 0".into()));
    assert_eq!(shown, "$ echo hi\n```\nhi\nexit 0\n```");
    assert_eq!(kept, "hi\nexit 0");
    let (shown, _) = card("cat x.md", Ok("```rust\nfn x() {}\n```".into()));
    assert!(shown.contains("~~~\n```rust"), "{shown}");
    let (shown, _) = card("x", Err("nope".into()));
    assert!(shown.contains("did not run: nope"), "{shown}");
}

/// The command runs, its card goes out from `shell`, and the exchange joins
/// the thread — so the next message can refer to it.
#[cfg(unix)]
#[test]
fn a_command_runs_and_joins_the_thread() {
    let thread = super::super::thread::SharedThread::default();
    let got: Arc<Mutex<Vec<PluginEvent>>> = Arc::default();
    let sink = Arc::clone(&got);
    spawn(
        "echo crew-bang-test".into(),
        Arc::clone(&thread),
        Arc::new(move |ev| sink.lock().unwrap().push(ev)),
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while got.lock().unwrap().is_empty() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let evs = got.lock().unwrap();
    let Some(PluginEvent::Message { sender, text, .. }) = evs.first() else {
        panic!("no card: {evs:?}");
    };
    assert_eq!(sender, SENDER);
    assert!(
        text.starts_with("$ echo crew-bang-test\n```") && text.contains("crew-bang-test\n"),
        "{text}"
    );
    let asked = super::super::thread::lock(&thread).asks();
    assert_eq!(asked, ["!echo crew-bang-test"]);
}
