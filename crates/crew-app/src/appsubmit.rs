//! What happens when you press Enter in the input bar: which pane the text is
//! for, whether it is a construct or keystrokes, and where it goes.
//!
//! Split out of [`crate::app`] for the line cap.
use crate::app::{bang_command, slash_command, star_command, submit_bytes, CrewApp};

impl CrewApp {
    /// Handle a submitted input line: `/command`s are run; everything else is
    /// written (with a newline) to the focused Terminal pane. Returns `true` if the
    /// app should exit (e.g. `/exit`).
    pub(crate) fn submit_input(&mut self, line: String) -> bool {
        if line.is_empty() {
            return false;
        }
        if let Some(cmd) = slash_command(&line) {
            return self.run_slash_command(cmd);
        }
        // `!cmd` runs a shell command in its own pane (like `/run`), regardless of
        // which pane is focused — a quick `ls`/`git status` without leaving the
        // agent pane you're driving.
        if let Some(cmd) = bang_command(&line) {
            if cmd.is_empty() {
                self.set_status("usage: !<command>");
            } else {
                self.run_in_pane(cmd);
            }
            return false;
        }
        // `*text` broadcasts one line to every terminal pane, explicitly — the
        // bar's replacement for depending on Cmd+S broadcast mode.
        if let Some(cmd) = star_command(&line) {
            if cmd.is_empty() {
                self.set_status("usage: *<text> — sends to every terminal");
            } else if self.write_terminal_targets(&submit_bytes(cmd), true) == 0 {
                self.set_status("no terminals to broadcast to");
            }
            return false;
        }
        // `??question` asks the AI to explain the focused pane's output; the
        // answer opens in the zoomed markdown viewer. Checked before `?` —
        // qmark_command would read `??x` as an ask for "?x".
        if let Some(question) = crate::askbar::explain_command(&line) {
            self.start_explain(question);
            return false;
        }
        // `?query` asks the AI for a shell command (à la Warp AI); the reply
        // lands back in the input bar, ready to edit or Enter.
        if let Some(query) = crate::askbar::qmark_command(&line) {
            if query.is_empty() {
                self.set_status("usage: ?<what you want> — ask ai for a command");
            } else {
                self.start_ask(query);
            }
            return false;
        }
        // `cd` in the input bar moves Crew's working directory, not the terminal's.
        if self.try_change_dir(&line) {
            return false;
        }
        match crate::route::route_bare(self.focused_target(), &self.check_command(&line)) {
            crate::route::BareRoute::TypeInto(_) => {
                // The focused idle shell receives the line as keystrokes.
                if self.write_terminal_targets(&submit_bytes(&line), false) == 0 {
                    self.set_status("no shell here — press Cmd+T to open one");
                }
            }
            crate::route::BareRoute::Spawn => self.run_in_pane(&line),
            crate::route::BareRoute::BuiltinHint(b) => {
                self.set_status(format!(
                    "{b} is a shell builtin — run it inside a shell pane"
                ));
            }
            crate::route::BareRoute::UnknownHint => {
                self.set_status(format!("not a command — !{line} runs it in a pane anyway"));
            }
        }
        false
    }

    /// The focused pane as routing sees it: `IdleShell` only for a visible
    /// terminal whose shell owns the prompt (`foreground_pid()` is `None`).
    /// Hidden panes are not "in the main area", so they never receive text.
    pub(crate) fn focused_target(&self) -> crate::route::Target {
        if let Some(p) = self.panes.get(self.focused) {
            if !p.hidden {
                if let crate::pane::PaneContent::Terminal(t) = &p.content {
                    if t.pty.foreground_pid().is_none() {
                        return crate::route::Target::IdleShell(self.focused);
                    }
                }
            }
        }
        crate::route::Target::Other
    }

    /// Resolve `line`'s first word, memoized — the palette preview re-checks on
    /// every keystroke and only the first word matters, so argument typing must
    /// not re-stat the PATH.
    pub(crate) fn check_command(&mut self, line: &str) -> crate::cmdcheck::Verdict {
        let word = crate::cmdcheck::first_word(line);
        if let (Some(w), Some((cached_w, v))) = (&word, &self.cmd_cache) {
            if w == cached_w {
                return v.clone();
            }
        }
        let v = crate::cmdcheck::resolve(line, &crate::cmdcheck::effective_path());
        if let Some(w) = word {
            self.cmd_cache = Some((w, v.clone()));
        }
        v
    }

    /// Set (or, when `name` is empty, clear) the focused pane's title override.
    pub(crate) fn name_focused_pane(&mut self, name: &str) {
        if let Some(p) = self.panes.get_mut(self.focused) {
            p.name = (!name.is_empty()).then(|| name.to_string());
            self.redraw();
        } else {
            self.set_status("no pane to name");
        }
    }
}
