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
    /// Spawn a new persistent pane running the line, in the given shape.
    Spawn(Shape),
}

/// How a spawned pane runs the line. A bare line is a shell command and the
/// shell is the judge — crew's own PATH check only picks which shell shape
/// can carry it, it never turns a line away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shape {
    /// A binary crew resolved: the job-control wrapper (`set -m; body;
    /// exec $SHELL`), whose busy detection is exact.
    Wrapped,
    /// A builtin, or a word crew could not resolve (an alias, a function, a
    /// keyword, a typo): the user's interactive shell, with the line typed as
    /// its first input, so rc-defined names are in scope and what a builtin
    /// sets survives into the prompt that follows.
    Interactive,
}

/// The shape a verdict calls for.
pub(crate) fn shape_for(verdict: &Verdict) -> Shape {
    match verdict {
        Verdict::Executable(_) => Shape::Wrapped,
        Verdict::Builtin(_) | Verdict::No => Shape::Interactive,
    }
}

/// Focused-shell-first: an idle shell receives anything (it is the judge of
/// what the text means); everything else spawns, shaped by the first word.
/// No line ends in a hint: the pane a mistyped word opens is a live shell
/// showing `command not found`, which is what a terminal does.
pub(crate) fn route_bare(target: Target, verdict: &Verdict) -> BareRoute {
    if let Target::IdleShell(i) = target {
        return BareRoute::TypeInto(i);
    }
    BareRoute::Spawn(shape_for(verdict))
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
            return row(
                format!("↵ broadcast to {}", crate::wording::count(n, "terminal")),
                "",
                true,
            );
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
        let verdict = self.check_command(&text);
        match route_bare(self.focused_target(), &verdict) {
            BareRoute::TypeInto(i) => {
                let title = self
                    .panes
                    .get(i)
                    .map(|p| p.title_text())
                    .unwrap_or_default();
                row(format!("↵ type into pane {} · {title}", i + 1), "", true)
            }
            // The dim column says what crew's check made of the first word;
            // Enter runs the line either way, so the row always submits.
            BareRoute::Spawn(_) => row(
                "↵ run — new pane".to_string(),
                match &verdict {
                    Verdict::Executable(_) => "",
                    Verdict::Builtin(_) => "shell builtin",
                    Verdict::No => "not on PATH",
                },
                true,
            ),
        }
    }
}
