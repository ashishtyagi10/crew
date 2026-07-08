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
}
