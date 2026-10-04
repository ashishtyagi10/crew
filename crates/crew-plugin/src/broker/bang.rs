//! `!cmd` — a shell command the person runs from the agent composer, as in
//! Claude Code's bash mode. They typed it, so no approval gate asks (the
//! read-only `CREW_SYS_MODE` still holds); it runs on a thread of its own,
//! bounded like `sys:run`, so the pane can still be answered; and its output
//! is shown and kept in the thread, so the next message can say "fix that".
use std::sync::Arc;

use crate::PluginEvent;

/// Who the output is from, on its card.
pub(crate) const SENDER: &str = "shell";

/// The command in a `!cmd` line, or `None` — a bare `!` is a message.
pub(crate) fn command(line: &str) -> Option<&str> {
    line.strip_prefix('!')
        .map(str::trim)
        .filter(|c| !c.is_empty())
}

/// Run `cmd` on a thread of its own; its card goes out through `emit`, and the
/// exchange joins `thread`.
pub(crate) fn spawn(
    cmd: String,
    thread: super::thread::SharedThread,
    emit: Arc<dyn Fn(PluginEvent) + Send + Sync>,
) {
    std::thread::spawn(move || {
        let output = match super::systools::read_only() {
            true => {
                Err("blocked \u{2014} CREW_SYS_MODE=readonly (CREW_SYS_MODE=full allows it)".into())
            }
            false => super::sysrun::run(&cmd),
        };
        let (card, kept) = card(&cmd, output);
        super::thread::lock(&thread).record(&format!("!{cmd}"), &kept);
        emit(super::relay::msg(SENDER, card));
    });
}

/// The card the pane shows, and what the thread keeps of it.
fn card(cmd: &str, output: Result<String, String>) -> (String, String) {
    let body = match output {
        Ok(text) if text.trim().is_empty() => "(no output)".to_string(),
        Ok(text) => text,
        Err(why) => format!("did not run: {why}"),
    };
    let fence = match body.contains("```") {
        true => "~~~",
        false => "```",
    };
    (
        format!("$ {cmd}\n{fence}\n{}\n{fence}", body.trim_end()),
        body,
    )
}

#[cfg(test)]
#[path = "bang_tests.rs"]
mod tests;
