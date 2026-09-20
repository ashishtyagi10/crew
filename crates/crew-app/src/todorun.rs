//! A todo that runs itself: `r` on a row, `/todo run`, `/todo project`.
//!
//! The list knew what a task was and where it belonged (`@project`) and
//! could not DO it; the rest of crew could run a line in a shell and drive
//! `claude`, `codex` and `opencode` as agents, and the two never met. This
//! is the meeting: the item's words are the task, verbatim; the project is
//! resolved to a directory ([`crate::todopane::projectdir`]) or the run
//! stops and says why; the agent is chosen by a ladder that prints its
//! reason ([`crate::todopane::agentpick`]); the pane opens IN that
//! directory with the task as the agent's opening prompt, under the same
//! persistent wrapper `/run` uses, so the agent's exit leaves a prompt
//! there. The item records the hand-off; nothing marks it done but you.
use std::path::{Path, PathBuf};

use crate::app::CrewApp;
use crate::applog::LogLevel;
use crate::todopane::agentpick::{self, Agent, Facts, Markers};
use crate::todopane::item::TodoRun;
use crate::todopane::{projectdir, store};

/// One word, single-quoted for `sh -c`: the task is prose and prose has
/// apostrophes.
fn quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

impl CrewApp {
    /// Every directory crew's panes are in, plus the bar's own — what the
    /// project resolver may infer from.
    fn pane_dirs(&self) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = self.panes.iter().filter_map(|p| p.dir.clone()).collect();
        if let Some(c) = self.spawn_cwd() {
            dirs.push(c.to_path_buf());
        }
        dirs
    }

    /// The CLIs the login-shell PATH can find — the `on_path` fact.
    fn agents_on_path(&mut self) -> Vec<Agent> {
        [Agent::Claude, Agent::Codex, Agent::Opencode]
            .into_iter()
            .filter(|a| {
                let bin = a.bin().unwrap_or_default();
                matches!(
                    self.check_command(bin),
                    crate::cmdcheck::Verdict::Executable(_)
                )
            })
            .collect()
    }

    /// Hand item `id` to an agent in its project's directory.
    pub(crate) fn run_todo(&mut self, id: u64) {
        let Some(it) = store::snapshot().into_iter().find(|it| it.id == id) else {
            return;
        };
        let Some(project) = it.project.clone() else {
            self.set_status("no @project on this item — tag it, then r");
            return;
        };
        let dirs = self.pane_dirs();
        let dir = projectdir::resolve(&project, &projectdir::bound(), &dirs).filter(|d| d.is_dir());
        let Some(dir) = dir else {
            self.ask_where(&project);
            return;
        };
        projectdir::bind(&project, &dir);
        let on_path = self.agents_on_path();
        let signins = crate::modelsignin::now();
        let signed: Vec<&str> = signins
            .iter()
            .filter(|o| o.signed_in)
            .map(|o| o.name.as_str())
            .collect();
        let serving = crate::modelsignin::serving();
        let facts = Facts {
            on_path: &on_path,
            signed_in: &signed,
            serving: serving.as_deref(),
        };
        let (agent, why) = match agentpick::pick(it.assignee.as_deref(), Markers::at(&dir), &facts)
        {
            Ok(p) => p,
            Err(e) => {
                self.set_status_err(format!("@{project} · {e}"));
                return;
            }
        };
        let label = format!("@{project} \u{b7} {}", agent.name());
        let opened = match agent.invocation(&it.title) {
            Some((bin, args)) => self.run_cli_in(bin, &args, &label, &dir),
            None => self.run_in_smith(&it.title, &dir),
        };
        if !opened {
            return;
        }
        let line = format!("\u{25b6} {label} \u{b7} {why} \u{b7} {}", dir.display());
        self.log_line(LogLevel::Info, &line);
        self.set_status(line);
        let started_ms = crate::chattime::unix_now_ms();
        store::mutate(|items| {
            if let Some(it) = items.iter_mut().find(|it| it.id == id) {
                it.run = Some(TodoRun {
                    agent: agent.name().to_string(),
                    started_ms,
                    // The pane the run can be found in later: the smith
                    // pane keeps its routing label whatever it was asked.
                    pane: if agent == Agent::Smith {
                        "crew".to_string()
                    } else {
                        label.clone()
                    },
                });
            }
        });
    }

    /// A project crew cannot place: the answer is one path away, so the
    /// bar is filled with the binding command up to that path and focused
    /// — Tab completes directories there as it does after `cd`. A bar the
    /// user is already typing in is never clobbered (the ask-bar rule); the
    /// status carries the command instead.
    fn ask_where(&mut self, project: &str) {
        let cmd = format!("/todo project {project} ");
        if self.input.text.is_empty() {
            self.input.text = cmd;
            self.input.focused = true;
            self.set_status(format!(
                "@{project} · no directory — type its path, Tab completes, Enter binds"
            ));
        } else {
            self.set_status(format!("@{project} · no directory — {cmd}~/path/to/it"));
        }
    }

    /// A CLI agent in a terminal pane at `dir`: the wrapper keeps the pane
    /// after the agent exits and its `set -m` keeps busy detection exact.
    fn run_cli_in(&mut self, bin: &str, args: &[String], label: &str, dir: &Path) -> bool {
        let body = std::iter::once(bin.to_string())
            .chain(args.iter().map(|a| quote(a)))
            .collect::<Vec<_>>()
            .join(" ");
        let shell = crate::spawn::default_shell();
        let (_, program, script) =
            crate::runpane::run_parts(&body, &shell, crate::runpane::bash_path());
        let before = self.panes.len();
        self.spawn_labeled_terminal_in(
            &program,
            &["-c".to_string(), script],
            label.to_string(),
            Some(dir.to_path_buf()),
        );
        self.panes.len() > before
    }

    /// The smith rung: a fresh agent smith pane is started IN `dir` (the
    /// broker's working directory is the bar's, so the bar moves there,
    /// as `cd` would); an open idle one is told where to work in words,
    /// because a broker's directory is fixed at its spawn; a busy one is
    /// refused aloud rather than queued in silence.
    fn run_in_smith(&mut self, task: &str, dir: &Path) -> bool {
        let crew = self
            .panes
            .iter()
            .position(|p| p.label.as_deref() == Some("crew"));
        let text = match crew {
            Some(i) => {
                if let crate::pane::PaneContent::Chat(c) = &self.panes[i].content {
                    if c.is_busy() {
                        self.set_status_err("agent smith is busy — run with #claude, or wait");
                        return false;
                    }
                }
                format!("in {}: {task}", dir.display())
            }
            None => {
                self.cwd = dir.to_path_buf();
                self.spawn_crew_pane();
                task.to_string()
            }
        };
        let i = self
            .panes
            .iter()
            .position(|p| p.label.as_deref() == Some("crew"));
        let Some(i) = i else {
            return false; // the spawn failed and said so in the status
        };
        if let crate::pane::PaneContent::Chat(c) = &mut self.panes[i].content {
            c.submit_command(text);
        }
        self.focused = i;
        self.input.focused = false;
        true
    }
}

#[cfg(test)]
#[path = "todorun_tests.rs"]
mod tests;
