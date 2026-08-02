//! `/doctor`'s probing half: everything that touches the live environment
//! (PATH, git, disk, the session's MCP host and counters). Split from
//! `doctor` so the render stays pure and the file under the line cap.
use std::path::Path;

use super::doctor::DoctorInputs;

/// Is `bin` an executable on the `:`-separated `path`?
pub(crate) fn on_path(bin: &str, path: &str) -> bool {
    path.split(':').filter(|d| !d.is_empty()).any(|d| {
        let p = Path::new(d).join(bin);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::metadata(&p)
                .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
                .unwrap_or(false)
        }
        #[cfg(not(unix))]
        {
            p.is_file()
        }
    })
}

/// Probe the live environment for the report. `session` supplies the counters
/// that only it knows (turns, tokens); everything else is probed here.
pub(crate) fn gather(session: &super::session::Session) -> DoctorInputs {
    use std::sync::atomic::Ordering;
    let path = std::env::var("PATH").unwrap_or_default();
    let active = super::discover::resolved_provider();
    DoctorInputs {
        // The same resolution the roster is built from, so `/doctor` reports a
        // stored key's provider rather than only an exported one's.
        provider: active.map(|p| p.name().to_string()),
        others: super::discover::configured_providers()
            .into_iter()
            .filter(|n| Some(n.as_str()) != active.map(|p| p.name()))
            .collect(),
        clis: ["claude", "codex", "opencode"]
            .iter()
            .map(|b| (b.to_string(), on_path(b, &path)))
            .collect(),
        bash: Path::new("/bin/bash").exists(),
        git: std::process::Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .output()
            .is_ok_and(|o| o.status.success()),
        skills: super::skills::load().len(),
        plugin_agents: super::plugins::load().len(),
        mcp_servers: crate::mcp::config::load().len(),
        mcp_detail: if crate::mcp::config::load().is_empty() {
            Vec::new()
        } else {
            session
                .lock_mcp()
                .report()
                .lines()
                .map(String::from)
                .collect()
        },
        memory: super::memory::load().map(|m| m.len()),
        resumable: super::sessionlog::tail().is_some(),
        sys_tools: super::systools::enabled(),
        sys_mode: super::systools::mode_label(),
        turns: session.turns.load(Ordering::Relaxed),
        tokens: session.tokens.load(Ordering::Relaxed),
        budget: super::session::token_budget(),
    }
}
