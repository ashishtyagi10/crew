//! The GitHub half, through the `gh` CLI: its login is the user's, so crew
//! holds no token. Every call is a short blocking process on the sync's own
//! thread. A Dock-launched crew has launchd's PATH, so the login shell's
//! PATH is handed down, with Homebrew's folders after it in case the shell
//! probe has not run yet.
use std::process::Command;

use crew_hive::childproc::no_console_window;

use super::plan::Remote;

fn gh(args: &[&str]) -> Result<String, String> {
    let probed = crate::cmdcheck::effective_path();
    let dirs = std::env::split_paths(&probed)
        .chain(["/opt/homebrew/bin", "/usr/local/bin"].map(Into::into));
    let path = std::env::join_paths(dirs).unwrap_or_else(|_| probed.into());
    let out = no_console_window(&mut Command::new("gh"))
        .args(args)
        .env("PATH", path)
        .env("GH_PROMPT_DISABLED", "1")
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => {
                "the gh CLI is not installed (brew install gh, then gh auth login)".to_string()
            }
            _ => format!("gh would not start: {e}"),
        })?;
    if out.status.success() {
        return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    Err(err.lines().next().unwrap_or("gh failed").trim().to_string())
}

/// File an issue; its number and URL.
pub(crate) fn create(repo: &str, title: &str, body: &str) -> Result<(u64, String), String> {
    let url = gh(&[
        "issue", "create", "-R", repo, "--title", title, "--body", body,
    ])?;
    let url = url.lines().last().unwrap_or_default().trim().to_string();
    number_of(&url)
        .map(|n| (n, url.clone()))
        .ok_or_else(|| format!("gh filed an issue but said {url:?}"))
}

/// The issue number at the end of an issue URL.
pub(crate) fn number_of(url: &str) -> Option<u64> {
    let (head, n) = url.rsplit_once('/')?;
    head.ends_with("/issues").then(|| n.parse().ok()).flatten()
}

/// The issue as GitHub has it now.
pub(crate) fn view(repo: &str, number: u64) -> Result<Remote, String> {
    let n = number.to_string();
    let json = gh(&["issue", "view", &n, "-R", repo, "--json", "state,title"])?;
    parse_view(&json).ok_or_else(|| format!("gh said {json:?} for {repo}#{n}"))
}

pub(crate) fn parse_view(json: &str) -> Option<Remote> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    Some(Remote {
        title: v.get("title")?.as_str()?.to_string(),
        open: v.get("state")?.as_str()? == "OPEN",
    })
}

pub(crate) fn retitle(repo: &str, number: u64, title: &str) -> Result<(), String> {
    gh(&[
        "issue",
        "edit",
        &number.to_string(),
        "-R",
        repo,
        "--title",
        title,
    ])
    .map(drop)
}

pub(crate) fn set_open(repo: &str, number: u64, open: bool) -> Result<(), String> {
    let verb = if open { "reopen" } else { "close" };
    gh(&["issue", verb, &number.to_string(), "-R", repo]).map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_number_is_read_off_the_issue_url() {
        assert_eq!(number_of("https://github.com/me/crew/issues/42"), Some(42));
        assert_eq!(number_of("https://github.com/me/crew/pull/42"), None);
        assert_eq!(number_of("nonsense"), None);
    }

    #[test]
    fn a_view_reads_title_and_state() {
        let r = parse_view(r#"{"state":"CLOSED","title":"fix it"}"#).unwrap();
        assert_eq!((r.title.as_str(), r.open), ("fix it", false));
        assert!(parse_view(r#"{"state":"OPEN","title":"x"}"#).unwrap().open);
        assert_eq!(parse_view("{}"), None);
    }
}
