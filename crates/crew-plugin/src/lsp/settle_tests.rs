#![cfg(unix)]
//! The bounded ask an edit's result makes, against the scripted peer in
//! `peer_tests.rs`: it never starts a server, it lets go by its cap, and it
//! answers only with what the server said that it had not said before.
use std::time::{Duration, Instant};

use crate::lsp::peer::{rust_analyzer, Peer};
use crew_lsp::Severity;

const TYPE_ERROR: &str = "fn b() -> i32 { \"s\" }\n";

/// A server not running yet is not started: a start holds the lock through
/// its spawn and handshake, far past an edit's bound.
#[test]
fn no_server_running_is_none_and_none_is_started() {
    let mut p = Peer::scripted("settle-none", rust_analyzer(), &[(0.2, Some("boom"))]);
    assert_eq!(
        p.host.new_diagnostics(&p.file, Duration::from_secs(1)),
        None
    );
    assert!(p.host.clients.is_empty(), "started a server");
}

/// A server that has not spoken about the new text by the cap is let go of
/// at the cap, not at SETTLE.
#[test]
fn a_server_silent_past_the_cap_is_let_go_of_at_the_cap() {
    let mut p = Peer::scripted("settle-cap", rust_analyzer(), &[(2.0, Some("late"))]);
    p.host.call("hover", &p.args()).unwrap();
    p.write(TYPE_ERROR);
    let started = Instant::now();
    let got = p.host.new_diagnostics(&p.file, Duration::from_millis(300));
    let took = started.elapsed();
    assert_eq!(got, Some(vec![]));
    assert!(
        took < Duration::from_secs(1),
        "waited past the cap: {took:?}"
    );
}

/// The first word is not the answer: rust-analyzer's first publish on a
/// change is its syntax pass, empty, a few ms before the type error.
#[test]
fn an_empty_first_word_is_waited_past_for_the_error() {
    let steps = [(0.4, None), (0.1, Some("mismatched types"))];
    let mut p = Peer::scripted("settle-second", rust_analyzer(), &steps);
    p.host.call("hover", &p.args()).unwrap();
    p.write(TYPE_ERROR);
    let got = p
        .host
        .new_diagnostics(&p.file, Duration::from_secs(2))
        .expect("a running server");
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(
        (got[0].severity, got[0].message.as_str()),
        (Severity::Error, "mismatched types")
    );
}

/// An error the server said about the old text and says again after the
/// edit is not news: rust-analyzer repeats `cargo check`'s word on the old
/// text until the check the save started finishes, and a fix read that way
/// is reported as the error it fixed.
#[test]
fn an_error_said_before_the_edit_and_again_after_it_is_not_news() {
    let steps = [(0.3, Some("old")), (0.5, Some("old"))];
    let mut p = Peer::scripted("settle-stale", rust_analyzer(), &steps);
    p.host.call("hover", &p.args()).unwrap();
    // The old text's word lands while nobody is asking.
    std::thread::sleep(Duration::from_millis(500));
    p.write("fn b() {}\n");
    let got = p.host.new_diagnostics(&p.file, Duration::from_secs(1));
    assert_eq!(got, Some(vec![]));
}
