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
    /// How it was started, so [`Plugin::restart`] can start it again.
    how: Spawn,
}

/// Everything a spawn needs, kept for a restart.
struct Spawn {
    cmd: String,
    args: Vec<String>,
    cwd: Option<std::path::PathBuf>,
    env: Vec<(String, String)>,
}

/// What the reader says when the broker's output ends — it exited, crashed or
/// was killed. Without it the pane never learned: no event ever came again,
/// and a task that died with its broker showed as running forever.
pub const BROKER_ENDED: &str = "the broker process ended";

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
        Self::spawn_with(cmd, args, cwd, &[])
    }

    /// [`Plugin::spawn_in`] with environment overrides for the child. A
    /// Dock-launched host carries launchd's minimal PATH, and a broker that
    /// inherits it cannot find Homebrew's `ant`, npm's `claude` or cargo's
    /// `codex` — so it reported a CLI the user had just signed in with as
    /// "not installed". The host already resolved the login-shell PATH for
    /// its own command detection; this is how it hands it down.
    pub fn spawn_with(
        cmd: &str,
        args: &[String],
        cwd: Option<&std::path::Path>,
        env: &[(String, String)],
    ) -> Result<Plugin> {
        let how = Spawn {
            cmd: cmd.to_string(),
            args: args.to_vec(),
            cwd: cwd.map(std::path::Path::to_path_buf),
            env: env.to_vec(),
        };
        let (child, stdin, rx) = start(&how)?;
        Ok(Plugin {
            child,
            stdin,
            rx,
            how,
        })
    }

    /// Start the broker again, the way it was first started: the old process
    /// tree is killed, and anything it still had in flight is gone with it.
    pub fn restart(&mut self) -> Result<()> {
        kill_tree(&mut self.child);
        let (child, stdin, rx) = start(&self.how)?;
        (self.child, self.stdin, self.rx) = (child, stdin, rx);
        Ok(())
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

fn start(how: &Spawn) -> Result<(Child, ChildStdin, Receiver<PluginEvent>)> {
    let mut command = Command::new(&how.cmd);
    no_console_window(&mut command);
    command
        .args(&how.args)
        .envs(how.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    // Only an existing directory: `current_dir` on a vanished path fails
    // the whole spawn, and a broker in the host's CWD beats no broker.
    if let Some(dir) = how.cwd.as_deref().filter(|d| d.is_dir()) {
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
                    return; // nobody is listening: the host is gone
                }
            }
            // unparseable lines are silently dropped
        }
        // The output ended. A host still listening hears about it; one that
        // dropped or restarted this broker has no receiver, and this is lost.
        let _ = tx.send(PluginEvent::Error {
            message: BROKER_ENDED.to_string(),
        });
    });

    Ok((child, stdin, rx))
}

/// Kill the broker and everything it started.
fn kill_tree(child: &mut Child) {
    // The GROUP first: the broker spawns agent CLIs, and those are the
    // expensive things to leave behind. `spawn` makes the broker a group
    // leader, so its pid doubles as the group id.
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGKILL);
    }
    // Then the child itself — belt and braces on unix, and the whole
    // mechanism everywhere else.
    let _ = child.kill();
    let _ = child.wait();
}

impl Drop for Plugin {
    /// Kill the child on drop. Dropping a [`std::process::Child`] only *detaches*
    /// it — without this, closing a `/crew` pane would orphan the still-running
    /// `crew --broker-plugin` subprocess (and any agents it spawned).
    fn drop(&mut self) {
        kill_tree(&mut self.child);
    }
}

#[cfg(test)]
#[path = "host_tests.rs"]
mod tests;
