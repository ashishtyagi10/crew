//! The `claude` relay agent, LIVE: one `claude -p <body> --output-format
//! stream-json` run per call, read line by line (`crew_hive::claudestream`)
//! so the pane sees the model think, write and call its tools while the
//! task runs, instead of a blank 180 s and a timeout. Claude Code's own
//! tools stay ON — this is an agent working in the pane's directory, and
//! the tool lines are the feedback the user asked for.
//!
//! The deadline is an IDLE one: it restarts on every line the CLI prints.
//! A task that reads a repo for ten minutes is alive the whole time; a CLI
//! that goes silent for the whole window is hung and is killed. The window
//! is the broker's call timeout with a floor (`IDLE_FLOOR`), because one
//! long-running tool call (a build, a test run) prints nothing until it
//! returns.
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crew_hive::childproc::no_console_window;
use crew_hive::claudestream::{parse_line, Fold};

use super::adapter::{Adapter, HopStream, Usage};
use super::claudelane::{finish, Lane};
use super::clauderesume::{Resume, Task};
use super::clistop::{end, Stop};
use super::run::on_path;

/// The least idle time a streaming agent is granted, whatever the call
/// timeout says: a tool call prints nothing while it runs.
const IDLE_FLOOR: Duration = Duration::from_secs(600);

pub struct ClaudeAgent {
    pub program: String,
    pub model: Option<String>,
    /// Test seam: the idle floor (`IDLE_FLOOR` in the real adapter).
    pub idle_floor: Duration,
    /// The session the next message continues (`clauderesume`).
    pub resume: Resume,
}

impl ClaudeAgent {
    pub fn new(model: Option<String>) -> Self {
        Self {
            program: "claude".into(),
            model: model.filter(|m| !m.is_empty()),
            idle_floor: IDLE_FLOOR,
            resume: Resume::shared(),
        }
    }

    /// The argv one call becomes: print mode with the body as an ARGUMENT
    /// (never piped as chatter), the streamed envelope (`--verbose` is what
    /// lets `-p` stream; partial messages carry the deltas), the model when
    /// one is pinned, and the session it continues. No `--tools ""`: the
    /// agent keeps its hands.
    pub fn args(&self, body: &str, resume: Option<&str>) -> Vec<String> {
        let mut v = vec![
            "-p".to_string(),
            body.to_string(),
            "--output-format".into(),
            "stream-json".into(),
            "--verbose".into(),
            "--include-partial-messages".into(),
        ];
        if let Some(m) = &self.model {
            v.push("--model".into());
            v.push(m.clone());
        }
        if let Some(id) = resume {
            v.extend(["--resume".into(), id.to_string()]);
        }
        v
    }

    pub(super) fn run_once(
        &self,
        body: &str,
        timeout: Duration,
        stream: &HopStream,
        resume: Option<&str>,
        task: &Task,
    ) -> Result<(String, Usage), String> {
        let program = &self.program;
        let mut child = no_console_window(&mut Command::new(program))
            .args(self.args(body, resume))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to launch {program}: {e}"))?;
        let stdout = child.stdout.take().expect("stdout was piped");
        let (tx, rx) = mpsc::channel::<String>();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        // Drained on its own thread so a chatty CLI can never fill the pipe.
        let stderr = child.stderr.take().expect("stderr was piped");
        let (etx, erx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut s = String::new();
            let mut r = stderr;
            let _ = r.read_to_string(&mut s);
            let _ = etx.send(s);
        });

        let idle = timeout.max(self.idle_floor);
        let mut lane = Lane::new();
        let mut fold = Fold::default();
        let mut last = Instant::now();
        let stop = Stop::current();
        loop {
            if stop.pressed() {
                (stream.on_thought)("");
                return Err(end(&mut child));
            }
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(line) => {
                    last = Instant::now();
                    let ev = parse_line(&line);
                    lane.forward(ev.as_ref(), stream);
                    fold.push(ev.as_ref(), &line);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if last.elapsed() >= idle {
                        let _ = child.kill();
                        let _ = child.wait();
                        (stream.on_thought)("");
                        return Err(format!("{program}: no output for {idle:?} \u{2014} killed"));
                    }
                }
            }
        }
        let _ = child.wait();
        (stream.on_thought)(""); // the hop is over: flush the gate's tail
        let stderr = erx
            .recv_timeout(Duration::from_millis(250))
            .unwrap_or_default();
        if let Some(r) = fold.result.as_ref() {
            self.resume.keep(&r.session_id, task);
        }
        finish(fold, program, &stderr)
    }
}

impl Adapter for ClaudeAgent {
    fn name(&self) -> &str {
        "claude"
    }

    fn model(&self) -> &str {
        self.model.as_deref().unwrap_or("")
    }

    fn probe(&self) -> bool {
        on_path(&self.program)
    }

    fn reads_files(&self) -> bool {
        true
    }

    fn call(&self, body: &str, timeout: Duration) -> Result<String, String> {
        self.run(body, timeout, &HopStream::noop()).map(|(t, _)| t)
    }

    fn call_with_usage(&self, body: &str, timeout: Duration) -> Result<(String, Usage), String> {
        self.run(body, timeout, &HopStream::noop())
    }

    fn call_with_usage_ticked(
        &self,
        body: &str,
        timeout: Duration,
        stream: &HopStream,
    ) -> Result<(String, Usage), String> {
        self.run(body, timeout, stream)
    }
}

#[cfg(test)]
#[path = "claudeagent_tests.rs"]
mod tests;
