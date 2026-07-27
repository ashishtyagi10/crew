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
            "md" => self.spawn_md_pane(""),     // show usage hint
            "smith" | "crew" => self.spawn_crew_pane(), // /crew kept as an alias
            "settings" => self.spawn_settings_pane(),
            "shell" => self.spawn_new_pane(),
            // Reopen last session's shells (their cwds snapshot on quit).
            "restore" => self.restore_session(),
            // Self-update in the background: progress shows in the left-nav UPDATE
            // card; the new binary applies on /restart — Crew never restarts itself.
            "update" => self.start_update(),
            "clear" => self.clear_focused_scrollback(),
            "clearall" => self.clear_all_scrollback(),
            "clearlog" => self.clear_log(),
            "only" => self.close_other_panes(),
            "closeall" => self.close_all_panes(),
            "pwd" => self.copy_cwd(),
            "about" => self.spawn_about_pane(),
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
            "glass" => self.glass_command(""),
            "weight" => self.weight_command(""),
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
                } else if let Some(a) = other.strip_prefix("glass ") {
                    self.glass_command(a.trim());
                } else if let Some(w) = other.strip_prefix("weight ") {
                    self.weight_command(w.trim());
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

    /// Handle `/glass [off|low|medium|high]` and `/glass window [<pct>|off]`.
    ///
    /// The first form sets how frosted the pane cards are; the per-theme look
    /// is derived from the active theme (see `crew_theme::glass`), so light,
    /// dark and CRT each get their own treatment from this one knob. The
    /// second form makes the WINDOW itself translucent so the desktop shows
    /// through the page — text and pane fills stay solid either way.
    ///
    /// Bare `/glass` reports both. Persisted; applied on the next frame.
    pub(crate) fn glass_command(&mut self, arg: &str) {
        let arg = arg.trim();
        // `/glass window …` is the window-translucency form.
        if let Some(rest) = arg.strip_prefix("window") {
            return self.glass_window_command(rest.trim());
        }
        if arg.is_empty() {
            let pct = (self.config.window_opacity * 100.0).round() as i32;
            let level = self.config.glass_level().as_str();
            self.set_status(format!("glass: {level}, window {pct}% opaque"));
            return;
        }
        let Some(level) = crew_theme::GlassLevel::parse(arg) else {
            self.set_status("usage: /glass [off|low|medium|high] | /glass window <pct>");
            return;
        };
        self.config.glass = level.as_str().to_string();
        self.config.save();
        self.apply_glass();
        self.set_status(format!("glass: {}", level.as_str()));
        self.redraw();
    }

    /// `/glass window <pct>` — window translucency. `off`/`100` is opaque.
    fn glass_window_command(&mut self, arg: &str) {
        let opacity = match arg {
            "" => {
                let pct = (self.config.window_opacity * 100.0).round() as i32;
                self.set_status(format!("window {pct}% opaque — /glass window <pct>"));
                return;
            }
            "off" | "opaque" => 1.0,
            // A bare `on` picks a translucency that reads as glass without
            // making the text fight the wallpaper behind it.
            "on" => 0.85,
            s => {
                let Some(pct) = s
                    .trim_end_matches('%')
                    .parse::<f32>()
                    .ok()
                    .filter(|p| (0.0..=100.0).contains(p))
                else {
                    self.set_status("usage: /glass window <0-100>|on|off");
                    return;
                };
                pct / 100.0
            }
        };
        self.config.window_opacity = opacity.clamp(crate::config::MIN_WINDOW_OPACITY, 1.0);
        self.config.save();
        self.apply_glass();
        let pct = (self.config.window_opacity * 100.0).round() as i32;
        // Say when the floor overrode the request, rather than silently
        // ignoring a number the user typed.
        let msg = match opacity < crate::config::MIN_WINDOW_OPACITY {
            true => format!("window {pct}% opaque (floor — any sheerer and crew is unfindable)"),
            false => format!("window {pct}% opaque"),
        };
        self.set_status(msg);
        self.redraw();
    }

    /// Push both glass settings into the renderer.
    pub(crate) fn apply_glass(&mut self) {
        let level = self.config.glass_level();
        let opacity = self.config.window_opacity;
        if let Some(r) = &mut self.renderer {
            r.set_glass(level);
            r.set_window_opacity(opacity);
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

    // --- /glass -------------------------------------------------------------

    /// The last status message, for asserting what the user was told.
    fn status(app: &CrewApp) -> String {
        app.status.clone().map(|(m, _)| m).unwrap_or_default()
    }

    #[test]
    fn glass_sets_each_level() {
        let mut app = CrewApp::default();
        for name in ["off", "low", "medium", "high"] {
            app.glass_command(name);
            assert_eq!(app.config.glass, name);
            assert_eq!(app.config.glass_level().as_str(), name);
        }
    }

    #[test]
    fn glass_bad_arg_leaves_the_level_untouched() {
        let mut app = CrewApp::default();
        app.glass_command("high");
        app.glass_command("frosty");
        assert_eq!(app.config.glass, "high", "bad arg must not change glass");
        assert!(status(&app).starts_with("usage:"), "{}", status(&app));
    }

    #[test]
    fn bare_glass_reports_both_settings() {
        let mut app = CrewApp::default();
        app.glass_command("");
        let s = status(&app);
        assert!(s.contains("medium"), "{s}");
        assert!(s.contains("100%"), "{s}");
    }

    #[test]
    fn glass_window_accepts_percentages_and_words() {
        let mut app = CrewApp::default();
        app.glass_command("window 70");
        assert!((app.config.window_opacity - 0.70).abs() < 1e-6);
        // A trailing % is the natural way to type it.
        app.glass_command("window 60%");
        assert!((app.config.window_opacity - 0.60).abs() < 1e-6);
        app.glass_command("window off");
        assert_eq!(app.config.window_opacity, 1.0);
        app.glass_command("window on");
        assert!(app.config.window_opacity < 1.0);
    }

    /// The floor is the difference between a translucent window and a lost one.
    /// It must clamp AND say that it did, rather than quietly ignoring the ask.
    #[test]
    fn glass_window_floors_absurd_transparency() {
        let mut app = CrewApp::default();
        app.glass_command("window 0");
        assert_eq!(app.config.window_opacity, crate::config::MIN_WINDOW_OPACITY);
        assert!(status(&app).contains("floor"), "{}", status(&app));
    }

    #[test]
    fn glass_window_rejects_out_of_range_and_nonsense() {
        let mut app = CrewApp::default();
        app.glass_command("window 55");
        app.glass_command("window 900");
        app.glass_command("window -5");
        app.glass_command("window clear");
        assert!(
            (app.config.window_opacity - 0.55).abs() < 1e-6,
            "a rejected value must not move the setting"
        );
    }

    /// `/glass window` and `/glass <level>` are separate knobs; setting one must
    /// not disturb the other.
    #[test]
    fn glass_level_and_window_are_independent() {
        let mut app = CrewApp::default();
        app.glass_command("low");
        app.glass_command("window 80");
        assert_eq!(app.config.glass, "low");
        assert!((app.config.window_opacity - 0.80).abs() < 1e-6);
        app.glass_command("high");
        assert!(
            (app.config.window_opacity - 0.80).abs() < 1e-6,
            "changing the frost level moved the window opacity"
        );
    }

    /// An unreadable config value must not render the app flat with no
    /// explanation — it falls back to the default strength.
    #[test]
    fn unknown_configured_level_falls_back() {
        let mut app = CrewApp::default();
        app.config.glass = "chunky".to_string();
        assert_eq!(app.config.glass_level(), crew_theme::GlassLevel::Medium);
    }
}
