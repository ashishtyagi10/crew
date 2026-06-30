use crate::{PluginCommand, PluginEvent};
use anyhow::Result;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};

pub struct Plugin {
    // The broker subprocess. Killed explicitly on drop (see `impl Drop`).
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<PluginEvent>,
}

impl Plugin {
    pub fn spawn(cmd: &str, args: &[String]) -> Result<Plugin> {
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;

        let stdout = child.stdout.take().expect("stdout was piped");
        let stdin = child.stdin.take().expect("stdin was piped");

        let (tx, rx) = mpsc::channel::<PluginEvent>();

        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => break,
                };
                if let Ok(ev) = serde_json::from_str::<PluginEvent>(&line) {
                    if tx.send(ev).is_err() {
                        break;
                    }
                }
                // unparseable lines are silently dropped
            }
        });

        Ok(Plugin { child, stdin, rx })
    }

    pub fn send(&mut self, cmd: &PluginCommand) -> Result<()> {
        writeln!(self.stdin, "{}", serde_json::to_string(cmd)?)?;
        self.stdin.flush()?;
        Ok(())
    }

    pub fn try_recv(&self) -> Vec<PluginEvent> {
        let mut events = Vec::new();
        while let Ok(ev) = self.rx.try_recv() {
            events.push(ev);
        }
        events
    }

    /// PID of the child process (the broker), e.g. for liveness checks.
    pub fn child_id(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for Plugin {
    /// Kill the child on drop. Dropping a [`std::process::Child`] only *detaches*
    /// it — without this, closing a `/crew` pane would orphan the still-running
    /// `crew --broker-plugin` subprocess (and any agents it spawned).
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::Duration;

    #[test]
    fn dropping_the_plugin_kills_the_child() {
        // A long-lived child standing in for the broker subprocess.
        let p = Plugin::spawn("sh", &["-c".to_string(), "sleep 30".to_string()]).unwrap();
        let pid = p.child_id();
        drop(p);
        std::thread::sleep(Duration::from_millis(300));
        // `kill -0` succeeds only while the process exists; once killed and reaped
        // it exits non-zero.
        let alive = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        assert!(
            !alive,
            "broker child {pid} should be killed when the Plugin drops"
        );
    }
}
