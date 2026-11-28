//! "open a PR", the draft half: the coder reads the branch's commits and diff
//! against the remote's default branch and writes a title and body. Nothing
//! is pushed or opened — that waits for the user's exact "apply"
//! (`gitprapply`). The agent call is the one `gitmsg` makes for a commit.
use crate::PluginEvent;

use super::gitpr::{hold_pr, PendingPr};
use super::gitprcheck::{free_branch, range, ready};
use super::relay::msg;
use super::session::{call_timeout, Session};
use super::stdio::roster;

/// Diff budget interpolated into the prompt (chars), as `gitmsg`'s.
const DIFF_CAP: usize = 12_000;

/// The one-completion prompt for the coder.
pub(crate) fn pr_prompt(base: &str, log: &str, diff: &str) -> String {
    let mut d = diff.to_string();
    if d.len() > DIFF_CAP {
        let mut cut = DIFF_CAP;
        while !d.is_char_boundary(cut) {
            cut -= 1;
        }
        d.truncate(cut);
        d.push_str("\n… (diff clipped)");
    }
    format!(
        "Write a GitHub pull request for these commits onto `{base}`. First \
         line: the title, an imperative Conventional Commits `type(scope): \
         subject` (≤72 chars). Then a blank line and a short Markdown body: \
         what changed and why, and how it was verified ONLY if the commits \
         say so. State nothing the commits and diff do not show.\n\
         Reply with the title and body only: no prose around them, no code \
         fences.\n\nCOMMITS:\n{log}\n\nDIFF:\n{d}"
    )
}

/// Split a reply into `(title, body)`: fences and a leading "title:" label
/// are shed; the first line is the title.
pub(crate) fn parse(reply: &str) -> (String, String) {
    let clean = super::gitmsg::clean_message(reply);
    let (first, rest) = clean.split_once('\n').unwrap_or((clean.as_str(), ""));
    let first = first.trim().trim_start_matches('#').trim();
    let lower = first.to_lowercase();
    let title = match lower.starts_with("title:") {
        true => first[6..].trim(),
        false => first,
    };
    (title.trim_matches('`').to_string(), rest.trim().to_string())
}

/// `<type>/<slug>` from a Conventional Commits title.
pub(crate) fn branch_name(title: &str) -> String {
    let (kind, subject) = match title.split_once(':') {
        Some((head, s)) if !head.contains(' ') => {
            let k: String = head
                .chars()
                .take_while(|c| c.is_ascii_alphabetic())
                .collect();
            (k.to_lowercase(), s)
        }
        _ => (String::new(), title),
    };
    let kind = if kind.is_empty() { "feat".into() } else { kind };
    let mut slug = String::new();
    for w in subject
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
    {
        if slug.len() + w.len() > 40 {
            break;
        }
        if !slug.is_empty() {
            slug.push('-');
        }
        slug.push_str(w);
    }
    if slug.is_empty() {
        slug = "changes".into();
    }
    format!("{kind}/{slug}")
}

/// The exact commands apply will run, for the draft to show.
pub(crate) fn commands(p: &PendingPr) -> String {
    let mut c = String::new();
    if p.create_branch {
        c.push_str(&format!("git checkout -b {}\n", p.branch));
    }
    c.push_str(&format!(
        "git push -u origin {b}\ngh pr create --base {base} --head {b} --title {t:?} --body-file <the body above>",
        b = p.branch,
        base = p.base,
        t = p.title
    ));
    c
}

/// Draft the pull request for the current branch.
pub(crate) fn draft(
    session: &mut Session,
    emit: &mut dyn FnMut(PluginEvent) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let say = |t: String| msg("agent smith", t);
    let dir = match super::gitmsg::project_dir() {
        Ok(d) => d,
        Err(e) => return emit(say(format!("pull request: {e}"))),
    };
    let r = match ready(&dir) {
        Ok(r) => r,
        Err(e) => return emit(say(format!("pull request: {e}"))),
    };
    let (log, diff) = match range(&dir, &r.base_ref) {
        Ok(x) => x,
        Err(e) => return emit(say(format!("pull request: {e}"))),
    };
    let reg = session.registry();
    if reg.is_empty() {
        return emit(say(roster(&reg)));
    }
    let author = super::elect::elect(
        "write a pull request title and description from commits",
        &reg.infos(),
        None,
    );
    emit(say(format!(
        "drafting a pull request: {} → {}…",
        r.branch, r.base
    )))?;
    emit(PluginEvent::Activity {
        agent: author.clone(),
        state: "thinking".into(),
        from: "pr".into(),
    })?;
    let reply = reg
        .get(&author)
        .map(|a| a.call(&pr_prompt(&r.base, &log, &diff), call_timeout()));
    emit(PluginEvent::Activity {
        agent: String::new(),
        state: "idle".into(),
        from: String::new(),
    })?;
    let (title, body) = match reply {
        Some(Ok(t)) => parse(&t),
        Some(Err(e)) => return emit(say(format!("pull request draft failed: {e}"))),
        None => return emit(say("pull request stopped — the coder went missing".into())),
    };
    if title.is_empty() {
        return emit(say("the draft came back empty — try again".into()));
    }
    let create_branch = r.branch == r.base;
    let branch = match create_branch {
        true => free_branch(&dir, &branch_name(&title)),
        false => r.branch.clone(),
    };
    let p = PendingPr {
        branch,
        create_branch,
        base: r.base,
        title,
        body,
    };
    emit(msg(
        &format!("{author} → user"),
        format!(
            "{} → {}\n\ntitle: {}\n\n{}\n\nwill run:\n{}",
            p.branch,
            p.base,
            p.title,
            p.body,
            commands(&p)
        ),
    ))?;
    let note = match create_branch {
        true => format!(
            " (you are on {}, so a new branch is created and {} is not pushed)",
            p.base, p.base
        ),
        false => String::new(),
    };
    hold_pr(session, p);
    emit(say(format!(
        "proposal ready{note} — say \u{201c}apply\u{201d} to open it, or ask again to re-draft"
    )))
}
