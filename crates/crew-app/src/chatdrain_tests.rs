//! The drain's contract for the broker's picker events: a `SignIn` or
//! `SignOut` is the broker's WHOLE answer to a bare `/login` or `/logout`,
//! so it must settle the pane the way a `Message` does. When it did not, the
//! popup opened over a pane still "awaiting" a reply — and the pick itself
//! was queued behind a reply that was never going to come. Nothing happened.
use crate::chat::ChatPane;
use crate::chatkeys::ChatInput;
use crate::chatpalette::Kind;
use crate::loginpick::Auth;
use crew_plugin::Plugin;

/// A pane whose broker prints `line` once, then idles — right after the
/// bare construct was sent (so `awaiting` is latched, as any send does).
fn pane_emitting(line: &str) -> ChatPane {
    let script = format!("printf '%s\\n' '{line}'; cat >/dev/null\n");
    let plugin = Plugin::spawn("sh", &["-c".to_string(), script]).unwrap();
    let mut p = ChatPane::new(plugin, "crew".into());
    p.connected = true;
    p.awaiting = true;
    p
}

/// Drain until the picker is open (the child's line lands on a background
/// thread, so it is not there on the first poll); bounded so a broken
/// emitter fails instead of hanging.
fn poll_until_picker(p: &mut ChatPane) {
    let start = std::time::Instant::now();
    while p.palette.is_none() {
        p.poll();
        assert!(
            start.elapsed() < std::time::Duration::from_secs(5),
            "no picker opened"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// Enter on the picker's first row, then what the pane did with the pick.
fn pick_first_row(p: &mut ChatPane) {
    p.on_input(ChatInput::Enter, &std::env::temp_dir());
    assert!(p.palette.is_none(), "the pick closes the picker");
}

#[test]
fn the_sign_in_picker_settles_the_pane_so_the_pick_sends_at_once() {
    let mut p = pane_emitting(r#"{"type":"sign_in","options":[{"name":"qwen","device":true}]}"#);
    poll_until_picker(&mut p);
    assert_eq!(
        p.palette.as_ref().map(|s| s.kind),
        Some(Kind::Auth(Auth::In))
    );
    assert!(!p.awaiting, "the picker IS the reply to a bare /login");
    assert!(!p.is_busy(), "an open picker is an idle pane");

    pick_first_row(&mut p); // qwen — a device flow crew runs itself
    assert!(
        p.queued.is_empty(),
        "the pick went straight to the broker, not the queue: {:?}",
        p.queued
    );
    assert_eq!(
        p.messages.last().map(|m| m.text.as_str()),
        Some("/login qwen"),
        "the pick is echoed as the user's own line"
    );
    assert!(
        p.awaiting,
        "the pick is a send of its own, awaiting the flow's first line"
    );
}

#[test]
fn the_sign_out_picker_settles_the_pane_too() {
    let mut p = pane_emitting(
        r#"{"type":"sign_out","options":[{"name":"qwen","device":true,"signed_in":true}]}"#,
    );
    poll_until_picker(&mut p);
    assert_eq!(
        p.palette.as_ref().map(|s| s.kind),
        Some(Kind::Auth(Auth::Out))
    );
    assert!(!p.awaiting, "the picker IS the reply to a bare /logout");

    pick_first_row(&mut p);
    assert!(
        p.queued.is_empty(),
        "queued instead of sent: {:?}",
        p.queued
    );
    assert_eq!(
        p.messages.last().map(|m| m.text.as_str()),
        Some("/logout qwen")
    );
}
