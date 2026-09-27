//! A key is listed under the place it works.
//!
//! Ctrl+Shift+M and Ctrl+O only act on an agent pane (`keys.rs`), but sat
//! under "everywhere" with a `Chat:` prefix doing the section's job — and
//! "Chat" is not what that pane is called.

use crate::helptable::{BINDINGS, CHAT_BINDINGS};

#[test]
fn agent_pane_keys_live_in_the_agent_pane_section() {
    for key in ["Ctrl+Shift+M", "Ctrl+O"] {
        assert!(CHAT_BINDINGS.iter().any(|(k, _)| *k == key), "{key}");
        assert!(!BINDINGS.iter().any(|(k, _)| *k == key), "{key}");
    }
}

#[test]
fn no_row_names_its_section_in_the_description() {
    for (k, d) in BINDINGS.iter().chain(CHAT_BINDINGS) {
        assert!(!d.starts_with("Chat:"), "{k}: {d}");
    }
}

#[test]
fn cmd_m_says_it_is_the_window() {
    let (_, d) = BINDINGS.iter().find(|(k, _)| k.contains("Cmd+M")).unwrap();
    assert!(d.contains("the window"), "{d}");
}
