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
        let mut command = Command::new(cmd);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
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
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::Duration;

    /// The child's own children go too. A broker spawns agent CLIs, and
    /// killing a parent does not kill its children — measured before the fix,
    /// a killed broker left a running `claude` alive and reparented, still
    /// working and still spending. `spawn` puts the broker in its own process
    /// group so `drop` can take the group.
    #[test]
    #[cfg(unix)]
    fn dropping_the_plugin_kills_the_whole_tree() {
        // A child that spawns a grandchild, records its pid, and waits — so
        // both are alive when the plugin is dropped. The pid goes to a file
        // rather than stdout: the reader thread forwards parsed events only.
        let pidfile = std::env::temp_dir().join(format!(
            "crew-tree-test-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let script = format!(
            "sh -c 'sleep 30' & echo $! > {} ; sleep 30",
            pidfile.display()
        );
        let p = Plugin::spawn("sh", &["-c".to_string(), script]).unwrap();
        let mut grandchild = 0i32;
        for _ in 0..40 {
            std::thread::sleep(Duration::from_millis(50));
            if let Ok(t) = std::fs::read_to_string(&pidfile) {
                if let Ok(pid) = t.trim().parse() {
                    grandchild = pid;
                    break;
                }
            }
        }
        assert!(grandchild > 0, "grandchild never reported its pid");
        let alive = |pid: i32| {
            Command::new("kill")
                .args(["-0", &pid.to_string()])
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };
        assert!(alive(grandchild), "grandchild should be running");
        drop(p);
        std::thread::sleep(Duration::from_millis(400));
        assert!(
            !alive(grandchild),
            "grandchild {grandchild} outlived the plugin that owned it"
        );
        let _ = std::fs::remove_file(&pidfile);
    }

    /// Unique temp names, so two of these can run at once.
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

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
