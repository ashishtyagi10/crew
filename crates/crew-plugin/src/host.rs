use crate::{PluginCommand, PluginEvent};
use anyhow::Result;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};

pub struct Plugin {
    // held to keep the child process alive; dropped on Plugin drop
    _child: Child,
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

        Ok(Plugin {
            _child: child,
            stdin,
            rx,
        })
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
}
