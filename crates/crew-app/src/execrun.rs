//! `crew exec`: drive one broker child to the end of one task. The broker is this same binary
//! re-exec'd with `--broker-plugin`, over JSON lines — the wire the `/crew` pane uses.
use crate::execcli::Opts;
use crate::execevent::{Run, EXIT_FAILED};
use crew_hive::childproc::no_console_window;
use crew_plugin::{PluginCommand, PluginEvent};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Run `opts`; the exit code. stdout gets the answer only.
pub(crate) fn run(opts: &Opts) -> i32 {
    let fail = |m: String| {
        eprintln!("crew exec: {m}");
        EXIT_FAILED
    };
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(e) => return fail(format!("cannot find the crew binary: {e}")),
    };
    // stderr is inherited so the broker's own diagnostics stay on the caller's stderr.
    let mut cmd = Command::new(exe);
    no_console_window(&mut cmd);
    let spawned = cmd
        .arg("--broker-plugin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn();
    let mut child = match spawned {
        Ok(c) => c,
        Err(e) => return fail(format!("cannot start the broker: {e}")),
    };
    let mut stdin = child.stdin.take().expect("stdin was piped");
    let stdout = child.stdout.take().expect("stdout was piped");
    let (tx, rx) = mpsc::channel::<PluginEvent>();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Ok(ev) = serde_json::from_str(&line) {
                if tx.send(ev).is_err() {
                    return;
                }
            }
        }
    });
    let mut send = |c: &PluginCommand| -> std::io::Result<()> {
        writeln!(stdin, "{}", serde_json::to_string(c).unwrap_or_default())?;
        stdin.flush()
    };
    let hello = PluginCommand::Hello { v: 1 };
    let go = PluginCommand::Send {
        channel: "crew".into(),
        text: opts.prompt.clone(),
        seq: 1,
    };
    if send(&hello).and_then(|_| send(&go)).is_err() {
        return fail("the broker closed its input".into());
    }
    let mut state = Run::new(opts.allow);
    // A closed channel means the broker's output ended before the task did.
    while !state.done {
        let Ok(ev) = rx.recv() else {
            reap(&mut child);
            return fail("the broker exited before the task finished".into());
        };
        let r = state.absorb(ev);
        r.log.iter().for_each(|l| eprintln!("{l}"));
        if let Some(c) = r.reply {
            let _ = send(&c);
        }
    }
    drop(stdin); // EOF: the broker finishes up and exits
    reap(&mut child);
    if let Some(a) = &state.answer {
        let _ = writeln!(std::io::stdout(), "{a}");
    }
    if state.answer.is_none() && state.errors.is_empty() && !state.refused {
        eprintln!("crew exec: the task ended with no answer");
    }
    state.exit_code()
}

/// Wait for the broker to exit; after a grace period kill it, so a script never hangs on it.
fn reap(child: &mut Child) {
    let t = Instant::now();
    while t.elapsed() < Duration::from_secs(10) {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let _ = child.kill();
    let _ = child.wait();
}
