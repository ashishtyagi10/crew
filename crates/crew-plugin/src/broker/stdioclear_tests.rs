//! `/clear` is a fresh conversation: a correction proposed in the old one
//! is not left waiting for a "keep" the new one never asked for.
use super::*;

#[test]
fn a_fresh_start_drops_a_waiting_correction() {
    let session = Session::new();
    assert!(
        super::super::ruledpropose::propose(&session.recall, "don't touch Cargo.lock").is_some()
    );
    fresh_start(&session);
    let r = super::super::recall::lock(&session.recall);
    assert_eq!(r.proposed(), None);
    assert!(r.ruled().is_empty(), "dropped, not kept");
}
