//! `/tab` — tabs by name: list them, open one, step, rename, and send the
//! focused pane to another. The chords (Cmd+T, Cmd+Shift+[ / ], Cmd+Opt+N)
//! reach the same [`crate::tabsapp`] actions.
use crate::app::CrewApp;

const USAGE: &str =
    "/tab [n|new|next|prev|rename <name>|move <n>] \u{b7} Cmd+T new \u{b7} Cmd+Shift+[ ] step";

impl CrewApp {
    /// Run `/tab <arg>`; a bare `/tab` lists the tabs.
    pub(crate) fn tab_command(&mut self, arg: &str) {
        let arg = arg.trim();
        let (verb, rest) = arg
            .split_once(' ')
            .map_or((arg, ""), |(v, r)| (v, r.trim()));
        match verb {
            "" => {
                let list = self.tab_summary();
                self.set_status(list);
            }
            "new" => self.new_tab(),
            "next" => self.step_tab(1),
            "prev" => self.step_tab(-1),
            "rename" | "name" => self.rename_tab(rest),
            "move" => match rest.parse::<usize>() {
                Ok(n) => self.move_pane_to_tab(n),
                Err(_) => self
                    .set_status("usage: /tab move <n> \u{2014} one past the last opens a new tab"),
            },
            v => match v.parse::<usize>() {
                Ok(n) => self.goto_tab(n),
                Err(_) => self.set_status(USAGE),
            },
        }
    }

    /// Name the tab on screen; an empty name goes back to its first pane's.
    pub(crate) fn rename_tab(&mut self, name: &str) {
        let id = self.tabs.active;
        if let Some(t) = self.tabs.get_mut(id) {
            t.name = (!name.is_empty()).then(|| name.to_string());
        }
        let shown = self.tab_title(id);
        self.set_status(format!("tab named {shown}"));
        self.redraw();
    }

    /// `1 build · 2 web*` — every tab, the one on screen starred.
    pub(crate) fn tab_summary(&self) -> String {
        let rows: Vec<String> = (self.tabs.list.iter().enumerate())
            .map(|(k, t)| {
                let star = if t.id == self.tabs.active { "*" } else { "" };
                format!("{} {}{star}", k + 1, self.tab_title(t.id))
            })
            .collect();
        format!("tabs: {}", rows.join(" \u{b7} "))
    }
}
