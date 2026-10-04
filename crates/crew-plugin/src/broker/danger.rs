//! The few shell commands even the default mode stops to ask about.
//!
//! In auto-approve every tool call runs: the person who typed the task is the
//! approval, and the task's checkpoint is the undo. A handful of commands are
//! outside what a checkpoint can put back — they delete outside the project,
//! rewrite a remote's history, run with root, pipe the internet into a shell,
//! publish, or wipe a disk — and those ask first. YOLO asks about nothing,
//! these included.
//!
//! Precision over recall, on purpose: a prompt on `rm -rf target` teaches
//! people to press Enter without reading, which is worse than no prompt. Only
//! shapes that are dangerous wherever they run are listed.

/// Why `cmd` should ask before it runs even in auto-approve — `None` for an
/// ordinary command. The reason finishes the sentence "… — it …".
pub(crate) fn of(cmd: &str) -> Option<&'static str> {
    let lower = cmd.to_ascii_lowercase();
    // Each `;`, `&&`, `||` and `|` piece is a command of its own: `cd x && rm -rf /`.
    let pieces: Vec<Vec<&str>> = lower
        .split(['\n', ';', '&', '|'])
        .map(|p| p.split_whitespace().collect())
        .filter(|w: &Vec<&str>| !w.is_empty())
        .collect();
    if pipes_into_shell(&lower) {
        return Some("runs a script straight from the internet");
    }
    pieces.iter().find_map(|w| piece(w))
}

fn piece(w: &[&str]) -> Option<&'static str> {
    // `env X=1 sudo …`, `time rm …`: judge the command, not its wrapper.
    let w: Vec<&str> = w
        .iter()
        .copied()
        .skip_while(|t| t.contains('=') || matches!(*t, "env" | "time" | "nohup" | "command"))
        .collect();
    let has = |f: &str| w.iter().any(|t| *t == f);
    match w.first().copied()? {
        "sudo" | "doas" => Some("runs as root"),
        "rm" if recursive_force(&w) && w.iter().skip(1).any(|t| outside(t)) => {
            Some("deletes recursively outside the project")
        }
        "rm" if recursive_force(&w)
            && w.iter().skip(1).any(|t| matches!(*t, "." | "*" | "./*")) =>
        {
            Some("deletes the whole project")
        }
        "git" => match w.get(1).copied()? {
            "push" if w.iter().any(|t| t.starts_with("--force") || *t == "-f") => {
                Some("force-pushes, rewriting the remote's history")
            }
            "push"
                if w.iter()
                    .any(|t| t.starts_with("--delete") || t.starts_with(':')) =>
            {
                Some("deletes a branch on the remote")
            }
            "reset" if has("--hard") => Some("throws away uncommitted work"),
            "clean" if w.iter().any(|t| t.starts_with('-') && t.contains('f')) => {
                Some("deletes untracked files")
            }
            _ => None,
        },
        "mkfs" | "fdisk" | "diskutil" if !has("list") => Some("can wipe a disk"),
        "dd" if w.iter().any(|t| t.starts_with("of=/dev/")) => Some("writes straight to a disk"),
        "shutdown" | "reboot" | "halt" | "poweroff" => Some("turns the machine off"),
        "chmod" | "chown" if has("-r") && w.iter().any(|t| outside(t)) => {
            Some("changes permissions outside the project")
        }
        "npm" | "cargo" | "yarn" | "pnpm" | "twine" | "gem" if has("publish") => {
            Some("publishes a package")
        }
        "terraform" if has("destroy") => Some("destroys infrastructure"),
        "kubectl" if has("delete") => Some("deletes from a cluster"),
        _ if w.windows(2).any(|p| {
            let q = |t: &str| t.trim_matches(['"', '\'']).to_string();
            q(p[0]) == "drop" && matches!(q(p[1]).as_str(), "database" | "table")
        }) =>
        {
            Some("drops a database table")
        }
        _ => None,
    }
}

/// `rm`'s flags ask for both recursion and force (`-rf`, `-fr`, `-Rf`,
/// `-r -f`, `--recursive --force`).
fn recursive_force(w: &[&str]) -> bool {
    let flags: String = w
        .iter()
        .filter(|t| t.starts_with('-') && !t.starts_with("--"))
        .map(|t| &t[1..])
        .collect();
    let long = |f: &str| w.iter().any(|t| *t == f);
    (flags.contains('r') || long("--recursive")) && (flags.contains('f') || long("--force"))
}

/// A target that is not inside the project: the root, home or the parent.
fn outside(t: &str) -> bool {
    let t = t.trim_matches(['"', '\'']);
    t.starts_with('/') || t.starts_with('~') || t.starts_with("$home") || t.starts_with("..")
}

/// `curl … | sh`, `wget -O- … | bash`: whatever the server sends, run.
fn pipes_into_shell(lower: &str) -> bool {
    let fetches = lower.contains("curl ") || lower.contains("wget ");
    let shell = ["| sh", "|sh", "| bash", "|bash", "| zsh", "|zsh"];
    fetches && shell.iter().any(|s| lower.contains(s))
}

#[cfg(test)]
#[path = "danger_tests.rs"]
mod tests;
