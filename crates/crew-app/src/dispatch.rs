//! Slash-command dispatch: maps a `/command` (and its `<arg>` forms) typed in
//! the input bar to the matching `CrewApp` action. Kept in sync with the palette
//! list in `suggest::COMMANDS`.
use crate::app::CrewApp;

impl CrewApp {
    /// Run a `/command` typed in the input bar. Returns `true` if the app should exit.
    pub(crate) fn run_slash_command(&mut self, cmd: &str) -> bool {
        match cmd {
            "exit" => return true,
            "keys" => self.help_open = true,
            "far" => self.spawn_far_pane(),
            "goal" => self.spawn_goal_pane(""), // show usage hint
            "batch" => self.spawn_batch_pane(""), // show usage hint
            "md" => self.spawn_md_pane(""),     // show usage hint
            "smith" | "crew" => self.spawn_crew_pane(), // /crew kept as an alias
            "settings" => self.spawn_settings_pane(),
            "shell" => self.spawn_new_pane(),
            // Reopen last session's shells (their cwds snapshot on quit).
            "restore" => self.restore_session(),
            // Self-update in the background: progress shows in the left-nav UPDATE
            // card and Crew auto-restarts into the new build — no separate shell.
            "update" => self.start_update(),
            "clear" => self.clear_focused_scrollback(),
            "clearall" => self.clear_all_scrollback(),
            "clearlog" => self.clear_log(),
            "only" => self.close_other_panes(),
            "closeall" => self.close_all_panes(),
            "pwd" => self.copy_cwd(),
            "about" => self.set_status(concat!("crew v", env!("CARGO_PKG_VERSION"))),
            "copy" => self.copy_scrollback(),
            "dump" => self.dump_focused_pane(""),
            "diff" => self.diff_in_pane(),
            "run" => self.run_in_pane(""), // show usage hint
            "font" => self.set_font_cmd(""),
            // Relaunch as a fresh detached process (picks up an installed
            // `/update` and external config edits) and exit this one.
            "restart" => return self.restart_crew(),
            "theme" => self.set_theme_cmd(""),
            "crt" => self.crt_command(""),
            "notify" => self.notify_command(""),
            "broadcast" => self.toggle_broadcast(),
            "zoom" => self.toggle_zoom(),
            "sidebar" => self.toggle_sidebar(),
            "name" => self.name_focused_pane(""), // clear the pane's name
            "findall" => self.find_all(""),       // show usage hint
            other => {
                if let Some(term) = other.strip_prefix("findall ") {
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
                    self.spawn_md_pane(f.trim());
                } else if let Some(n) = other.strip_prefix("notify ") {
                    self.notify_command(n.trim());
                } else if let Some(t) = other.strip_prefix("theme ") {
                    self.set_theme_cmd(t.trim());
                } else if let Some(a) = other.strip_prefix("crt ") {
                    self.crt_command(a.trim());
                }
            }
        }
        false
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

    /// Handle `/crt [on|off|auto]`: force the CRT tube post-process on or off, or
    /// (`auto`) follow the theme's own `crt` flag. Bare `/crt` toggles the
    /// current effective state into an explicit override. Persisted; the
    /// renderer reads the effective state every frame, so a redraw applies it.
    pub(crate) fn crt_command(&mut self, arg: &str) {
        let msg = match arg {
            "" => {
                // Toggle: pin the opposite of what's showing now.
                let next = !self.effective_crt();
                self.config.crt = Some(next);
                if next {
                    "CRT on"
                } else {
                    "CRT off"
                }
            }
            "on" => {
                self.config.crt = Some(true);
                "CRT on"
            }
            "off" => {
                self.config.crt = Some(false);
                "CRT off"
            }
            "auto" => {
                self.config.crt = None;
                if self.effective_crt() {
                    "CRT auto (on for this theme)"
                } else {
                    "CRT auto (off for this theme)"
                }
            }
            _ => {
                self.set_status("usage: /crt [on|off|auto]");
                return;
            }
        };
        self.config.save();
        self.set_status(msg);
        self.redraw();
    }
}

#[cfg(test)]
mod tests {
    use crate::app::CrewApp;

    #[test]
    fn notify_off_then_on_toggles_the_master_switch() {
        let mut app = CrewApp::default();
        assert!(app.config.notify);
        app.notify_command("off");
        assert!(!app.config.notify);
        app.notify_command("on");
        assert!(app.config.notify);
    }

    #[test]
    fn notify_add_appends_a_pattern_then_clear_empties() {
        let mut app = CrewApp::default();
        app.notify_command("add error");
        assert_eq!(app.config.notify_patterns, vec!["error".to_string()]);
        app.notify_command("clear");
        assert!(app.config.notify_patterns.is_empty());
    }

    #[test]
    fn notify_add_without_text_adds_nothing() {
        let mut app = CrewApp::default();
        app.notify_command("add    ");
        assert!(app.config.notify_patterns.is_empty());
    }

    #[test]
    fn crt_on_off_auto_set_the_override() {
        let mut app = CrewApp::default();
        assert_eq!(app.config.crt, None, "defaults to following the theme");
        app.crt_command("on");
        assert_eq!(app.config.crt, Some(true));
        app.crt_command("off");
        assert_eq!(app.config.crt, Some(false));
        app.crt_command("auto");
        assert_eq!(app.config.crt, None);
    }

    #[test]
    fn bare_crt_toggles_the_effective_state() {
        let mut app = CrewApp::default();
        // A paper theme is CRT-off by default, so the first bare toggle pins on.
        let before = app.effective_crt();
        app.crt_command("");
        assert_eq!(app.config.crt, Some(!before));
        app.crt_command("");
        assert_eq!(app.config.crt, Some(before));
    }

    #[test]
    fn crt_unknown_arg_leaves_state_untouched() {
        let mut app = CrewApp::default();
        app.crt_command("on");
        app.crt_command("wobble");
        assert_eq!(app.config.crt, Some(true), "bad arg must not change state");
    }
}
