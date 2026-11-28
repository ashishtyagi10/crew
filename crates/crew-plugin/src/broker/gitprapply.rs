//! "open a PR", the apply half: reached only by the exact confirm word in
//! `intent::gate`, never by a model. Re-checks what the draft checked, makes
//! the branch if one was proposed, pushes it (never `--force`, never the base
//! branch) and opens the pull request with `gh`.
use crate::PluginEvent;

use super::gitpr::{gh, git, PendingPr, SharedPr};
use super::gitprcheck::ready;
use super::relay::msg;
use super::session::Session;

fn take(pr: &SharedPr) -> Option<PendingPr> {
    pr.lock().unwrap_or_else(|e| e.into_inner()).take()
}

/// Run the pending PR; the result is the one line to report.
fn open(p: &PendingPr) -> Result<String, String> {
    let dir = super::gitmsg::project_dir()?;
    let r = ready(&dir)?;
    if p.branch == p.base {
        return Err(format!("refusing to push the base branch {}", p.base));
    }
    match p.create_branch {
        true if r.branch != r.base => {
            return Err(format!(
                "the branch changed to {} since the draft — ask again",
                r.branch
            ))
        }
        false if r.branch != p.branch => {
            return Err(format!(
                "the branch changed to {} since the draft — ask again",
                r.branch
            ))
        }
        true => {
            git(&dir, &["checkout", "-b", &p.branch]).map_err(|e| format!("branch failed: {e}"))?;
        }
        false => {}
    }
    git(&dir, &["push", "-u", "origin", &p.branch]).map_err(|e| format!("push failed: {e}"))?;
    let file = std::env::temp_dir().join(format!(
        "crew-pr-{}-{}.md",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::write(&file, &p.body).map_err(|e| format!("body file: {e}"))?;
    let out = gh(
        &dir,
        &[
            "pr",
            "create",
            "--base",
            &p.base,
            "--head",
            &p.branch,
            "--title",
            &p.title,
            "--body-file",
            &file.to_string_lossy(),
        ],
    );
    let _ = std::fs::remove_file(&file);
    let out = out.map_err(|e| format!("gh pr create failed: {e}"))?;
    let url = out
        .lines()
        .rev()
        .find(|l| l.contains("://"))
        .unwrap_or("opened");
    Ok(format!("opened: {}", url.trim()))
}

/// Open the pending pull request.
pub(crate) fn apply(
    session: &mut Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let Some(p) = take(&session.pr) else {
        return emit(msg(
            "agent smith",
            "no proposal — ask me to open a PR first",
        ));
    };
    let text = open(&p).unwrap_or_else(|e| format!("pull request: {e}"));
    emit(msg("agent smith", text))
}
