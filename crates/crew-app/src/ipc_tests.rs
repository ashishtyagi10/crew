use super::*;
use crate::ipc_types::PaneCard;

#[test]
fn socket_round_trips_a_panes_request() {
    // Unique temp path (no Date/rand available; use the test thread id).
    let path = std::env::temp_dir().join(format!(
        "crew-ipc-test-{:?}.sock",
        std::thread::current().id()
    ));
    let handle = spawn_at(path.clone()).expect("bind");

    // Client: connect, send a Panes request.
    let mut client = UnixStream::connect(&path).expect("connect");
    client.write_all(b"{\"op\":\"Panes\",\"v\":1}\n").unwrap();
    client.flush().unwrap();

    // App side: receive, reply with a roster.
    let incoming = handle
        .rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    assert_eq!(incoming.req, Request::Panes { v: 1 });
    incoming
        .reply
        .send(Reply::Roster {
            panes: vec![PaneCard {
                id: "p0".into(),
                label: None,
                kind: "terminal".into(),
                running: None,
                dir: None,
                busy: false,
            }],
        })
        .unwrap();

    // Client reads the reply line.
    let mut buf = String::new();
    BufReader::new(&mut client).read_line(&mut buf).unwrap();
    let reply: Reply = serde_json::from_str(buf.trim()).unwrap();
    assert!(matches!(reply, Reply::Roster { panes } if panes.len() == 1));
    drop(handle); // unlinks the socket
}

#[test]
fn socket_name_is_default_or_per_instance_and_path_safe() {
    assert_eq!(socket_name(None), "crew-ipc.sock");
    assert_eq!(socket_name(Some("alpha")), "crew-ipc-alpha.sock");
    // Empty / all-unsafe ids fall back to the shared default socket.
    assert_eq!(socket_name(Some("")), "crew-ipc.sock");
    assert_eq!(socket_name(Some("///")), "crew-ipc.sock");
    // Path-traversal attempts are stripped to a safe filename fragment.
    assert_eq!(
        socket_name(Some("../etc/passwd")),
        "crew-ipc-etcpasswd.sock"
    );
    assert_eq!(socket_name(Some("a/b\\c")), "crew-ipc-abc.sock");
}

#[test]
fn instance_of_reads_the_id_from_a_socket_name() {
    assert_eq!(instance_of("crew-ipc.sock").as_deref(), Some("default"));
    assert_eq!(instance_of("crew-ipc-alpha.sock").as_deref(), Some("alpha"));
    assert_eq!(instance_of("crew-ipc-a-b.sock").as_deref(), Some("a-b"));
    assert_eq!(instance_of("other.sock"), None);
    assert_eq!(instance_of("crew-ipc.txt"), None);
}

#[test]
fn list_instances_in_finds_only_crew_sockets() {
    let dir =
        std::env::temp_dir().join(format!("crew-inst-test-{:?}", std::thread::current().id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    for f in ["crew-ipc.sock", "crew-ipc-alpha.sock", "notes.txt"] {
        std::fs::write(dir.join(f), b"").unwrap();
    }
    let mut got = list_instances_in(&dir);
    got.sort();
    assert_eq!(got, vec!["alpha".to_string(), "default".to_string()]);
    let _ = std::fs::remove_dir_all(&dir);
}
