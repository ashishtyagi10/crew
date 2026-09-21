//! The typed doors to a run: `/todo run [@project]` and `/todo project
//! <name> <dir>` — and the ask that fills the bar with the latter. Split
//! from [`crate::todorun`] for the line cap, along the line between doing
//! the run and naming which item, or which directory.
use crate::app::CrewApp;
use crate::todopane::{item, projectdir, store};

impl CrewApp {
    /// `/todo run [@project]`: the selected row of the focused todo pane
    /// wins; else the open item that is due soonest — of `@project`, or of
    /// any project at all. The due date is the priority: the list already
    /// orders overdue first, then by due, then undated by age, and what
    /// runs next is what the list shows first ([`item::display_order`]).
    pub(crate) fn todo_run_command(&mut self, tag: Option<&str>) {
        let selected = match self.panes.get(self.focused).map(|p| &p.content) {
            Some(crate::pane::PaneContent::Todo(t)) if tag.is_none() => {
                t.sel.and_then(|s| t.id_at(s))
            }
            _ => None,
        };
        let items = store::snapshot();
        let filter = item::Filters {
            project: tag.map(|t| t.trim_start_matches('@')),
            who: None,
        };
        let soonest = item::display_order(&items, filter, false)
            .into_iter()
            .map(|i| &items[i])
            .find(|it| it.project.is_some())
            .map(|it| it.id);
        match selected.or(soonest) {
            Some(id) => self.run_todo(id),
            None => self.set_status("usage: /todo run [@project] — or select a row and press r"),
        }
    }

    /// `/todo project <name> <dir>`: bind a project the resolver cannot
    /// place, or correct one it placed wrong.
    pub(crate) fn todo_project_command(&mut self, name: &str, dir: &str) {
        let name = name.trim_start_matches('@');
        let dir = crate::cmdcheck::expand_home(dir);
        if !dir.is_dir() {
            self.set_status_err(format!("{} is not a directory", dir.display()));
            return;
        }
        projectdir::bind(name, &dir);
        self.set_status(format!("@{name} \u{2192} {}", dir.display()));
        // The run whose ask this answers continues — only that one: a bind
        // for some other project drops the memory rather than misfiling it.
        let asked = self.todo_pending_run.take().filter(|&id| {
            store::snapshot().iter().any(|it| {
                it.id == id
                    && it
                        .project
                        .as_deref()
                        .is_some_and(|p| p.eq_ignore_ascii_case(name))
            })
        });
        if let Some(id) = asked {
            self.run_todo(id);
        }
    }

    /// A project crew cannot place: the answer is one path away, so the
    /// bar is filled with the binding command up to that path and focused
    /// — Tab completes directories there as it does after `cd`. A bar the
    /// user is already typing in is never clobbered (the ask-bar rule); the
    /// status carries the command instead.
    pub(crate) fn ask_where(&mut self, project: &str) {
        let cmd = format!("/todo project {project} ");
        if self.input.text.is_empty() {
            self.input.text = cmd;
            self.input.focused = true;
            self.set_status(format!(
                "@{project} · no directory — type its path, Tab completes, Enter binds and runs"
            ));
        } else {
            self.set_status(format!("@{project} · no directory — {cmd}~/path/to/it"));
        }
    }
}
