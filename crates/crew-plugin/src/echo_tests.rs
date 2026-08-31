use super::*;

#[test]
fn hello_yields_ready() {
    let evs = respond(&PluginCommand::Hello { v: 1 });
    assert_eq!(evs.len(), 1);
    match &evs[0] {
        PluginEvent::Ready {
            v,
            provider,
            channels,
        } => {
            assert_eq!(*v, 1);
            assert_eq!(provider, "echo");
            assert_eq!(channels, &["general"]);
        }
        _ => panic!("expected Ready"),
    }
}

#[test]
fn send_yields_message() {
    let evs = respond(&PluginCommand::Send {
        channel: "general".into(),
        text: "hi".into(),
    });
    assert_eq!(evs.len(), 1);
    match &evs[0] {
        PluginEvent::Message {
            channel,
            sender,
            text,
            ts,
            ..
        } => {
            assert_eq!(channel, "general");
            assert_eq!(sender, "echo");
            assert_eq!(text, "hi");
            assert_eq!(ts, "");
        }
        _ => panic!("expected Message"),
    }
}

#[test]
fn subscribe_yields_empty() {
    let evs = respond(&PluginCommand::Subscribe {
        channel: "general".into(),
    });
    assert!(evs.is_empty());
}
