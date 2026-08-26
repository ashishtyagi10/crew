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
            "model" => self.set_model_cmd(""),  // show usage hint
            "batch" => self.spawn_batch_pane(""), // show usage hint
            "md" | "view" => self.open_view(""), // show usage hint
            "smith" | "crew" => self.spawn_crew_pane(), // /crew kept as an alias
            "settings" => self.spawn_settings_pane(),
            "todo" => self.spawn_todo_pane(),
            "shell" => self.spawn_new_pane(),
            // Reopen last session's shells (their cwds snapshot on quit).
            "restore" => self.restore_session(),
            // Self-update in the background: progress shows in the left-nav UPDATE
            // card; once the install lands, Crew restarts itself into the new
            // build (an already-parked install restarts immediately).
            "update" => return self.start_update(),
            // Absorbed into /update (which now restarts after installing). A
            // bare status beats the fuzzy matcher here, which would otherwise
            // suggest /restore — a different action entirely.
            "restart" => self.set_status("/restart merged into /update — it installs and restarts"),
            "clear" => self.clear_focused_scrollback(),
            "clearall" => self.clear_all_scrollback(),
            "clearlog" => self.clear_log(),
            "only" => self.close_other_panes(),
            "closeall" => self.close_all_panes(),
            "pwd" => self.copy_cwd(),
            "about" => self.spawn_about_pane(),
            "log" => self.open_log(),
            "copy" => self.copy_scrollback(),
            "dump" => self.dump_focused_pane(""),
            "diff" => self.diff_in_pane(),
            "run" => self.run_in_pane(""), // show usage hint
            "font" => self.set_font_cmd(""),
            "theme" => self.set_theme_cmd(""),
            "crt" => self.crt_command(""),
            "weight" => self.weight_command(""),
            "smooth" => self.smooth_command(""),
            "gradient" => self.gradient_command(""),
            "notify" => self.notify_command(""),
            "broadcast" => self.toggle_broadcast(),
            "zoom" => self.toggle_zoom(),
            "sidebar" => self.toggle_sidebar(),
            "name" => self.name_focused_pane(""), // clear the pane's name
            "findall" => self.find_all(""),       // show usage hint
            other => {
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
                    self.open_view(f.trim());
                } else if let Some(f) = other.strip_prefix("view ") {
                    self.open_view(f.trim());
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
                } else if let Some(g) = other.strip_prefix("gradient ") {
                    self.gradient_command(g.trim());
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
        false
    }

    /// Handle `/todo done [@project]` (the done-history view, optionally
    /// pre-filtered) and `/todo show` / `/todo hide` (done items inline on
    /// the active list). Bare `/todo` (the active list) dispatches directly
    /// and never reaches here.
    pub(crate) fn todo_command(&mut self, arg: &str) {
        let mut words = arg.split_whitespace();
        let usage = "usage: /todo [done [@project] | show | hide]";
        match (words.next(), words.next(), words.next()) {
            (Some(verb @ ("show" | "hide")), None, None) => self.todo_show_done(verb == "show"),
            (Some("done"), tag, None) => {
                let filter = match tag {
                    None => None,
                    Some(t) => match t.strip_prefix('@').filter(|t| !t.is_empty()) {
                        Some(t) => Some(t.to_string()),
                        None => {
                            self.set_status(usage);
                            return;
                        }
                    },
                };
                self.spawn_todo_pane_done(filter);
            }
            _ => self.set_status(usage),
        }
    }

    /// `/todo show` / `/todo hide`: flip done items on or off in a todo
    /// pane's list — the typed way to the header's `[show N done]` button.
    /// It acts on the focused todo pane, else the most recent one, and
    /// spawns a list if none is open (so the command works from a cold
    /// start, like `/todo done` does).
    fn todo_show_done(&mut self, show: bool) {
        let is_todo =
            |p: &crate::pane::Pane| matches!(p.content, crate::pane::PaneContent::Todo(_));
        let target = Some(self.focused)
            .filter(|&i| self.panes.get(i).is_some_and(is_todo))
            .or_else(|| self.panes.iter().rposition(is_todo));
        let i = match target {
            Some(i) => i,
            None => {
                self.spawn_todo_pane();
                self.panes.len() - 1
            }
        };
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

    /// Push both glass settings into the renderer, and keep the window's own
    /// opaque flag in step with them.
    ///
    /// The window half is not optional bookkeeping: the renderer only controls
    /// the alpha of the frame *crew* draws, while the OS-drawn title bar
    /// composites against `NSWindow.isOpaque`. Setting one without the other is
    /// what left the title bar see-through at full opacity.
    pub(crate) fn apply_glass(&mut self) {
        let level = self.config.glass_level();
        let opacity = self.config.window_opacity;
        if let Some(r) = &mut self.renderer {
            r.set_glass(level);
            r.set_window_opacity(opacity);
        }
        if let Some(w) = &self.window {
            w.set_transparent(crate::config::wants_window_transparency(opacity));
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
                let next = self.effective_crt().is_none();
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
                if self.effective_crt().is_some() {
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

    /// Handle `/weight [<name>|<300-900>]`: set the base text weight so the font
    /// reads thicker or lighter. Accepts named steps (thin/normal/medium/
    /// semibold/bold/black) or a raw CSS number. Bare `/weight` reports the
    /// current value. Persisted and applied live.
    pub(crate) fn weight_command(&mut self, arg: &str) {
        let named = |a: &str| -> Option<u16> {
            Some(match a {
                "thin" | "light" => 300,
                "normal" | "regular" => 400,
                "medium" => 500,
                "semibold" | "semi" => 600,
                "bold" => 700,
                "black" | "heavy" => 900,
                _ => return None,
            })
        };
        let weight = match arg {
            "" => {
                self.set_status(format!(
                    "font weight {} (/weight [thin|normal|medium|semibold|bold|black|<300-900>])",
                    self.config.font_weight
                ));
                return;
            }
            a => match named(a).or_else(|| a.parse::<u16>().ok()) {
                Some(w) => w.clamp(300, 900),
                None => {
                    self.set_status(
                        "usage: /weight [thin|normal|medium|semibold|bold|black|<300-900>]",
                    );
                    return;
                }
            },
        };
        self.config.font_weight = weight;
        self.config.save();
        if let Some(r) = &mut self.renderer {
            r.set_font_weight(Some(weight));
        }
        self.set_status(format!("font weight {weight}"));
        self.redraw();
    }

    /// Handle `/smooth [off|light|medium|heavy|<0-255>]`: set the CoreText-style
    /// font smoothing strength (stem darkening — how full the glyphs read).
    /// Bare `/smooth` reports the current value. Persisted and applied live.
    /// The keyword ladder is `smoothlvl` — shared with the Settings form's
    /// Smoothing picker, so the two surfaces can never disagree.
    pub(crate) fn smooth_command(&mut self, arg: &str) {
        let named = crate::smoothlvl::strength_of;
        let strength = match arg {
            "" => {
                self.set_status(format!(
                    "font smoothing {} (/smooth [off|light|medium|heavy|<0-255>])",
                    self.config.font_smooth
                ));
                return;
            }
            a => match named(a).or_else(|| a.parse::<u16>().ok().map(|s| s.min(255) as u8)) {
                Some(s) => s,
                None => {
                    self.set_status("usage: /smooth [off|light|medium|heavy|<0-255>]");
                    return;
                }
            },
        };
        self.config.font_smooth = strength;
        self.config.save();
        if let Some(r) = &mut self.renderer {
            r.set_text_smoothing(Some(strength));
        }
        self.set_status(format!("font smoothing {strength}"));
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
        let before = app.effective_crt().is_some();
        app.crt_command("");
        assert_eq!(app.config.crt, Some(!before));
        app.crt_command("");
        assert_eq!(app.config.crt, Some(before));
    }

    /// An unrecognised command must SAY so. It used to fall through in
    /// silence, which looked exactly like a command that ran and had nothing
    /// to report — and would have hidden a palette row whose dispatch arm was
    /// deleted.
    #[test]
    fn an_unknown_command_says_so_and_guesses() {
        let mut app = CrewApp::default();
        app.run_slash_command("setings");
        let s = app.status.clone().expect("a status was set").0;
        assert!(s.contains("unknown command /setings"), "{s}");
        assert!(s.contains("/settings"), "a near miss should be named: {s}");

        app.run_slash_command("wobblefish");
        let s = app.status.clone().expect("a status was set").0;
        assert!(s.contains("unknown command /wobblefish"), "{s}");
        assert!(!s.contains("did you mean"), "no guess from nonsense: {s}");
    }

    /// `/restart` is gone (merged into `/update`), but typing it must teach,
    /// not fall through to the fuzzy matcher — which would suggest /restore,
    /// a different action entirely.
    #[test]
    fn restart_is_a_migration_stub_pointing_at_update() {
        let mut app = CrewApp::default();
        let exit = app.run_slash_command("restart");
        assert!(!exit, "the stub must not exit or relaunch anything");
        let s = app.status.clone().expect("a status was set").0;
        assert!(s.contains("/update"), "{s}");
        assert!(
            !s.contains("unknown"),
            "not an unknown command, a merged one: {s}"
        );
    }

    #[test]
    fn crt_unknown_arg_leaves_state_untouched() {
        let mut app = CrewApp::default();
        app.crt_command("on");
        app.crt_command("wobble");
        assert_eq!(app.config.crt, Some(true), "bad arg must not change state");
    }

    #[test]
    fn weight_defaults_to_semibold_and_named_steps_set_it() {
        let mut app = CrewApp::default();
        assert_eq!(app.config.font_weight, 600, "SemiBold out of the box");
        app.weight_command("bold");
        assert_eq!(app.config.font_weight, 700);
        app.weight_command("medium");
        assert_eq!(app.config.font_weight, 500);
        app.weight_command("black");
        assert_eq!(app.config.font_weight, 900);
    }

    #[test]
    fn weight_accepts_a_raw_number_clamped_to_range() {
        let mut app = CrewApp::default();
        app.weight_command("650");
        assert_eq!(app.config.font_weight, 650);
        app.weight_command("5000"); // clamps
        assert_eq!(app.config.font_weight, 900);
    }

    #[test]
    fn weight_bad_arg_leaves_it_untouched() {
        let mut app = CrewApp::default();
        app.weight_command("bold");
        app.weight_command("chunky");
        assert_eq!(
            app.config.font_weight, 700,
            "bad arg must not change weight"
        );
    }

    #[test]
    fn smooth_defaults_on_and_named_steps_set_it() {
        let mut app = CrewApp::default();
        assert_eq!(
            app.config.font_smooth,
            crew_render::DEFAULT_SMOOTH,
            "smoothing on out of the box"
        );
        app.smooth_command("off");
        assert_eq!(app.config.font_smooth, 0);
        app.smooth_command("heavy");
        assert_eq!(app.config.font_smooth, 170);
        app.smooth_command("light");
        assert_eq!(app.config.font_smooth, 60);
        app.smooth_command("medium");
        assert_eq!(app.config.font_smooth, crew_render::DEFAULT_SMOOTH);
    }

    #[test]
    fn smooth_accepts_a_raw_number_clamped_to_a_byte() {
        let mut app = CrewApp::default();
        app.smooth_command("42");
        assert_eq!(app.config.font_smooth, 42);
        app.smooth_command("9000"); // clamps
        assert_eq!(app.config.font_smooth, 255);
    }

    /// A Settings-form save routes its config through `apply_settings`; the
    /// smoothing it carries must land on `app.config` — the key `/smooth`
    /// then reads — or the form's picker would look editable while changing
    /// nothing. Fails if the apply path (or `clamped()`) drops `font_smooth`.
    #[test]
    fn settings_apply_adopts_the_forms_smoothing() {
        let _g = crate::app::theme_test_guard();
        let mut app = CrewApp::default();
        let mut pane = crate::settingspane::SettingsPane::new(app.config.clone(), Vec::new());
        pane.focus = crate::settingspane::FIELDS
            .iter()
            .position(|&f| f == crate::settingspane::Field::Smooth)
            .unwrap();
        crate::settingspane::cycle_value(&mut pane, false); // medium → heavy
        let crate::settingspane::SettingsAction::Apply(cfg) = pane.save() else {
            panic!("save must apply");
        };
        app.apply_settings(*cfg);
        assert_eq!(app.config.font_smooth, 170);
        app.smooth_command("");
        let s = app.active_status().unwrap();
        assert!(s.contains("170"), "/smooth reports the form's value: {s}");
    }

    #[test]
    fn smooth_bad_arg_leaves_it_untouched() {
        let mut app = CrewApp::default();
        app.smooth_command("170");
        app.smooth_command("glassy");
        assert_eq!(
            app.config.font_smooth, 170,
            "bad arg must not change smoothing"
        );
    }

    // --- glass ----------------------------------------------------------------

    /// An unreadable config value must not render the app flat with no
    /// explanation — it falls back to the default strength.
    #[test]
    fn unknown_configured_level_falls_back() {
        let mut app = CrewApp::default();
        app.config.glass = "chunky".to_string();
        assert_eq!(app.config.glass_level(), crew_theme::GlassLevel::Medium);
    }

    // --- /todo done -----------------------------------------------------------

    fn last_todo(app: &CrewApp) -> &crate::todopane::TodoPane {
        match &app.panes.last().expect("a pane spawned").content {
            crate::pane::PaneContent::Todo(t) => t,
            _ => panic!("expected a todo pane"),
        }
    }

    #[test]
    fn todo_done_opens_the_history_view_with_an_optional_filter() {
        let _g = crate::todopane::store::test_guard(vec![]);
        let mut app = CrewApp::default();
        app.run_slash_command("todo");
        assert!(!last_todo(&app).done_view, "bare /todo is the active list");

        app.run_slash_command("todo done");
        let t = last_todo(&app);
        assert!(t.done_view, "/todo done opens the history");
        assert_eq!(t.filter, None);

        app.run_slash_command("todo done @crew");
        let t = last_todo(&app);
        assert!(t.done_view);
        assert_eq!(t.filter.as_deref(), Some("crew"), "the arg pre-filters");
    }

    /// `/todo show` / `/todo hide` are the typed half of the header button —
    /// they act on the todo pane you are looking at and say what happened.
    #[test]
    fn todo_show_and_hide_flip_the_done_items_on_the_open_pane() {
        let _g = crate::todopane::store::test_guard(vec![]);
        let mut app = CrewApp::default();
        app.run_slash_command("todo");
        let opened = app.panes.len();

        app.run_slash_command("todo show");
        assert_eq!(app.panes.len(), opened, "reuses the open list, no new pane");
        assert!(last_todo(&app).show_done, "/todo show reveals them");

        app.run_slash_command("todo hide");
        assert!(!last_todo(&app).show_done, "/todo hide puts them back");
        let s = app.status.clone().expect("a status was set").0;
        assert!(s.contains("done items hidden"), "{s}");
    }

    /// From a cold start it opens the list first — the command works before
    /// any todo pane exists, the way `/todo done` does.
    #[test]
    fn todo_show_spawns_a_list_when_none_is_open() {
        let _g = crate::todopane::store::test_guard(vec![]);
        let mut app = CrewApp::default();
        app.run_slash_command("todo show");
        let t = last_todo(&app);
        assert!(t.show_done);
        assert!(!t.done_view, "the list, not the history");
    }

    /// The history is done-only; asking to show done items there means "put
    /// me back on the list with them in it", not "nothing to do".
    #[test]
    fn todo_show_walks_back_out_of_the_history_view() {
        let _g = crate::todopane::store::test_guard(vec![]);
        let mut app = CrewApp::default();
        app.run_slash_command("todo done");
        assert!(last_todo(&app).done_view);
        app.run_slash_command("todo show");
        let t = last_todo(&app);
        assert!(!t.done_view, "left the log");
        assert!(t.show_done, "with the done items on the list");
    }

    #[test]
    fn a_bad_todo_arg_teaches_the_usage_instead_of_spawning() {
        let _g = crate::todopane::store::test_guard(vec![]);
        let mut app = CrewApp::default();
        let before = app.panes.len();
        app.run_slash_command("todo wobble");
        assert_eq!(app.panes.len(), before, "no pane from a bad arg");
        let s = app.status.clone().expect("a status was set").0;
        assert!(s.contains("usage: /todo"), "{s}");
    }
}
