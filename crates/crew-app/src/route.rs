//! Where a bare (un-prefixed) input-bar line goes. Pure decision so every
//! row of the spec's routing table is unit-testable; `submit_input` supplies
//! the two inputs and acts on the answer.
use crate::cmdcheck::Verdict;

/// The focused pane, as routing sees it: a terminal whose shell owns the
/// prompt (idle), or anything else — busy terminal, chat/md/settings pane,
/// hidden pane, or no pane at all.
pub(crate) enum Target {
    IdleShell(usize),
    Other,
}

/// The routing decision for a bare line.
pub(crate) enum BareRoute {
    /// Type the line into the idle focused shell (pane index).
    TypeInto(usize),
    /// Spawn a new persistent pane running the line.
    Spawn,
    /// Shell builtin — a throwaway pane would discard its effect; hint.
    BuiltinHint(String),
    /// Unresolvable — hint instead of spawning a dead pane.
    UnknownHint,
}

/// Focused-shell-first: an idle shell receives anything (it is the judge of
/// what the text means); everything else routes by what the first word is.
pub(crate) fn route_bare(target: Target, verdict: &Verdict) -> BareRoute {
    if let Target::IdleShell(i) = target {
        return BareRoute::TypeInto(i);
    }
    match verdict {
        Verdict::Executable(_) => BareRoute::Spawn,
        Verdict::Builtin(b) => BareRoute::BuiltinHint(b.clone()),
        Verdict::No => BareRoute::UnknownHint,
    }
}

impl crate::app::CrewApp {
    /// The palette's live answer to "what will Enter do with this text?" —
    /// zero rows for input another surface owns (slash palette, cd ghost,
    /// empty), one row otherwise. Display-only: Enter semantics live solely
    /// in `submit_input`, this row just mirrors them (`fill` = the text, so
    /// even a stray menu-Enter is identical to a plain submit).
    pub(crate) fn input_preview(&mut self) -> Vec<crate::suggest::MenuItem> {
        use crate::suggest::MenuItem;
        let text = self.input.text.clone();
        // Any `/`-leading text belongs to slash dispatch — `submit_input`
        // routes it there unconditionally (run_slash_command silently no-ops
        // on unrecognized commands, it never falls through to route_bare).
        // So the preview must stay silent for ALL `/`-led text, not just what
        // the slash palette recognizes — otherwise an unrecognized slash
        // command (e.g. `/bin/echo hi`, or `/foo`) would show a submit-labeled
        // row promising a spawn/type-into that Enter will never actually do.
        if text.is_empty() || text.starts_with('/') {
            return Vec::new();
        }
        let row = |label: String, desc: &str, submit: bool| {
            vec![MenuItem {
                label,
                desc: desc.to_string(),
                fill: text.clone(),
                submit,
            }]
        };
        if let Some(cmd) = crate::app::star_command(&text) {
            if cmd.is_empty() {
                // Same as bang below: an empty payload isn't a submit — Enter
                // shows the usage hint, so the preview must match it, not a
                // broadcast row promising a spawn that will never happen.
                return row(
                    "usage: *<text> — sends to every terminal".to_string(),
                    "",
                    false,
                );
            }
            let n = self
                .panes
                .iter()
                .filter(|p| matches!(p.content, crate::pane::PaneContent::Terminal(_)))
                .count();
            return row(format!("↵ broadcast to {n} terminals"), "", true);
        }
        if let Some(cmd) = crate::app::bang_command(&text) {
            if cmd.is_empty() {
                // Bare `!` submits to a usage hint (see app.rs submit_input),
                // not a spawn — the preview must mirror that, not show a
                // submit-labeled row Enter will never honor.
                return row("usage: !<command>".to_string(), "", false);
            }
            return row("↵ run in a new pane (forced)".to_string(), "", true);
        }
        if crate::cwd::cd_arg(&text).is_some() {
            return Vec::new();
        }
        match route_bare(self.focused_target(), &self.check_command(&text)) {
            BareRoute::TypeInto(i) => {
                let title = self
                    .panes
                    .get(i)
                    .map(|p| p.title_text())
                    .unwrap_or_default();
                row(format!("↵ type into pane {} · {title}", i + 1), "", true)
            }
            BareRoute::Spawn => row("↵ run — new pane".to_string(), "", true),
            BareRoute::BuiltinHint(b) => row(
                format!("{b} is a shell builtin — run it inside a shell pane"),
                "",
                false,
            ),
            BareRoute::UnknownHint => row(
                "not a command — !… runs it in a pane anyway".to_string(),
                "",
                false,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmdcheck::Verdict;

    #[test]
    fn idle_shell_wins_over_everything() {
        // Even a resolvable command goes INTO an idle focused shell.
        let r = route_bare(Target::IdleShell(2), &Verdict::Executable("ls".into()));
        assert!(matches!(r, BareRoute::TypeInto(2)));
        // …and so does prose: the shell is the judge of what it means.
        let r = route_bare(Target::IdleShell(0), &Verdict::No);
        assert!(matches!(r, BareRoute::TypeInto(0)));
    }

    #[test]
    fn busy_or_nonterminal_focus_diverts_by_verdict() {
        assert!(matches!(
            route_bare(Target::Other, &Verdict::Executable("claude".into())),
            BareRoute::Spawn
        ));
        assert!(matches!(
            route_bare(Target::Other, &Verdict::Builtin("export".into())),
            BareRoute::BuiltinHint(b) if b == "export"
        ));
        assert!(matches!(
            route_bare(Target::Other, &Verdict::No),
            BareRoute::UnknownHint
        ));
    }

    /// A Far pane, just enough to seed `CrewApp::panes` for preview tests
    /// (mirrors the identically-named private helpers in panemanage.rs,
    /// navcard.rs, and app_tests.rs — no shared helper exists yet, so each
    /// test module keeps its own).
    fn far_pane(name: &str) -> crate::pane::Pane {
        use crate::farpane::FarPane;
        use crate::pane::{Pane, PaneContent};
        use crew_term::GridSize;
        Pane {
            content: PaneContent::Far(FarPane::new(std::env::temp_dir())),
            grid: GridSize { cols: 80, rows: 24 },
            rect: crate::layout::Rect {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
            },
            label: None,
            name: Some(name.to_string()),
            dir: None,
            activity: false,
            bell: false,
            hidden: false,
        }
    }

    #[test]
    fn preview_labels_spawn_and_hint_rows() {
        let mut app = crate::app::CrewApp::default();
        app.panes.push(far_pane("files"));
        app.focused = 0;
        // Resolvable → a submit row naming the new pane destination.
        app.input.text = "ls".into();
        let rows = app.input_preview();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].label.contains("new pane"), "got: {}", rows[0].label);
        assert!(rows[0].submit);
        // Unresolvable → a dim non-submit hint row.
        app.input.text = "definitely-not-a-command-xyz".into();
        let rows = app.input_preview();
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].submit);
        assert!(
            rows[0].label.contains("not a command"),
            "got: {}",
            rows[0].label
        );
    }

    #[test]
    fn preview_is_silent_for_slash_cd_and_empty() {
        let mut app = crate::app::CrewApp::default();
        app.input.text = "/theme".into();
        assert!(app.input_preview().is_empty(), "slash palette owns / input");
        app.input.text = "cd ~/code".into();
        assert!(
            app.input_preview().is_empty(),
            "cd keeps its ghost, no card"
        );
        app.input.text = String::new();
        assert!(app.input_preview().is_empty());
    }

    #[test]
    fn preview_is_silent_for_unrecognized_slash_command() {
        // `/definitely-not-a-palette-cmd` matches no slash-palette row, but
        // `submit_input` still routes it to slash dispatch (which silently
        // no-ops) — never to route_bare. The preview must not show a
        // submit-labeled spawn/type-into row Enter will never honor.
        let mut app = crate::app::CrewApp::default();
        app.input.text = "/definitely-not-a-palette-cmd".into();
        assert!(
            app.input_preview().is_empty(),
            "slash dispatch owns all /-led text, even unrecognized commands"
        );
    }

    #[test]
    fn preview_counts_broadcast_targets() {
        let mut app = crate::app::CrewApp::default();
        app.input.text = "* echo hi".into();
        let rows = app.input_preview();
        assert_eq!(rows.len(), 1);
        assert!(
            rows[0].label.contains("0 terminals"),
            "got: {}",
            rows[0].label
        );
    }

    #[test]
    fn bare_prefixes_show_usage_hint_not_submit_row() {
        // Bare `!` and `*` (empty payload) submit to a usage-hint status in
        // submit_input, not a spawn/broadcast — the preview must mirror that
        // with a single non-submit row, not a submit-labeled one Enter will
        // never honor.
        let mut app = crate::app::CrewApp::default();
        app.input.text = "!".into();
        let rows = app.input_preview();
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].submit);
        assert_eq!(rows[0].label, "usage: !<command>");

        app.input.text = "*".into();
        let rows = app.input_preview();
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].submit);
        assert_eq!(rows[0].label, "usage: *<text> — sends to every terminal");
    }
}
