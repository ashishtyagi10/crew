//! `/todo` and `/notify`: the two commands that speak to a pane's own store
//! rather than to how crew looks. Split out of [`crate::dispatch`] for the
//! line cap.
use crate::app::CrewApp;

#[cfg(test)]
#[path = "dispatchtodo_tests.rs"]
mod tests;

impl CrewApp {
    /// Handle `/todo done [@project|#who]` (the done-history view,
    /// optionally pre-filtered), `/todo show` / `/todo hide` (done items
    /// inline on the active list) and `/todo by who` / `/todo by flat` (band
    /// the list under each `#assignee`, or don't). Bare `/todo` (the active
    /// list) dispatches directly and never reaches here.
    pub(crate) fn todo_command(&mut self, arg: &str) {
        let mut words = arg.split_whitespace();
        let usage = "usage: /todo [done [@project|#who] | show | hide | by who|flat]";
        match (words.next(), words.next(), words.next()) {
            (Some(verb @ ("show" | "hide")), None, None) => self.todo_show_done(verb == "show"),
            (Some("by"), Some(how @ ("who" | "flat")), None) => self.todo_group(how == "who"),
            (Some("done"), tag, None) => {
                let filter = match tag {
                    None => None,
                    Some(t) => {
                        match crate::todopane::parse::lone_tag(t).filter(|(_, n)| !n.is_empty()) {
                            Some((s, n)) => Some((s, n.to_string())),
                            None => {
                                self.set_status(usage);
                                return;
                            }
                        }
                    }
                };
                self.spawn_todo_pane_done(filter);
            }
            _ => self.set_status(usage),
        }
    }

    /// `/todo by who` / `/todo by flat`: band the list under each
    /// `#assignee`, or lay it flat again — the typed way to the list's `g`.
    /// Acts on the same pane `/todo show` does, spawning one if none is open.
    fn todo_group(&mut self, on: bool) {
        let i = self.todo_target();
        if let crate::pane::PaneContent::Todo(t) = &mut self.panes[i].content {
            t.set_done_view(false);
            t.set_grouped(on);
            let n = crate::todopane::group::people(&t.items, t.filters()).len();
            self.focused = i;
            self.input.focused = false;
            self.set_status(match (on, n) {
                (true, 0) => "nobody is assigned yet — tag an item #name".to_string(),
                (true, 1) => "grouped by 1 assignee".to_string(),
                (true, n) => format!("grouped by {n} assignees"),
                (false, _) => "one flat list".to_string(),
            });
        }
    }

    /// The todo pane a `/todo` sub-command acts on: the focused one, else
    /// the most recent, else a fresh one — so every sub-command works from
    /// a cold start.
    fn todo_target(&mut self) -> usize {
        let is_todo =
            |p: &crate::pane::Pane| matches!(p.content, crate::pane::PaneContent::Todo(_));
        let target = Some(self.focused)
            .filter(|&i| self.panes.get(i).is_some_and(is_todo))
            .or_else(|| self.panes.iter().rposition(is_todo));
        match target {
            Some(i) => i,
            None => {
                self.spawn_todo_pane();
                self.panes.len() - 1
            }
        }
    }

    /// `/todo show` / `/todo hide`: flip done items on or off in a todo
    /// pane's list — the typed way to the header's `[show N done]` button.
    /// It acts on the focused todo pane, else the most recent one, and
    /// spawns a list if none is open (so the command works from a cold
    /// start, like `/todo done` does).
    fn todo_show_done(&mut self, show: bool) {
        let i = self.todo_target();
        if let crate::pane::PaneContent::Todo(t) = &mut self.panes[i].content {
            // The history view is done-only already; `/todo show` there means
            // "back to the list, with the done items in it".
            t.set_done_view(false);
            t.set_show_done(show);
            let n = t.items.iter().filter(|it| it.done).count();
            self.focused = i;
            self.input.focused = false;
            self.set_status(match (show, n) {
                (true, 0) => "nothing done yet".to_string(),
                (true, n) => format!("showing {n} done item{}", if n == 1 { "" } else { "s" }),
                (false, _) => "done items hidden".to_string(),
            });
        }
    }

    /// Handle `/notify [on|off|add <text>|clear]`: with no argument it reports the
    /// current state; otherwise it toggles the master switch or edits the watched
    /// output patterns (persisted, and pushed to live panes).
    pub(crate) fn notify_command(&mut self, arg: &str) {
        match arg {
            "" => {
                let state = if self.config.notify { "on" } else { "off" };
                self.set_status(format!(
                    "notifications {state} · {} pattern(s) · {} recent",
                    self.config.notify_patterns.len(),
                    self.notifier.len()
                ));
            }
            "on" => {
                self.config.notify = true;
                self.config.save();
                self.set_status("notifications on");
            }
            "off" => {
                self.config.notify = false;
                self.config.save();
                self.set_status("notifications off");
            }
            "clear" => {
                self.config.notify_patterns.clear();
                self.config.save();
                self.apply_notify_patterns();
                self.set_status("notify patterns cleared");
            }
            other => {
                if let Some(p) = other.strip_prefix("add ") {
                    let p = p.trim();
                    if p.is_empty() {
                        self.set_status("usage: /notify add <text>");
                        return;
                    }
                    self.config.notify_patterns.push(p.to_string());
                    self.config.save();
                    self.apply_notify_patterns();
                    self.set_status(format!("watching output for \"{p}\""));
                } else {
                    self.set_status("usage: /notify [on|off|add <text>|clear]");
                }
            }
        }
    }
}
