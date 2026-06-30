use super::*;

#[test]
fn escape_key_requests_pane_close() {
    assert_eq!(
        chat_key(&Key::Named(NamedKey::Escape), true),
        ChatInput::Close
    );
}

#[test]
fn a_released_key_is_ignored() {
    // Only key presses act; releases (including Escape) do nothing.
    assert_eq!(
        chat_key(&Key::Named(NamedKey::Escape), false),
        ChatInput::Ignore
    );
}

#[test]
fn typed_characters_and_edits_are_classified() {
    assert_eq!(
        chat_key(&Key::Character("a".into()), true),
        ChatInput::Char('a')
    );
    assert_eq!(
        chat_key(&Key::Named(NamedKey::Space), true),
        ChatInput::Char(' ')
    );
    assert_eq!(
        chat_key(&Key::Named(NamedKey::Enter), true),
        ChatInput::Enter
    );
    assert_eq!(
        chat_key(&Key::Named(NamedKey::Backspace), true),
        ChatInput::Backspace
    );
}

#[test]
fn classify_spawn_pane_returns_host_action() {
    let ev = PluginEvent::SpawnPane {
        command: "sh".into(),
        args: vec![],
        label: "x".into(),
    };
    let result = classify(&ev);
    assert_eq!(
        result,
        Some(HostAction::SpawnPane {
            command: "sh".into(),
            args: vec![],
            label: "x".into(),
        })
    );
}

#[test]
fn classify_message_returns_none() {
    let ev = PluginEvent::Message {
        channel: "general".into(),
        sender: "bob".into(),
        text: "hello".into(),
        ts: "t".into(),
    };
    assert_eq!(classify(&ev), None);
}

#[test]
fn classify_send_pane_returns_host_action() {
    let ev = PluginEvent::SendPane {
        label: "a".into(),
        text: "hi".into(),
    };
    assert_eq!(
        classify(&ev),
        Some(HostAction::SendPane {
            label: "a".into(),
            text: "hi".into(),
        })
    );
}
