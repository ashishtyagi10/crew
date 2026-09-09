use crew_plugin::{Plugin, PluginCommand, PluginEvent};
use std::time::{Duration, Instant};

fn drain_until<F: Fn(&PluginEvent) -> bool>(p: &Plugin, pred: F) -> bool {
    let end = Instant::now() + Duration::from_secs(3);
    while Instant::now() < end {
        if p.try_recv().iter().any(&pred) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

// The broker's CWD *is* the project: session logs, `.crew/specialists.json`
// and every other project-relative read resolve against it. A pane spawned
// from a Dock-launched app (CWD `/`) must therefore be able to place its
// plugin child in the pane's tracked directory instead of inheriting `/`.
#[test]
fn spawn_in_places_the_child_in_the_given_directory() {
    let dir = std::env::temp_dir().join("crew-host-cwd-test");
    std::fs::create_dir_all(&dir).unwrap();
    let want = dir.canonicalize().unwrap();
    // A minimal hand-rolled plugin: one Message event whose text is the
    // child's physical working directory.
    let script = r#"printf '{"type":"message","channel":"c","sender":"s","text":"%s","ts":"0"}\n' "$(pwd -P)""#;
    let p = Plugin::spawn_in("/bin/sh", &["-c".into(), script.into()], Some(&dir)).unwrap();
    assert!(
        drain_until(&p, |e| matches!(
            e,
            PluginEvent::Message { text, .. } if std::path::Path::new(text) == want
        )),
        "child did not run in {want:?}"
    );
}

// A Dock-launched host's broker inherits launchd's PATH and then cannot find
// the very CLI it just told the user to install. The host hands the child its
// resolved environment; the child must actually see it.
#[test]
fn spawn_with_hands_the_child_its_environment() {
    let script = r#"printf '{"type":"message","channel":"c","sender":"s","text":"%s","ts":"0"}\n' "$CREW_HOST_TEST_PATH""#;
    let env = [(
        "CREW_HOST_TEST_PATH".to_string(),
        "/opt/homebrew/bin:/usr/bin".to_string(),
    )];
    let p = Plugin::spawn_with("/bin/sh", &["-c".into(), script.into()], None, &env).unwrap();
    assert!(
        drain_until(&p, |e| matches!(
            e,
            PluginEvent::Message { text, .. } if text == "/opt/homebrew/bin:/usr/bin"
        )),
        "child did not receive the env"
    );
}

#[test]
fn echo_roundtrip() {
    let mut p = Plugin::spawn(env!("CARGO_BIN_EXE_crew-echo-plugin"), &[]).unwrap();
    p.send(&PluginCommand::Hello { v: 1 }).unwrap();
    assert!(drain_until(&p, |e| matches!(
        e,
        PluginEvent::Ready { provider, .. } if provider == "echo"
    )));
    p.send(&PluginCommand::Send {
        channel: "general".into(),
        text: "ping".into(),
    })
    .unwrap();
    assert!(drain_until(&p, |e| matches!(
        e,
        PluginEvent::Message { text, .. } if text == "ping"
    )));
}
