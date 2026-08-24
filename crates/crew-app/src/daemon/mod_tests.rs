//! The resident's contract: it reports itself truthfully, it stays silent on ops that belong to
//! the ask endpoint, and it refuses to bind over a daemon that is already alive.
use super::*;
use crate::ipc_types::{Reply, Request, PROTOCOL_V};

fn status(uptime_s: u64, sessions: usize) -> Status {
    Status {
        pid: 4242,
        uptime_s,
        sessions,
        version: "9.9.9".to_string(),
    }
}

#[test]
fn status_reply_carries_every_field_verbatim() {
    let st = status(77, 3);
    let reply = answer(&Request::daemon_status(), &st).expect("daemon serves DaemonStatus");
    assert_eq!(
        reply,
        Reply::Daemon {
            pid: 4242,
            uptime_s: 77,
            sessions: 3,
            version: "9.9.9".to_string(),
        }
    );
}

/// A client that dials the daemon with an ask op has reached the wrong endpoint. Silence (the
/// connection closes unanswered) is the honest outcome — answering `Panes` with an empty roster
/// would tell the caller "this crew has no panes", which is a different and false statement.
#[test]
fn ask_ops_are_not_served_by_the_daemon() {
    let st = status(1, 0);
    assert_eq!(answer(&Request::Panes { v: PROTOCOL_V }, &st), None);
    assert_eq!(
        answer(
            &Request::Ask {
                v: PROTOCOL_V,
                from: "a".into(),
                to: "b".into(),
                question: "q".into(),
                id: "1".into(),
            },
            &st
        ),
        None
    );
}

/// A fresh daemon reports THIS process, no sessions (the registry is genuinely empty until 1.2
/// moves session ownership off the pane), and the running build's version.
#[test]
fn a_fresh_daemon_reports_itself() {
    let st = Daemon::new().status();
    assert_eq!(st.pid, std::process::id());
    assert_eq!(st.sessions, 0);
    assert_eq!(st.version, crate::appregister::VERSION);
    assert!(
        st.uptime_s < 5,
        "a just-created daemon is not {}s old",
        st.uptime_s
    );
}

#[cfg(unix)]
mod live {
    use super::*;

    /// A unique socket path under the temp dir — short, because a Unix socket path is capped
    /// around 100 bytes and `$TMPDIR` on macOS is already long.
    fn tmp_sock(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("crewd-t{}-{}.sock", std::process::id(), tag))
    }

    /// Stand a real resident up on a temporary endpoint and wait for it to answer.
    fn serve(path: &std::path::Path) -> std::thread::JoinHandle<i32> {
        let p = path.to_path_buf();
        let h = std::thread::spawn(move || run_at(p));
        for _ in 0..200 {
            if probe_at(path).is_some() {
                return h;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        panic!("daemon never came up on {}", path.display());
    }

    /// End to end over the real transport: bind, dial, deserialize. This is the wiring the
    /// pure tests above cannot cover — wire types, socket framing, and the serve loop together.
    #[test]
    fn a_running_daemon_answers_a_probe_over_the_socket() {
        let path = tmp_sock("probe");
        let _h = serve(&path);
        let st = probe_at(&path).expect("a bound daemon answers");
        assert_eq!(st.pid, std::process::id());
        assert_eq!(st.sessions, 0);
        assert_eq!(st.version, crate::appregister::VERSION);
        let _ = std::fs::remove_file(&path);
    }

    /// `ipc::spawn_at` reclaims a stale socket unconditionally. Without the liveness probe in
    /// `run_at`, a second `crew daemon run` would silently unlink the live resident's path and
    /// bind its own — two residents, one address, and every client reaching the newer one.
    #[test]
    fn a_second_daemon_refuses_to_hijack_the_endpoint() {
        let path = tmp_sock("hijack");
        let _h = serve(&path);
        assert_eq!(run_at(path.clone()), 1, "second daemon must refuse to bind");
        // The FIRST daemon still owns the path and still answers.
        assert_eq!(
            probe_at(&path).map(|s| s.pid),
            Some(std::process::id()),
            "the original resident survived the second start attempt"
        );
        let _ = std::fs::remove_file(&path);
    }

    /// Nothing bound: probing is a clean miss, not a hang or a panic.
    #[test]
    fn probing_a_dead_endpoint_reports_nothing() {
        assert!(probe_at(&tmp_sock("absent")).is_none());
    }
}
