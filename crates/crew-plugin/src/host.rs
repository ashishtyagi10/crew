use crate::{PluginCommand, PluginEvent};
use anyhow::Result;
use crew_hive::childproc::no_console_window;
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
        Self::spawn_in(cmd, args, None)
    }

    /// [`Plugin::spawn`] with an explicit working directory for the child.
    /// The broker treats its CWD as the project (session logs,
    /// `.crew/specialists.json`), so a host whose own CWD is meaningless —
    /// a Dock-launched app runs at `/` — must place the child in the pane's
    /// tracked directory instead of letting it inherit.
    pub fn spawn_in(cmd: &str, args: &[String], cwd: Option<&std::path::Path>) -> Result<Plugin> {
        let mut command = Command::new(cmd);
        no_console_window(&mut command);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        // Only an existing directory: `current_dir` on a vanished path fails
        // the whole spawn, and a broker in the host's CWD beats no broker.
        if let Some(dir) = cwd.filter(|d| d.is_dir()) {
            command.current_dir(dir);
        }
        // Its own process group, so [`Plugin::drop`] can take the whole tree
        // and not just the broker. Killing a parent does not kill its
        // children: measured, a broker killed while an agent CLI was running
        // left that CLI alive and reparented — still working, still spending.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command.spawn()?;

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
        // The GROUP first: the broker spawns agent CLIs, and those are the
        // expensive things to leave behind. `spawn` makes the broker a group
        // leader, so its pid doubles as the group id.
        #[cfg(unix)]
        unsafe {
            libc::kill(-(self.child.id() as libc::pid_t), libc::SIGKILL);
        }
        // Then the child itself — belt and braces on unix, and the whole
        // mechanism everywhere else.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
#[path = "host_tests.rs"]
mod tests;
