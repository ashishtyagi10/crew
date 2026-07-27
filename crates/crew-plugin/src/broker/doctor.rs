//! `/doctor` — a Claude Code-style health check for the AI stack: which
//! provider will answer, which agent CLIs are installed, whether MCP / skills
//! / plugin agents / memory / a resumable session are loaded, and whether the
//! git and bash prerequisites hold. Pure rendering over a [`DoctorInputs`]
//! snapshot, so every line is unit-testable; `gather` does the real probing.
use std::path::Path;

/// Everything the report renders, gathered up front.
pub(crate) struct DoctorInputs {
    /// The provider that will back the inbuilt agents, if any.
    pub provider: Option<String>,
    /// Which of the known agent CLIs (claude, codex, opencode) are on PATH.
    pub clis: Vec<(String, bool)>,
    /// `/bin/bash` present (run panes' job-control wrapper needs it).
    pub bash: bool,
    /// Inside a git work tree (checkpoints, /commit, /review need it).
    pub git: bool,
    pub skills: usize,
    pub plugin_agents: usize,
    pub mcp_servers: usize,
    /// Bytes of standing memory loaded, if any.
    pub memory: Option<usize>,
    /// A previous session is resumable.
    pub resumable: bool,
    pub sys_tools: bool,
    pub sys_mode: &'static str,
    /// Turns taken and approximate tokens spent this session, plus the relay
    /// token budget. Inherited from the deleted `/status`: everything else it
    /// reported is visible in the pane footer, but these three were nowhere
    /// else, and a construct's deletion must not take the only copy of its
    /// information with it.
    pub turns: u64,
    pub tokens: u64,
    pub budget: usize,
}

/// One report line: `✓` when healthy, `✗` when broken, `–` for "absent but
/// fine".
fn line(mark: char, label: &str, detail: &str) -> String {
    if detail.is_empty() {
        format!("{mark} {label}")
    } else {
        format!("{mark} {label}: {detail}")
    }
}

/// Render the full report.
pub(crate) fn render(i: &DoctorInputs) -> String {
    let mut out = vec!["crew doctor — the AI stack at a glance".to_string()];
    out.push(match &i.provider {
        Some(p) => line('✓', "provider", p),
        None => line(
            '✗',
            "provider",
            "none — sign in to claude, codex or opencode (crew finds them), \
             or set DASHSCOPE_API_KEY, OPENROUTER_API_KEY or ANTHROPIC_API_KEY \
             for the inbuilt agents",
        ),
    });
    for (name, found) in &i.clis {
        out.push(if *found {
            line('✓', name, "on PATH")
        } else {
            line(
                '–',
                name,
                "not installed (sign in to one for a keyless roster)",
            )
        });
    }
    out.push(if i.bash {
        line('✓', "bash", "/bin/bash (run panes use it for job control)")
    } else {
        line(
            '✗',
            "bash",
            "/bin/bash missing — run panes lose busy detection",
        )
    });
    out.push(if i.git {
        line('✓', "git", "inside a repository")
    } else {
        line(
            '–',
            "git",
            "not a repository — /checkpoint, /commit, /review need one",
        )
    });
    let opt = |n: usize, what: &str, hint: &str| {
        if n > 0 {
            line('✓', what, &format!("{n} loaded"))
        } else {
            line('–', what, hint)
        }
    };
    out.push(opt(
        i.skills,
        "skills",
        "none (add .md files under .crew/skills)",
    ));
    out.push(opt(
        i.plugin_agents,
        "plugin agents",
        "none (JSON manifests under .crew/agents)",
    ));
    out.push(opt(
        i.mcp_servers,
        "mcp servers",
        "none (declare in .crew/mcp.json)",
    ));
    out.push(match i.memory {
        Some(n) => line('✓', "memory", &format!("{n} bytes standing (#<note> adds)")),
        None => line('–', "memory", "none (#<note> starts one)"),
    });
    out.push(if i.resumable {
        line('✓', "resumable session", "/resume restores the last one")
    } else {
        line('–', "resumable session", "none yet")
    });
    out.push(if i.sys_tools {
        line('✓', "sys tools", i.sys_mode)
    } else {
        line('–', "sys tools", "disabled (CREW_SYS_TOOLS=0)")
    });
    // The session line `/status` used to carry. `~n/turn` only once there is a
    // turn to average over — a `0/turn` reading would be meaningless, and
    // turns == 0 would divide by zero.
    let per_turn = match i.tokens.checked_div(i.turns) {
        Some(avg) => format!(" (~{avg}/turn)"),
        None => String::new(),
    };
    let budget = if i.budget == 0 {
        "unlimited budget".to_string()
    } else {
        format!("~{} tok budget", i.budget)
    };
    out.push(line(
        '✓',
        "session",
        &format!(
            "{} turn{} · ~{} tok{per_turn} · {budget}",
            i.turns,
            if i.turns == 1 { "" } else { "s" },
            i.tokens,
        ),
    ));
    out.join("\n")
}

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
    DoctorInputs {
        // The same resolution the roster is built from, so `/doctor` reports a
        // stored key's provider rather than only an exported one's.
        provider: super::discover::resolved_provider().map(|p| p.name().to_string()),
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
        memory: super::memory::load().map(|m| m.len()),
        resumable: super::sessionlog::tail().is_some(),
        sys_tools: super::systools::enabled(),
        sys_mode: super::systools::mode_label(),
        turns: session.turns.load(Ordering::Relaxed),
        tokens: session.tokens.load(Ordering::Relaxed),
        budget: super::session::token_budget(),
    }
}

#[cfg(test)]
#[path = "doctor_tests.rs"]
mod tests;
