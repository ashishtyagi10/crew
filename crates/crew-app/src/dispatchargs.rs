//! The `<arg>` forms of the slash commands — one `else if let` chain, and the
//! "no such command" answer at the end of it.
//!
//! Split out of [`crate::dispatch`] for the line cap, and moved whole rather
//! than divided by subject: the chain's ORDER is load-bearing (`find ` must be
//! tried after `findall `, or `/findall x` routes to `/find`), and an order
//! spread across files is one nobody can read.
use crate::app::CrewApp;

impl CrewApp {
    /// Run a slash command that carries an argument, or say no such command
    /// exists — with the closest match, when there is one.
    pub(crate) fn run_slash_with_arg(&mut self, other: &str) {
        if let Some(a) = other.strip_prefix("todo ") {
            self.todo_command(a.trim());
        } else if let Some(term) = other.strip_prefix("findall ") {
            self.find_all(term);
        } else if let Some(term) = other.strip_prefix("find ") {
            self.find_in_terminal(term.trim());
        } else if let Some(n) = other.strip_prefix("name ") {
            self.name_focused_pane(n.trim());
        } else if let Some(c) = other.strip_prefix("run ") {
            self.run_in_pane(c);
        } else if let Some(f) = other.strip_prefix("dump ") {
            self.dump_focused_pane(f);
        } else if let Some(n) = other.strip_prefix("font ") {
            self.set_font_cmd(n);
        } else if let Some(g) = other.strip_prefix("goal ") {
            self.spawn_goal_pane(g.trim());
        } else if let Some(f) = other.strip_prefix("batch ") {
            self.spawn_batch_pane(f.trim());
        } else if let Some(f) = other.strip_prefix("md ") {
            // `/md` is the markdown-shaped door: a document, in a
            // window, where it can be edited. `/view` keeps opening a
            // pane, which is what Cmd+click on a path, `/diff` and
            // `/out` all want.
            self.queue_doc_window(f.trim());
        } else if let Some(f) = other.strip_prefix("view ") {
            self.open_view(f.trim());
        } else if let Some(f) = other.strip_prefix("doc ") {
            // The window is opened on the next tick: only a winit
            // callback holding the ACTIVE event loop can make one,
            // and a command dispatch is not one (see `docwin`).
            self.queue_doc_window(f.trim());
        } else if let Some(n) = other.strip_prefix("notify ") {
            self.notify_command(n.trim());
        } else if let Some(t) = other.strip_prefix("theme ") {
            self.set_theme_cmd(t.trim());
        } else if let Some(a) = other.strip_prefix("crt ") {
            self.crt_command(a.trim());
        } else if let Some(w) = other.strip_prefix("weight ") {
            self.weight_command(w.trim());
        } else if let Some(s) = other.strip_prefix("smooth ") {
            self.smooth_command(s.trim());
        } else if let Some(s) = other.strip_prefix("gamma ") {
            self.gamma_command(s.trim());
        } else if let Some(f) = other.strip_prefix("tools ") {
            self.open_tools(f.trim());
        } else if let Some(a) = other.strip_prefix("watching ") {
            self.open_watching(a.trim());
        } else if let Some(n) = other.strip_prefix("out ") {
            self.open_last_output(n.trim());
        } else if let Some(m) = other.strip_prefix("marks ") {
            self.marks_command(m.trim());
        } else if let Some(m) = other.strip_prefix("motion ") {
            self.motion_command(m.trim());
        } else if let Some(d) = other.strip_prefix("density ") {
            self.density_command(d.trim());
        } else if let Some(v) = other.strip_prefix("invisibles ") {
            self.invisibles_command(v.trim());
        } else if let Some(l) = other.strip_prefix("leading ") {
            self.leading_command(l.trim());
        } else if let Some(c) = other.strip_prefix("contrast ") {
            self.contrast_command(c.trim());
        } else if let Some(v) = other.strip_prefix("shapes ") {
            self.shapes_command(v.trim());
        } else if let Some(g) = other.strip_prefix("gradient ") {
            self.gradient_command(g.trim());
        } else if let Some(o) = other.strip_prefix("opacity ") {
            self.opacity_command(o.trim());
        } else if let Some(g) = other.strip_prefix("grain ") {
            self.grain_command(g.trim());
        } else if let Some(m) = other.strip_prefix("model ") {
            self.set_model_cmd(m.trim());
        } else {
            // Nothing matched. This used to fall through in silence,
            // so a typo did nothing and looked exactly like a command
            // that ran and had nothing to say — and a palette row
            // whose arm was deleted would have looked the same. The
            // broker has always answered "unknown construct"; the app
            // owes the same courtesy.
            let hint = crate::suggest::closest_command(other);
            self.set_status(match hint {
                Some(c) => format!("unknown command /{other} — did you mean {c}?"),
                None => format!("unknown command /{other}"),
            });
        }
    }
}
