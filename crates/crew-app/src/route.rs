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
                header: false,
                dim: false,
                needs: None,
                color: None,
                ..Default::default()
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
        if crate::askbar::explain_command(&text).is_some() {
            // `??` with or without a question submits (a default question
            // stands in), so the preview always shows a submit row.
            return row("↵ explain this pane's output with ai".to_string(), "", true);
        }
        if let Some(query) = crate::askbar::qmark_command(&text) {
            if query.is_empty() {
                // Bare `?` submits to a usage hint, mirroring `!` and `*`.
                return row(
                    "usage: ?<what you want> — ask ai for a command".to_string(),
                    "",
                    false,
                );
            }
            return row("↵ ask ai for a command".to_string(), "", true);
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
#[path = "route_tests.rs"]
mod tests;
