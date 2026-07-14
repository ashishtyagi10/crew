use super::*;
use std::sync::mpsc::channel;

#[test]
fn ask_to_unknown_address_is_unreachable() {
    let mut app = CrewApp::default(); // no panes
    let (tx, rx) = channel::<Reply>();
    app.service_request(
        Request::Ask {
            v: 1,
            from: "a".into(),
            to: "nope".into(),
            question: "q".into(),
            id: "q1".into(),
        },
        tx,
        0,
    );
    assert!(matches!(
        rx.try_recv().unwrap(),
        Reply::NoAnswer {
            reason: NoAnswer::Unreachable,
            ..
        }
    ));
    assert!(app.pending_asks.is_empty(), "no pending ask registered");
}

#[test]
fn panes_request_serves_a_roster() {
    let mut app = CrewApp::default();
    let (tx, rx) = channel::<Reply>();
    app.service_request(Request::Panes { v: 1 }, tx, 0);
    assert!(matches!(rx.try_recv().unwrap(), Reply::Roster { .. }));
}

/// End-to-end over a REAL pty: inject a question into a terminal pane
/// running a cooperating responder, pump the poll loop, and confirm the
/// answer comes back — exercising inject → capture tap → liveness → verdict
/// (and proving the echo of the injected instruction is not mis-read).
#[test]
fn live_terminal_pane_answers_through_the_full_pipeline() {
    use crate::layout::Rect;
    use crate::pane::{Pane, PaneContent, TermPane};
    use crew_term::{GridSize, PtyTerm};
    use std::io::Write;
    use std::time::{Duration, Instant};

    let pty = PtyTerm::spawn(GridSize { cols: 80, rows: 24 }, "sh").unwrap();
    // A cooperating responder: on any line that MENTIONS the marker, print
    // an answer line that BEGINS with it. (Mirrors what an instructed LLM
    // agent does; the echoed instruction line has the marker mid-line and
    // must not be mistaken for the answer.)
    {
        let mut w = pty.writer();
        w.write_all(
            b"while IFS= read -r l; do case \"$l\" in *CREW-ANS-*) \
              printf 'CREW-ANS-qtest: 42\\n';; esac; done\n",
        )
        .unwrap();
        w.flush().unwrap();
    }
    let input = pty.writer();
    let mut app = CrewApp::default();
    app.panes.push(Pane {
        content: PaneContent::Terminal(Box::new(TermPane {
            pty,
            input,
            cmd: None,
            cmd_since: None,
        })),
        grid: GridSize { cols: 80, rows: 24 },
        rect: Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        },
        label: None,
        name: None,
        dir: None,
        activity: false,
        bell: false,
        hidden: false,
        attention: None,
    });

    // Ask pane p0. Fixed id so the responder can hardcode its marker.
    let (tx, rx) = channel::<Reply>();
    app.service_request(
        Request::Ask {
            v: 1,
            from: "tester".into(),
            to: "p0".into(),
            question: "the answer please".into(),
            id: "qtest".into(),
        },
        tx,
        0,
    );
    assert_eq!(app.pending_asks.len(), 1, "ask registered");

    // Pump the loop: try_read fills the capture (as the pane-drain loop
    // does), then pump_asks advances the ask.
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut answer = None;
    let mut now = 0u64;
    while Instant::now() < deadline {
        now += 16;
        if let PaneContent::Terminal(t) = &mut app.panes[0].content {
            t.pty.try_read();
        }
        app.pump_asks(now);
        if let Ok(r) = rx.try_recv() {
            answer = Some(r);
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let ok = matches!(&answer, Some(Reply::Answered { text }) if text == "42");
    assert!(
        ok,
        "expected ANSWERED 42 from the live responder, got {answer:?}"
    );
    assert!(app.pending_asks.is_empty(), "resolved ask is cleared");
}
