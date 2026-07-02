//! Slash constructs typed into the `/crew` pane. Anything starting with `/`
//! is a broker command rather than a task: `/help` lists the constructs,
//! `/agents` reports the roster with each agent's current model.
use crate::PluginEvent;

use super::relay::msg;
use super::session::Session;

/// Whether `text` addresses the broker's command router.
pub(crate) fn is_command(text: &str) -> bool {
    text.trim_start().starts_with('/')
}

/// One-line summaries of every construct, shown by `/help`.
pub(crate) const HELP: &str = "constructs:\n\
    /help — this list\n\
    /agents — the roster with each agent's model\n\
    @<agent> <task> — choose who starts the relay";

/// Handle a `/command` line; emits reply events through `emit`.
pub(crate) fn handle(
    session: &mut Session,
    text: &str,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let line = text.trim().trim_start_matches('/');
    let (cmd, _rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    match cmd {
        "help" => emit(msg("crew", HELP)),
        "agents" => emit(msg("crew", agents_report(session))),
        other => emit(msg(
            "crew",
            format!("unknown construct /{other} — try /help"),
        )),
    }
}

/// The roster, one agent per line: name, role hint, and the model it runs.
fn agents_report(session: &Session) -> String {
    let reg = session.registry();
    if reg.is_empty() {
        return super::stdio::roster(&reg);
    }
    let lines: Vec<String> = reg
        .infos()
        .iter()
        .map(|a| {
            let model = if a.model.is_empty() {
                "(own model)".to_string()
            } else {
                a.model.clone()
            };
            format!("\u{25aa} {} \u{2014} {} \u{2014} {model}", a.name, a.role)
        })
        .collect();
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(text: &str) -> Vec<PluginEvent> {
        let mut session = Session::new();
        let mut out = Vec::new();
        handle(&mut session, text, &mut |ev| {
            out.push(ev);
            Ok(())
        })
        .unwrap();
        out
    }

    fn text_of(ev: &PluginEvent) -> &str {
        match ev {
            PluginEvent::Message { text, .. } => text,
            _ => "",
        }
    }

    #[test]
    fn detects_commands() {
        assert!(is_command("/help"));
        assert!(is_command("  /agents"));
        assert!(!is_command("do the thing"));
        assert!(!is_command("@planner go"));
    }

    #[test]
    fn help_lists_constructs() {
        let evs = run("/help");
        assert_eq!(evs.len(), 1);
        let t = text_of(&evs[0]);
        assert!(t.contains("/agents"), "{t}");
    }

    #[test]
    fn unknown_command_points_at_help() {
        let evs = run("/frobnicate now");
        let t = text_of(&evs[0]);
        assert!(t.contains("unknown construct /frobnicate"), "{t}");
        assert!(t.contains("/help"), "{t}");
    }

    #[test]
    fn agents_reports_roster_or_keys_hint() {
        // In tests no API key is guaranteed; either a roster line or the
        // no-agents hint is acceptable — both are a Message.
        let evs = run("/agents");
        assert_eq!(evs.len(), 1);
        assert!(!text_of(&evs[0]).is_empty());
    }
}
