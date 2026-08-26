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

/// The user's preferred shell from `$SHELL`, falling back to
/// [`fallback_shell`]. `$SHELL` is a Unix convention and is normally unset on
/// Windows, so there it is an opt-in override (a Git-Bash user can point it at
/// their own shell) rather than the usual answer.
pub(crate) fn default_shell() -> String {
    std::env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(fallback_shell)
}

/// The shell to use when the user has expressed no preference, and the
/// last-resort second try in [`crate::pane::spawn_pane`].
#[cfg(unix)]
pub(crate) fn fallback_shell() -> String {
    "/bin/sh".to_string()
}

/// Windows has no `/bin/sh`: without this every pane failed to open with
/// "couldn't open shell", which is what made the platform build-but-not-run.
/// `%COMSPEC%` is set on every Windows install and names `cmd.exe`; the
/// literal is only for a stripped environment where even that is missing.
#[cfg(windows)]
pub(crate) fn fallback_shell() -> String {
    std::env::var("COMSPEC")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "cmd.exe".to_string())
}

/// The shell a new terminal pane opens first. On Windows this is PowerShell —
/// what Windows Terminal opens and what the platform's users expect — with
/// [`fallback_shell`]'s `cmd.exe` catching the (essentially impossible) host
/// with no PowerShell. On Unix the user's `$SHELL` is already the right answer.
#[cfg(windows)]
pub(crate) fn preferred_shell() -> String {
    std::env::var("SHELL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "powershell.exe".to_string())
}

#[cfg(unix)]
pub(crate) fn preferred_shell() -> String {
    default_shell()
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
        let shell = preferred_shell();
        match spawn_pane(&shell, &fallback_shell(), grid, self.spawn_cwd()) {
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
                        tail: Default::default(),
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
                    born_ms: crate::anim::now_ms(),
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
            born_ms: crate::anim::now_ms(),
        });
        self.focus_new_pane();
    }

    /// Spawn the todo-list pane (the shared store loads on open) and focus it.
    pub(crate) fn spawn_todo_pane(&mut self) {
        let grid = self
            .renderer
            .as_ref()
            .map(Self::current_grid)
            .unwrap_or(FALLBACK_SIZE);
        self.panes.push(Pane {
            content: PaneContent::Todo(crate::todopane::TodoPane::new()),
            grid,
            rect: PLACEHOLDER_RECT,
            label: None,
            name: None,
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
            attention: None,
            born_ms: crate::anim::now_ms(),
        });
        self.focus_new_pane();
    }

    /// Spawn a todo pane already open on the done-history view (`/todo
    /// done`), optionally pre-filtered to one `@project`.
    pub(crate) fn spawn_todo_pane_done(&mut self, filter: Option<String>) {
        self.spawn_todo_pane();
        if let Some(PaneContent::Todo(t)) = self.panes.last_mut().map(|p| &mut p.content) {
            t.filter = filter;
            t.set_done_view(true);
        }
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
            born_ms: crate::anim::now_ms(),
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
            born_ms: crate::anim::now_ms(),
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
        let old_pools = self.config.auto_pool_selections();
        self.config = cfg;
        // Apply theme selection: if the saved theme is a rotation mode name,
        // resume rotation in its pool (dark, light, or OS-following); if it's a
        // fixed theme name, pin that theme and stop rotation. This ensures a
        // theme chosen in the Settings pane isn't overridden by the rotation.
        // Reconcile ONLY when the config's selection differs from what's live.
        // `apply_selection(Mode(..))` re-picks a theme and restarts the
        // 10-minute clock, and every Settings save, `/theme`, and Cmd+= zoom
        // routes through `apply_settings` → here — so applying it
        // unconditionally re-rolled the theme on config touches that had
        // nothing to do with themes, and the rotation's own clock could never
        // run out. (It also made rotation look livelier than it is, masking
        // that the font rotation beside it has no such path.)
        // Auto's per-appearance pairing rides config too: republish it, and
        // if it changed while auto is the live mode, force a re-apply below —
        // "live" would otherwise be true (still Mode(Auto)) and a config edit
        // pairing night with the CRT pool wouldn't show until the next OS
        // flip or rotation tick.
        let (pool_dark, pool_light) = self.config.auto_pool_selections();
        let pools_changed = (pool_dark, pool_light) != old_pools;
        crew_theme::set_auto_pools(pool_dark, pool_light);
        // The light-hours window (`auto_light_from`/`auto_light_to`) rides
        // config the same way, so republish the clock sources here too and
        // treat a flipped verdict exactly like a changed pairing — otherwise
        // widening the window to cover right now wouldn't show until the next
        // tick crossed a boundary that no longer exists.
        let was_auto_dark = crew_theme::auto_dark();
        self.config.publish_appearance_sources();
        let auto_side_changed = pools_changed || crew_theme::auto_dark() != was_auto_dark;
        let want = self.config.theme_selection();
        let live = match want {
            crew_theme::Selection::Mode(m) => {
                crew_theme::mode() == Some(m)
                    && !(m == crew_theme::RandomMode::Auto && auto_side_changed)
            }
            crew_theme::Selection::Fixed(id) => {
                crew_theme::mode().is_none() && crew_theme::current_id() == id
            }
        };
        if !live {
            crew_theme::apply_selection(want, crate::chattime::unix_now_ms());
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
            r.set_font_weight(Some(self.config.font_weight));
            r.set_text_smoothing(Some(self.config.font_smooth));
            r.set_paper_texture(self.config.paper_texture);
            r.set_paper_grain(self.config.paper_grain);
        }
        // Glass rides the same path: the settings form is the only place these
        // two are set, so a save that didn't push them would leave the sheet
        // and the window opacity a restart behind.
        self.apply_glass();
        crate::motion::set_level(self.config.motion_level());
        // The gradient level rides the same path. Turning it OFF must also
        // put the poles back where they were — the shift is a live global,
        // and left where the last breath stopped it the canvas would keep a
        // colour the setting says it no longer wears.
        self.apply_gradient();
        // A manual family pick in Settings stops rotation; otherwise a live
        // rotation keeps its current pick on top of the re-applied config.
        if self.config.font_family != old_family {
            // Say so. This used to flip the flag silently, and the natural
            // reaction to a rotated pick you dislike — pinning your own font
            // back — is exactly what lands here, so rotation died without a
            // word and looked like "/font random only works once".
            let was_rotating = self.font_rotate.on;
            self.font_rotate.on = false;
            self.font_rotate.current = None;
            self.config.font_random = false;
            if was_rotating {
                let fam = self
                    .config
                    .font_family
                    .clone()
                    .unwrap_or_else(|| "system monospace".to_string());
                self.set_status(format!(
                    "font pinned: {fam} — rotation off (/font random to resume)"
                ));
            }
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

    /// `/theme [dark|light|crt|auto]`: switch the active theme live, persist
    /// the choice, and repaint. Each name enters a rotation over its palette
    /// pool (`auto`'s pool follows the OS appearance). Legacy names
    /// (`random-*` and the individual palette names) still resolve for
    /// back-compat. With no/unknown arg, report the current selection.
    pub(crate) fn set_theme_cmd(&mut self, arg: &str) {
        let arg = arg.trim();
        if arg.is_empty() {
            self.set_status(crate::themereport::live_report());
            return;
        }
        let Some(sel) = crew_theme::parse_selection(arg) else {
            let names = crew_theme::THEME_MODES
                .iter()
                .map(|m| m.as_str())
                .collect::<Vec<_>>()
                .join(" | ");
            // ERROR level, so it also steps onto the canvas as a toast: a
            // name this build doesn't know changes nothing on screen, and as
            // a three-second flash on the input bar's border the reason was
            // routinely missed — "/theme modern-light" on a build predating
            // that theme looked precisely like a theme that does nothing.
            self.set_status_err(format!("unknown theme '{arg}' ({names})"));
            return;
        };
        crew_theme::apply_selection(sel, crate::chattime::unix_now_ms());
        self.config.theme = Some(sel.label().to_string());
        // Re-apply the accent default (it follows the theme when the user hasn't
        // set an explicit accent).
        crate::palette::set_accent(self.config.accent_rgb());
        // Choosing a theme is a statement of intent: stale `/crt` pins and a
        // glass `off` from some earlier experiment stop overriding it.
        if self.config.reset_look_overrides() {
            self.apply_glass();
        }
        self.config.save();
        self.redraw();
        // Switching TO auto reports which half it just handed you, not the
        // bare word "auto" — same reason as the no-arg branch above.
        self.set_status(crate::themereport::live_report());
    }
}

#[cfg(test)]
#[path = "spawn_tests.rs"]
mod tests;
