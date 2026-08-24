//! The resident's contract: it reports itself truthfully, it stays silent on ops that belong to
//! the ask endpoint, and it refuses to bind over a daemon that is already alive.
use super::session::{SessionProc, Spawner};
use super::*;
use crate::ipc_types::{Reply, Request, PROTOCOL_V};
use std::path::Path;

/// A session process that is always running and does nothing — the registry's behaviour is
/// covered in `session_tests`; here it only has to exist so sessions can be counted.
struct Idle;
impl SessionProc for Idle {
    fn alive(&mut self) -> bool {
        true
    }
    fn kill(&mut self) {}
}
struct IdleSpawner;
impl Spawner for IdleSpawner {
    fn spawn(&mut self, _cwd: Option<&Path>) -> std::io::Result<Box<dyn SessionProc>> {
        Ok(Box::new(Idle))
    }
}

fn daemon() -> Daemon {
    Daemon::with_spawner(Box::new(IdleSpawner))
}

fn open(d: &mut Daemon, label: &str) -> String {
    match answer(
        &Request::OpenSession {
            v: PROTOCOL_V,
            label: label.to_string(),
            cwd: None,
        },
        d,
    ) {
        Some(Reply::Session { id }) => id,
        other => panic!("expected a session id, got {other:?}"),
    }
}

/// A fresh daemon reports THIS process, no sessions, and the running build's version.
#[test]
fn a_fresh_daemon_reports_itself() {
    let st = daemon().status();
    assert_eq!(st.pid, std::process::id());
    assert_eq!(st.sessions, 0);
    assert_eq!(st.version, crate::appregister::VERSION);
    assert!(
        st.uptime_s < 5,
        "a just-created daemon is not {}s old",
        st.uptime_s
    );
}

/// The status reply is not a fixed shape — its session count tracks the registry, which is the
/// number the whole "does my work survive?" question hangs on.
#[test]
fn the_status_reply_counts_the_live_registry() {
    let mut d = daemon();
    open(&mut d, "crew");
    open(&mut d, "smith");
    match answer(&Request::daemon_status(), &mut d) {
        Some(Reply::Daemon { pid, sessions, .. }) => {
            assert_eq!(pid, std::process::id());
            assert_eq!(sessions, 2);
        }
        other => panic!("expected a daemon status, got {other:?}"),
    }
}

/// Open, list, close — over the request/reply surface a client actually sees.
#[test]
fn sessions_can_be_opened_listed_and_closed_over_the_wire_types() {
    let mut d = daemon();
    let id = open(&mut d, "crew");
    match answer(&Request::Sessions { v: PROTOCOL_V }, &mut d) {
        Some(Reply::Sessions { sessions }) => {
            assert_eq!(sessions.len(), 1);
            assert_eq!(sessions[0].id, id);
            assert_eq!(sessions[0].label, "crew");
            assert!(sessions[0].alive);
        }
        other => panic!("expected a session list, got {other:?}"),
    }
    assert_eq!(
        answer(
            &Request::CloseSession {
                v: PROTOCOL_V,
                id: id.clone()
            },
            &mut d
        ),
        Some(Reply::Closed {
            id: id.clone(),
            was_alive: true
        })
    );
    match answer(&Request::Sessions { v: PROTOCOL_V }, &mut d) {
        Some(Reply::Sessions { sessions }) => assert!(sessions.is_empty()),
        other => panic!("expected an empty list, got {other:?}"),
    }
}

/// Closing something that does not exist is an explained failure, not a cheerful success.
#[test]
fn closing_an_unknown_session_fails_by_name() {
    let mut d = daemon();
    match answer(
        &Request::CloseSession {
            v: PROTOCOL_V,
            id: "s99".to_string(),
        },
        &mut d,
    ) {
        Some(Reply::Failed { message }) => assert!(
            message.contains("s99"),
            "the failure names the id: {message}"
        ),
        other => panic!("expected a failure, got {other:?}"),
    }
}

/// A client that dials the daemon with an ask op has reached the wrong endpoint. Silence is the
/// honest outcome — answering `Panes` with an empty roster would say "this crew has no panes",
/// which is a different and false statement.
#[test]
fn ask_ops_are_not_served_by_the_daemon() {
    let mut d = daemon();
    assert_eq!(answer(&Request::Panes { v: PROTOCOL_V }, &mut d), None);
    assert_eq!(
        answer(
            &Request::Ask {
                v: PROTOCOL_V,
                from: "a".into(),
                to: "b".into(),
                question: "q".into(),
                id: "1".into(),
            },
            &mut d
        ),
        None
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
    fn serve(path: &Path) -> std::thread::JoinHandle<i32> {
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

    /// End to end over the real transport: bind, dial, deserialize. This is the wiring the pure
    /// tests above cannot cover — wire types, socket framing, and the serve loop together.
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
    /// bind its own — two residents, one address, every client reaching the newer one.
    #[test]
    fn a_second_daemon_refuses_to_hijack_the_endpoint() {
        let path = tmp_sock("hijack");
        let _h = serve(&path);
        assert_eq!(run_at(path.clone()), 1, "second daemon must refuse to bind");
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
