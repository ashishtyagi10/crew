use std::io::Write;
use std::path::Path;

use crate::app::{CrewApp, FALLBACK_SIZE};
use crate::config::CrewConfig;
use crate::farpane::FarPane;
use crate::layout::Rect;
use crate::pane::{spawn_pane, Pane, PaneContent, TermPane};
use crate::settingspane::SettingsPane;
use crew_term::PtyTerm;

/// A zero rect; `build_frame`'s relayout assigns the real pane rect next frame.
pub(crate) const PLACEHOLDER_RECT: Rect = Rect {
    x: 0.0,
    y: 0.0,
    w: 0.0,
    h: 0.0,
};

/// The user's preferred shell from `$SHELL`, falling back to `/bin/sh`.
pub(crate) fn default_shell() -> String {
    std::env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/bin/sh".to_string())
}

/// Env vars handed to labeled (run/diff/edit) pane spawns: the login-shell
/// PATH commands were *detected* against. The `-c` wrapper shell is non-login,
/// so without this a Dock-launched Crew (launchd's minimal PATH) routes
/// `claude` to a pane that can't find it — and the fallback interactive shell
/// sources rc files under the same broken PATH, spraying "command not found".
pub(crate) fn hydrated_env() -> Vec<(String, String)> {
    vec![("PATH".to_string(), crate::cmdcheck::effective_path())]
}

impl CrewApp {
    /// The directory new terminals start in — Crew's tracked working directory,
    /// the same one shown in the input-bar legend and moved by `cd`. `None` only
    /// before it has been seeded (e.g. in tests), so the child inherits ours.
    pub(crate) fn spawn_cwd(&self) -> Option<&Path> {
        (!self.cwd.as_os_str().is_empty()).then_some(self.cwd.as_path())
    }

    /// Spawn a new terminal pane and focus it.
    pub fn spawn_new_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        let shell = default_shell();
        match spawn_pane(&shell, "/bin/sh", grid, self.spawn_cwd()) {
            Ok(pane) => {
                self.panes.push(pane);
                self.focus_new_pane();
                self.apply_notify_patterns();
            }
            // Surface the failure in the UI — stderr is invisible in the GUI.
            Err(e) => self.set_status(format!("couldn't open shell: {e}")),
        }
    }

    /// Spawn a labeled terminal pane running `command args` and focus it.
    pub fn spawn_labeled_terminal(&mut self, command: &str, args: &[String], label: String) {
        let cwd = self.spawn_cwd().map(std::path::Path::to_path_buf);
        self.spawn_labeled_terminal_in(command, args, label, cwd);
    }

    /// As [`Self::spawn_labeled_terminal`], but starts the pane in `cwd` (used by
    /// the Far command line to run in the active panel's directory). `None` falls
    /// back to the process's inherited directory.
    pub fn spawn_labeled_terminal_in(
        &mut self,
        command: &str,
        args: &[String],
        label: String,
        cwd: Option<std::path::PathBuf>,
    ) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        let env = hydrated_env();
        let env: Vec<(&str, &str)> = env.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        match PtyTerm::spawn_with_env(grid, command, args, cwd.as_deref(), &env) {
            Ok(pty) => {
                let input = pty.writer();
                // rect/grid are placeholders; build_frame's relayout sizes the pane
                // to the content area (right of the sidebar) on the next frame.
                let pane = Pane {
                    content: PaneContent::Terminal(Box::new(TermPane {
                        pty,
                        input,
                        cmd: None,
                        cmd_since: None,
                    })),
                    grid,
                    rect: PLACEHOLDER_RECT,
                    label: Some(label),
                    name: None,
                    dir: cwd,
                    activity: false,
                    bell: false,
                    hidden: false,
                    attention: None,
                };
                self.panes.push(pane);
                self.focus_new_pane();
                self.apply_notify_patterns();
                self.redraw();
            }
            // Surface the failure in the UI — stderr is invisible in the GUI.
            Err(e) => self.set_status(format!("couldn't run {command}: {e}")),
        }
    }

    /// Send `text + newline` to the pane labeled `label` (if Terminal).
    pub fn send_to_label(&mut self, label: &str, text: &str) {
        for pane in &mut self.panes {
            if pane.label.as_deref() == Some(label) {
                if let PaneContent::Terminal(t) = &mut pane.content {
                    if let Err(e) = t
                        .input
                        .write_all(text.as_bytes())
                        .and_then(|_| t.input.write_all(b"\n"))
                        .and_then(|_| t.input.flush())
                    {
                        eprintln!("send_to_label write error: {e}");
                    }
                }
                return;
            }
        }
    }

    /// Spawn a settings pane showing the app config and focus it.
    pub(crate) fn spawn_settings_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        let families = self
            .renderer
            .as_mut()
            .map(|r| r.monospace_families())
            .unwrap_or_default();
        self.panes.push(Pane {
            content: PaneContent::Settings(SettingsPane::new(self.config.clone(), families)),
            grid,
            rect: PLACEHOLDER_RECT,
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
        });
        self.focus_new_pane();
    }

    /// Spawn a Far dual-pane file-manager pane rooted at Crew's cwd, and focus it.
    pub(crate) fn spawn_far_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        let cwd = self
            .spawn_cwd()
            .map(Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_default();
        self.panes.push(Pane {
            content: PaneContent::Far(FarPane::new(cwd)),
            grid,
            rect: PLACEHOLDER_RECT,
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
        });
        self.focus_new_pane();
    }

    /// Plan `goal` into a task graph off-thread and run it in a swarm pane. An
    /// empty goal just shows a usage hint (no pane).
    pub(crate) fn spawn_goal_pane(&mut self, goal: &str) {
        let goal = goal.trim();
        if goal.is_empty() {
            self.set_status("usage: /goal <text>");
            return;
        }
        self.push_swarm_pane(crate::swarmpane::SwarmPane::for_goal(goal.to_string()));
    }

    /// Run a batch of jobs read from a file (one job per line) as an all-parallel
    /// swarm. An empty path shows a usage hint; an unreadable/empty file reports
    /// why instead of opening an empty pane.
    pub(crate) fn spawn_batch_pane(&mut self, path: &str) {
        let path = path.trim();
        if path.is_empty() {
            self.set_status("usage: /batch <file> (one job per line)");
            return;
        }
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                self.set_status(format!("batch: cannot read {path}: {e}"));
                return;
            }
        };
        let jobs = crate::swarmpane::jobs_from_lines(&text);
        if jobs.is_empty() {
            self.set_status(format!("batch: no jobs in {path}"));
            return;
        }
        let n = jobs.len();
        match crate::swarmpane::SwarmPane::for_batch(jobs) {
            Ok(swarm) => {
                self.push_swarm_pane(swarm);
                self.set_status(format!("batch: running {n} jobs"));
            }
            Err(e) => self.set_status(format!("batch: {e}")),
        }
    }

    /// Push a swarm pane into the grid and focus it.
    fn push_swarm_pane(&mut self, swarm: crate::swarmpane::SwarmPane) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        self.panes.push(Pane {
            content: PaneContent::Swarm(swarm),
            grid,
            rect: PLACEHOLDER_RECT,
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
        });
        self.focus_new_pane();
        self.redraw();
    }

    /// Apply updated config: set font family + size live, persist to disk, and redraw.
    pub(crate) fn apply_settings(&mut self, cfg: CrewConfig) {
        self.apply_config(cfg);
        self.config.save();
    }

    /// Adopt `cfg` and apply it live (font family/size to the renderer, and a
    /// redraw to pick up nav width/visibility) *without* writing it back — used
    /// by `apply_settings`, which then persists.
    pub(crate) fn apply_config(&mut self, cfg: CrewConfig) {
        let old_family = self.config.font_family.clone();
        self.config = cfg;
        // Apply theme selection: if the saved theme is a rotation mode name,
        // resume rotation in its pool (dark, light, or OS-following); if it's a
        // fixed theme name, pin that theme and stop rotation. This ensures a
        // theme chosen in the Settings pane isn't overridden by the rotation.
        match self
            .config
            .theme
            .as_deref()
            .and_then(crew_theme::parse_selection)
        {
            Some(sel) => crew_theme::apply_selection(sel, crate::chattime::unix_now_ms()),
            None => crew_theme::apply_selection(
                crew_theme::Selection::Fixed(self.config.theme_id()),
                crate::chattime::unix_now_ms(),
            ),
        }
        // Apply the themeable accent app-wide (render code reads it via palette).
        crate::palette::set_accent(self.config.accent_rgb());
        let scale = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0);
        if let Some(r) = &mut self.renderer {
            r.set_font_family(self.config.font_family.clone());
            r.set_font_size(self.config.font_size * scale);
            r.set_paper_texture(self.config.paper_texture);
            r.set_paper_grain(self.config.paper_grain);
        }
        // A manual family pick in Settings stops rotation; otherwise a live
        // rotation keeps its current pick on top of the re-applied config.
        if self.config.font_family != old_family {
            self.font_rotate.on = false;
            self.font_rotate.current = None;
            self.config.font_random = false;
        } else if let (true, Some(fam)) = (self.font_rotate.on, self.font_rotate.current.clone()) {
            if let Some(r) = &mut self.renderer {
                r.set_font_family(Some(fam));
            }
        }
        // Pick up any change to the watched notification patterns on live panes.
        self.apply_notify_patterns();
        self.redraw();
    }

    /// Set the font size (clamped to the config's valid range), applying it live
    /// and persisting — shared by the Cmd+= / Cmd+- / Cmd+0 zoom chords.
    pub(crate) fn set_font(&mut self, size: f32) {
        let mut cfg = self.config.clone();
        cfg.font_size = size;
        self.apply_settings(cfg.clamped());
        self.set_status(format!("font size {}", self.config.font_size as i32));
    }

    /// `/theme [<name>|random-dark|random-light|auto]`: switch the active
    /// theme live, persist the choice, and repaint. A mode name enters
    /// rotation (dark pool, light pool, or OS-appearance-following); any
    /// fixed theme name pins that theme and stops rotation. With no/unknown
    /// arg, report the current selection.
    pub(crate) fn set_theme_cmd(&mut self, arg: &str) {
        let arg = arg.trim();
        if arg.is_empty() {
            self.set_status(format!("theme: {}", crew_theme::selection_label()));
            return;
        }
        let Some(sel) = crew_theme::parse_selection(arg) else {
            let names = crew_theme::ALL_THEMES
                .iter()
                .map(|t| t.as_str())
                .chain(["random-dark", "random-light", "auto"])
                .collect::<Vec<_>>()
                .join(" | ");
            self.set_status(format!("unknown theme '{arg}' ({names})"));
            return;
        };
        crew_theme::apply_selection(sel, crate::chattime::unix_now_ms());
        self.config.theme = Some(
            match sel {
                crew_theme::Selection::Fixed(id) => id.as_str(),
                crew_theme::Selection::Mode(m) => m.as_str(),
            }
            .to_string(),
        );
        // Re-apply the accent default (it follows the theme when the user hasn't
        // set an explicit accent).
        crate::palette::set_accent(self.config.accent_rgb());
        self.config.save();
        self.redraw();
        self.set_status(format!("theme: {}", crew_theme::selection_label()));
    }
}

#[cfg(test)]
#[path = "spawn_tests.rs"]
mod tests;
